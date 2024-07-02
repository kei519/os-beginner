use core::sync::atomic::Ordering;

use crate::{syscall, ERRNO};

#[repr(C, i32)]
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Null = 0,
    Quit,
    MouseMove {
        x: i32,
        y: i32,
        dx: i32,
        dy: i32,
        buttons: u8,
    },
    MouseButton {
        x: i32,
        y: i32,
        press: bool,
        button: u8,
    },
}

impl AppEvent {
    pub fn discripinant(&self) -> i32 {
        unsafe { *(self as *const _ as *const i32) }
    }
}

impl Default for AppEvent {
    fn default() -> Self {
        Self::Null
    }
}

/// 失敗した場合は `0` を返す。
pub fn read_event(events: &mut [AppEvent]) -> usize {
    let res = unsafe { syscall::__read_event(events.as_ptr() as _, events.len() as _) };
    if res.error != 0 {
        ERRNO.store(res.error, Ordering::Relaxed);
        0
    } else {
        res.value as _
    }
}
