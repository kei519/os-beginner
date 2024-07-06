use core::{
    fmt::Display,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
};

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

use crate::{errno::ErrNo, syscall};

type Result<T> = core::result::Result<T, ErrNo>;

pub fn open(path: impl Display, flags: FileFlags) -> Result<File> {
    #[cfg(not(feature = "alloc"))]
    let res = {
        use crate::buf::CStrBuf;
        use core::fmt::Write as _;

        let mut buf = [0; 1024];
        let mut buf = CStrBuf::new_unchecked(&mut buf);
        write!(buf, "{}", path).unwrap();
        unsafe { syscall::__open_file(buf.to_cstr().as_ptr() as _, flags.0 as _) }
    };

    #[cfg(feature = "alloc")]
    let res = {
        use alloc::ffi::CString;
        use alloc::format;

        let path = format!("{}", path);
        let path = match CString::new(path) {
            Ok(s) => s,
            Err(_) => return Err(ErrNo::EINVAL),
        };
        unsafe { syscall::__open_file(path.as_ptr() as _, flags.0 as _) }
    };

    if res.error != 0 {
        Err(res.error.into())
    } else {
        Ok(File(res.value as _))
    }
}

#[cfg(feature = "alloc")]
const BUF_SIZE: usize = 4096;

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    #[cfg(feature = "alloc")]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let mut total = 0;
        let mut in_buf = alloc::vec![0; BUF_SIZE];
        loop {
            let n = match self.read(&mut in_buf)? {
                // EOF
                0 => return Ok(total),
                n => n,
            };
            buf.extend(&in_buf[..n]);
            total += n;
        }
    }

    #[cfg(feature = "alloc")]
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        let mut in_buf = alloc::vec![];
        self.read_to_end(&mut in_buf)?;
        buf.push_str(core::str::from_utf8(&in_buf).map_err(|_| ErrNo::EILSEQ)?);
        Ok(in_buf.len())
    }
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
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

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let res = unsafe { syscall::__put_string(self.0 as _, buf.as_ptr() as _, buf.len() as _) };
        if res.error != 0 {
            Err(res.error.into())
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
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
