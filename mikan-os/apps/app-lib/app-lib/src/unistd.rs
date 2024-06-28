use core::sync::atomic::Ordering;

use crate::{syscall::__put_string, ERRNO};

/// ファイルディスクリプタ `fd` に `buf` の内容を書き込む。
///
/// Linux の write システムコールをイメージしているが、
/// Rust で文字列長を渡す必要はないので要求しない。
pub fn write(fd: i32, buf: &str) -> isize {
    let res = unsafe { __put_string(fd as _, buf.as_ptr() as _, buf.len() as _) };
    if res.error == 0 {
        res.value as _
    } else {
        ERRNO.store(res.error, Ordering::Relaxed);
        -1
    }
}
