use core::{mem, ptr};

use crate::{
    asmfunc,
    timer::{Timer, TASK_TIMER_PERIOD, TASK_TIMER_VALUE, TIMER_MANAGER},
};

pub static TASK_A_CTX: TaskContext = TaskContext::new();
pub static mut TASK_B_CTX: TaskContext = TaskContext::new();

static mut CURRENT_TASK: Option<&'static TaskContext> = None;

pub fn init() {
    unsafe { CURRENT_TASK = Some(&TASK_A_CTX) };
    let mut manager = TIMER_MANAGER.lock_wait();
    let timeout = manager.current_tick() + TASK_TIMER_PERIOD;
    manager.add_timer(Timer::new(timeout, TASK_TIMER_VALUE));
}

pub fn switch_task() {
    unsafe {
        let old_task = CURRENT_TASK.unwrap();
        CURRENT_TASK = Some(if ptr::eq(old_task, &TASK_A_CTX) {
            &*ptr::addr_of!(TASK_B_CTX)
        } else {
            &TASK_A_CTX
        });

        asmfunc::switch_context(CURRENT_TASK.unwrap(), old_task);
    }
}

#[repr(C, align(16))]
#[derive(Debug)]
pub struct TaskContext {
    pub cr3: u64,
    pub rip: u64,
    pub rflags: u64,
    pub reserved1: u64,
    pub cs: u64,
    pub ss: u64,
    pub fs: u64,
    pub gs: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcs: u64,
    pub rdx: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub fxsafe_area: [u8; 512],
}

impl TaskContext {
    pub const fn new() -> Self {
        unsafe { mem::zeroed() }
    }

    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }

    pub fn as_mut_ptr(&mut self) -> *mut Self {
        self as *mut _
    }
}

impl Default for TaskContext {
    fn default() -> Self {
        Self::new()
    }
}
