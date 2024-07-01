#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::Ordering};

use app_lib::{
    args::Args,
    errno::ErrNo,
    events::{self, AppEvent},
    exit, graphics, kernel_log,
    logger::LogLevel,
    main, println, ERRNO,
};

extern crate app_lib;

#[main]
fn main(_: Args) -> i32 {
    let layer_id = graphics::open_window(200, 100, 10, 10, "winhello");
    if layer_id == 0 {
        exit(ERRNO.load(Ordering::Relaxed));
    }

    graphics::win_write_string(layer_id, 7, 24, 0xc00000, "hello world!");
    graphics::win_write_string(layer_id, 24, 40, 0x00c000, "hello world!");
    graphics::win_write_string(layer_id, 40, 56, 0x0000c0, "hello world!");

    let mut events = [AppEvent::Null; 1];
    loop {
        let n = events::read_event(&mut events);
        if n == 0 {
            println!(
                "ReadEvent failed: {}",
                ErrNo::from(ERRNO.load(Ordering::Relaxed)),
            );
            break;
        }
        match events[0] {
            AppEvent::Quit => break,
            _ => println!("unknow event: type = {}", events[0] as i32),
        }
    }
    graphics::close_window(layer_id);
    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
