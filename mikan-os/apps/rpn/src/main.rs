#![no_std]
#![no_main]

extern crate app_lib;

use app_lib::{args::Args, kernel_log, logger::LogLevel, main, println};
use core::panic::PanicInfo;

static mut STACK_PTR: isize = -1;
static mut STACK: [i64; 100] = [0; 100];

#[main]
fn main(args: Args) -> i32 {
    for arg in args.iter().skip(1) {
        match arg {
            "+" => {
                let b = pop();
                let a = pop();
                push(a + b);
            }
            "-" => {
                let b = pop();
                let a = pop();
                push(a - b);
            }
            arg => {
                push(arg.parse().unwrap());
            }
        }
    }

    let result: i32 = if unsafe { STACK_PTR } < 0 {
        0
    } else {
        pop() as _
    };

    println!("{}", result);
    result
}

fn pop() -> i64 {
    unsafe {
        let value = STACK[STACK_PTR as usize];
        STACK_PTR -= 1;
        value
    }
}

fn push(value: i64) {
    unsafe {
        STACK_PTR += 1;
        STACK[STACK_PTR as usize] = value;
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
