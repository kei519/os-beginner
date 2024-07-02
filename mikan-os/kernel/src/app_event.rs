#[repr(C, i32)]
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Null = 0,
    Quit,
    MouseMove {
        x: i32,
        y: i32,
        dx: i32,
        dy: i32,
        buttons: u8,
    },
}

impl Default for AppEvent {
    fn default() -> Self {
        Self::Null
    }
}
