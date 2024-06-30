#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::Ordering};

use app_lib::{
    args::Args, kernel_log, logger::LogLevel, main, open_window, win_fill_rectangle, ERRNO,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

extern crate app_lib;

const WIDTH: i32 = 100;
const HEIGHT: i32 = 100;

#[main]
fn main(args: Args) -> i32 {
    let layer_id = open_window(WIDTH + 8, HEIGHT + 28, 10, 10, "stars");
    if layer_id == 0 {
        return ERRNO.load(Ordering::Relaxed);
    }
    win_fill_rectangle(layer_id, 4, 24, WIDTH, HEIGHT, 0x000000);
    let num_stars = args
        .get_as_str(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(100);
    let mut rng = SmallRng::from_seed([0; 32]);
    for _ in 0..num_stars {
        let x = rng.gen_range(0..WIDTH - 2);
        let y = rng.gen_range(0..WIDTH - 2);
        win_fill_rectangle(layer_id, 4 + x, 24 + y, 2, 2, 0xfff100);
    }
    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
