#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::Ordering};

use app_lib::{
    args::Args,
    graphics, kernel_log,
    logger::LogLevel,
    main, println,
    time::{self, TimerInfo},
    ERRNO,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

extern crate app_lib;

const WIDTH: i32 = 100;
const HEIGHT: i32 = 100;

#[main]
fn main(args: Args) -> i32 {
    let layer_id = graphics::open_window(WIDTH + 8, HEIGHT + 28, 10, 10, "stars");
    if layer_id == 0 {
        return ERRNO.load(Ordering::Relaxed);
    }
    graphics::win_fill_rectangle(layer_id, 4, 24, WIDTH, HEIGHT, 0x000000);
    let num_stars = args
        .get_as_str(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(100);

    let TimerInfo {
        tick: tick_start,
        freq: timer_freq,
    } = time::get_current_tick();

    let mut rng = SmallRng::seed_from_u64(tick_start);
    for _ in 0..num_stars {
        let x = rng.gen_range(0..WIDTH - 2);
        let y = rng.gen_range(0..WIDTH - 2);
        graphics::win_fill_rectangle(layer_id, 4 + x, 24 + y, 2, 2, 0xfff100);
    }

    let TimerInfo { tick: tick_end, .. } = time::get_current_tick();
    println!(
        "{} stars in {} ms.",
        num_stars,
        (tick_end - tick_start) * 1000 / timer_freq
    );

    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
