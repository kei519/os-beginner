use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    cmp, hint, ptr,
    sync::atomic::{AtomicBool, Ordering::*},
};

use crate::{
    syscall::{self, SysResult},
    ERRNO,
};

#[cfg_attr(feature = "global_alloc", global_allocator)]
static GLOBAL: Global = Global::new();

pub struct Global {
    dpage_end: UnsafeCell<usize>,
    program_break: UnsafeCell<usize>,
    lock: AtomicBool,
}

unsafe impl Sync for Global {}

impl Global {
    pub const fn new() -> Self {
        Self {
            dpage_end: UnsafeCell::new(0),
            program_break: UnsafeCell::new(0),
            lock: AtomicBool::new(false),
        }
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn dpage_end(&self) -> &mut usize {
        &mut *self.dpage_end.get()
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn program_break(&self) -> &mut usize {
        &mut *self.program_break.get()
    }

    fn lock(&self) -> Lock<'_> {
        while self
            .lock
            .compare_exchange_weak(false, true, Acquire, Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }

        Lock { lock: &self.lock }
    }
}

unsafe impl GlobalAlloc for Global {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _lock = self.lock();
        let dpage_end = self.dpage_end();
        let program_break = self.program_break();

        if *dpage_end == 0 || *dpage_end < *program_break + layout.size() {
            let num_pages = (layout.size() + 4095) / 4096;

            let page_align = layout.align() / 4096;
            // page_align ページ確保すれば、そのなかに必ず align を満たしているアドレスが存在する
            let num_pages = cmp::max(num_pages, page_align);

            // 足りない場合はアラインメントをどうやっても満たせないからその隙間は無視する
            *program_break = match syscall::__demand_pages(num_pages as _) {
                SysResult { value, error: 0 } => value as _,
                SysResult { error, .. } => {
                    ERRNO.store(error, Relaxed);
                    return ptr::null_mut();
                }
            };
            *dpage_end = *program_break + num_pages * 4096;
        }

        // アラインメント調整
        match *program_break % layout.align() {
            0 => {}
            rem => *program_break += layout.align() - rem,
        }
        let prev_break = *program_break;
        *program_break += layout.size();
        prev_break as _
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // 解放は特になにもしない
    }
}

impl Default for Global {
    fn default() -> Self {
        Self::new()
    }
}

struct Lock<'a> {
    lock: &'a AtomicBool,
}

impl<'a> Drop for Lock<'a> {
    fn drop(&mut self) {
        self.lock.store(false, Relaxed);
    }
}
