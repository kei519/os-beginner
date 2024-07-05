#![no_std]
#![no_main]

use app_lib::{
    fs::{self, FileFlags, Read as _, Write as _},
    println,
};

extern crate app_lib;

#[app_lib::main]
fn main(args: app_lib::args::Args) -> i32 {
    if args.len() < 3 {
        println!("Usage: {} <src> <dest>", args.get_as_str(0).unwrap());
        return 1;
    }
    let src = args.get_as_str(1).unwrap();
    let dest = args.get_as_str(2).unwrap();

    let mut src_file = match fs::open(src, FileFlags::RDONLY) {
        Ok(f) => f,
        Err(_) => {
            println!("failed to open for read: {}", src);
            return 1;
        }
    };

    let mut dest_file = match fs::open(dest, FileFlags::WRONLY | FileFlags::CREAT) {
        Ok(f) => f,
        Err(_) => {
            println!("failed to open for write: {}", dest);
            return 1;
        }
    };

    let mut buf = [0; 256];
    loop {
        let bytes = match src_file.read(&mut buf) {
            Ok(0) => break,
            Ok(len) => len,
            Err(_) => {
                println!("failed to read from {}", src);
                return 1;
            }
        };
        match dest_file.write(&buf[..bytes]) {
            Ok(len) if len == bytes => {}
            _ => {
                println!("failed to write to {}", dest);
                return 1;
            }
        }
    }

    0
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    app_lib::kernel_log!(app_lib::logger::LogLevel::Error, "paniced: {}", info);
    app_lib::exit(-1)
}
