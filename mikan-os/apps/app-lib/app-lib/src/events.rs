use core::sync::atomic::Ordering;

use crate::{syscall, ERRNO};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Null = 0,
    Quit,
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
