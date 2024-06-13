use core::{
    any,
    cell::UnsafeCell,
    hint,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering::*},
};

/// 1度のみ実行時に初期化可能。
/// 起動時のはじめの方に初期化を行い、以後読み取りしか行わない静的変数として使うことを想定。
pub struct OnceStatic<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    lock: AtomicBool,
    is_initialized: AtomicBool,
}

unsafe impl<T: Send + Sync> Sync for OnceStatic<T> {}

impl<T> OnceStatic<T> {
    /// 初期化されてない状態を作成。
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            lock: AtomicBool::new(false),
            is_initialized: AtomicBool::new(false),
        }
    }

    /// これを使うくらいなら普通の静的変数を使えばよいと思うが、念の為作っておく。
    pub const fn from_value(value: T) -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::new(value)),
            lock: AtomicBool::new(true),
            is_initialized: AtomicBool::new(true),
        }
    }

    /// 初期化する。
    /// 1度しか呼び出さないこと。
    /// 2度目以降は `panic` を起こす。
    pub fn init(&self, value: T) {
        while !self.lock.swap(true, Relaxed) {
            hint::spin_loop();
        }

        if self.is_initialized.load(Relaxed) {
            panic!("{} is already initialized", any::type_name::<Self>());
        } else {
            unsafe { (*self.data.get()).write(value) };
            self.is_initialized.store(true, Release);
        };

        self.lock.store(false, Relaxed);
    }
}

impl<T> Default for OnceStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AsRef<T> for OnceStatic<T> {
    /// 参照を返す。
    /// デバッグじ以外は初期化済みかのチェックは行わない。
    fn as_ref(&self) -> &T {
        // デバッグ時は初期化漏れ変数を調べる
        #[cfg(debug_assertions)]
        if !self.is_initialized.load(Acquire) {
            panic!("{} is uninitialized", any::type_name::<Self>());
        }

        unsafe { (*self.data.get()).assume_init_ref() }
    }
}

impl<T: Copy> OnceStatic<T> {
    /// [Copy] を実装している `T` に関してはその参照をコピーしたものを返す。
    pub fn get(&self) -> T {
        *self.as_ref()
    }
}
