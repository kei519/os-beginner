use alloc::{boxed::Box, collections::VecDeque, sync::Arc, vec, vec::Vec};
use core::{
    arch::asm,
    mem, ptr,
    sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
};

use crate::{
    asmfunc::{self, restore_context},
    collections::HashMap,
    error::{Code, Result},
    file::FileDescriptor,
    make_error,
    message::Message,
    segment::{KERNEL_CS, KERNEL_SS},
    sync::Mutex,
    terminal::{DEFAULT_APP_STACK_SIZE, FILE_MAP_END},
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
        TASK_MANAGER
            .new_task()
            .set_level(TASK_MANAGER.current_level)
            .wake_up(-1);

        TASK_MANAGER
            .new_task()
            .init_context(task_idle, 0, 0)
            .set_level(0)
            .wake_up(-1);
    }

    let mut timer_manager = TIMER_MANAGER.lock_wait();
    let timeout = timer_manager.current_tick() + TASK_TIMER_PERIOD;
    timer_manager.add_timer(Timer::new(timeout, TASK_TIMER_VALUE, 1));
}

pub fn switch_task(current_ctx: &TaskContext) {
    unsafe { TASK_MANAGER.switch_task(current_ctx) };
}

pub fn new_task() -> &'static mut Task {
    unsafe { TASK_MANAGER.new_task() }
}

pub fn sleep(id: u64) -> Result<()> {
    unsafe { TASK_MANAGER.sleep(id) }
}

/// `level` が負の場合は前回のレベルのまま使われる。
pub fn wake_up(id: u64, level: i32) -> Result<()> {
    unsafe { TASK_MANAGER.wake_up(id, level) }
}

/// 今走っているタスクを返す。
///
/// すべてのタスクがスリープしている場合は `panic` を起こす。
/// つまり、割り込みハンドラから呼び出すべきではない。
pub fn current_task() -> Arc<Task> {
    unsafe { TASK_MANAGER.current_task() }
}

pub fn current_task_checked() -> Option<Arc<Task>> {
    unsafe { TASK_MANAGER.current_task_checked() }
}

/// ID が `id` のタスクが存在しなかった場合はエラーを返す。
pub fn send_message(id: u64, msg: Message) -> Result<()> {
    unsafe { TASK_MANAGER.send_message(id, msg) }
}

/// 現在のタスクを `exit_code` で終了させる。
/// 二度と戻ってこない。
pub fn finish(exit_code: i32) -> ! {
    unsafe { TASK_MANAGER.finish(exit_code) }
}

/// `task_id` のタスクが終了するのを待機し、終了したらその ExitCode を返す。
pub fn wait_finish(task_id: u64) -> Result<i32> {
    unsafe { TASK_MANAGER.wait_finish(task_id) }
}

pub fn get_task(task_id: u64) -> Option<Arc<Task>> {
    unsafe { TASK_MANAGER.get_task(task_id) }
}

