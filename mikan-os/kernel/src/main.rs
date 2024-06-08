#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use core::{arch::asm, panic::PanicInfo};
use uefi::table::boot::MemoryMap;

use kernel::{
    acpi::RSDP,
    asmfunc::{cli, sti, sti_hlt},
    console::{self, PanicConsole},
    error::Result,
    font,
    frame_buffer_config::FrameBufferConfig,
    graphics::{PixelColor, PixelWrite, Vector2D, FB_CONFIG},
    interrupt,
    layer::{self, LAYER_MANAGER, SCREEN},
    log,
    logger::{set_log_level, LogLevel},
    memory_manager,
    message::{self, Message},
    mouse, paging, pci, printk, printkln, segment,
    timer::{self, Timer, TIMER_MANAGER},
    window::Window,
    xhci::{self, XHC},
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

/// メインウィンドウの初期化を行う。
fn initialize_main_window() -> u32 {
    let mut layer_manager = LAYER_MANAGER.lock_wait();

    let mut main_window = Window::new(160, 52, SCREEN.lock_wait().pixel_format());
    main_window.draw_window(b"Hello Window");
    let main_window_id = layer_manager.new_layer(main_window);
    layer_manager
        .layer(main_window_id)
        .r#move(Vector2D::new(300, 100))
        .set_draggable(true);

    layer_manager.up_down(main_window_id, 2);

    main_window_id
}

// この呼び出しの前にスタック領域を変更するため、でかい構造体をそのまま渡せなくなる
// それを避けるために参照で渡す
#[custom_attribute::kernel_entry(KERNEL_MAIN_STACK, STACK_SIZE = 1024 * 1024)]
fn kernel_entry(
    frame_buffer_config: &'static FrameBufferConfig,
    memory_map: &'static MemoryMap,
    kernel_base: usize,
    kernel_size: usize,
    acpi_table: &RSDP,
) {
    // メモリアロケータの初期化
    memory_manager::GLOBAL.init(memory_map, kernel_base, kernel_size);
    FB_CONFIG.init(frame_buffer_config.clone());

    if let Err(err) = main(acpi_table) {
        printkln!("{}", err);
    }
}

fn main(acpi_table: &RSDP) -> Result<()> {
    layer::init();
    console::init();

    printk!("Welcome to MikanOS!\n");
    set_log_level(LogLevel::Warn);

    segment::init();
    paging::init();
    interrupt::init();

    pci::init()?;
    xhci::init();

    let main_window_id = initialize_main_window();
    mouse::init();

    // FIXME: 最初に登録されるレイヤーは背景ウィンドウなので、`layer_id` 1 を表示すれば
    //        必ず全て表示されるが、ハードコードは良くなさそう
    LAYER_MANAGER.lock_wait().draw_id(1);

    acpi_table.init()?;
    timer::init();

    {
        let mut manager = TIMER_MANAGER.lock_wait();
        manager.add_timer(Timer::new(200, 2));
        manager.add_timer(Timer::new(600, -1));
    }

    loop {
        let tick = TIMER_MANAGER.lock_wait().current_tick();
        {
            let mut layer_manager = LAYER_MANAGER.lock_wait();
            let window = layer_manager.layer(main_window_id).window_mut();
            window.fill_rectangle(
                Vector2D::new(24, 28),
                Vector2D::new(8 * 10, 16),
                &PixelColor::new(0xc6, 0xc6, 0xc6),
            );
            font::write_string(
                window,
                Vector2D::new(24, 28),
                format!("{:010}", tick).as_bytes(),
                &PixelColor::new(0, 0, 0),
            );
            layer_manager.draw_id(main_window_id);
        }

        cli();
        let msg = match message::pop_main_queue() {
            Some(msg) => msg,
            None => {
                sti_hlt();
                continue;
            }
        };
        sti();

        match msg {
            Message::InterruptXHCI => {
                let mut xhc = XHC.lock_wait();
                while xhc.primary_event_ring().has_front() {
                    if let Err(err) = xhc.process_event() {
                        log!(LogLevel::Error, "Error while process_evnet: {}", err);
                    }
                }
            }
            Message::InterruptLAPICTimer => printkln!("Timer interrupt"),
            Message::TimerTimeout(timer) => {
                printkln!(
                    "Timer: timeout = {}, value = {}",
                    timer.timeout(),
                    timer.value()
                );
                if timer.value() > 0 {
                    TIMER_MANAGER
                        .lock_wait()
                        .add_timer(Timer::new(timer.timeout() + 100, timer.value() + 1));
                }
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write as _;
    cli();
    // エラーのたびに新しいインスタンスを作るので、最後に発生したエラーが表示される
    write!(&mut PanicConsole::new(), "{}", info).unwrap();
    halt()
}

fn halt() -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
