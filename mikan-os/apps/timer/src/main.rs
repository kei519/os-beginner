#![no_std]
#![no_main]

use core::panic::PanicInfo;

use app_lib::{
    args::Args,
    events::{self, AppEvent},
    kernel_log,
    logger::LogLevel,
    main, println, time,
    time::TimerMode,
};

extern crate app_lib;

#[main]
fn main(args: Args) -> i32 {
    if args.len() <= 1 {
        println!("Usage: timer <msec>");
        return 1;
    }

    // 上で引数の数を確認しているから、この unwrap は成功する
    let Ok(duration_ms) = args.get_as_str(1).unwrap().parse() else {
        println!("Usage: timer <msec>");
        return 1;
    };
    let mode = TimerMode::new().set_relative(true);
    let timeout = time::create_timer(mode, 1, duration_ms);
    println!("timer created. timeout = {}", timeout);

    let mut events = [AppEvent::Null; 1];
    loop {
        events::read_event(&mut events);
        if let AppEvent::Timer { .. } = events[0] {
            println!("{} msecs elapsed!", duration_ms);
            break;
        } else {
            println!("unknow event: type = {}", events[0].discripinant());
        }
    }
    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
