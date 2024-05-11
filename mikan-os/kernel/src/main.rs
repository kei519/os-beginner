#![no_std]
#![no_main]

extern crate alloc;

mod asmfunc;
mod bitfield;
mod console;
mod error;
mod font;
mod font_data;
mod frame_buffer;
mod frame_buffer_config;
mod graphics;
mod interrupt;
mod layer;
mod logger;
mod memory_manager;
mod memory_map;
mod mouse;
mod paging;
mod pci;
mod segment;
mod sync;
mod timer;
mod usb;
mod window;
mod x86_descriptor;

use alloc::{boxed::Box, collections::VecDeque};
use console::Console;
use core::{
    arch::asm,
    mem::size_of,
    panic::PanicInfo,
    sync::atomic::{AtomicU32, Ordering},
};
use frame_buffer::FrameBuffer;
use frame_buffer_config::{FrameBufferConfig, PixelFormat};
use graphics::{
    BgrResv8BitPerColorPixelWriter, PixelColor, PixelWriter, RgbResv8BitPerColorPixelWriter,
    Vector2D,
};
use interrupt::{notify_end_of_interrupt, InterruptFrame, Message};
use layer::LayerManager;
use mouse::MouseCursor;
use pci::Device;
use sync::{Mutex, OnceMutex};
use timer::stop_lapic_timer;
use uefi::table::boot::MemoryMap;

use crate::{
    asmfunc::{cli, get_cs, load_idt, set_cs_ss, set_ds_all, sti, sti_hlt},
    bitfield::BitField,
    graphics::draw_desktop,
    interrupt::{InterruptDescriptor, InterruptDescriptorAttribute, InterruptVector, MessageType},
    logger::{set_log_level, LogLevel},
    mouse::{MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH, MOUSE_TRANSPARENT_COLOR},
    usb::{Controller, HIDMouseDriver},
    window::Window,
};

/// カーネル用スタック
#[repr(align(16))]
struct KernelStack {
    _buf: [u8; STACK_SIZE],
}
impl KernelStack {
    const fn new() -> Self {
        Self {
            _buf: [0; STACK_SIZE],
        }
    }
}
#[allow(unused)]
#[no_mangle]
static KERNEL_MAIN_STACK: KernelStack = KernelStack::new();

/// ピクセル描画を担う。
static PIXEL_WRITER: OnceMutex<Box<dyn PixelWriter + Send>> = OnceMutex::new();

/// デスクトップ背景の色
const DESKTOP_BG_COLOR: PixelColor = PixelColor::new(45, 118, 237);
/// デスクトップ前景の色
const DESKTOP_FG_COLOR: PixelColor = PixelColor::new(255, 255, 255);

/// コンソール処理を担う。
static CONSOLE: OnceMutex<Console> = OnceMutex::new();

static LAYER_MANAGER: OnceMutex<LayerManager> = OnceMutex::new();

/// 本当のフレームバッファを表す `FrameBuffer`。
static SCREEN: OnceMutex<FrameBuffer> = OnceMutex::new();

static IDT: Mutex<[InterruptDescriptor; 256]> =
    Mutex::new([InterruptDescriptor::const_default(); 256]);

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            use $crate::timer;
            timer::start_lapic_timer();
            write!($crate::CONSOLE.lock(), $($arg)*).unwrap();
            let elapsed = timer::lapic_timer_elapsed();
            timer::stop_lapic_timer();
            write!($crate::CONSOLE.lock(), "[{:9}]", elapsed).unwrap();
        }
    };
}

#[macro_export]
macro_rules! printkln {
    () => ($crate::printk!("\n"));
    ($($arg:tt)*) => ($crate::printk!("{}\n", format_args!($($arg)*)));
}

static MOUSE_CURSOR: OnceMutex<MouseCursor> = OnceMutex::new();
static MOUSE_LAYER_ID: AtomicU32 = AtomicU32::new(0);

fn mouse_observer(displacement_x: i8, displacement_y: i8) {
    let elapsed = {
        let mut layer_maneger = LAYER_MANAGER.lock();
        let layer_id = MOUSE_LAYER_ID.load(Ordering::Acquire);
        layer_maneger
            .layer(layer_id)
            .move_relative(Vector2D::new(displacement_x as i32, displacement_y as i32));
        timer::start_lapic_timer();
        layer_maneger.draw();
        let elapsed = timer::lapic_timer_elapsed();
        stop_lapic_timer();
        elapsed
    };
    printkln!("mouse_obserer: elapsed = {}", elapsed);
}

