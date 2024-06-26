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
