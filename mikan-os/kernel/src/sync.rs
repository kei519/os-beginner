use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicIsize, Ordering::*},
};

/// スレッドセーフなら内部可変性を持つ構造体。
///
/// 1度に1つの排他参照しか作れない。
pub struct Mutex<T> {
    data: UnsafeCell<T>,
    lock: AtomicBool,
}

unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
            lock: AtomicBool::new(false),
        }
    }

    /// ロックを取得できれば取得した [MutexGuard] を返す。
    pub fn lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.lock.swap(true, Acquire) {
            None
        } else {
            Some(MutexGuard {
                data: unsafe { &mut *self.data.get() },
                lock: &self.lock,
            })
        }
    }

    /// ロックを取得できるまで待機する。
    pub fn lock_wait(&self) -> MutexGuard<'_, T> {
        loop {
            match self.lock() {
                Some(guard) => return guard,
                None => spin_loop(),
            }
        }
    }
}

/// 可変の static 変数として使える、実行時に初期化が可能な [Mutex]。
///
/// ただし、初期化していないアクセスは未定義動作を引き起こすので注意。
pub struct OnceMutex<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    lock: AtomicBool,
    is_initialized: AtomicBool,
}

unsafe impl<T: Send> Sync for OnceMutex<T> {}

impl<T> OnceMutex<T> {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            lock: AtomicBool::new(false),
            is_initialized: AtomicBool::new(false),
        }
    }

    pub const fn from_value(value: T) -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::new(value)),
            lock: AtomicBool::new(false),
            is_initialized: AtomicBool::new(true),
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized.load(Relaxed)
    }

    pub fn init(&self, value: T) -> bool {
        while self.lock.swap(true, Acquire) {
            spin_loop();
        }

        let ret = if self.is_initialized() {
            false
        } else {
            unsafe { (*self.data.get()).write(value) };
            self.is_initialized.store(true, Release);
            true
        };

        self.lock.store(false, Release);
        ret
    }

    /// ロックを取得できれば取得した [MutexGuard] を返す。
    pub fn lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.lock.swap(true, Acquire) {
            None
        } else {
            Some(MutexGuard {
                data: unsafe { (*self.data.get()).assume_init_mut() },
                lock: &self.lock,
            })
        }
    }

    /// ロックを取得できるまで待機する。
    pub fn lock_wait(&self) -> MutexGuard<'_, T> {
        loop {
            match self.lock() {
                Some(guard) => return guard,
                None => spin_loop(),
            }
        }
    }

    pub fn lock_checked(&self) -> Option<MutexGuard<'_, T>> {
        if self.is_initialized() {
            self.lock()
        } else {
            None
        }
    }

    pub fn lock_checked_wait(&self) -> Option<MutexGuard<'_, T>> {
        if self.is_initialized() {
            Some(self.lock_wait())
        } else {
            None
        }
    }
}

/// [Mutex]、[OnceMutex] のロック、間接参照を行う構造体。
pub struct MutexGuard<'this, T> {
    data: &'this mut T,
    lock: &'this AtomicBool,
}

unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.store(false, Release);
    }
}

/// [RwLock] 等が借用されていないときの `count` の値。
const UNUSED: isize = 0;
/// [RwLock] 等が共有参照される最大値。
const BORROW_MAX: isize = isize::MAX / 2;

/// スレッドセーフな内部可変性を持つ構造体。
///
/// 1度に1つの排他参照、または複数の共有参照を作れる。
pub struct RwLock<T> {
    /// 内部データを持つ。
    data: UnsafeCell<T>,
    /// 参照カウンタ。
    counter: AtomicIsize,
}

unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// [RwLock] のコンストラクタ。
    pub const fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
            counter: AtomicIsize::new(0),
        }
    }

    /// 読み取りのための共有参照を取得する。
    ///
    /// 参照が得られるまで無限に待機する。
    pub fn read(&self) -> ReadGuard<'_, T> {
        let mut c = self.counter.load(Relaxed);
        loop {
            if !(UNUSED..=BORROW_MAX).contains(&c) {
                spin_loop();
                c = self.counter.load(Relaxed);
                continue;
            }
            if let Err(e) = self
                .counter
                .compare_exchange_weak(c, c + 1, Acquire, Relaxed)
            {
                c = e;
                continue;
            }

            return ReadGuard {
                data: unsafe { &*self.data.get() },
                counter: &self.counter,
            };
        }
    }

    /// 書き込みのための排他参照を取得する。
    ///
    /// 参照が得られるまで無限に待機する。
    pub fn write(&self) -> WriteGuard<'_, T> {
        while self
            .counter
            .compare_exchange(0, 1, Acquire, Relaxed)
            .is_err()
        {
            spin_loop();
        }

        WriteGuard {
            data: unsafe { &mut *self.data.get() },
            counter: &self.counter,
        }
    }
}

