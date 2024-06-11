use crate::{graphics::Vector2D, timer::Timer};

/// 発信元のタスクを知らせる必要がない場合は `src_task` を `0` にして使用する。
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Message {
    pub ty: MessageType,
    pub src_task: u64,
}

impl From<MessageType> for Message {
    fn from(value: MessageType) -> Self {
        Self {
            ty: value,
            src_task: 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(u32)]
pub enum MessageType {
    InterruptXHCI,
    TimerTimeout(Timer),
    KeyPush {
        modifier: u8,
        keycode: u8,
        ascii: u8,
    },
    Layer {
        op: LayerOperation,
        layer_id: u32,
        pos: Vector2D<i32>,
    },
    LayerFinish,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LayerOperation {
    Move,
    MoveRelative,
    Draw,
}
