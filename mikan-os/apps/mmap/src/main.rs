#![no_std]
#![no_main]

use app_lib::{
    fs::{self, FileFlags},
    print, println,
};

extern crate alloc;
extern crate app_lib;

#[app_lib::main]
fn main(_: app_lib::args::Args) -> i32 {
    let mut file = match fs::open("/memmap", FileFlags::RDONLY) {
        Ok(f) => f,
        Err(e) => return e.into(),
    };
    let mem = match file.memmap() {
        Ok(m) => m,
        Err(e) => return e.into(),
    };
    for &b in mem.iter() {
        print!("{}", b as char);
    }
    println!("\nread from mapped file ({} bytes)", mem.len());

    0
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    app_lib::println!("{}", info);
    app_lib::exit(-1)
}
