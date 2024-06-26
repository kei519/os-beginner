use core::{
    ffi::CStr,
    fmt::{Display, Write as _},
};

use crate::{buf::CStrBuf, syscall::*};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(C)]
pub enum LogLevel {
    Error = 3,
    Warn = 4,
    Info = 6,
    Debug = 7,
}

/// ヌル終端文字列 `s` を ログレベル `level` としてカーネルのログに表示する。
pub fn kernel_log_with_cst(level: LogLevel, s: &CStr) {
    unsafe { __log_string(level as _, s.as_ptr() as _) };
}

/// 1024 バイトのバッファを用意し、`content` を [Display] を使ってヌル終端文字列として
/// ログレベル `level` でカーネルのログに表示する。
///
/// 1024 バイトを超えた場合は `panic` を起こす。
pub fn kernel_log_with_format(level: LogLevel, content: impl Display) {
    let mut buf = [0; 1024];
    // 上で 1024 バイト確保しているから、失敗しない
    let mut s = CStrBuf::new_unchecked(&mut buf);
    write!(s, "{}", content).unwrap();
    unsafe { __log_string(level as _, s.to_cstr().as_ptr() as _) };
}

/// 1024 バイトのバッファを用いてフォーマットしたヌル終端文字列をカーネルログに表示する。
/// 1024 バイトを超えた場合は `panic` を起こす。
#[macro_export]
macro_rules! kernel_log {
    ($level:expr, $fmt:expr, $($args:tt)*) => {
        $crate::logger::kernel_log_with_format($level, ::core::format_args!($fmt, $($args)*));
    };
    ($level:expr, $fmt:expr) => {
        $crate::logger::kernel_log_with_format($level, $fmt);
    }
}
