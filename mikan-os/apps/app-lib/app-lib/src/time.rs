use crate::syscall::{self, SysResult};

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
