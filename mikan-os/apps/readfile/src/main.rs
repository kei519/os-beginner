#![no_std]
#![no_main]

use app_lib::{
    fs::{self, FileFlags, Read as _},
    print, println,
};

extern crate app_lib;

#[app_lib::main]
fn main(args: app_lib::args::Args) -> i32 {
    let path = if args.len() >= 2 {
        args.get_as_str(1).unwrap()
    } else {
        "/memmap"
    };

    let mut file = match fs::open(path, FileFlags::RDONLY) {
        Ok(f) => f,
        Err(_) => {
            println!("failed to open: {}", path);
            return 1;
        }
    };

    let mut buf = [0; 256];
    let mut num_line = 0;
    'l: loop {
        let len = match file.read(&mut buf) {
            // EOF
            Ok(0) => break,
            Ok(l) => l,
            Err(_) => {
                println!("failed to get a line");
                return 1;
            }
        };
        let s = match core::str::from_utf8(&buf[..len]) {
            Ok(s) => s,
            Err(_) => {
                println!("not utf-8 encoded");
                return 1;
            }
        };
        // 改行で終わっていない場合に続くように split_inclusive を使う
        for line in s.split_inclusive('\n') {
            if num_line >= 3 {
                break 'l;
            }
            print!("{}", line);
            num_line += 1;
        }
    }
    println!("----");
    0
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    app_lib::kernel_log!(app_lib::logger::LogLevel::Error, "paniced: {}", info);
    app_lib::exit(-1)
}
