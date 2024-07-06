use core::{
    alloc::{GlobalAlloc, Layout},
    mem::size_of,
    ptr,
};

use uefi::table::boot::MemoryMap;

use crate::{
    // asmfunc,
    bitfield::BitField as _,
    error::{Code, Result},
    make_error,
    memory_map,
    sync::{Mutex, RwLock},
};

/// メモリーマネージャー。
pub static MEMORY_MANAGER: BitmapMemoryManager = BitmapMemoryManager::new();

/// グローバルアロケータ。
#[global_allocator]
pub static GLOBAL: Global = Global::new();

const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;
const GIB: usize = 1024 * MIB;

/// 1フレームで取り扱うメモリのサイズ。
pub const BYTES_PER_FRAME: usize = 4 * KIB;

/// フレームを表す構造体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameId {
    id: usize,
}

impl FrameId {
    /// ID から [FrameId] を作る。
    pub const fn new(id: usize) -> Self {
        Self { id }
    }

    pub fn from_addr(addr: usize) -> Self {
        Self {
            id: addr / BYTES_PER_FRAME,
        }
    }

    /// ID を取得する。
    pub fn id(&self) -> usize {
        self.id
    }

    /// フレームの先頭へのポインタ。
    pub fn frame(&self) -> *mut u8 {
        (self.id * BYTES_PER_FRAME) as *mut u8
    }
}

/// ビットマップ配列の要素型。
type MapLineType = usize;

/// [BitmapMemoryManager] で管理できる最大の物理メモリのサイズ。
#[allow(clippy::manual_bits)]
const BITS_PER_MAP_LINE: usize = 8 * size_of::<MapLineType>();

/// [MAX_PHYSICAL_MEMORY] のメモリを管理するのに必要なフレーム数。
const MAX_PHYSICAL_MEMORY: usize = 128 * GIB;

/// ビットマップ配列の1要素のビット数 == 1要素で扱えるフレーム数
const FRAME_COUNT: usize = MAX_PHYSICAL_MEMORY / BYTES_PER_FRAME;

const UEFI_PAGE_SIZE: usize = 4 * KIB;

/// ビットを使ってメモリの使用可能領域を管理する構造体。
pub struct BitmapMemoryManager {
    /// フレームが使用可能かどうかを保持しておく。
    alloc_map: Mutex<[MapLineType; FRAME_COUNT / BITS_PER_MAP_LINE]>,
    /// 使用可能領域の最初のフレーム。
    range_begin: RwLock<FrameId>,
    /// 使用可能領域の最後のフレーム。
    range_end: RwLock<FrameId>,
    /// ロック。
    locker: Mutex<()>,
}

impl BitmapMemoryManager {
    /// [BitmapMemoryManager] を作る。
    const fn new() -> Self {
        Self {
            alloc_map: Mutex::new([0; FRAME_COUNT / BITS_PER_MAP_LINE]),
            range_begin: RwLock::new(FrameId::new(0)),
            range_end: RwLock::new(FrameId::new(0)),
            locker: Mutex::new(()),
        }
    }

    /// [MemoryMap] を元に [BitmapMemoryManager] を初期化する。
    ///
    /// * `memory_map` - メモリ情報。
    /// * `kernel_base` - カーネルが展開されたメモリの先頭。
    /// * `kernel_size` - 展開されたカーネルのサイズ。
    pub fn init(&self, memory_map: &MemoryMap, kernel_base: usize, kernel_size: usize) {
        // 同時に初期化されないようにロックを取得
        let _lock = self.locker.lock_wait();

        // 使用可能領域の最初が 0 でない場合は初期化済み
        if self.range_begin.read().id() != 0 {
            return;
        }

        let mut available_end = 0;
        for desc in memory_map.entries() {
            // available_end から desc.phys_start までは使用不可能領域のはずだから、
            // そこを割り当て済みとする
            if available_end < desc.phys_start as usize {
                self.mark_allocated(
                    FrameId::from_addr(available_end),
                    get_num_frames(desc.phys_start as usize - available_end),
                );
            }

            let phys_end = desc.phys_start as usize + desc.page_count as usize * UEFI_PAGE_SIZE;
            if memory_map::is_available(desc.ty) {
                available_end = phys_end;
            }
        }

        self.mark_allocated(FrameId::from_addr(kernel_base), get_num_frames(kernel_size));

        *self.range_begin.write() = FrameId::new(1);
        *self.range_end.write() = FrameId::from_addr(available_end);
    }

    pub fn allocate(&self, num_frames: usize) -> Result<FrameId> {
        // 他のスレッドが同時に空き領域を探して、
        // 空いていた領域を同時に割り当てないようにするため、
        // ロックを取得
        let _lock = self.locker.lock_wait();
        let mut start_frame_id = self.range_begin.read().id();
        loop {
            let mut i = 0;
            while i < num_frames {
                if start_frame_id + i >= self.range_end.read().id() {
                    return Err(make_error!(Code::NoEnoughMemory));
                }
                if self.is_allocated(FrameId::new(start_frame_id + i)) {
                    break;
                }
                i += 1;
            }

            if i == num_frames {
                self.mark_allocated(FrameId::new(start_frame_id), num_frames);
                return Ok(FrameId::new(start_frame_id));
            }

            start_frame_id += i + 1;
        }
    }

