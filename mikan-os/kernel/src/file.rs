use core::{
    cmp,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
};

use crate::fat::{self, DirectoryEntry};

#[derive(Clone)]
pub struct FileDescriptor {
    inner: InnerFileDescriptor,
}

impl FileDescriptor {
    pub fn new_fat(fat_entry: &'static DirectoryEntry) -> Self {
        Self {
            inner: InnerFileDescriptor::Fat {
                fat_entry,
                rd_off: 0,
                rd_cluster: fat_entry.first_cluster() as _,
                rd_cluster_off: 0,
            },
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        match self.inner {
            InnerFileDescriptor::Fat {
                fat_entry,
                ref mut rd_off,
                ref mut rd_cluster,
                ref mut rd_cluster_off,
            } => {
                let len = cmp::min(buf.len(), fat_entry.file_size as usize - *rd_off);
                let bytes_per_cluster = fat::BYTES_PER_CLUSTER.get() as usize;

                let mut total = 0;
                while total < len {
                    let n = cmp::min(len - total, bytes_per_cluster);
                    let sec = fat::get_sector_by_cluster::<u8>(*rd_cluster, n);
                    buf[total..total + n].copy_from_slice(sec);
                    total += n;

                    *rd_cluster_off += n;
                    if *rd_cluster_off == bytes_per_cluster {
                        *rd_cluster = fat::next_cluster(*rd_cluster);
                        *rd_cluster_off = 0;
                    }
                }

                total
            }
        }
    }
}

#[derive(Clone)]
enum InnerFileDescriptor {
    Fat {
        /// ファイルディスクリプタが指すファイルへの参照。
        fat_entry: &'static DirectoryEntry,
        /// ファイル先頭からの読み込みオフセット。
        rd_off: usize,
        /// `rd_off` が指す位置に対応するクラスタの番号。
        rd_cluster: u64,
        /// クラスタ先頭からのオフセット。
        rd_cluster_off: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFlags(i32);

impl FileFlags {
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    pub const ACCMODE: Self = Self(3);
    pub const RDONLY: Self = Self(0);
    pub const WRONLY: Self = Self(1);
    pub const RDWR: Self = Self(2);
}

impl From<FileFlags> for i32 {
    fn from(value: FileFlags) -> Self {
        value.0
    }
}

impl From<i32> for FileFlags {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl BitOr for FileFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for FileFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl BitAnd for FileFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for FileFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}

impl BitXor for FileFlags {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for FileFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0
    }
}

impl Not for FileFlags {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}