//! ACPI テーブル定義や操作用プログラムを集めたファイル。

use core::{marker::PhantomData, mem, ops::Index, ptr, str};

use crate::{
    asmfunc,
    bitfield::BitField as _,
    error::{Code, Result},
    log,
    logger::LogLevel,
    make_error,
    sync::OnceMutex,
};

pub static FADT: OnceMutex<&'static FADT> = OnceMutex::new();

const PM_TIMER_FREQ: u32 = 3579545;

/// RSDP（Root System Description Pointer）
#[repr(packed)]
#[derive(Debug, Clone)]
pub struct RSDP {
    sig: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    revision: u8,
    pub rsdt_addr: u32,
    pub length: u32,
    pub xsdt_addr: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

impl RSDP {
    pub fn init(&self) -> Result<()> {
        if !self.is_valid() {
            log!(LogLevel::Error, "RSDP is not valid");
            return Err(make_error!(Code::InvalidFormat));
        }

        let xsdt = self.xsdt();
        if !xsdt.header.is_valid(b"XSDT") {
            log!(LogLevel::Error, "XSDT is not valid");
            return Err(make_error!(Code::InvalidFormat));
        }

        for i in 0..xsdt.count() {
            let entry = &xsdt[i];
            if entry.is_valid(b"FACP") {
                FADT.init(unsafe { &*(entry as *const _ as *const FADT) });
                break;
            }
        }

        if !FADT.is_initialized() {
            log!(LogLevel::Error, "FADT is not found");
            return Err(make_error!(Code::InvalidFormat));
        }

        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        if &self.sig != b"RSD PTR " {
            log!(
                LogLevel::Debug,
                "invalid signature: {}",
                str::from_utf8(&self.sig).unwrap()
            );
            return false;
        }
        if self.revision != 2 {
            log!(LogLevel::Debug, "ACPI version must be 2: {}", self.revision);
            return false;
        }
        match sum_bytes(self, 20) {
            0 => {}
            sum => {
                log!(LogLevel::Debug, "sum of 20 bytes must be 0: {}", sum);
                return false;
            }
        }
        match sum_bytes(self, 36) {
            0 => {}
            sum => {
                log!(LogLevel::Debug, "sum of 36 bytes must be 0: {}", sum);
                return false;
            }
        }
        true
    }

    pub fn xsdt(&self) -> &'static XSDT {
        let ptr = unsafe { ptr::read_unaligned(ptr::addr_of!(self.xsdt_addr)) };
        unsafe { &*(ptr as *const XSDT) }
    }
}

#[repr(packed)]
#[derive(Debug)]
pub struct DescriptionHeader {
    pub sig: [u8; 4],
    pub len: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

impl DescriptionHeader {
    pub fn is_valid(&self, expected_sig: &[u8; 4]) -> bool {
        if &self.sig != expected_sig {
            log!(
                LogLevel::Debug,
                "invalid signature: {}",
                str::from_utf8(expected_sig).unwrap()
            );
            return false;
        }
        match sum_bytes(self, self.len as usize) {
            0 => {}
            sum => {
                log!(
                    LogLevel::Debug,
                    "sum of {} bytes must be 0: {}",
                    self.len(),
                    sum
                );
                return false;
            }
        }
        true
    }

    fn len(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.len)) }
    }
}

/// XSDT（Extended System Descriptor Table）
///
/// XSDT 自体の情報を表すヘッダのあとに、各データ構造へのアドレスが並ぶ。
#[repr(packed)]
pub struct XSDT {
    pub header: DescriptionHeader,
    /// 実際は [DescriptionHeader] へのアドレスが `self.header.len()` に対応した数だけ並んでいる。
    addrs: PhantomData<()>,
}

impl XSDT {
    /// XSDT が持つデータへのポインタの数。
    pub fn count(&self) -> usize {
        (self.header.len() as usize - mem::size_of::<DescriptionHeader>()) / mem::size_of::<u64>()
    }
}

impl Index<usize> for XSDT {
    type Output = DescriptionHeader;
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.count() {
            panic!("out of index");
        }

        unsafe {
            let entry_addr_ptr =
                ptr::addr_of!(self.addrs).byte_add(index * mem::size_of::<u64>()) as *const u64;
            let entry_addr = ptr::read_unaligned(entry_addr_ptr) as usize;

            &*(entry_addr as *const DescriptionHeader)
        }
    }
}

#[repr(packed)]
pub struct FADT {
    pub header: DescriptionHeader,
    pub reserved1: [u8; 76 - mem::size_of::<DescriptionHeader>()],
    pub pm_tmr_blk: u32,
    pub reserved2: [u8; 112 - 80],
    pub flags: u32,
    pub reserved3: [u8; 276 - 116],
}

impl FADT {
    fn pm_tmr_blk(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.pm_tmr_blk)) }
    }

    fn flags(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.flags)) }
    }
}

/// `msec` ミリ秒待機する。
pub fn wait_milli_seconds(msec: u64) {
    let fadt = FADT.lock_wait();
    let pm_timer_32 = fadt.flags().get_bit(8);
    let pm_tmr_blk = fadt.pm_tmr_blk() as u16;

    let start = asmfunc::io_in_32(pm_tmr_blk);
    let end = start + PM_TIMER_FREQ * msec as u32 / 1000;
    let end = if pm_timer_32 { end } else { end.get_bits(..24) };

    if end < start {
        // overflow
        while asmfunc::io_in_32(pm_tmr_blk) >= start {}
    }
    while asmfunc::io_in_32(pm_tmr_blk) < end {}
}

fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    sum_bytes_u8(data as *const _ as *const u8, bytes)
}

fn sum_bytes_u8(data: *const u8, bytes: usize) -> u8 {
    (0..bytes).fold(0, |acc, offset| acc + unsafe { *data.add(offset) })
}
