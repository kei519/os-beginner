#![no_std]
#![no_main]

use alloc::{string::String, vec::Vec};
use app_lib::{
    eprintln,
    fs::{self, FileFlags, Read},
    io, println,
};

extern crate alloc;
extern crate app_lib;

#[app_lib::main]
fn main(args: app_lib::args::Args) -> i32 {
    let mut file = if args.len() >= 2 {
        let path = args.skip(1).next().unwrap();
        match fs::open(&path, FileFlags::RDONLY) {
            Ok(f) => f,
            Err(_) => {
                eprintln!("failed to open '{}'", path);
                return 1;
            }
        }
    } else {
        io::stdin()
    };

    let mut s = String::new();
    if let Err(e) = file.read_to_string(&mut s) {
        eprintln!("failed to read : {}", e);
    };
    let mut lines: Vec<_> = s.lines().collect();
    lines.sort();

    for line in lines {
        println!("{}", line);
    }

    0
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    app_lib::eprintln!("{}", info);
    app_lib::exit(-1)
}
