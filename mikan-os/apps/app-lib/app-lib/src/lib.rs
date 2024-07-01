#![no_std]
#![cfg(target_arch = "x86_64")]

mod syscall;

pub mod args;
pub mod buf;
pub mod errno;
pub mod events;
pub mod graphics;
pub mod logger;
pub mod stdio;
pub mod time;
pub mod unistd;

pub use app_lib_macros::main;

pub use errno::ERRNO;

pub fn exit(exit_code: i32) -> ! {
    unsafe { syscall::__exit(exit_code as _) };
    core::unreachable!("syscall exit never returns")
}
