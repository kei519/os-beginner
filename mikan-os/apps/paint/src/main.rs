#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::Ordering};

use app_lib::{
    args::Args,
    errno::ErrNo,
    events::{self, AppEvent},
    graphics::{self, win_fill_rectangle},
    kernel_log,
    logger::LogLevel,
    main, println, ERRNO,
};

extern crate app_lib;

const WIDTH: i32 = 200;
const HEIGHT: i32 = 130;

fn is_inside(x: i32, y: i32) -> bool {
    (4..4 + WIDTH).contains(&x) && (24..24 + HEIGHT).contains(&y)
}

#[main]
fn main(_: Args) -> i32 {
    let layer_id = graphics::open_window(WIDTH + 8, HEIGHT + 28, 10, 10, "paint");
    if layer_id == 0 {
        return ERRNO.load(Ordering::Relaxed);
    }

    let mut events = [AppEvent::Null; 1];
    loop {
        let n = events::read_event(&mut events);
        if n == 0 {
            println!(
                "read_event failed: {}",
                ErrNo::from(ERRNO.load(Ordering::Relaxed))
            );
            break;
        }
        match events[0] {
            AppEvent::Quit => break,
            AppEvent::MouseMove {
                x,
                y,
                dx,
                dy,
                buttons,
            } => {
                let press = buttons & 1 == 1;
                let prev_x = x - dx;
                let prev_y = y - dy;
                if press && is_inside(prev_x, prev_y) {
                    graphics::win_draw_line(layer_id, prev_x, prev_y, x, y, 0x000000);
                }
            }
            AppEvent::MouseButton {
                x,
                y,
                press,
                button,
            } => {
                if press && button == 0 {
                    win_fill_rectangle(layer_id, x, y, 1, 1, 0x000000);
                }
            }
            event => println!("unknown event: type = {}", event.discripinant()),
        }
    }

    graphics::close_window(layer_id);
    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
