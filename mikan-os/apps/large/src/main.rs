#![no_std]
#![no_main]

use core::{
    ffi::{c_char, CStr},
    hint,
    panic::PanicInfo,
    ptr,
};

use app_lib::{exit, kernel_log, logger::LogLevel};

extern crate app_lib;

#[no_mangle]
extern "sysv64" fn _start(argc: i32, argv: *const *const c_char) {
    let args = unsafe { &*ptr::slice_from_raw_parts(argv, argc as usize) };
    let args = args
        .iter()
        .map(|&p| unsafe { CStr::from_ptr(p) }.to_str().unwrap());

    exit(main(args))
}

static TABLE: [u8; 3 * 1024 * 1024] = [0; 3 * 1024 * 1024];

fn main(args: impl IntoIterator<Item = &'static str>) -> i32 {
    hint::black_box(&TABLE);
    args.into_iter()
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
