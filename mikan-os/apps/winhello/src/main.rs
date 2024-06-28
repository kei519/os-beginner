#![no_std]
#![no_main]

use core::{
    ffi::{c_char, CStr},
    panic::PanicInfo,
    ptr,
};

use app_lib::{exit, kernel_log, logger::LogLevel, open_window};

extern crate app_lib;

#[no_mangle]
extern "sysv64" fn _start(argc: i32, argv: *const *const c_char) {
    let args = unsafe { &*ptr::slice_from_raw_parts(argv, argc as usize) };
    let args = args
        .iter()
        .map(|&p| unsafe { CStr::from_ptr(p) }.to_str().unwrap());

    exit(main(args))
}

fn main(_args: impl IntoIterator<Item = &'static str>) -> i32 {
    open_window(200, 100, 10, 10, "winhello");
    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
