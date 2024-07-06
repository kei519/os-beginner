#![no_std]
#![no_main]

use alloc::{vec, vec::Vec};
use app_lib::{
    fs::{self, FileFlags, Read as _},
    println,
};

extern crate alloc;
extern crate app_lib;

#[app_lib::main]
fn main(args: app_lib::args::Args) -> i32 {
    let args: Vec<_> = args.into_iter().collect();
    let (filename, ch) = if args.len() >= 3 {
        (args[1].as_str(), args[2].parse::<u8>().unwrap())
    } else {
        ("/memmap", b'\n')
    };
    let mut file = match fs::open(filename, FileFlags::RDONLY) {
        Ok(s) => s,
        Err(_) => {
            println!("failed to open {}", filename);
            return 1;
        }
    };
    let mut buf = vec![];
    let total = match file.read_to_end(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            println!("error: {e}");
            println!("failed to read {}", filename);
            return 1;
        }
    };
    println!("size of {} = {} bytes", filename, total);

    let num = buf.into_iter().filter(|&b| b == ch).count();
    println!("the number of '{}' (0x{:02x}) = {}", ch as char, ch, num);
    0
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    app_lib::println!("{}", info);
    app_lib::exit(-1)
}
