use core::{
    mem,
    ops::{Index, IndexMut},
    ptr, slice,
};

use alloc::sync::Arc;

use crate::{
    asmfunc::{self, set_cr3},
    bitfield::BitField as _,
    error::{Code, Result},
    file::FileDescriptor,
    make_error,
    memory_manager::{FrameId, BYTES_PER_FRAME, MEMORY_MANAGER},
    sync::Mutex,
    task::{self, FileMapping, Task, TaskContext},
    terminal::APP_STACK_ADDR,
};

pub const PAGE_DIRECTORY_COUNT: usize = 64;

const PAGE_SIZE_4K: u64 = 4096;
const PAGE_SIZE_2M: u64 = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: u64 = 512 * PAGE_SIZE_2M;

static PML4_TABLE: Mutex<PageTable<u64, 512>> = Mutex::new(PageTable::<_, 512>::new(0));
static PDP_TABLE: Mutex<PageTable<u64, 512>> = Mutex::new(PageTable::<_, 512>::new(0));
static PAGE_DIRECTORY: Mutex<PageTable<[u64; 512], PAGE_DIRECTORY_COUNT>> =
    Mutex::new(PageTable::<_, PAGE_DIRECTORY_COUNT>::new([0; 512]));

pub fn init() {
    setup_indentity_page_table();
}

/// # Safety
///
/// 割り込みを禁止した状態で呼び出すこと
pub fn handle_page_fault(error_code: u64, causal_addr: u64) -> Result<()> {
    // 割り込み禁止状態で呼び出させるため、ここでは割り込みを禁止、許可しない
    let task = task::current_task();

    // P=1 なのでページレベルの権限違反でエラーが起きた
    if error_code.get_bit(0) {
        return Err(make_error!(Code::AlreadyAllocated));
    }
    if (task.dpaging_begin()..task.dpaging_end()).contains(&causal_addr)
        || (APP_STACK_ADDR - task.app_stack_size()..APP_STACK_ADDR).contains(&causal_addr)
    {
        return setup_page_maps(LinearAddress4Level { addr: causal_addr }, 1);
    }
    let file_maps = task.file_maps().lock_wait();
    if let Some(map) = find_file_mapping(&file_maps, causal_addr) {
        prepare_page_cache(
            task.files().lock_wait().get(&map.fd).unwrap(),
            map,
            causal_addr,
        )
    } else {
        Err(make_error!(Code::IndexOutOfRange))
    }
}

pub fn new_page_map() -> Result<&'static mut [PageMapEntry]> {
    let frame = MEMORY_MANAGER.allocate(1)?;
    unsafe { ptr::write_bytes(frame.frame(), 0, BYTES_PER_FRAME) };

    let e = unsafe {
        &mut *slice::from_raw_parts_mut(
            frame.frame() as _,
            BYTES_PER_FRAME / mem::size_of::<PageMapEntry>(),
        )
    };
    Ok(e)
}

pub fn set_new_page_map_if_not_present(
    entry: &mut PageMapEntry,
) -> Result<&'static mut [PageMapEntry]> {
    if entry.persent() {
        return Ok(entry.mut_pointer());
    }

    let child_map = new_page_map()?;
    entry.set_pointer(&child_map[0]);
    entry.set_present(true);

    Ok(child_map)
}

pub fn setup_page_map(
    page_map: &mut [PageMapEntry],
    page_map_level: i32,
    mut addr: LinearAddress4Level,
    mut num_4kpages: usize,
) -> Result<usize> {
    while num_4kpages > 0 {
        let entry_index = addr.part(page_map_level) as usize;

        let child_map = set_new_page_map_if_not_present(&mut page_map[entry_index])?;
        page_map[entry_index].set_writable(true);
        page_map[entry_index].set_user(true);

        if page_map_level == 1 {
            num_4kpages -= 1;
        } else {
            num_4kpages = setup_page_map(child_map, page_map_level - 1, addr, num_4kpages)?;
        }

        if entry_index == 511 {
            break;
        }

        addr.set_part(page_map_level, entry_index as u64 + 1);
        for level in 1..=page_map_level - 1 {
            addr.set_part(level, 0)
        }
    }

    Ok(num_4kpages)
}

pub fn setup_page_maps(addr: LinearAddress4Level, num_4kpages: usize) -> Result<()> {
    let pml4_table =
        unsafe { &mut *slice::from_raw_parts_mut(asmfunc::get_cr3() as *mut PageMapEntry, 512) };
    setup_page_map(pml4_table, 4, addr, num_4kpages)?;
    Ok(())
}

