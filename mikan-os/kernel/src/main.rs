#![no_std]
#![no_main]

extern crate alloc;

use alloc::{boxed::Box, format};
use core::{arch::asm, mem::size_of, panic::PanicInfo, sync::atomic::Ordering};
use uefi::table::boot::MemoryMap;

use kernel::{
    asmfunc::{cli, get_cs, load_idt, set_cs_ss, set_ds_all, sti},
    bitfield::BitField as _,
    console::{Console, CONSOLE, DESKTOP_BG_COLOR, DESKTOP_FG_COLOR},
    font,
    frame_buffer::FrameBuffer,
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{
        draw_desktop, BgrResv8BitPerColorPixelWriter, PixelColor, PixelWriter,
        RgbResv8BitPerColorPixelWriter, Vector2D, PIXEL_WRITER,
    },
    interrupt::{
        notify_end_of_interrupt, InterruptDescriptor, InterruptDescriptorAttribute, InterruptFrame,
        InterruptVector, Message, MessageType, IDT, MAIN_QUEUE,
    },
    layer::{LayerManager, LAYER_MANAGER, SCREEN},
    log,
    logger::{set_log_level, LogLevel},
    memory_manager,
    mouse::{
        self, MouseCursor, MOUSE_CURSOR, MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH, MOUSE_LAYER_ID,
        MOUSE_TRANSPARENT_COLOR,
    },
    paging,
    pci::{self, Device},
    printk, printkln, segment, timer,
    usb::{Controller, HIDMouseDriver, XHC},
    window::Window,
    x86_descriptor,
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

#[no_mangle]
static KERNEL_MAIN_STACK: KernelStack = KernelStack::new();

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
    let fb_config = frame_buffer_config.clone();
    let pixel_writer: Box<dyn PixelWriter + Send> = match frame_buffer_config.pixel_format {
        PixelFormat::Rgb => Box::new(RgbResv8BitPerColorPixelWriter::new(fb_config)),
        PixelFormat::Bgr => Box::new(BgrResv8BitPerColorPixelWriter::new(fb_config)),
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

    HIDMouseDriver::set_default_observer(mouse::mouse_observer);

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
    let (bglayer_id, console_id, main_window_id) = {
        let mut layer_manager = LAYER_MANAGER.lock();
        let screen = SCREEN.lock();
        let frame_width = screen.horizontal_resolution() as u32;
        let frame_height = screen.vertical_resolution() as u32;
        let pixel_format = screen.pixef_format();

        let bgwindow = Window::new(frame_width, frame_height, pixel_format);
        let bglayer_id = layer_manager.new_layer(bgwindow);
        draw_desktop(layer_manager.layer(bglayer_id).window_mut());

        let mut mouse_window = Window::new(
            MOUSE_CURSOR_WIDTH as u32,
            MOUSE_CURSOR_HEIGHT as u32,
            pixel_format,
        );
        mouse_window.set_transparent_color(Some(MOUSE_TRANSPARENT_COLOR));
        mouse::draw_mouse_cursor(&mut mouse_window, &Vector2D::new(0, 0));
        let mouse_layer_id = layer_manager.new_layer(mouse_window);
        layer_manager
            .layer(mouse_layer_id)
            .move_relative(Vector2D::new(200, 200));

        let mut main_window = Window::new(160, 52, pixel_format);
        main_window.draw_window(b"Hello Window");
        let main_window_id = layer_manager.new_layer(main_window);
        layer_manager
            .layer(main_window_id)
            .r#move(Vector2D::new(300, 100))
            .set_draggable(true);

        let console = CONSOLE.lock();
        let console_window = Window::new(
            console.column_num() as u32 * 8,
            console.row_num() as u32 * 16,
            pixel_format,
        );
        let console_id = layer_manager.new_layer(console_window);

        layer_manager.up_down(bglayer_id, 0);
        layer_manager.up_down(console_id, 1);
        layer_manager.up_down(main_window_id, 2);
        layer_manager.up_down(mouse_layer_id, 3);
        MOUSE_LAYER_ID.store(mouse_layer_id, Ordering::Release);

        (bglayer_id, console_id, main_window_id)
    };
    CONSOLE.lock().set_layer(console_id);
    LAYER_MANAGER.lock().draw_id(bglayer_id);

    let mut count = 0;
    loop {
        count += 1;
        {
            let mut layer_manager = LAYER_MANAGER.lock();
            let window = layer_manager.layer(main_window_id).window_mut();
            window.fill_rectangle(
                Vector2D::new(24, 28),
                Vector2D::new(8 * 10, 16),
                &PixelColor::new(0xc6, 0xc6, 0xc6),
            );
            font::write_string(
                window,
                Vector2D::new(24, 28),
                format!("{:010}", count).as_bytes(),
                &PixelColor::new(0, 0, 0),
            );
            layer_manager.draw_id(main_window_id);
        }

        cli();
        let msg = {
            let mut main_queue = MAIN_QUEUE.lock();

            if main_queue.len() == 0 {
                // 待機中ロックがかかったままになるため、明示的にドロップしておく
                drop(main_queue);
                sti();
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
