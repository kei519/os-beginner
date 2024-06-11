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
    interrupt, keyboard,
    layer::{self, LAYER_MANAGER, SCREEN},
    log,
    logger::{set_log_level, LogLevel},
    memory_manager,
    message::Message,
    mouse, paging, pci, printk, printkln, segment,
    task::{self, Stack},
    timer::{self, Timer, TIMER_MANAGER},
    window::Window,
    xhci::{self, XHC},
};

#[no_mangle]
static KERNEL_MAIN_STACK: Stack<STACK_SIZE> = Stack::new();

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

/// テキストウィンドウを表示、登録し、そのレイヤー ID を返す。
fn initialize_text_window() -> u32 {
    let win_w = 160;
    let win_h = 52;

    let mut window = Window::new(win_w, win_h, SCREEN.lock_wait().pixel_format());
    window.draw_window(b"Text Box Test");
    window.draw_text_box(
        Vector2D::new(4, 24),
        Vector2D::new(win_w as i32 - 8, win_h as i32 - 24 - 4),
    );

    let mut layer_manager = LAYER_MANAGER.lock_wait();
    let layer_id = layer_manager.new_layer(window);
    layer_manager
        .layer(layer_id)
        .r#move(Vector2D::new(350, 200))
        .set_draggable(true);

    layer_manager.up_down(layer_id, i32::MAX);

    layer_id
}

fn draw_text_cursor(visible: bool, index: i32, window: &mut Window) {
    let color = PixelColor::to_color(if visible { 0 } else { 0xffffff });
    let pos = Vector2D::new(8 + 8 * index, 24 + 5);
    window.fill_rectangle(pos, Vector2D::new(7, 15), &color);
}

/// `task_b()` 用のウィンドウを初期化、登録しそのレイヤー ID を返す。
fn initialize_task_b_window() -> u32 {
    let mut window = Window::new(160, 52, FB_CONFIG.lock_wait().pixel_format);
    window.draw_window(b"TaskB Window");

    let mut manager = LAYER_MANAGER.lock_wait();
    let id = manager.new_layer(window);
    manager
        .layer(id)
        .set_draggable(true)
        .r#move(Vector2D::new(100, 100));
    manager.up_down(id, i32::MAX);
    id
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

    let main_window_id = initialize_main_window();
    let text_window_id = initialize_text_window();
    let task_b_window_id = initialize_task_b_window();

    // FIXME: 最初に登録されるレイヤーは背景ウィンドウなので、`layer_id` 1 を表示すれば
    //        必ず全て表示されるが、ハードコードは良くなさそう
    LAYER_MANAGER.lock_wait().draw_id(1);

    acpi_table.init()?;
    timer::init();

    // カーソル点滅用のタイマを追加
    let textbox_cursor_timer = 1;
    let timer_05sec = (timer::TIMER_FREQ as f64 * 0.5) as u64;
    TIMER_MANAGER
        .lock_wait()
        .add_timer(Timer::new(timer_05sec, textbox_cursor_timer));
    let mut textbox_cursor_visible = false;

    task::init();
    let main_task = task::current_task();
    let taskb_id = task::new_task()
        .init_context(task_b, 45, task_b_window_id)
        .wake_up(-1)
        .id();

    xhci::init();
    mouse::init();
    keyboard::init();

    let mut text_window_index = 0;
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
        let msg = match main_task.receive_message() {
            Some(msg) => msg,
            None => {
                main_task.sleep();
                sti();
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
            Message::TimerTimeout(timer) => {
                if timer.value() == textbox_cursor_timer {
                    TIMER_MANAGER.lock_wait().add_timer(Timer::new(
                        timer.timeout() + timer_05sec,
                        textbox_cursor_timer,
                    ));
                    textbox_cursor_visible = !textbox_cursor_visible;
                    let mut layer_manager = LAYER_MANAGER.lock_wait();
                    draw_text_cursor(
                        textbox_cursor_visible,
                        text_window_index,
                        layer_manager.layer(text_window_id).window_mut(),
                    );
                    layer_manager.draw_id(text_window_id);
                }
            }
            Message::KeyPush { ascii, .. } => {
                // `input_text_window(ascii)` の代わり
                'input_text_window: {
                    if ascii == 0 {
                        break 'input_text_window;
                    }

                    let pos = |index| Vector2D::new(8 + 8 * index, 24 + 6);

                    let mut manager = LAYER_MANAGER.lock_wait();
                    let window = manager.layer(text_window_id).window_mut();

                    let max_chars = (window.width() as i32 - 16) / 8;
                    if ascii == 0x08 && text_window_index > 0 {
                        draw_text_cursor(false, text_window_index, window);
                        text_window_index -= 1;
                        window.fill_rectangle(
                            pos(text_window_index),
                            Vector2D::new(8, 16),
                            &PixelColor::to_color(0xffffff),
                        );
                        draw_text_cursor(true, text_window_index, window);
                    } else if ascii >= b' ' && text_window_index < max_chars {
                        draw_text_cursor(false, text_window_index, window);
                        font::write_ascii(
                            window,
                            pos(text_window_index),
                            ascii,
                            &PixelColor::to_color(0),
                        );
                        text_window_index += 1;
                        draw_text_cursor(true, text_window_index, window);
                    }
                    manager.draw_id(text_window_id);
                }
                // S で task_b を眠らせ W で起こす
                if ascii == b's' {
                    printkln!("sleep task_b: {:?}", task::sleep(taskb_id));
                } else if ascii == b'w' {
                    printkln!("wakeup task_b: {:?}", task::wake_up(taskb_id, -1));
                }
            }
        }
    }
}

fn task_b(task_id: u64, data: i64, layer_id: u32) {
    printkln!(
        "task_b: task_id={}, data={}, layer_id={}",
        task_id,
        data,
        layer_id
    );
    let mut count = 0;
    loop {
        count += 1;
        let mut manager = LAYER_MANAGER.lock_wait();
        let window = manager.layer(layer_id).window_mut();
        window.fill_rectangle(
            Vector2D::new(24, 28),
            Vector2D::new(8 * 10, 16),
            &PixelColor::to_color(0xc6c6c6),
        );
        font::write_string(
            window,
            Vector2D::new(24, 28),
            format!("{:010}", count).as_bytes(),
            &PixelColor::to_color(0),
        );
        manager.draw_id(layer_id);
        drop(manager);
        // TODO: 描画をメインスレッドに依頼するようにして削除する
        sti_hlt();
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