pub fn clean_page_maps(addr: LinearAddress4Level) {
    let pml4_table =
        unsafe { &mut *slice::from_raw_parts_mut(asmfunc::get_cr3() as *mut PageMapEntry, 512) };
    // PML4 テーブルの1つしかエントリがないことを仮定している
    let pdp_table = pml4_table[addr.pml4() as usize].mut_pointer();
    pml4_table[addr.pml4() as usize].data = 0;
    clean_page_map(pdp_table, 3);

    MEMORY_MANAGER.free(FrameId::from_addr(pdp_table.as_mut_ptr() as _), 1);
}

pub fn clean_page_map(page_maps: &mut [PageMapEntry], page_map_level: i32) {
    for entry in page_maps {
        if !entry.persent() {
            continue;
        }

        if page_map_level > 1 {
            clean_page_map(entry.mut_pointer(), page_map_level - 1);
        }

        let entry_ptr = entry.pointer().as_ptr();
        MEMORY_MANAGER.free(FrameId::from_addr(entry_ptr as _), 1);
        entry.data = 0;
    }
}

/// 新しい PML4 テーブルを作り、下位 256 ページ（OS 用）を元の PML4 テーブルからコピーする。
/// そしてそれを CR3 に設定し、新しい PML4 への排他参照を返す。
pub fn setup_pml4(current_task: &Arc<Task>) -> Result<&'static mut [PageMapEntry]> {
    let pml4 = new_page_map()?;

    let current_pml4_ptr = asmfunc::get_cr3();
    // PML4 の下位半分（OS 領域）のみをコピーする
    unsafe {
        ptr::copy_nonoverlapping(
            current_pml4_ptr as *const PageMapEntry,
            pml4.as_mut_ptr(),
            1 << 8,
        )
    };

    let cr3 = pml4.as_ptr() as _;
    asmfunc::set_cr3(cr3);

    // Safety: 実質最後の命令だけが unsafe
    //         だが、これはただの 64 bit のコピーで1命令にコンパイルされるはずなので、
    //         シングルコアでは途中で他の命令と競合することはない
    unsafe {
        #[allow(clippy::transmute_ptr_to_ref)]
        let ctx: &mut TaskContext = mem::transmute(current_task.context().as_ptr());
        ctx.cr3 = cr3;
    }
    Ok(pml4)
}

/// 現在の PML4 テーブルを削除し、OS 用の元の PML4 テーブルを CR3 に設定する。
pub fn free_pml4(current_task: &Arc<Task>) {
    let cr3 = current_task.context().cr3;
    // Safety: これも setup_pml4 と同じ
    unsafe {
        #[allow(clippy::transmute_ptr_to_ref)]
        let ctx: &mut TaskContext = mem::transmute(current_task.context().as_ptr());
        ctx.cr3 = 0;
    }
    reset_cr3();

    let frame = FrameId::from_addr(cr3 as _);
    MEMORY_MANAGER.free(frame, 1);
}

#[repr(align(4096))]
struct PageTable<T, const N: usize> {
    table: [T; N],
}

impl<T, const N: usize> PageTable<T, N> {
    fn len(&self) -> usize {
        N
    }

    fn as_ptr(&self) -> *const T {
        self.table.as_ptr()
    }
}

impl<T: Copy, const N: usize> PageTable<T, N> {
    const fn new(value: T) -> Self {
        Self { table: [value; N] }
    }
}

impl<T, const N: usize> Index<usize> for PageTable<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.table[index]
    }
}

impl<T, const N: usize> IndexMut<usize> for PageTable<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.table[index]
    }
}

pub fn setup_indentity_page_table() {
    let mut pml4_table = PML4_TABLE.lock_wait();
    let mut pdp_table = PDP_TABLE.lock_wait();
    let mut page_directory = PAGE_DIRECTORY.lock_wait();

    pml4_table[0] = pdp_table.as_ptr() as u64 | 0x003;

    for i_pdpt in 0..page_directory.len() {
        pdp_table[i_pdpt] = page_directory[i_pdpt].as_ptr() as u64 | 0x003;

        for i_pd in 0..page_directory[0].len() {
            page_directory[i_pdpt][i_pd] =
                (i_pdpt as u64 * PAGE_SIZE_1G + i_pd as u64 * PAGE_SIZE_2M) | 0x083;
        }
    }

    set_cr3(pml4_table.as_ptr() as u64);
}

