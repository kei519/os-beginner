const COUNT_MAX: u32 = u32::MAX;

/// 割り込みの発生方法の設定を行うレジスタ。
const LVT_TIMER: *mut u32 = 0xfee0_0320 as *mut u32;

/// カウンタの初期値の初期値を保持するレジスタ。
const INITIAL_COUNT: *mut u32 = 0xfee0_0380 as *mut u32;

/// カウンタの現在値を保持しているレジスタ。
const CURRENT_COUNT: *mut u32 = 0xfee0_0390 as *mut u32;

/// カウンタの減少スピードの設定を行うレジスタ。
const DIVIDE_CONFIG: *mut u32 = 0xfee0_03e0 as *mut u32;

pub fn initialize_lapic_timer() {
    unsafe {
        *DIVIDE_CONFIG = 0b1011; // divide 1:1
        *LVT_TIMER = (0b001 << 16) | 32; // masked, one-shot
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
