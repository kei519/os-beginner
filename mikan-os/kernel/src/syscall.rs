use core::{ffi::CStr, slice};

use crate::{
    asmfunc,
    errno::ErrNo,
    log,
    logger::LogLevel,
    msr::{IA32_EFER, IA32_FMASK, IA32_LSTAR, IA32_STAR},
    task, terminal,
};

pub type SyscallFuncType = extern "sysv64" fn(u64, u64, u64, u64, u64, u64) -> Result;

#[no_mangle]
pub static SYSCALL_TABLE: [SyscallFuncType; 3] = [log_string, put_string, exit];

pub fn init() {
    asmfunc::write_msr(IA32_EFER, 0x0501);
    asmfunc::write_msr(IA32_LSTAR, asmfunc::syscall_entry as usize as _);
    // [47:32] が syscall 時に設定されるセグメント
    // [64:48] が sysret 時に設定されるセグメント を決める
    asmfunc::write_msr(IA32_STAR, 8 << 32 | (16 | 3) << 48);
    asmfunc::write_msr(IA32_FMASK, 0);
}

#[repr(C)]
pub struct Result {
    value: u64,
    error: i32,
}

impl Result {
    fn new(value: u64, error: impl Into<i32>) -> Self {
        Self {
            value,
            error: error.into(),
        }
    }

    fn value(value: u64) -> Self {
        Self::new(value, 0)
    }

    fn error(error: impl Into<i32>) -> Self {
        Self::new(0, error)
    }
}

impl From<ErrNo> for Result {
    fn from(value: ErrNo) -> Self {
        Self::error(value)
    }
}

extern "sysv64" fn log_string(arg1: u64, arg2: u64, _: u64, _: u64, _: u64, _: u64) -> Result {
    let log_level: LogLevel = match arg1.try_into() {
        Ok(level) => level,
        Err(_) => return ErrNo::EPERM.into(),
    };

    let s = match unsafe { CStr::from_ptr(arg2 as _) }.to_str() {
        Ok(s) => s,
        Err(_) => return ErrNo::EINVAL.into(),
    };

    log!(log_level, "{}", s);
    Result::value(s.len() as _)
}

extern "sysv64" fn put_string(arg1: u64, arg2: u64, arg3: u64, _: u64, _: u64, _: u64) -> Result {
    let fd = arg1;
    let s: &[u8] = unsafe { slice::from_raw_parts(arg2 as _, arg3 as _) };

    if fd == 1 {
        let task_id = task::current_task().id();
        // システムコールを呼び出す可能性があるのは、ターミナル上で起動したアプリだけなので、
        // そのターミナルは必ず存在するため、unwrap は必ず成功する
        let mut terminal = terminal::get_term(task_id).unwrap();
        terminal.print(s);
        Result::value(s.len() as _)
    } else {
        ErrNo::EBADF.into()
    }
}

extern "sysv64" fn exit(arg1: u64, _: u64, _: u64, _: u64, _: u64, _: u64) -> Result {
    asmfunc::cli();
    let task = task::current_task();
    asmfunc::sti();
    Result::new(*task.os_stack_ptr(), arg1 as i32)
}
