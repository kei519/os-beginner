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
    MouseButton {
        x: i32,
        y: i32,
        press: bool,
        button: i32,
    },
    Timer {
        timeout: u64,
        value: i32,
    },
    KeyPush {
        modifier: u8,
        keycode: u8,
        ascii: u8,
        press: bool,
    },
}

impl Default for AppEvent {
    fn default() -> Self {
        Self::Null
    }
}