    pub fn free(&self, start_frame: FrameId, num_frames: usize) {
        for i in 0..num_frames {
            self.set_bit(FrameId::new(start_frame.id() + i), false);
        }
    }

    pub fn stat(&self) -> MemoryStat {
        let mut sum = 0;
        let map = self.alloc_map.lock_wait();
        let range_begin = *self.range_begin.read();
        let range_end = *self.range_end.read();
        for i in range_begin.id / BITS_PER_MAP_LINE..range_end.id / BITS_PER_MAP_LINE {
            sum += map[i].count_ones() as usize;
        }
        MemoryStat {
            allocated_frames: sum,
            total_frames: range_end.id - range_begin.id,
        }
    }

    /// あるフレームから数フレームを割り当て済みにする。
    ///
    /// * `start_frame` - 割り当て済みにする最初のフレーム。
    /// * `num_frames` - 割り当て済みにするフレームの数。
    fn mark_allocated(&self, start_frame: FrameId, num_frames: usize) {
        self.set_bits(start_frame, num_frames, true);
    }

    /// 指定されたフレームが割り当て済みかどうかを返す。
    ///
    /// * `frame` - 割り当て済みか判定するフレーム。
    fn is_allocated(&self, frame: FrameId) -> bool {
        let line_index = frame.id() / BITS_PER_MAP_LINE;
        let bit_index = frame.id() % BITS_PER_MAP_LINE;

        self.alloc_map.lock_wait()[line_index].get_bit(bit_index as u32)
    }

    /// 指定されたフレームが割り当て済みかどうかを変更する。
    ///
    /// * `frame` - 変更するフレーム。
    /// * `allocated` - 割り当て済みかどうか。
    fn set_bit(&self, frame: FrameId, allocated: bool) {
        let line_index = frame.id() / BITS_PER_MAP_LINE;
        let bit_index = frame.id() % BITS_PER_MAP_LINE;

        let mut map = self.alloc_map.lock_wait();
        map[line_index].set_bit(bit_index as u32, allocated);
    }

    /// 指定されたフレームから数フレームが割り当て済みかどうかを変更する。
    ///
    /// * `frame` - 最初のフレーム。
    /// * `num_frames` - 変更するフレームの数。
    /// * `allocated` - 割り当て済みかどうか。
    fn set_bits(&self, frame: FrameId, mut num_frames: usize, allocated: bool) {
        let allocated = if allocated { MapLineType::MAX } else { 0 };

        let mut line_index = frame.id() / BITS_PER_MAP_LINE;
        let mut bit_index = frame.id() % BITS_PER_MAP_LINE;

        let mut map = self.alloc_map.lock_wait();
        while num_frames > 0 {
            if bit_index + num_frames > BITS_PER_MAP_LINE {
                map[line_index].set_bits(bit_index as u32..BITS_PER_MAP_LINE as u32, allocated);
                num_frames -= BITS_PER_MAP_LINE - bit_index;
            } else {
                map[line_index]
                    .set_bits(bit_index as u32..(bit_index + num_frames) as u32, allocated);
                num_frames -= num_frames;
            }
            line_index += 1;
            bit_index = 0;
        }
    }
}

/// 割り当てられた領域の先頭から必要な領域を渡し、
/// 先頭をその分増加させいくだけのメモリ割り当てを行う。
pub struct Global {
    /// 現在の先頭アドレスを表す。
    cur: Mutex<usize>,
    /// 割り当てられる領域の末尾のアドレスを表す。
    end: Mutex<usize>,
}

impl Global {
    pub const fn new() -> Self {
        Self {
            cur: Mutex::new(0),
            end: Mutex::new(0),
        }
    }

    pub fn init(&self, num_frames: usize) {
        // Sefety: シングルコア時に初期化されるはずなのでロックは取らない
        if self.is_initialized() {
            panic!("initialize Global multiple times");
        }

        let start = match MEMORY_MANAGER.allocate(num_frames) {
            Ok(ptr) => ptr,
            Err(e) => panic!("{}", e),
        };

        *self.cur.lock_wait() = start.frame() as _;
        *self.end.lock_wait() = start.frame() as usize + num_frames * BYTES_PER_FRAME;
    }

    fn is_initialized(&self) -> bool {
        *self.cur.lock_wait() != 0
    }
}

unsafe impl GlobalAlloc for Global {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut cur = self.cur.lock_wait();

        let start = *cur;
        // アラインメント調整
        let start = match start % layout.align() {
            0 => start,
            rem => start + layout.align() - rem,
        };

        // 領域を超えないか確認
        let end = start + layout.size();
        if end >= *self.end.lock_wait() {
            return ptr::null_mut();
        }
        *cur = end;
        start as _
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

pub struct MemoryStat {
    pub allocated_frames: usize,
    pub total_frames: usize,
}

fn get_num_frames(size: usize) -> usize {
    (size + BYTES_PER_FRAME - 1) / BYTES_PER_FRAME
}
