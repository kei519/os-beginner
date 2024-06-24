#![no_std]
#![no_main]

use core::{
    arch::{asm, global_asm},
    ffi::{c_char, CStr},
    panic::PanicInfo,
    ptr,
};

static mut STACK_PTR: isize = -1;
static mut STACK: [i64; 100] = [0; 100];

#[no_mangle]
extern "sysv64" fn _start(argc: i32, argv: *const *const c_char) -> i32 {
    let args = unsafe { &*ptr::slice_from_raw_parts(argv, argc as usize) };
    let args = args
        .into_iter()
        .map(|&p| unsafe { CStr::from_ptr(p) }.to_str().unwrap());

    main(args)
}

fn main(args: impl IntoIterator<Item = &'static str>) -> i32 {
    for arg in args.into_iter().skip(1) {
        match arg {
            "+" => {
                let b = pop();
                let a = pop();
                push(a + b);
                log_string(LogLevel::Warn, c"+")
            }
            "-" => {
                let b = pop();
                let a = pop();
                push(a - b);
                log_string(LogLevel::Warn, c"-")
            }
            arg => {
                push(arg.parse().unwrap());
                log_string(LogLevel::Warn, c"#")
            }
        }
    }

    let _ret = if unsafe { STACK_PTR } < 0 {
        0
    } else {
        pop() as _
    };
    log_string(LogLevel::Warn, c"hello, this is rpn");
    loop {}
}

fn pop() -> i64 {
    unsafe {
        let value = STACK[STACK_PTR as usize];
        STACK_PTR -= 1;
        value
    }
}

fn push(value: i64) {
    unsafe {
        STACK_PTR += 1;
        STACK[STACK_PTR as usize] = value;
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") };
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(C)]
pub enum LogLevel {
    Error = 3,
    Warn = 4,
    Info = 6,
    Debug = 7,
}

fn log_string(loglevel: LogLevel, str: &CStr) {
    unsafe { log_string_unsafe(loglevel, str.as_ptr()) };
}

extern "sysv64" {
    fn log_string_unsafe(loglevel: LogLevel, str: *const c_char);
}

global_asm! { r#"
.global log_string_unsafe
log_string_unsafe:
    mov eax, 0x80000000
    mov r10, rcx
    syscall
    ret
"#
}
