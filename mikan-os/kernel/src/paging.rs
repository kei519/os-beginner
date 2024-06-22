use core::{
    ops::{Index, IndexMut},
    slice,
};

use crate::{asmfunc::set_cr3, bitfield::BitField as _, sync::Mutex};

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
