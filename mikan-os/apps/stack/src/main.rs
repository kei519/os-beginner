#![no_std]
#![no_main]

use core::hint;

extern crate alloc;
extern crate app_lib;

#[app_lib::main]
fn main(args: app_lib::args::Args) -> i32 {
    let stack = [0u8; 3 * 1024 * 1024];
    hint::black_box(&stack);
    args.len() as _
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    app_lib::println!("{}", info);
    app_lib::exit(-1)
}
