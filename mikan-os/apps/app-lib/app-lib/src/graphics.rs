use core::{
    fmt::{Display, Write as _},
    sync::atomic::Ordering::Relaxed,
};

use crate::{buf::CStrBuf, *};

/// ウィンドウ描画時のフラグを表す。
//  0 bit: 再描画を行わない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerFlags(u32);

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
pub fn open_window(w: i32, h: i32, x: i32, y: i32, title: impl Display) -> u32 {
    let mut buf = [0; 1024];
    let mut s = CStrBuf::new_unchecked(&mut buf);
    write!(s, "{}", title).unwrap();
    let res = unsafe {
        syscall::__open_window(w as _, h as _, x as _, y as _, s.to_cstr().as_ptr() as _)
    };
    if res.error != 0 {
        ERRNO.store(res.error, Relaxed);
        0
    } else {
        res.value as _
    }
}

pub fn win_write_string(layer_id: u32, x: i32, y: i32, color: u32, s: impl Display) {
    let mut buf = [0; 1024];
    let mut buf = CStrBuf::new_unchecked(&mut buf);
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
        ERRNO.store(res.error, Relaxed);
    }
}

pub fn win_write_string_with_flags(
    layer_id: u32,
    x: i32,
    y: i32,
    color: u32,
    s: impl Display,
    flags: LayerFlags,
) {
    let mut buf = [0; 1024];
    let mut buf = CStrBuf::new_unchecked(&mut buf);
    write!(buf, "{}", s).unwrap();
    let res = unsafe {
        syscall::__win_write_string(
            layer_id as u64 | (flags.0 as u64) << 32,
            x as _,
            y as _,
            color as _,
            buf.to_cstr().as_ptr() as _,
        )
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
