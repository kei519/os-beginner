use core::mem;

use alloc::{boxed::Box, vec, vec::Vec};

use crate::{
    asmfunc,
    segment::{KERNEL_CS, KERNEL_SS},
    timer::{Timer, TASK_TIMER_PERIOD, TASK_TIMER_VALUE, TIMER_MANAGER},
};

/// [OnceMutex] や [Mutex] で持ちたいが、ロックを取得してからコンテキストスイッチをすると
/// ロックを取得したままコンテキストスイッチが起こり、以後コンテキストスイッチができなくなるため、
/// `static mut` で持つ。
static mut TASK_MANAGER: TaskManager = TaskManager::new();

/// それぞれのタスクで実行される関数を表す。
///
/// * task_id
/// * data
/// * layer_id - Window がない場合は `0`。
pub type TaskFunc = fn(u64, i64, u32);

pub fn init() {
    unsafe { TASK_MANAGER.new_task() };

    let mut timer_manager = TIMER_MANAGER.lock_wait();
    let timeout = timer_manager.current_tick() + TASK_TIMER_PERIOD;
    timer_manager.add_timer(Timer::new(timeout, TASK_TIMER_VALUE));
}

pub fn switch_task() {
    unsafe { TASK_MANAGER.switch_task() };
}

pub fn new_task() -> &'static mut Task {
    unsafe { TASK_MANAGER.new_task() }
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

/// # Remarks
///
/// [SIZE] は16の倍数でないといけない。
#[repr(align(16))]
pub struct Stack<const SIZE: usize> {
    _buf: [u8; SIZE],
}

impl<const SIZE: usize> Stack<SIZE> {
    pub const fn new() -> Self {
        if SIZE % 16 != 0 {
            panic!("stack size must be a multiple of 16");
        }
        Self { _buf: [0; SIZE] }
    }

    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }

    pub fn end_ptr(&self) -> *const Self {
        unsafe { self.as_ptr().add(1) }
    }
}

impl<const SIZE: usize> Default for Stack<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Task<const STACK_SIZE: usize = 4096> {
    id: u64,
    _stack: Box<Stack<STACK_SIZE>>,
    context: TaskContext,
}

impl<const STACK_SIZE: usize> Task<STACK_SIZE> {
    pub const DEFAULT_STACK_SIZE: usize = 4096;

    pub const STACK_SIZE: usize = STACK_SIZE;

    /// スタックの確保を行い、その最後のアドレスを `self.context.rsp` に設定する。
    pub fn new(id: u64) -> Self {
        let stack = Box::new(Stack::new());
        let context = TaskContext {
            rsp: stack.end_ptr() as u64 - 8,
            ..TaskContext::new()
        };

        Self {
            id,
            _stack: stack,
            context,
        }
    }

    pub fn init_context(&mut self, f: TaskFunc, data: i64, layer_id: u32) {
        self.context.cr3 = asmfunc::get_cr3();
        self.context.rflags = 0x202;
        self.context.cs = KERNEL_CS as u64;
        self.context.ss = KERNEL_SS as u64;
        self.context.rip = f as *const () as u64;
        self.context.rdi = self.id;
        self.context.rsi = data as u64;
        self.context.rdx = layer_id as u64;
    }

    pub fn context(&self) -> &TaskContext {
        &self.context
    }
}

struct TaskManager {
    tasks: Vec<Task>,
    latest_id: u64,
    current_task_index: usize,
}

impl TaskManager {
    const fn new() -> Self {
        Self {
            tasks: vec![],
            latest_id: 0,
            current_task_index: 0,
        }
    }

    fn new_task(&mut self) -> &mut Task {
        self.latest_id += 1;
        self.tasks.push(Task::new(self.latest_id));
        // 今追加したばかりなので、この unwrap() は必ず成功する
        self.tasks.last_mut().unwrap()
    }

    fn switch_task(&mut self) {
        let next_task_index = match self.current_task_index + 1 {
            index if index >= self.tasks.len() => 0,
            index => index,
        };

        let current_task = &self.tasks[self.current_task_index];
        let next_task = &self.tasks[next_task_index];
        self.current_task_index = next_task_index;

        asmfunc::switch_context(next_task.context(), current_task.context());
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