/// 可変の static 変数として使える、実行時に初期化が可能な [RwLock]。
///
/// ただし、初期化していないアクセスは未定義動作を引き起こすので注意。
pub struct OnceRwLock<T> {
    /// 内部データを持つ。
    data: UnsafeCell<MaybeUninit<T>>,
    /// 参照カウンタ。
    counter: AtomicIsize,
    /// 初期化されているかどうかを表す。
    is_initialized: AtomicBool,
}

unsafe impl<T: Send + Sync> Sync for OnceRwLock<T> {}

impl<T> OnceRwLock<T> {
    /// [OnceMutex] のコンストラクタ。
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            counter: AtomicIsize::new(0),
            is_initialized: AtomicBool::new(false),
        }
    }

    /// [OnceMutex] の、初期化も行うコンストラクタ。
    ///
    /// * `value` - 設定する値。
    pub const fn from_value(value: T) -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::new(value)),
            counter: AtomicIsize::new(0),
            is_initialized: AtomicBool::new(true),
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized.load(Relaxed)
    }

    /// [OnceMutex] の初期化を行う。
    ///
    /// * `value` - 設定する値。
    ///
    /// # 戻り値
    /// 初期化できたかどうかを返す。
    ///
    /// 既に初期化されている場合は初期化されない。
    pub fn init(&self, value: T) -> bool {
        while self
            .counter
            .compare_exchange(0, -1, Acquire, Relaxed)
            .is_err()
        {
            spin_loop();
        }

        let ret = if self.is_initialized() {
            false
        } else {
            unsafe { (*self.data.get()).write(value) };
            self.is_initialized.store(true, Relaxed);
            true
        };

        self.counter.store(0, Release);
        ret
    }

    /// 読み取りのための共有参照を取得する。
    ///
    /// 参照が得られるまで無限に待機する。
    ///
    /// ただし、初期化されていない場合の動作は未定義。
    pub fn read(&self) -> ReadGuard<'_, T> {
        let mut c = self.counter.load(Relaxed);
        loop {
            if !(UNUSED..=BORROW_MAX).contains(&c) {
                spin_loop();
                c = self.counter.load(Relaxed);
                continue;
            }
            if let Err(e) = self
                .counter
                .compare_exchange_weak(c, c + 1, Acquire, Relaxed)
            {
                c = e;
                continue;
            }

            return ReadGuard {
                data: unsafe { (*self.data.get()).assume_init_ref() },
                counter: &self.counter,
            };
        }
    }

    /// 読み取りのための共有参照を取得する。
    ///
    /// [read()][Self::read()] との違いは、初期化が行われていない場合に未定義動作でなく、
    /// `None` が返ること。
    pub fn read_checked(&self) -> Option<ReadGuard<'_, T>> {
        if self.is_initialized() {
            Some(self.read())
        } else {
            None
        }
    }

    /// 書き込みのための排他参照を取得する。
    ///
    /// 参照が得られるまで無限に待機する。
    ///
    /// ただし、初期化されていない場合の動作は未定義。
    pub fn write(&self) -> WriteGuard<'_, T> {
        while self
            .counter
            .compare_exchange(UNUSED, -1, Acquire, Relaxed)
            .is_err()
        {
            spin_loop();
        }

        WriteGuard {
            data: unsafe { (*self.data.get()).assume_init_mut() },
            counter: &self.counter,
        }
    }

    /// 書き込みのための排他参照を取得する。
    ///
    /// [write()][Self::write()] との違いは、初期化が行われていない場合に未定義動作でなく、
    /// `None` が返ること。
    pub fn write_checked(&self) -> Option<WriteGuard<'_, T>> {
        if self.is_initialized() {
            Some(self.write())
        } else {
            None
        }
    }
}

/// [RwLock] を読み取るときに、そのデータと参照カウンタを管理するために使われる構造体。
pub struct ReadGuard<'this, T> {
    data: &'this T,
    counter: &'this AtomicIsize,
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Release);
    }
}

/// [RwLock] に書き込むときに、そのデータと参照カウンタを管理するために使われる構造体。
pub struct WriteGuard<'this, T> {
    data: &'this mut T,
    counter: &'this AtomicIsize,
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.counter.store(0, Release);
    }
}
