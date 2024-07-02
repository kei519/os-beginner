use crate::graphics::{Rectangle, Vector2D};

/// 発信元のタスクを知らせる必要がない場合は `src_task` を `0` にして使用する。
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Message {
    pub ty: MessageType,
    pub src_task: u64,
}

impl Message {
    pub fn from_move(task_id: u64, layer_id: u32, pos: Vector2D<i32>) -> Message {
        Message {
            ty: MessageType::Layer {
                op: LayerOperation::Move,
                layer_id,
                pos,
                size: Vector2D::new(-1, -1),
            },
            src_task: task_id,
        }
    }

    pub fn from_move_relative(task_id: u64, layer_id: u32, diff: Vector2D<i32>) -> Message {
        Message {
            ty: MessageType::Layer {
                op: LayerOperation::MoveRelative,
                layer_id,
                pos: diff,
                size: Vector2D::new(-1, -1),
            },
            src_task: task_id,
        }
    }

    pub fn from_draw(task_id: u64, layer_id: u32) -> Message {
        Message {
            ty: MessageType::Layer {
                op: LayerOperation::Draw,
                layer_id,
                pos: Vector2D::new(0, 0),
                size: Vector2D::new(-1, -1),
            },
            src_task: task_id,
        }
    }

    pub fn from_draw_area(task_id: u64, layer_id: u32, area: Rectangle<i32>) -> Message {
        Message {
            ty: MessageType::Layer {
                op: LayerOperation::DrawArea,
                layer_id,
                pos: area.pos,
                size: area.size,
            },
            src_task: task_id,
        }
    }
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
    TimerTimeout {
        timeout: u64,
        value: i32,
    },
    KeyPush {
        modifier: u8,
        keycode: u8,
        ascii: u8,
    },
    Layer {
        op: LayerOperation,
        layer_id: u32,
        pos: Vector2D<i32>,
        size: Vector2D<i32>,
    },
    LayerFinish,
    MouseMove {
        x: i32,
        y: i32,
        dx: i32,
        dy: i32,
        buttons: u8,
    },
    MouseButton {
        x: i32,
        y: i32,
        press: bool,
        button: i32,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LayerOperation {
    Move,
    MoveRelative,
    Draw,
    DrawArea,
}
