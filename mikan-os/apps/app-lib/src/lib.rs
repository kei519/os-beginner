#![no_std]
#![cfg(target_arch = "x86_64")]

// TODO: エントリーポイント処理をプロセスマクロにする

mod syscall;

pub mod buf;
pub mod errno;
pub mod logger;
pub mod stdio;
pub mod unistd;

pub use errno::ERRNO;

pub fn exit(exit_code: i32) -> ! {
    unsafe { syscall::__exit(exit_code as _) };
    core::unreachable!("syscall exit never returns")
}
