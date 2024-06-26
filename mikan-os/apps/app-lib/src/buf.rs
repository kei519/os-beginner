use core::{ffi::CStr, fmt::Write};

/// 最後にヌル文字を持った文字列へのフォーマットを提供する。
#[derive(Debug)]
pub struct CStrBuf<'a> {
    buf: &'a mut [u8],
    cur: usize,
}

impl<'a> CStrBuf<'a> {
    /// \[[u8]\] への排他参照 `buf` から [CStrBuf] を作る。
    /// ただしヌル文字を入れる余裕がない `buf` の場合は [None] を返す。
    pub fn new(buf: &'a mut [u8]) -> Option<Self> {
        // ヌル終端するスペースがないものは不正
        if buf.is_empty() {
            return None;
        }
        Some(Self::new_unchecked(buf))
    }

    /// \[[u8]\] への排他参照 `buf` から [CStrBuf] を作る。
    /// ただし `buf` の長さが 0 の場合は `panic` を起こす。
    pub fn new_unchecked(buf: &'a mut [u8]) -> Self {
        // ヌル終端されていることを保証する
        buf[0] = 0;
        Self { buf, cur: 0 }
    }

    /// バッファに保存している文字列を返す。
    pub fn to_str(&self) -> &str {
        // `buf` に書き込むときは必ず Write を通す必要があり、Write では str
        // 以外書き込まれないため安全
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.cur]) }
    }

    /// バッファに保存している文字列をヌル終端されている C の文字列として返す。
    pub fn to_cstr(&self) -> &CStr {
        // 書き込む方法は fmt::Write しかなく、その実装でヌル文字を代入しているから、安全
        unsafe { CStr::from_ptr(self.buf.as_ptr() as _) }
    }
}

impl<'a> Write for CStrBuf<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // ヌル文字分のスペースが必要
        if self.cur + s.len() + 1 > self.buf.len() {
            return Err(core::fmt::Error);
        }
        for &b in s.as_bytes() {
            if b == 0 {
                return Err(core::fmt::Error);
            }
            self.buf[self.cur] = b;
            self.cur += 1;
        }
        self.buf[self.cur] = 0;
        Ok(())
    }
}

/// 文字列へのフォーマットを提供する。
#[derive(Debug)]
pub struct StrBuf<'a> {
    buf: &'a mut [u8],
    cur: usize,
}

impl<'a> StrBuf<'a> {
    /// \[[u8]\] への排他参照 `buf` から [StrBuf] を作る。
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, cur: 0 }
    }

    /// バッファに保存している文字列を返す。
    pub fn to_str(&self) -> &str {
        // `buf` に書き込むときは必ず Write を通す必要があり、Write では str
        // 以外書き込まれないため安全
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.cur]) }
    }
}

impl<'a> Write for StrBuf<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if self.cur + s.len() > self.buf.len() {
            return Err(core::fmt::Error);
        }
        for &b in s.as_bytes() {
            self.buf[self.cur] = b;
            self.cur += 1;
        }
        Ok(())
    }
}