fn switch_ehci2xhci(xhc_dev: &Device) {
    let mut intel_ehc_exist = false;
    let devices = pci::DEVICES.read();
    for device in &*devices {
        if device.class_code().r#match(0x0c, 0x03, 0x20) && device.read_vendor_id() == 0x8086 {
            intel_ehc_exist = true;
            break;
        }
    }
    if !intel_ehc_exist {
        return;
    }

    let superspeed_ports = xhc_dev.read_conf_reg(0xdc);
    xhc_dev.write_conf_reg(0xd8, superspeed_ports);
    let ehci2xhci_ports = xhc_dev.read_conf_reg(0xd4);
    xhc_dev.write_conf_reg(0xd0, ehci2xhci_ports);
    log!(
        LogLevel::Debug,
        "switch_ehci2xhci: SS = {:02x}, xHCI = {:02x}",
        superspeed_ports,
        ehci2xhci_ports
    );
}

static XHC: OnceMutex<Controller> = OnceMutex::new();

static MAIN_QUEUE: Mutex<VecDeque<Message>> = Mutex::new(VecDeque::new());

#[custom_attribute::interrupt]
fn int_handler_xhci(_frame: &InterruptFrame) {
    MAIN_QUEUE
        .lock()
        .push_back(Message::new(MessageType::InteruptXHCI));
    notify_end_of_interrupt();
}

