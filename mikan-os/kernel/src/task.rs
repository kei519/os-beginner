use core::mem;

use alloc::{boxed::Box, collections::VecDeque, sync::Arc, vec, vec::Vec};

use crate::{
    asmfunc,
    error::{Code, Result},
    make_error,
    segment::{KERNEL_CS, KERNEL_SS},
    timer::{Timer, TASK_TIMER_PERIOD, TASK_TIMER_VALUE, TIMER_MANAGER},
};

/// [OnceMutex] や [Mutex] で持ちたいが、ロックを取得してからコンテキストスイッチをすると
/// ロックを取得したままコンテキストスイッチが起こり、以後コンテキストスイッチができなくなるため、
/// `static mut` で持つ。
///
/// # Safety
///
/// 同じタスクに対して同時に操作をしないこと。
static mut TASK_MANAGER: TaskManager = TaskManager::new();

/// それぞれのタスクで実行される関数を表す。
///
/// * task_id
/// * data
/// * layer_id - Window がない場合は `0`。
pub type TaskFunc = fn(u64, i64, u32);

pub fn init() {
    unsafe {
        TASK_MANAGER.new_task().wake_up();
    }

    let mut timer_manager = TIMER_MANAGER.lock_wait();
    let timeout = timer_manager.current_tick() + TASK_TIMER_PERIOD;
    timer_manager.add_timer(Timer::new(timeout, TASK_TIMER_VALUE));
}

pub fn switch_task(current_sleep: bool) {
    unsafe { TASK_MANAGER.switch_task(current_sleep) };
}

pub fn new_task() -> &'static mut Task {
    unsafe { TASK_MANAGER.new_task() }
}

pub fn sleep(id: u64) -> Result<()> {
    unsafe { TASK_MANAGER.sleep(id) }
}

pub fn wake_up(id: u64) -> Result<()> {
    unsafe { TASK_MANAGER.wake_up(id) }
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

    pub fn init_context(&mut self, f: TaskFunc, data: i64, layer_id: u32) -> &mut Self {
        self.context.cr3 = asmfunc::get_cr3();
        self.context.rflags = 0x202;
        self.context.cs = KERNEL_CS as u64;
        self.context.ss = KERNEL_SS as u64;
        self.context.rip = f as *const () as u64;
        self.context.rdi = self.id;
        self.context.rsi = data as u64;
        self.context.rdx = layer_id as u64;
        self
    }

    pub fn context(&self) -> &TaskContext {
        &self.context
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn sleep(&self) -> &Self {
        // TASK_MANAGER に登録されている Task しか呼べないはずなので OK
        unsafe { TASK_MANAGER.sleep(self.id) }.unwrap();
        self
    }

    pub fn wake_up(&self) -> &Self {
        // TASK_MANAGER に登録されている Task しか呼べないはずなので OK
        unsafe { TASK_MANAGER.wake_up(self.id) }.unwrap();
        self
    }
}

struct TaskManager {
    tasks: Vec<Arc<Task>>,
    latest_id: u64,
    running: VecDeque<Arc<Task>>,
}

impl TaskManager {
    const fn new() -> Self {
        Self {
            tasks: vec![],
            latest_id: 0,
            running: VecDeque::new(),
        }
    }

    fn new_task(&mut self) -> &mut Task {
        self.latest_id += 1;
        self.tasks.push(Arc::new(Task::new(self.latest_id)));
        // 今追加したばかりで、running にはまだ追加されていないから、この unwrap() は必ず成功する
        self.tasks
            .last_mut()
            .and_then(|task| Arc::get_mut(task))
            .unwrap()
    }

    fn switch_task(&mut self, current_sleep: bool) {
        let current_task = self.running.pop_front().unwrap();
        if !current_sleep {
            self.running.push_back(current_task.clone());
        }
        let next_task = self.running.front().unwrap();

        asmfunc::switch_context(next_task.context(), current_task.context());
    }

    fn sleep(&mut self, id: u64) -> Result<()> {
        // そもそも指定された id のタスクが存在するか確認
        if !self.tasks.iter().any(|task| task.id == id) {
            return Err(make_error!(Code::NoSuchTask));
        }

        match self
            .running
            .iter()
            .enumerate()
            .find(|(_, task)| task.id == id)
            .map(|(i, _)| i)
        {
            None => {}
            Some(index) => match index {
                0 => {
                    self.switch_task(true);
                }
                index => {
                    self.running.remove(index);
                }
            },
        };
        Ok(())
    }

    fn wake_up(&mut self, id: u64) -> Result<()> {
        let task = match self.tasks.iter().find(|task| task.id == id) {
            Some(task) => task,
            None => return Err(make_error!(Code::NoSuchTask)),
        };

        match self.running.iter().find(|task| task.id == id) {
            None => self.running.push_back(task.clone()),
            _ => {}
        }
        Ok(())
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
