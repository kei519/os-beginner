use core::{
    cmp, mem,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
};

use alloc::sync::Arc;

use crate::{
    bitfield::BitField,
    error::Result,
    fat::{self, DirectoryEntry, BYTES_PER_CLUSTER, END_OF_CLUSTER_CHAIN},
    keyboard::{LCONTROL_BIT, RCONTROL_BIT},
    message::MessageType,
    task::Task,
    terminal::TerminalRef,
};

pub struct FileDescriptor {
    inner: InnerFileDescriptor,
}

impl FileDescriptor {
    pub fn new_fat(fat_entry: &'static mut DirectoryEntry) -> Self {
        let cluster = fat_entry.first_cluster() as _;
        Self {
            inner: InnerFileDescriptor::Fat {
                fat_entry,
                rd_off: 0,
                rd_cluster: cluster,
                rd_cluster_off: 0,
                wr_off: 0,
                wr_cluster: cluster,
                wr_cluster_off: 0,
            },
        }
    }

    pub fn new_term(task: Arc<Task>, term: TerminalRef) -> Self {
        Self {
            inner: InnerFileDescriptor::Terminal { task, term },
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        match self.inner {
            InnerFileDescriptor::Fat {
                ref fat_entry,
                ref mut rd_off,
                ref mut rd_cluster,
                ref mut rd_cluster_off,
                ..
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

                *rd_off += total;
                total
            }
            InnerFileDescriptor::Terminal {
                ref task,
                ref mut term,
            } => {
                loop {
                    // Task::recieve_message は Mutex でガードされているので、
                    // 割り込みは禁止しなくて良い
                    let msg = match task.receive_message() {
                        Some(m) => m,
                        None => {
                            task.sleep();
                            continue;
                        }
                    };
                    if let MessageType::KeyPush {
                        ascii,
                        press,
                        modifier,
                        keycode,
                    } = msg.ty
                    {
                        if !press {
                            continue;
                        }
                        if modifier.get_bit(LCONTROL_BIT) | modifier.get_bit(RCONTROL_BIT) {
                            let mut s = [b'^', 0];
                            s[1] = ascii.to_ascii_uppercase();
                            term.print(&s);
                            // D
                            if keycode == 7 {
                                // EOT
                                return 0;
                            }
                            continue;
                        }

                        buf[0] = ascii;
                        term.print(&buf[..1]);
                        return 1;
                    }
                }
            }
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self.inner {
            InnerFileDescriptor::Fat {
                ref mut fat_entry,
                ref mut wr_off,
                ref mut wr_cluster,
                ref mut wr_cluster_off,
                ref mut rd_cluster,
                ..
            } => {
                let bytes_per_cluster = BYTES_PER_CLUSTER.get() as _;
                let num_cluster = |bytes| (bytes + bytes_per_cluster - 1) / bytes_per_cluster;

                // コンストラクタで初期化しているので、ここが 0 なのは新規ファイルのみ
                if *wr_cluster == 0 {
                    *wr_cluster = fat::allocate_cluster_chain(num_cluster(buf.len()))?;
                    *rd_cluster = *wr_cluster;
                    fat_entry.set_first_cluster(*wr_cluster as _);
                }

                let mut total = 0;
                while total < buf.len() {
                    if *wr_cluster_off == bytes_per_cluster as _ {
                        let next_cluster = fat::next_cluster(*wr_cluster);
                        *wr_cluster = if next_cluster == END_OF_CLUSTER_CHAIN {
                            fat::extend_cluster(*wr_cluster, num_cluster(buf.len() - total))
                        } else {
                            next_cluster
                        };
                        *wr_cluster_off = 0;
                    }

                    let sec = fat::get_sector_by_cluster(*wr_cluster, bytes_per_cluster);
                    let n = cmp::min(buf.len() - total, bytes_per_cluster - *wr_cluster_off);
                    sec[*wr_cluster_off..*wr_cluster_off + n]
                        .copy_from_slice(&buf[total..total + n]);

                    total += n;
                    *wr_cluster_off += n;
                }

                *wr_off += buf.len();
                fat_entry.file_size = *wr_off as _;
                Ok(total)
            }
            InnerFileDescriptor::Terminal { ref mut term, .. } => {
                term.print(buf);
                Ok(buf.len())
            }
        }
    }

    pub fn size(&self) -> usize {
        match self.inner {
            InnerFileDescriptor::Fat { ref fat_entry, .. } => fat_entry.file_size as _,
            InnerFileDescriptor::Terminal { .. } => 0,
        }
    }

    pub fn load(&self, buf: &mut [u8], mut offset: usize) -> usize {
        match self.inner {
            InnerFileDescriptor::Terminal { .. } => 0,
            InnerFileDescriptor::Fat { ref fat_entry, .. } => {
                // ここで作った fat_entry は 'static ライフタイムを要求されるが、
                // 実際はこのスコープ内で落ちる変数に渡すだけなので問題ない
                // ただ排他参照にするのが大丈夫かは怪しい。
                // ただ他の方法が思いつかないので一旦これで
                let fat_entry: &mut DirectoryEntry = unsafe { mem::transmute_copy(fat_entry) };
                let rd_off = offset;

                let mut rd_cluster = fat_entry.first_cluster() as _;
                let bytes_per_cluster = BYTES_PER_CLUSTER.get() as _;
                while offset >= bytes_per_cluster {
                    offset -= bytes_per_cluster;
                    rd_cluster = fat::next_cluster(rd_cluster);
                }

                let inner = InnerFileDescriptor::Fat {
                    fat_entry,
                    rd_off,
                    rd_cluster,
                    rd_cluster_off: offset,
                    wr_off: 0,
                    wr_cluster: 0,
                    wr_cluster_off: 0,
                };
                let mut fd = Self { inner };
                fd.read(buf)
            }
        }
    }
}

enum InnerFileDescriptor {
    Fat {
        /// ファイルディスクリプタが指すファイルへの参照。
        fat_entry: &'static mut DirectoryEntry,
        /// 読み込みのファイル先頭からの読み込みオフセット。
        rd_off: usize,
        /// 読み込みの`rd_off` が指す位置に対応するクラスタの番号。
        rd_cluster: u64,
        /// 読み込みのクラスタ先頭からのオフセット。
        rd_cluster_off: usize,
        /// 書き込みのファイル先頭からのオフセット
        wr_off: usize,
        /// 書き込みのクラスタ番号
        wr_cluster: u64,
        /// 書き込みのクラスタ先頭からのオフセット
        wr_cluster_off: usize,
    },
    Terminal {
        task: Arc<Task>,
        term: TerminalRef,
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
    pub const CREAT: Self = Self(0o100);
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
