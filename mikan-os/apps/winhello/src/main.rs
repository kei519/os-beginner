#![no_std]
#![no_main]

use core::{
    ffi::{c_char, CStr},
    panic::PanicInfo,
    ptr,
    sync::atomic::Ordering,
};

use app_lib::{exit, kernel_log, logger::LogLevel, open_window, win_write_string, ERRNO};

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
    let layer_id = open_window(200, 100, 10, 10, "winhello");
    if layer_id == 0 {
        exit(ERRNO.load(Ordering::Relaxed));
    }

    win_write_string(layer_id, 7, 24, 0xc00000, "hello world!");
    win_write_string(layer_id, 24, 40, 0x00c000, "hello world!");
    win_write_string(layer_id, 40, 56, 0x0000c0, "hello world!");
    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
