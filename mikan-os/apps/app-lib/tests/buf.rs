use app_lib::buf::*;
use core::fmt::Write as _;

#[test]
fn test_cstr_buf() {
    let mut buf = [0; 128];

    assert!(CStrBuf::new(&mut buf[..0]).is_none());

    let mut s = CStrBuf::new(&mut buf).unwrap();
    assert_eq!(s.to_str(), "");
    assert_eq!(s.to_cstr(), c"");

    write!(s, "{} {}", 0, 1).unwrap();
    assert_eq!(s.to_str(), "0 1");
    assert_eq!(s.to_cstr(), c"0 1");

    write!(s, ", addr: {:08x}", 0xabcd).unwrap();
    assert_eq!(s.to_str(), "0 1, addr: 0000abcd");
    assert_eq!(s.to_cstr(), c"0 1, addr: 0000abcd");

    let mut s = CStrBuf::new(&mut buf).unwrap();
    assert_eq!(s.to_str(), "");
    assert_eq!(s.to_cstr(), c"");

    for i in 0..128 / 4 - 1 {
        write!(s, "{:04}", i).unwrap();
    }
    write!(s, "123").unwrap();
    assert!(write!(s, "test").is_err());
    assert!(write!(s, " ").is_err());
}

#[test]
fn test_str_buf() {
    let mut buf = [0; 128];

    let mut s = StrBuf::new(&mut buf);
    assert_eq!(s.to_str(), "");

    write!(s, "{} {}", 0, 1).unwrap();
    assert_eq!(s.to_str(), "0 1");

    write!(s, ", addr: {:08x}", 0xabcd).unwrap();
    assert_eq!(s.to_str(), "0 1, addr: 0000abcd");

    let mut s = StrBuf::new(&mut buf);
    assert_eq!(s.to_str(), "");

    for i in 0..128 / 4 {
        write!(s, "{:04}", i).unwrap();
    }
    assert!(write!(s, " ").is_err());
    assert!(write!(s, "test").is_err());
}
