#![no_std]
#![no_main]

use app_lib::{
    events::{self, AppEvent},
    graphics, println,
};

extern crate alloc;
extern crate app_lib;

#[app_lib::main]
fn main(_: app_lib::args::Args) -> i32 {
    let layer_id = graphics::open_window(200, 100, 10, 10, "こんにちは");
    if layer_id == 0 {
        return 1;
    }

    graphics::win_write_string(layer_id, 7, 24, 0xc00000, "おはよう 世界！");
    graphics::win_write_string(layer_id, 24, 40, 0x00c000, "こんにちは 世界！");
    graphics::win_write_string(layer_id, 40, 56, 0x0000c0, "こんばんは 世界！");

    let mut events = [AppEvent::Null; 1];
    loop {
        let n = events::read_event(&mut events);
        if n == 0 {
            println!("ReadEvent failed");
            return 1;
        }
        match events[0] {
            AppEvent::Quit => break,
            event => println!("unknown event: type = {}", event.discripinant()),
        }
    }

    graphics::close_window(layer_id);
    0
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    app_lib::println!("{}", info);
    app_lib::exit(-1)
}
