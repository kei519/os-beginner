#![no_std]
#![cfg(target_arch = "x86_64")]

mod syscall;

pub mod args;
pub mod buf;
pub mod errno;
pub mod logger;
pub mod stdio;
pub mod unistd;

pub use app_lib_macros::main;

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

pub fn win_write_string(layer_id: u32, x: i32, y: i32, color: u32, s: impl core::fmt::Display) {
    use core::fmt::Write as _;
    let mut buf = [0; 1024];
    let mut buf = buf::CStrBuf::new_unchecked(&mut buf);
    write!(buf, "{}", s).unwrap();
    let res = unsafe {
        syscall::__win_write_string(
            layer_id as _,
            x as _,
            y as _,
            color as _,
            buf.to_cstr().as_ptr() as _,
        )
    };
    if res.error != 0 {
        ERRNO.store(res.error, core::sync::atomic::Ordering::Relaxed);
    }
}

pub fn win_fill_rectangle(layer_id: u32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    let res = unsafe {
        syscall::__win_fill_rectangle(layer_id as _, x as _, y as _, w as _, h as _, color as _)
    };
    if res.error != 0 {
        ERRNO.store(res.error, core::sync::atomic::Ordering::Relaxed);
    }
}
