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

/// 作成したウィンドウのレイヤー ID を返す。
/// ただし作成に失敗した場合は `0` を返す。
pub fn open_window(w: i32, h: i32, x: i32, y: i32, title: impl core::fmt::Display) -> u32 {
    use core::fmt::Write as _;
    let mut buf = [0; 1024];
    let mut s = buf::CStrBuf::new_unchecked(&mut buf);
    write!(s, "{}", title).unwrap();
    let res = unsafe {
        syscall::__open_window(w as _, h as _, x as _, y as _, s.to_cstr().as_ptr() as _)
    };
    if res.error != 0 {
        ERRNO.store(res.error, core::sync::atomic::Ordering::Relaxed);
        0
    } else {
        res.value as _
    }
}
