#[cfg(not(feature = "alloc"))]
pub use inner::Args;

#[cfg(not(feature = "alloc"))]
mod inner {
    use core::{
        ffi::{c_char, CStr},
        slice,
    };

    /// コマンドライン引数を表す構造体。
    pub struct Args {
        args: &'static [*const c_char],
    }

    impl Args {
        /// カーネルから渡される `argc`, `argv` から [Args] を構成する。
        ///
        /// # Safety
        ///
        /// `argv` はヌル終端された文字列へのポインタの配列へのポインタでなければならない。
        /// また、`argv` が指す配列の長さは `argc` でなければならない。
        pub unsafe fn new(argc: usize, argv: *const *const c_char) -> Self {
            let args = unsafe { slice::from_raw_parts(argv, argc) };
            Self { args }
        }

        /// `index` 番目の引数が存在すればそれを返す。
        /// 存在しない場合は `None` を返す。
        pub fn get_as_str(&self, index: usize) -> Option<&'static str> {
            self.args
                .get(index)
                .map(|&ptr| unsafe { CStr::from_ptr(ptr) }.to_str().unwrap())
        }

        /// コマンドライン引数の数を返す。
        pub fn len(&self) -> usize {
            self.args.len()
        }

        pub fn is_empty(&self) -> bool {
            self.args.is_empty()
        }

        /// `&str` の [Iterator] に変換する。
        pub fn iter(&self) -> impl Iterator<Item = &'static str> {
            self.args
                .iter()
                .map(|&ptr| unsafe { CStr::from_ptr(ptr) }.to_str().unwrap())
        }
    }
}

#[cfg(feature = "alloc")]
pub use inner::Args;

#[cfg(feature = "alloc")]
mod inner {
    use alloc::string::String;
    use core::{
        ffi::{c_char, CStr},
        fmt::Debug,
        slice,
    };

    /// コマンドライン引数を表す構造体。
    pub struct Args {
        args: &'static [*const c_char],
        count: usize,
        rcount: usize,
    }

    impl Args {
        /// カーネルから渡される `argc`, `argv` から [Args] を構成する。
        ///
        /// # Safety
        ///
        /// `argv` はヌル終端された文字列へのポインタの配列へのポインタでなければならない。
        /// また、`argv` が指す配列の長さは `argc` でなければならない。
        pub unsafe fn new(argc: usize, argv: *const *const c_char) -> Self {
            let args = unsafe { slice::from_raw_parts(argv, argc) };
            Self {
                args,
                count: 0,
                rcount: 0,
            }
        }
    }

    impl Iterator for Args {
        type Item = String;

        fn next(&mut self) -> Option<Self::Item> {
            if self.len() != 0 {
                self.count += 1;
                Some(
                    unsafe { CStr::from_ptr(self.args[self.count - 1]) }
                        .to_string_lossy()
                        .into_owned(),
                )
            } else {
                None
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = self.len();
            (len, Some(len))
        }
    }

    impl DoubleEndedIterator for Args {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.len() != 0 {
                self.rcount += 1;
                Some(
                    unsafe { CStr::from_ptr(self.args[self.args.len() - self.rcount]) }
                        .to_string_lossy()
                        .into_owned(),
                )
            } else {
                None
            }
        }
    }

    impl ExactSizeIterator for Args {
        fn len(&self) -> usize {
            self.args.len() - self.count - self.rcount
        }
    }

    impl Debug for Args {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_list()
                .entries(
                    self.args
                        .iter()
                        .map(|&ptr| unsafe { CStr::from_ptr(ptr) }.to_str().map_err(|_| "�")),
                )
                .finish()
        }
    }
}