pub fn reset_cr3() {
    set_cr3(PML4_TABLE.lock_wait().as_ptr() as _);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PageMapEntry {
    pub data: u64,
}

impl PageMapEntry {
    pub fn persent(&self) -> bool {
        self.data.get_bit(0)
    }

    pub fn set_present(&mut self, value: bool) {
        self.data.set_bit(0, value);
    }

    pub fn writable(&self) -> bool {
        self.data.get_bit(1)
    }

    pub fn set_writable(&mut self, value: bool) {
        self.data.set_bit(1, value);
    }

    pub fn user(&self) -> bool {
        self.data.get_bit(2)
    }

    pub fn set_user(&mut self, value: bool) {
        self.data.set_bit(2, value);
    }

    pub fn write_through(&self) -> bool {
        self.data.get_bit(3)
    }

    pub fn set_write_through(&mut self, value: bool) {
        self.data.set_bit(3, value);
    }

    pub fn cache_disable(&self) -> bool {
        self.data.get_bit(4)
    }

    pub fn set_cache_disable(&mut self, value: bool) {
        self.data.set_bit(4, value);
    }

    pub fn accessed(&self) -> bool {
        self.data.get_bit(5)
    }

    pub fn set_accessed(&mut self, value: bool) {
        self.data.set_bit(5, value);
    }

    pub fn dirty(&self) -> bool {
        self.data.get_bit(6)
    }

    pub fn set_dirty(&mut self, value: bool) {
        self.data.set_bit(6, value);
    }

    pub fn huge_page(&self) -> bool {
        self.data.get_bit(7)
    }

    pub fn set_huge_page(&mut self, value: bool) {
        self.data.set_bit(7, value);
    }

    pub fn global(&self) -> bool {
        self.data.get_bit(8)
    }

    pub fn set_global(&mut self, value: bool) {
        self.data.set_bit(8, value);
    }

    pub fn addr(&self) -> u64 {
        self.data.get_bits(12..52)
    }

    pub fn set_addr(&mut self, value: u64) {
        self.data.set_bits(12..52, value);
    }

    pub fn as_ptr(&self) -> *const Self {
        self as _
    }

    pub fn as_mut_ptr(&mut self) -> *mut Self {
        self as _
    }

    pub fn pointer(&self) -> &'static [Self] {
        unsafe { slice::from_raw_parts((self.addr() << 12) as *const _, 512) }
    }

    pub fn mut_pointer(&mut self) -> &'static mut [Self] {
        unsafe { &mut *(slice::from_raw_parts_mut((self.addr() << 12) as *mut _, 512)) }
    }

    pub fn set_pointer(&mut self, p: &Self) {
        self.set_addr((p.as_ptr() as u64) >> 12)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LinearAddress4Level {
    pub addr: u64,
}

impl LinearAddress4Level {
    pub fn offset(&self) -> u64 {
        self.addr.get_bits(0..11)
    }
    pub fn set_offset(&mut self, value: u64) {
        self.addr.set_bits(0..12, value)
    }

    pub fn page(&self) -> u64 {
        self.addr.get_bits(12..21)
    }
    pub fn set_page(&mut self, value: u64) {
        self.addr.set_bits(12..21, value)
    }

    pub fn dir(&self) -> u64 {
        self.addr.get_bits(21..30)
    }

    pub fn set_dir(&mut self, value: u64) {
        self.addr.set_bits(21..30, value)
    }

    pub fn pdp(&self) -> u64 {
        self.addr.get_bits(30..39)
    }

    pub fn set_pdp(&mut self, value: u64) {
        self.addr.set_bits(30..39, value)
    }

    pub fn pml4(&self) -> u64 {
        self.addr.get_bits(39..48)
    }

    pub fn set_pml4(&mut self, value: u64) {
        self.addr.set_bits(39..48, value)
    }

    pub fn rem(&self) -> u64 {
        self.addr.get_bits(48..)
    }

    pub fn set_rem(&mut self, value: u64) {
        self.addr.set_bits(48.., value)
    }

    pub fn part(&self, page_map_level: i32) -> u64 {
        match page_map_level {
            0 => self.offset(),
            1 => self.page(),
            2 => self.dir(),
            3 => self.pdp(),
            4 => self.pml4(),
            _ => 0,
        }
    }

    pub fn set_part(&mut self, page_map_level: i32, value: u64) {
        match page_map_level {
            0 => self.set_offset(value),
            1 => self.set_page(value),
            2 => self.set_dir(value),
            3 => self.set_pdp(value),
            4 => self.set_pml4(value),
            _ => {}
        }
    }
}

/// `fmaps` の中から `causal_addr` に対応している [FileMapping] を探す。
fn find_file_mapping(fmaps: &[FileMapping], causal_addr: u64) -> Option<&FileMapping> {
    fmaps
        .iter()
        .find(|m| (m.vaddr_begin..m.vaddr_end).contains(&causal_addr))
}

/// `map` と `causal_addr` に従って1ページ分のファイルの内容を `fd` からメモリにキャッシュする。
fn prepare_page_cache(fd: &FileDescriptor, map: &FileMapping, causal_addr: u64) -> Result<()> {
    let mut page_vaddr = LinearAddress4Level { addr: causal_addr };
    page_vaddr.set_offset(0);
    setup_page_maps(page_vaddr, 1)?;

    let file_offset = page_vaddr.addr - map.vaddr_begin;
    let page_cache =
        unsafe { slice::from_raw_parts_mut(page_vaddr.addr as *mut u8, BYTES_PER_FRAME) };
    fd.load(page_cache, file_offset as _);
    Ok(())
}
