#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::Ordering};

use app_lib::{
    args::Args,
    errno::ErrNo,
    events::{self, AppEvent},
    graphics::{self, LayerFlags},
    kernel_log,
    logger::LogLevel,
    main, println, ERRNO,
};

extern crate app_lib;

const CANVAS_SIZE: i32 = 100;
const EYE_SIZE: i32 = 10;

fn draw_eye(layer_id: u32, mouse_x: i32, mouse_y: i32, color: u32) {
    let center_x = (mouse_x - CANVAS_SIZE / 2 - 4) as f64;
    let center_y = (mouse_y - CANVAS_SIZE / 2 - 24) as f64;

    let direction = libm::atan2(center_y, center_x);
    let distance = libm::sqrt(libm::pow(center_x, 2.) + libm::pow(center_y, 2.));
    let distance = distance.min((CANVAS_SIZE / 2 - EYE_SIZE / 2) as f64);

    let eye_center_x = libm::cos(direction) * distance;
    let eye_center_y = libm::sin(direction) * distance;
    let eye_x = eye_center_x as i32 + CANVAS_SIZE / 2 + 4;
    let eye_y = eye_center_y as i32 + CANVAS_SIZE / 2 + 24;

    graphics::win_fill_rectangle(
        layer_id,
        eye_x - EYE_SIZE / 2,
        eye_y - EYE_SIZE / 2,
        EYE_SIZE,
        EYE_SIZE,
        color,
    );
}

#[main]
fn main(_: Args) -> i32 {
    let layer_id = graphics::open_window(CANVAS_SIZE + 8, CANVAS_SIZE + 28, 10, 10, "eye");
    if layer_id == 0 {
        return ERRNO.load(Ordering::Relaxed);
    }

    graphics::win_fill_rectangle(layer_id, 4, 24, CANVAS_SIZE, CANVAS_SIZE, 0xffffff);

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
            AppEvent::MouseMove { x, y, .. } => {
                graphics::win_fill_rectangle_with_flags(
                    layer_id,
                    4,
                    24,
                    CANVAS_SIZE,
                    CANVAS_SIZE,
                    0xffffff,
                    LayerFlags::new().set_redraw(false),
                );
                draw_eye(layer_id, x, y, 0x000000);
            }
            AppEvent::Null => unreachable!(),
            _ => {}
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
