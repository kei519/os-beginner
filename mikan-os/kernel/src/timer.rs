use core::sync::atomic::{AtomicU64, Ordering};

use alloc::collections::BinaryHeap;

use crate::{
    acpi,
    interrupt::{self, InterruptVector, Message},
    sync::OnceMutex,
};

const COUNT_MAX: u32 = u32::MAX;

/// 割り込みの発生方法の設定を行うレジスタ。
const LVT_TIMER: *mut u32 = 0xfee0_0320 as *mut u32;

/// カウンタの初期値の初期値を保持するレジスタ。
const INITIAL_COUNT: *mut u32 = 0xfee0_0380 as *mut u32;

/// カウンタの現在値を保持しているレジスタ。
const CURRENT_COUNT: *mut u32 = 0xfee0_0390 as *mut u32;

/// カウンタの減少スピードの設定を行うレジスタ。
const DIVIDE_CONFIG: *mut u32 = 0xfee0_03e0 as *mut u32;

pub static TIMER_MANAGER: OnceMutex<TimerManager> = OnceMutex::new();

/// LAPIC タイマーの周波数。
pub static LAPIC_TIMER_FREQ: AtomicU64 = AtomicU64::new(0);

/// 1秒間に [TIMER_MANAGER] の `tick()` が発生する回数。
const TIMER_FREQ: u64 = 100;

pub fn init() {
    TIMER_MANAGER.init(TimerManager::new());
    unsafe {
        *DIVIDE_CONFIG = 0b1011; // divide 1:1
        *LVT_TIMER = 0b001 << 16; // masked, one-shot
    }

    // 100 ミリ秒の時間経過で LAPIC タイマのカウンタがどれだけ増えるか確認する
    start_lapic_timer();
    acpi::wait_milli_seconds(100);
    let elapsed = lapic_timer_elapsed();
    stop_lapic_timer();

    LAPIC_TIMER_FREQ.store(elapsed as u64 * 10, Ordering::Relaxed);

    unsafe {
        *DIVIDE_CONFIG = 0b1011; // divide 1:1
        *LVT_TIMER = (0b010 << 16) | InterruptVector::LAPICTimer as u32; // not-masked, periodic
        *INITIAL_COUNT = (LAPIC_TIMER_FREQ.load(Ordering::Relaxed) / TIMER_FREQ) as u32;
    }
}

/// Local APIC タイマーのカウントを開始する。
pub fn start_lapic_timer() {
    unsafe { *INITIAL_COUNT = COUNT_MAX };
}

/// Local APIC タイマーの経過時間を取得する。
pub fn lapic_timer_elapsed() -> u32 {
    unsafe { COUNT_MAX - *CURRENT_COUNT }
}

/// Local APIC タイマーのカウントを停止する。
pub fn stop_lapic_timer() {
    unsafe { *INITIAL_COUNT = 0 };
}

pub fn lapic_timer_on_interrupt() {
    if let Some(mut manager) = TIMER_MANAGER.lock() {
        manager.tick();
    }
}

#[derive(Debug, Default)]
pub struct TimerManager {
    tick: u64,
    timers: BinaryHeap<Timer>,
}

impl TimerManager {
    fn new() -> Self {
        Self {
            tick: 0,
            timers: BinaryHeap::new(),
        }
    }

    fn tick(&mut self) {
        self.tick += 1;
        loop {
            match self.timers.peek() {
                Some(t) if t.timeout() <= self.tick => {}
                // 先頭（最もタイムアウト時間が短いもの）がタイムアウトしていなければ、
                // 他のを見る必要はない
                _ => break,
            }
            let t = self.timers.pop().unwrap();

            let m = Message::TimerTimeout(t);
            interrupt::push_main_queue(m);
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    pub fn add_timer(&mut self, timer: Timer) {
        self.timers.push(timer);
    }
}

#[derive(Debug, Default, Clone)]
pub struct Timer {
    timeout: u64,
    value: i32,
}

impl Timer {
    pub fn new(timeout: u64, value: i32) -> Self {
        Self { timeout, value }
    }

    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    pub fn value(&self) -> i32 {
        self.value
    }
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.timeout == other.timeout
    }
}

impl Eq for Timer {}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // timeout カウント数が短い方が先にタイムアウトするから、
        // その順に並べる。
        self.timeout.cmp(&other.timeout).reverse()
    }
}
