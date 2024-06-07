//! ACPI テーブル定義や操作用プログラムを集めたファイル。

use core::str;

use crate::{
    error::{Code, Result},
    log,
    logger::LogLevel,
    make_error,
};

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
            Err(make_error!(Code::InvalidFormat))
        } else {
            Ok(())
        }
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
}

fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    sum_bytes_u8(data as *const _ as *const u8, bytes)
}

fn sum_bytes_u8(data: *const u8, bytes: usize) -> u8 {
    (0..bytes).fold(0, |acc, offset| acc + unsafe { *data.add(offset) })
}
