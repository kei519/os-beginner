use alloc::{format, string::String};
use core::{
    ffi::{c_char, CStr},
    mem::size_of,
    panic,
};

use crate::{printk, sync::RwLock};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(C)]
pub enum LogLevel {
    Error = 3,
    Warn = 4,
    Info = 6,
    Debug = 7,
}

pub static LOG_LEVEL: RwLock<LogLevel> = RwLock::new(LogLevel::Warn);

pub fn set_log_level(level: LogLevel) {
    *LOG_LEVEL.write() = level;
}

pub fn get_log_level() -> LogLevel {
    *LOG_LEVEL.read()
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        if $level <= $crate::logger::get_log_level() {
            $crate::printkln!("{}", format_args!($($arg)*));
        }
    }
}

/// C++ 側から呼び出されるログ出力関数。
///
/// # Note
/// 対応しているフォーマット指定子は以下の通り。
///
/// - `%s`: 文字列
/// - `%d`: 10進整数
/// - `%x`: 16進整数
/// - `%u`: 符号なし10進整数
/// - `%c`: 文字
/// - `%p`: ポインタ
///
/// 必要に応じて今後増加させる。
///
/// また、本来は `AL` レジスタが引数に使用されているベクターレジスタの数を保存しているが、
/// `AL` レジスタを取得するのは大変なため、浮動小数点数は非対応。
///
/// # Safety
///
/// この関数は C++ 側から呼ばれることを想定しているため、Rust 側では使わないこと。
#[export_name = "_Z3Log8LogLevelPKcz"]
pub unsafe extern "sysv64" fn log_cpp(
    level: LogLevel,
    format: *const c_char,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    args: u64,
) -> i32 {
    // ログレベル以下のログは出力しない
    if level > get_log_level() {
        return 0;
    }

    let s = unsafe { CStr::from_ptr(format) }
        .to_str()
        .expect("Can't transform.");

    // スタックに積まれた引数のポインタ
    let p_args = &args as *const u64;

    // 既に読んだ引数の数
    let mut num_read = 0;

    // `%` のあとの文字によっては引数を変換する必要があるため、
    // `%` のあとであるかを管理する
    let mut arg_maybe_needed = false;

    // 0 埋めの有無
    let mut padding = false;

    let mut digit = 0;

    // 最終的に表示する文字列
    let mut str = String::with_capacity(s.len());
    for c in s.chars() {
        if arg_maybe_needed {
            // 引数を u64 として取得
            let arg = match num_read {
                0 => arg1,
                1 => arg2,
                2 => arg3,
                3 => arg4,
                // 7 つ目以降の引数はスタックに積まれているため、ポインタを進めて取得
                // pointer.add() は最適化されて（？）使えないため、pointer.byte_add() を使用
                i => unsafe { *p_args.byte_add((i - 4) * size_of::<u64>()) },
            };

            match c {
                // 文字列
                's' => {
                    let s = unsafe { CStr::from_ptr(arg as *const c_char) }
                        .to_str()
                        .unwrap();
                    str.push_str(s);
                }
                // 10進整数
                'd' => {
                    if padding {
                        if digit != 0 {
                            str.push_str(&format!("{:0digit$}", arg as i64));
                        } else {
                            str.push_str(&format!("{:0}", arg as i64));
                        }
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if digit != 0 {
                            str.push_str(&format!("{:digit$}", arg as i64));
                        } else {
                            str.push_str(&format!("{}", arg as i64));
                        }
                    }
                }
                // 16進整数
                'x' => {
                    if padding {
                        if digit != 0 {
                            str.push_str(&format!("{:0digit$x}", arg as i64));
                        } else {
                            str.push_str(&format!("{:0x}", arg as i64));
                        }
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if digit != 0 {
                            str.push_str(&format!("{:digit$x}", arg as i64));
                        } else {
                            str.push_str(&format!("{:x}", arg as i64));
                        }
                    }
                }
                // 符号なし10進整数
                'u' => {
                    if padding {
                        if digit != 0 {
                            str.push_str(&format!("{:0digit$}", arg));
                        } else {
                            str.push_str(&format!("{:0}", arg));
                        }
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if digit != 0 {
                            str.push_str(&format!("{:digit$}", arg));
                        } else {
                            str.push_str(&format!("{}", arg));
                        }
                    }
                }
                // 文字
                'c' => str.push(arg as u8 as char),
                // ポインタ
                'p' => str.push_str(&format!("{:p}", arg as *const u8)),
                // % のエスケープ
                '%' => {
                    arg_maybe_needed = false;
                    str.push('%');
                    continue;
                }
                // C では long だが、Rust（特にこの実装）では関係ない
                'l' => continue,
                '0' => {
                    padding = true;
                    continue;
                }
                d @ '1'..='9' => {
                    digit = digit * 10 + (d as u8 - 0x30) as usize;
                    continue;
                }
                // 非対応
                _ => panic!("Unknown format specifier: %{}", c),
            }
            arg_maybe_needed = false;
            padding = false;
            digit = 0;
            num_read += 1;
        } else {
            #[allow(clippy::collapsible_if)]
            if c == '%' {
                arg_maybe_needed = true;
            } else {
                str.push(c);
            }
        }
    }

    // 元々の関数が改行コードを勝手に付けないため、こちらもそのようにする
    printk!("{}", str);

    0
}
