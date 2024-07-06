use core::{fmt::Display, sync::atomic::Ordering::Relaxed};

#[cfg(not(feature = "alloc"))]
use core::fmt::Write as _;

#[cfg(feature = "alloc")]
use alloc::{ffi::CString, format};

use crate::*;

#[cfg(not(feature = "alloc"))]
use crate::buf::CStrBuf;

#[cfg(feature = "alloc")]
use crate::errno::ErrNo;

/// ウィンドウ描画時のフラグを表す。
//  0 bit: 再描画を行わない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerFlags(u32);

#[allow(clippy::new_without_default)]
impl LayerFlags {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn redraw_enable(&self) -> bool {
        self.0 & 1 == 0
    }

    pub fn set_redraw(mut self, redraw: bool) -> Self {
        if redraw {
            self.0 &= !0b1;
        } else {
            self.0 |= 0b1;
        }
        self
    }
}

/// 作成したウィンドウのレイヤー ID を返す。
/// ただし作成に失敗した場合は `0` を返す。
///
#[cfg_attr(
    feature = "alloc",
    doc = "`title` がヌル文字を含んでいた場合必ず失敗する。"
)]
#[cfg_attr(
    not(feature = "alloc"),
    doc = "`title` が終端のヌル文字を含めて 1024 バイトを超えた場合は `panic` を起こす。"
)]
pub fn open_window(w: i32, h: i32, x: i32, y: i32, title: impl Display) -> u32 {
    #[cfg(not(feature = "alloc"))]
    let res = {
        let mut buf = [0; 1024];
        let mut s = CStrBuf::new_unchecked(&mut buf);
        write!(s, "{}", title).unwrap();
        unsafe { syscall::__open_window(w as _, h as _, x as _, y as _, s.to_cstr().as_ptr() as _) }
    };

    #[cfg(feature = "alloc")]
    let res = {
        let s = format!("{}", title);
        let s = match CString::new(s) {
            Ok(s) => s,
            Err(_) => {
                ERRNO.store(ErrNo::EINVAL.into(), Relaxed);
                return 0;
            }
        };
        unsafe { syscall::__open_window(w as _, h as _, x as _, y as _, s.as_ptr() as _) }
    };

    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
        0
    } else {
        res.value as _
    }
}

#[cfg_attr(
    feature = "alloc",
    doc = "`title` がヌル文字を含んでいた場合なにもせずに終了する。"
)]
#[cfg_attr(
    not(feature = "alloc"),
    doc = "`title` が終端のヌル文字を含めて 1024 バイトを超えた場合は `panic` を起こす。"
)]
pub fn win_write_string(layer_id: u32, x: i32, y: i32, color: u32, s: impl Display) {
    #[cfg(not(feature = "alloc"))]
    let res = {
        let mut buf = [0; 1024];
        let mut buf = CStrBuf::new_unchecked(&mut buf);
        write!(buf, "{}", s).unwrap();
        unsafe {
            syscall::__win_write_string(
                layer_id as _,
                x as _,
                y as _,
                color as _,
                buf.to_cstr().as_ptr() as _,
            )
        }
    };

    #[cfg(feature = "alloc")]
    let res = {
        let s = format!("{}", s);
        let s = match CString::new(s) {
            Ok(s) => s,
            Err(_) => {
                ERRNO.store(ErrNo::EINVAL.into(), Relaxed);
                return;
            }
        };
        unsafe {
            syscall::__win_write_string(layer_id as _, x as _, y as _, color as _, s.as_ptr() as _)
        }
    };

    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
    }
}

#[cfg_attr(
    feature = "alloc",
    doc = "`title` がヌル文字を含んでいた場合なにもせずに終了する。"
)]
#[cfg_attr(
    not(feature = "alloc"),
    doc = "`title` が終端のヌル文字を含めて 1024 バイトを超えた場合は `panic` を起こす。"
)]
pub fn win_write_string_with_flags(
    layer_id: u32,
    x: i32,
    y: i32,
    color: u32,
    s: impl Display,
    flags: LayerFlags,
) {
    #[cfg(not(feature = "alloc"))]
    let res = {
        let mut buf = [0; 1024];
        let mut buf = CStrBuf::new_unchecked(&mut buf);
        write!(buf, "{}", s).unwrap();
        unsafe {
            syscall::__win_write_string(
                layer_id as u64 | (flags.0 as u64) << 32,
                x as _,
                y as _,
                color as _,
                buf.to_cstr().as_ptr() as _,
            )
        }
    };

    #[cfg(feature = "alloc")]
    let res = {
        let s = format!("{}", s);
        let s = match CString::new(s) {
            Ok(s) => s,
            Err(_) => {
                ERRNO.store(ErrNo::EINVAL.into(), Relaxed);
                return;
            }
        };
        unsafe {
            syscall::__win_write_string(
                layer_id as u64 | (flags.0 as u64) << 32,
                x as _,
                y as _,
                color as _,
                s.as_ptr() as _,
            )
        }
    };

    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
    }
}

pub fn win_fill_rectangle(layer_id: u32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    let res = unsafe {
        syscall::__win_fill_rectangle(layer_id as _, x as _, y as _, w as _, h as _, color as _)
    };
    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
    }
}

pub fn win_fill_rectangle_with_flags(
    layer_id: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
    flags: LayerFlags,
) {
    let res = unsafe {
        syscall::__win_fill_rectangle(
            layer_id as u64 | (flags.0 as u64) << 32,
            x as _,
            y as _,
            w as _,
            h as _,
            color as _,
        )
    };
    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
    }
}

pub fn win_redraw(layer_id: u32) {
    let res = unsafe { syscall::__win_redraw(layer_id as _) };
    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
    }
}

pub fn win_redraw_with_flags(layer_id: u32, flags: LayerFlags) {
    let res = unsafe { syscall::__win_redraw(layer_id as u64 | (flags.0 as u64) << 32) };
    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
    }
}

pub fn win_draw_line(layer_id: u32, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let res = unsafe {
        syscall::__win_draw_line(
            layer_id as u64,
            x0 as _,
            y0 as _,
            x1 as _,
            y1 as _,
            color as _,
        )
    };
    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
    }
}

pub fn win_draw_line_with_flags(
    layer_id: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
    flags: LayerFlags,
) {
    let res = unsafe {
        syscall::__win_draw_line(
            layer_id as u64 | (flags.0 as u64) << 32,
            x0 as _,
            y0 as _,
            x1 as _,
            y1 as _,
            color as _,
        )
    };
    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
    }
}

pub fn close_window(layer_id: u32) {
    unsafe { syscall::__close_window(layer_id as _) };
}
