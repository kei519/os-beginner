use core::sync::atomic::Ordering;

use crate::{
    syscall::{self, SysResult},
    ERRNO,
};

/// タイマーから得られた情報を表す。
pub struct TimerInfo {
    /// タイマーカウント。
    pub tick: u64,
    /// 1秒間にタイマーが刻むカウントの数。
    pub freq: u64,
}

impl From<SysResult> for TimerInfo {
    fn from(value: SysResult) -> Self {
        Self {
            tick: value.value,
            freq: value.error as _,
        }
    }
}

pub fn get_current_tick() -> TimerInfo {
    unsafe { syscall::__get_current_tick() }.into()
}

/// タイマの動作モードを決める。
pub struct TimerMode(i32);

impl TimerMode {
    pub const fn new() -> Self {
        Self(0)
    }

    /// 相対モードかどうかを返す。
    pub fn is_relative(&self) -> bool {
        self.0 & 1 == 1
    }

    /// 相対モードかどうかを変更する。
    pub fn set_relative(mut self, is_relative: bool) -> Self {
        if is_relative {
            self.0 |= 1;
        } else {
            self.0 &= !1;
        }
        self
    }
}

impl Default for TimerMode {
    fn default() -> Self {
        Self::new()
    }
}

/// 設定されたタイマのタイムアウト時間（ms）を OS 起動時からの絶対時間で返す。
/// エラーが発生した場合は `0` を返す。
pub fn create_timer(mode: TimerMode, timer_value: i32, timeout_ms: u64) -> u64 {
    let res = unsafe { syscall::__create_timer(mode.0 as _, timer_value as _, timeout_ms as _) };
    if res.error != 0 {
        ERRNO.store(res.error, Ordering::Relaxed);
        0
    } else {
        res.value
    }
}