// この呼び出しの前にスタック領域を変更するため、でかい構造体をそのまま渡せなくなる
// それを避けるために参照で渡す
#[custom_attribute::kernel_entry(KERNEL_MAIN_STACK, STACK_SIZE = 1024 * 1024)]
fn kernel_entry(
    frame_buffer_config: &'static FrameBufferConfig,
    memory_map: &'static MemoryMap,
    kernel_base: usize,
    kernel_size: usize,
) {
    // 参照元は今後使用される可能性のあるメモリ領域にあるため、コピーしておく
    let frame_buffer_config = frame_buffer_config.clone();

    // メモリアロケータの初期化
    memory_manager::GLOBAL.init(memory_map, kernel_base, kernel_size);

    // ピクセルライターの生成
    let pixel_writer: Box<dyn PixelWriter + Send> = match frame_buffer_config.pixel_format {
        PixelFormat::Rgb => Box::new(RgbResv8BitPerColorPixelWriter::new(frame_buffer_config)),
        PixelFormat::Bgr => Box::new(BgrResv8BitPerColorPixelWriter::new(frame_buffer_config)),
    };
    PIXEL_WRITER.init(pixel_writer);

    draw_desktop(&mut **PIXEL_WRITER.lock());

    // コンソールの生成
    CONSOLE.init(Console::new(
        &PIXEL_WRITER,
        &DESKTOP_FG_COLOR,
        &DESKTOP_BG_COLOR,
        (frame_buffer_config.vertical_resolution - 50) / 16,
        frame_buffer_config.horizontal_resolution / 8,
    ));

    // welcome 文
    printk!("Welcome to MikanOS!\n");

    // ログレベルの設定
    set_log_level(LogLevel::Warn);

    // タイマーの初期化
    timer::initialize_lapic_timer();

    // セグメントの設定
    segment::setup_segments();

    const KERNEL_CS: u16 = 1 << 3;
    const KERNEL_SS: u16 = 2 << 3;
    set_ds_all(0);
    set_cs_ss(KERNEL_CS, KERNEL_SS);

    // ページングの設定
    paging::setup_indentity_page_table();

    // マウスカーソルの生成
    MOUSE_CURSOR.init(MouseCursor::new(
        &PIXEL_WRITER,
        DESKTOP_BG_COLOR,
        Vector2D::new(300, 200),
    ));

    // デバイス一覧の表示
    let result = pci::scan_all_bus();
    log!(LogLevel::Debug, "scan_all_bus: {:?}", result);

    let mut xhc_dev = None;
    {
        let devices = pci::DEVICES.read();
        let mut intel_found = false;
        for device in &*devices {
            let dev = device;
            let vendor_id = dev.read_vendor_id();
            log!(
                LogLevel::Debug,
                "{}.{}.{}: vend {:04x}, class {:08x}, head {:02x}",
                dev.bus(),
                dev.device(),
                dev.function(),
                vendor_id,
                dev.class_code(),
                dev.header_type()
            );

            // Intel 製を優先して xHC を探す
            if device.class_code().r#match(0x0c, 0x03, 0x30) {
                if intel_found {
                    continue;
                }

                if 0x8086 == vendor_id {
                    intel_found = true;
                }
                xhc_dev = Some(*device);
            }
        }

        if xhc_dev.is_some() {
            let xhc_dev = xhc_dev.unwrap();
            log!(
                LogLevel::Info,
                "xHC has been found: {}.{}.{}",
                xhc_dev.bus(),
                xhc_dev.device(),
                xhc_dev.function()
            );
        }
    }
    let mut xhc_dev = xhc_dev.unwrap();

    let cs = get_cs();
    {
        let mut idt = IDT.lock();
        idt[InterruptVector::XHCI as usize].set_idt_entry(
            InterruptDescriptorAttribute::new(
                x86_descriptor::SystemSegmentType::InterruptGate,
                0,
                true,
            ),
            int_handler_xhci,
            cs,
        );
        load_idt(
            (size_of::<InterruptDescriptor>() * idt.len()) as u16 - 1,
            idt.as_ptr() as u64,
        )
    }

    let bsp_local_apic_id = (unsafe { *(0xfee0_0020 as *const u32) } >> 24) as u8;
    xhc_dev
        .configure_msi_fixed_destination(
            bsp_local_apic_id,
            pci::MSITriggerMode::Level,
            pci::MSIDeliverMode::Fixed,
            InterruptVector::XHCI as u8,
            0,
        )
        .unwrap();
    let xhc_dev = xhc_dev;

    // xHC の BAR から情報を得る
    let xhc_bar = xhc_dev.read_bar(0);
    log!(LogLevel::Debug, "ReadBar: {:#x?}", xhc_bar);
    let xhc_mmio_base = xhc_bar.unwrap().get_bits(4..) << 4;
    log!(LogLevel::Debug, "xHC mmio_base = {:08x}", xhc_mmio_base);

    let mut xhc = Controller::new(xhc_mmio_base);

    if xhc_dev.read_vendor_id() == 0x8086 {
        switch_ehci2xhci(&xhc_dev);
    }

    let result = xhc.initialize();
    log!(LogLevel::Debug, "xhc.initialize: {:?}", result);

    log!(LogLevel::Info, "xHC starting");
    xhc.run().unwrap();

    XHC.init(xhc);

    HIDMouseDriver::set_default_observer(mouse_observer);

    {
        let mut xhc = XHC.lock();

        for i in 1..=xhc.max_ports() {
            let mut port = xhc.port_at(i);
            log!(
                LogLevel::Debug,
                "Port {}: IsConnected={}",
                i,
                port.is_connected()
            );

            if port.is_connected() {
                if let Err(err) = xhc.configure_port(&mut port) {
                    log!(LogLevel::Error, "failed to configure port: {}", err);
                    continue;
                }
            }
        }
    }

    let screen = match FrameBuffer::new(frame_buffer_config) {
        Ok(s) => s,
        Err(e) => {
            panic!(
                "failed to initialize frame buffer: {} as {}:{}",
                e,
                e.file(),
                e.line()
            );
        }
    };
    SCREEN.init(screen);
    LAYER_MANAGER.init(LayerManager::new(&SCREEN));

    // デッドロックを回避するために、`CONSOLE` の `wirter` 変更（これに伴って redraw される）は
    // ロックを解除してから行う
    let bglayer_id = {
        let mut layer_manager = LAYER_MANAGER.lock();
        let frame_width = frame_buffer_config.horizontal_resolution as u32;
        let framw_height = frame_buffer_config.vertical_resolution as u32;

        let bgwindow = Window::new(frame_width, framw_height, frame_buffer_config.pixel_format);
        let bglayer_id = layer_manager.new_layer(bgwindow);
        draw_desktop(layer_manager.layer(bglayer_id).widow());

        let mut mouse_window = Window::new(
            MOUSE_CURSOR_WIDTH as u32,
            MOUSE_CURSOR_HEIGHT as u32,
            frame_buffer_config.pixel_format,
        );
        mouse_window.set_transparent_color(Some(MOUSE_TRANSPARENT_COLOR));
        mouse::draw_mouse_cursor(&mut mouse_window, &Vector2D::new(0, 0));
        let mouse_layer_id = layer_manager.new_layer(mouse_window);

        layer_manager.layer(bglayer_id).r#move(Vector2D::new(0, 0));
        layer_manager
            .layer(mouse_layer_id)
            .move_relative(Vector2D::new(200, 200));

        layer_manager.up_down(bglayer_id, 0);
        layer_manager.up_down(mouse_layer_id, 1);
        MOUSE_LAYER_ID.store(mouse_layer_id, Ordering::Release);

        bglayer_id
    };
    CONSOLE.lock().set_layer(bglayer_id);
    LAYER_MANAGER.lock().draw();

    loop {
        cli();
        let msg = {
            let mut main_queue = MAIN_QUEUE.lock();

            if main_queue.len() == 0 {
                // 待機中ロックがかかったままになるため、明示的にドロップしておく
                drop(main_queue);
                sti_hlt();
                continue;
            }

            main_queue.pop_front().unwrap()
            // 割り込みを許可する前に MAIN_QUEUE のロック解除
        };
        sti();

        match msg.r#type() {
            MessageType::InteruptXHCI => {
                let mut xhc = XHC.lock();
                while xhc.primary_event_ring().has_front() {
                    if let Err(err) = xhc.process_event() {
                        log!(LogLevel::Error, "Error while process_evnet: {}", err);
                    }
                }
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 前の改行の有無をチェックし、なければ改行を追加する
    if !CONSOLE.lock().is_head() {
        printkln!();
    }
    printkln!("{}", info);
    halt()
}

fn halt() -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
