use alloc::collections::VecDeque;

use crate::{sync::Mutex, timer::Timer};

static MAIN_QUEUE: Mutex<VecDeque<Message>> = Mutex::new(VecDeque::new());

pub fn pop_main_queue() -> Option<Message> {
    MAIN_QUEUE.lock_wait().pop_front()
}

pub fn push_main_queue(msg: Message) {
    MAIN_QUEUE.lock_wait().push_back(msg)
}

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
