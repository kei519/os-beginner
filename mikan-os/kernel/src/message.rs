use crate::timer::Timer;

#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(u32)]
pub enum Message {
    InterruptXHCI,
    TimerTimeout(Timer),
    KeyPush {
        modifier: u8,
        keycode: u8,
        ascii: u8,
    },
}
