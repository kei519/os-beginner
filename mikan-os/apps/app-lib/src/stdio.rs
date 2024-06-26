/// 1024 バイトを超えると `panic` を起こす。
#[macro_export]
macro_rules! fprintf {
    ($fd:expr, $fmt:expr, $($args:tt)*) => {
        $crate::fprintf!($fd, ::core::format_args!($fmt, $($args)*));
    };
    ($fd:expr, $fmt:expr) => {
        {
            use ::core::fmt::Write as _;
            let mut buf = [0; 1024];
            let mut s = $crate::buf::StrBuf::new(&mut buf);
            ::core::write!(s, "{}", $fmt).unwrap();
            $crate::unistd::write($fd, s.to_str());
        }
    }
}

/// 1024 バイトを超えると `panic` を起こす。
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::fprintf!(1, $($arg)*);
    };
}

/// 1024 バイトを超えると `panic` を起こす。
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", ::core::format_args!($($arg)*))
    };
}
