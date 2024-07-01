#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Null = 0,
    Quit,
}

impl Default for AppEvent {
    fn default() -> Self {
        Self::Null
    }
}
