#![no_std]
#![no_main]

extern crate app_lib;

use app_lib::{kernel_log, logger::LogLevel, println};
use core::{
    ffi::{c_char, CStr},
    panic::PanicInfo,
    ptr, str,
};

static mut STACK_PTR: isize = -1;
static mut STACK: [i64; 100] = [0; 100];

#[no_mangle]
extern "sysv64" fn _start(argc: i32, argv: *const *const c_char) -> i32 {
    let args = unsafe { &*ptr::slice_from_raw_parts(argv, argc as usize) };
    let args = args
        .iter()
        .map(|&p| unsafe { CStr::from_ptr(p) }.to_str().unwrap());

    main(args)
}

fn main(args: impl IntoIterator<Item = &'static str>) -> i32 {
    for arg in args.into_iter().skip(1) {
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

    loop {}
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
