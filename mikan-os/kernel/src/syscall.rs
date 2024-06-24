use core::ffi::CStr;

use crate::{
    asmfunc, log,
    logger::LogLevel,
    msr::{IA32_EFER, IA32_FMASK, IA32_LSTAR, IA32_STAR},
    util::OnceStatic,
};

pub type SyscallFuncType = extern "sysv64" fn(u64, u64, u64, u64, u64, u64) -> i64;

#[no_mangle]
pub static SYSCALL_TABLE: OnceStatic<[SyscallFuncType; 1]> = OnceStatic::new();

pub fn init() {
    SYSCALL_TABLE.init([log_string]);

    asmfunc::write_msr(IA32_EFER, 0x0501);
    asmfunc::write_msr(IA32_LSTAR, asmfunc::syscall_entry as usize as _);
    // [47:32] が syscall 時に設定されるセグメント
    // [64:48] が sysret 時に設定されるセグメント を決める
    asmfunc::write_msr(IA32_STAR, 8 << 32 | (16 | 3) << 48);
    asmfunc::write_msr(IA32_FMASK, 0);
}

extern "sysv64" fn log_string(arg1: u64, arg2: u64, _: u64, _: u64, _: u64, _: u64) -> i64 {
    let log_level: LogLevel = match arg1.try_into() {
        Ok(level) => level,
        Err(_) => return -1,
    };

    let s = match unsafe { CStr::from_ptr(arg2 as _) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    log!(log_level, "{}", s);
    0
}
