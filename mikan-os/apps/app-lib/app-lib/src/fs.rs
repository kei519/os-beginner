use core::{
    fmt::{Display, Write as _},
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
};

use crate::{buf::CStrBuf, errno::ErrNo, syscall};

type Result<T> = core::result::Result<T, ErrNo>;

pub fn open(path: impl Display, flags: FileFlags) -> Result<File> {
    let mut buf = [0; 1024];
    let mut buf = CStrBuf::new_unchecked(&mut buf);
    write!(buf, "{}", path).unwrap();
    let res = unsafe { syscall::__open_file(buf.to_cstr().as_ptr() as _, flags.0 as _) };
    if res.error != 0 {
        Err(res.error.into())
    } else {
        Ok(File(res.value as _))
    }
}

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

pub struct File(i32);

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let res = unsafe { syscall::__read_file(self.0 as _, buf.as_ptr() as _, buf.len() as _) };
        if res.error != 0 {
            Err(res.error.into())
        } else {
            Ok(res.value as _)
        }
    }
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
