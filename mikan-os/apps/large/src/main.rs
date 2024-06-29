#![no_std]
#![no_main]

use core::{hint, panic::PanicInfo};

use app_lib::{args::Args, kernel_log, logger::LogLevel, main};

extern crate app_lib;

static TABLE: [u8; 3 * 1024 * 1024] = [0; 3 * 1024 * 1024];

#[main]
fn main(args: Args) -> i32 {
    hint::black_box(&TABLE);
    args.iter()
        .skip(1)
        .next()
        .map(|s| s.parse().unwrap_or_default())
        .unwrap_or_default()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
