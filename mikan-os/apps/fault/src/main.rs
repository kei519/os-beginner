#![no_std]
#![no_main]

use core::{arch::asm, mem, panic::PanicInfo};

use app_lib::{args::Args, kernel_log, logger::LogLevel, main};

extern crate app_lib;

#[main]
fn main(args: Args) -> i32 {
    let cmd = if args.len() >= 2 {
        args.get_as_str(1).unwrap()
    } else {
        "hlt"
    };

    unsafe {
        match cmd {
            "hlt" => asm!("hlt"),
            "wr_kernel" => {
                let p: &mut i32 = mem::transmute(0x100_usize);
                *p = 42;
            }
            "wr_app" => {
                let p: &mut i32 = mem::transmute(0xffff_8000_ffff_0000_usize);
                *p = 123;
            }
            "zero" => {
                asm! {
                    "mov rdi, 0",
                    "cqo",
                    "idiv rdi",
                }
            }
            _ => {}
        }
    }
    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
