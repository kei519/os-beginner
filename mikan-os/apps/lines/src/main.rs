#![no_std]
#![no_main]

use core::{f64::consts, panic::PanicInfo, sync::atomic::Ordering};

use app_lib::{args::Args, graphics, kernel_log, logger::LogLevel, main, ERRNO};

extern crate app_lib;

const RADIUS: i32 = 90;

const fn color(deg: i32) -> u32 {
    let deg = deg as u32;
    match deg {
        deg @ 0..=30 => ((255 * deg / 30) << 8) | 0xff0000,
        deg @ 31..=60 => (255 * (60 - deg) / 30) << 16 | 0x00ff00,
        deg @ 61..=90 => (255 * (deg - 60) / 30) | 0x00ff00,
        deg @ 91..=120 => (255 * (120 - deg) / 30) << 8 | 0x0000ff,
        deg @ 121..=150 => (255 * (deg - 120) / 30) << 16 | 0x0000ff,
        deg => (255 * (180 - deg) / 30) | 0xff0000,
    }
}

#[main]
fn main(_: Args) -> i32 {
    let layer_id = graphics::open_window(RADIUS * 2 + 10 + 8, RADIUS + 28, 10, 10, "lines");
    if layer_id == 0 {
        return ERRNO.load(Ordering::Relaxed);
    }

    let (x0, y0, x1, y1) = (4, 24, 4 + RADIUS + 10, 24 + RADIUS);
    for deg in (0..=90).step_by(5) {
        let x = (RADIUS as f64 * libm::cos(consts::PI * deg as f64 / 180.)) as i32;
        let y = (RADIUS as f64 * libm::sin(consts::PI * deg as f64 / 180.)) as i32;
        graphics::win_draw_line(layer_id, x0, y0, x0 + x, y0 + y, color(deg as i32));
        graphics::win_draw_line(layer_id, x1, y1, x1 + x, y1 - y, color(deg as i32 + 90));
    }

    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