#[no_mangle]
pub fn get_current_task_os_stack_pointer() -> u64 {
    *unsafe { TASK_MANAGER.current_task().os_stack_ptr() }
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
    pub rcx: u64,
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

pub struct Task<const STACK_SIZE: usize = { 8 * 4096 }> {
    id: u64,
    _stack: Box<Stack<STACK_SIZE>>,
    context: TaskContext,
    msgs: Mutex<VecDeque<Message>>,
    level: AtomicI32,
    running: AtomicBool,
    os_stack_ptr: u64,
    files: Mutex<HashMap<i32, Arc<Mutex<FileDescriptor>>>>,
    /// デマンドページングのアドレス範囲の起点。
    dpaging_begin: AtomicU64,
    /// デマンドページングのアドレス範囲の終点。
    dpaging_end: AtomicU64,
    app_stack_size: AtomicU64,
    file_map_end: AtomicU64,
    file_maps: Mutex<Vec<FileMapping>>,
}

impl<const STACK_SIZE: usize> Task<STACK_SIZE> {
    pub const DEFAULT_STACK_SIZE: usize = 4096;

    pub const STACK_SIZE: usize = STACK_SIZE;

    pub const DEFAULT_LEVEL: i32 = 1;

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
            msgs: Mutex::new(VecDeque::new()),
            level: Self::DEFAULT_LEVEL.into(),
            running: false.into(),
            os_stack_ptr: 0,
            files: Mutex::new(HashMap::new()),
            dpaging_begin: AtomicU64::new(0),
            dpaging_end: AtomicU64::new(0),
            app_stack_size: AtomicU64::new(DEFAULT_APP_STACK_SIZE),
            file_map_end: AtomicU64::new(FILE_MAP_END),
            file_maps: Mutex::new(vec![]),
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

    pub fn wake_up(&self, level: i32) -> &Self {
        // 終了しているタスクに送ろうとしても無視する
        let _ = unsafe { TASK_MANAGER.wake_up(self.id, level) };
        self
    }

    pub fn send_message(&self, msg: Message) {
        self.msgs.lock_wait().push_back(msg);
        self.wake_up(-1);
    }

    pub fn receive_message(&self) -> Option<Message> {
        self.msgs.lock_wait().pop_front()
    }

    pub fn os_stack_ptr(&self) -> &u64 {
        &self.os_stack_ptr
    }

    pub fn change_level_running(&self, level: i32) {
        unsafe { TASK_MANAGER.change_level_running(self.id, level) };
    }

    pub fn run_level(&self) -> i32 {
        self.level.load(Ordering::Relaxed)
    }

    pub fn files(&self) -> &Mutex<HashMap<i32, Arc<Mutex<FileDescriptor>>>> {
        &self.files
    }

    pub fn dpaging_begin(&self) -> u64 {
        self.dpaging_begin.load(Ordering::Relaxed)
    }

    pub fn set_dpaging_begin(&self, value: u64) {
        self.dpaging_begin.store(value, Ordering::Relaxed);
    }

    pub fn dpaging_end(&self) -> u64 {
        self.dpaging_end.load(Ordering::Relaxed)
    }

    pub fn set_dpaging_end(&self, value: u64) {
        self.dpaging_end.store(value, Ordering::Relaxed);
    }

    pub fn app_stack_size(&self) -> u64 {
        self.app_stack_size.load(Ordering::Relaxed)
    }

    pub fn set_app_stack_size(&self, value: u64) {
        self.app_stack_size.store(value, Ordering::Relaxed);
    }

    pub fn file_map_end(&self) -> u64 {
        self.file_map_end.load(Ordering::Relaxed)
    }

    pub fn set_file_map_end(&self, value: u64) {
        self.file_map_end.store(value, Ordering::Relaxed);
    }

    pub fn file_maps(&self) -> &Mutex<Vec<FileMapping>> {
        &self.file_maps
    }

    fn set_level(&self, level: i32) -> &Self {
        self.level.store(level, Ordering::Relaxed);
        self
    }

    fn set_running(&self, running: bool) -> &Self {
        self.running.store(running, Ordering::Relaxed);
        self
    }
}

impl<const N: usize> PartialEq for Task<N> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<const N: usize> Eq for Task<N> {}

pub const MAX_RUN_LEVEL: i32 = 3;

pub struct TaskManager {
    tasks: Vec<Arc<Task>>,
    latest_id: u64,
    running: [VecDeque<Arc<Task>>; MAX_RUN_LEVEL as usize + 1],
    current_level: i32,
    /// 次回のタスクスイッチ時にランレベルの見直しが必要かどうかを表す。
    level_changed: bool,
    /// key: 終了するのを待たれているタスクの ID。
    /// value: 終了するのを待っているタスクの ID。
    finish_waiter: HashMap<u64, u64>,
    /// key: 終了したタスクの ID。
    /// value: Exit Code。
    finish_tasks: HashMap<u64, i32>,
}

impl TaskManager {
    pub const MAX_LEVEL: i32 = MAX_RUN_LEVEL;

    const fn new() -> Self {
        Self {
            tasks: vec![],
            latest_id: 0,
            running: [
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
            ],
            current_level: MAX_RUN_LEVEL,
            level_changed: false,
            finish_waiter: HashMap::new(),
            finish_tasks: HashMap::new(),
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

    fn switch_task(&mut self, current_ctx: &TaskContext) {
        let current_task = self.rotete_current_run_queue(false);

        let current_task_ctx_addr = current_task.context() as *const _ as usize;
        unsafe { ptr::copy_nonoverlapping(current_ctx as _, current_task_ctx_addr as _, 1) };
        let next_task = self.current_task();

        if next_task != current_task {
            asmfunc::restore_context(next_task.context());
        }
    }

    /// ランキューの並び替えを行い、直前まで実行されていたタスクの [`Arc<Task>`][Arc] を返す。
    fn rotete_current_run_queue(&mut self, current_sleep: bool) -> Arc<Task> {
        let current_task = self.current_que_mut().pop_front().unwrap();
        if !current_sleep {
            self.current_que_mut().push_back(current_task.clone());
        }
        if self.current_que().is_empty() {
            self.level_changed = true;
        }

        if self.level_changed {
            self.level_changed = false;
            for lv in (0..=MAX_RUN_LEVEL).rev() {
                if !self.running[lv as usize].is_empty() {
                    self.current_level = lv;
                    break;
                }
            }
        }

        current_task
    }

    fn sleep(&mut self, id: u64) -> Result<()> {
        let task = match self.find_task_by_id(id) {
            Some(task) => task.clone(),
            None => return Err(make_error!(Code::NoSuchTask)),
        };

        task.set_running(false);

        // 現在実行中のタスクならタスクスイッチするだけで良い
        if self.current_task() == task {
            let current_task = self.rotete_current_run_queue(true);
            asmfunc::switch_context(self.current_task().context(), current_task.context());
            return Ok(());
        }

        erase(
            &mut self.running[task.level.load(Ordering::Relaxed) as usize],
            task.id,
        );

        Ok(())
    }

    /// `level` が負の場合は前回のレベルのまま使われる。
    fn wake_up(&mut self, id: u64, level: i32) -> Result<()> {
        let task = match self.find_task_by_id(id) {
            Some(task) => task,
            None => return Err(make_error!(Code::NoSuchTask)),
        };

        if task.running.load(Ordering::Relaxed) {
            self.change_level_running(id, level);
            return Ok(());
        }

        let level = if level < 0 {
            task.level.load(Ordering::Relaxed)
        } else {
            level
        };

        task.set_level(level);
        task.set_running(true);

        self.running[level as usize].push_back(task.clone());
        if level > self.current_level {
            // 次回タスクスイッチ時にランレベルの変更を行う。
            self.level_changed = true;
        }
        Ok(())
    }

    /// 今走っているタスクを返す。
    ///
    /// すべてのタスクがスリープしている場合は `panic` を起こす。
    /// つまり、割り込みハンドラから呼び出すべきではない。
    fn current_task(&self) -> Arc<Task> {
        self.current_que().front().unwrap().clone()
    }

    /// 今走っているタスクを返す。
    /// ない場合は `None` を返す。
    fn current_task_checked(&self) -> Option<Arc<Task>> {
        self.current_que().front().cloned()
    }

    /// ID が `id` のタスクが存在しなかった場合はエラーを返す。
    fn send_message(&self, id: u64, msg: Message) -> Result<()> {
        let task = match self.find_task_by_id(id) {
            Some(task) => task,
            None => return Err(make_error!(Code::NoSuchTask)),
        };

        task.send_message(msg);
        Ok(())
    }

    /// `level` が負の場合は元々のレベルを維持する（なにもしない）。
    /// # Remarks
    ///
    /// 存在しない `id` の場合は `panic` を起こす。
    fn change_level_running(&mut self, id: u64, level: i32) {
        let task = self.find_task_by_id(id).unwrap().clone();

        let task_level = task.level.load(Ordering::Relaxed);
        if level < 0 || level == task_level {
            return;
        }

        if task != *self.current_que().front().unwrap() {
            // レベルの変更
            erase(&mut self.running[task_level as usize], task.id);
            task.set_level(level);
            self.running[level as usize].push_back(task);
            if level > self.current_level {
                // レベルが上った場合は、最上位タスクの見直しを行う
                self.level_changed = true;
            }
            return;
        }

        // 上で先頭が今変更したいタスクなことが分かっているから、この unwrap は必ず成功
        let task = self.current_que_mut().pop_front().unwrap();
        task.set_level(level);
        self.running[level as usize].push_front(task.clone());
        if level >= self.current_level {
            self.current_level = level;
        } else {
            // レベルが下がった場合は、最上位タスクの見直しを行う
            self.current_level = level;
            self.level_changed = true;
        }
    }

    fn current_que(&self) -> &VecDeque<Arc<Task>> {
        &self.running[self.current_level as usize]
    }

    fn current_que_mut(&mut self) -> &mut VecDeque<Arc<Task>> {
        &mut self.running[self.current_level as usize]
    }

    fn find_task_by_id(&self, id: u64) -> Option<&Arc<Task>> {
        self.tasks.iter().find(|task| task.id == id)
    }

    fn finish(&mut self, exit_code: i32) -> ! {
        let current_task = self.rotete_current_run_queue(true);

        let task_id = current_task.id();
        // tasks に登録されていないタスクはないので unwrap() は必ず成功
        let index = self
            .tasks
            .iter()
            .position(|task| task.id == task_id)
            .unwrap();
        self.tasks.remove(index);

        self.finish_tasks.insert(task_id, exit_code);
        if let Some(waiter_id) = self.finish_waiter.remove(&task_id) {
            let _ = self.wake_up(waiter_id, -1);
        }

        restore_context(&self.current_task().context);
        unreachable!()
    }

    fn wait_finish(&mut self, task_id: u64) -> Result<i32> {
        let current_task = self.current_task();
        loop {
            if let Some(code) = self.finish_tasks.remove(&task_id) {
                return Ok(code);
            }
            self.finish_waiter.insert(task_id, current_task.id());
            // 今走っているタスクが登録されていないことはないので unwrap() は必ず成功
            self.sleep(current_task.id()).unwrap();
        }
    }

    fn get_task(&self, task_id: u64) -> Option<Arc<Task>> {
        self.tasks.iter().find(|task| task.id() == task_id).cloned()
    }
}

/// [`Arc<Task>`][Arc<Task>] の [VecDeque] から ID が `id` の [Task] を削除する。
///
/// `que` に存在しない `id` を指定した場合はなにもしない。
fn erase<const N: usize>(que: &mut VecDeque<Arc<Task<N>>>, id: u64) {
    if let Some(index) = que
        .iter()
        .enumerate()
        .find(|(_, task)| task.id == id)
        .map(|(id, _)| id)
    {
        que.remove(index);
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

fn task_idle(_: u64, _: i64, _: u32) {
    loop {
        unsafe { asm!("hlt") };
    }
}

#[derive(Debug, Clone)]
pub struct FileMapping {
    pub fd: i32,
    pub vaddr_begin: u64,
    pub vaddr_end: u64,
}
