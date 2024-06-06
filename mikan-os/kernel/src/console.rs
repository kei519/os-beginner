use core::fmt::{self, Write};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    font::write_ascii,
    frame_buffer_config::FrameBufferConfig,
    graphics::{PixelColor, PixelWrite, Rectangle, Vector2D},
    layer::LAYER_MANAGER,
    sync::OnceMutex,
    window::Window,
};

/// コンソール処理を担う。
pub static CONSOLE: OnceMutex<Console> = OnceMutex::new();

/// デスクトップ背景の色
pub const DESKTOP_BG_COLOR: PixelColor = PixelColor::new(45, 118, 237);
/// デスクトップ前景の色
pub const DESKTOP_FG_COLOR: PixelColor = PixelColor::new(255, 255, 255);

pub fn init(config: &FrameBufferConfig) {
    let row_num = (config.vertical_resolution - 50) / 16;
    let column_num = config.horizontal_resolution / 8;

    let mut layer_manager = LAYER_MANAGER.lock();
    let mut console_window = Window::new(
        column_num as u32 * 8,
        row_num as u32 * 16,
        config.pixel_format,
    );
    console_window.fill_rectangle(
        Vector2D::new(0, 0),
        Vector2D::new(8 * column_num as i32, 16 * row_num as i32),
        &DESKTOP_BG_COLOR,
    );
    let console_id = layer_manager.new_layer(console_window);
    layer_manager.up_down(console_id, 1);

    CONSOLE.init(Console::new(
        console_id,
        &DESKTOP_FG_COLOR,
        &DESKTOP_BG_COLOR,
        row_num,
        column_num,
    ));
}

pub struct Console {
    /// コンソールのレイヤー ID。
    layer_id: u32,
    /// 前面色。
    fg_color: &'static PixelColor,
    /// 背景色。
    bg_color: &'static PixelColor,
    /// 画面に描画する文字列を保持しておくバッファ。
    buffer: Box<[u8]>,
    /// 次に描画を行う行。
    cursor_row: usize,
    /// 次に描画を行う列。
    cursor_column: usize,
    /// 行のサイズ。
    row_num: usize,
    /// 列のサイズ。
    column_num: usize,
}

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write as _;
            write!($crate::console::CONSOLE.lock(), $($arg)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! printkln {
    () => ($crate::printk!("\n"));
    ($($arg:tt)*) => ($crate::printk!("{}\n", format_args!($($arg)*)));
}

impl Console {
    pub fn new(
        layer_id: u32,
        fg_color: &'static PixelColor,
        bg_color: &'static PixelColor,
        row_num: usize,
        column_num: usize,
    ) -> Self {
        let mut buf = Box::new(Vec::with_capacity(row_num * column_num));
        buf.resize(row_num * column_num, 0u8);
        Self {
            layer_id,
            fg_color,
            bg_color,
            buffer: buf.into_boxed_slice(),
            cursor_row: 0,
            cursor_column: 0,
            row_num,
            column_num,
        }
    }

    pub fn column_num(&self) -> usize {
        self.column_num
    }

    pub fn row_num(&self) -> usize {
        self.row_num
    }

    pub fn put_string(&mut self, s: &[u8]) {
        for &c in s {
            if c == b'\n' {
                self.new_line();
            } else if self.cursor_column < self.column_num - 1 {
                let pos = Vector2D::new(8 * self.cursor_column as i32, 16 * self.cursor_row as i32);
                write_ascii(
                    LAYER_MANAGER.lock().layer(self.layer_id).window_mut(),
                    pos,
                    c,
                    self.fg_color,
                );
                self.buffer[self.cursor_row * self.column_num + self.cursor_column] = c;
                self.cursor_column += 1;
            }
        }

        LAYER_MANAGER.lock().draw_id(self.layer_id);
    }

    pub fn new_line(&mut self) {
        self.cursor_column = 0;

        if self.cursor_row < self.row_num - 1 {
            self.cursor_row += 1;
            return;
        }

        // 背景の描画
        let mov_src = Rectangle {
            pos: Vector2D::new(0, 16),
            size: Vector2D::new(8 * self.column_num as i32, 16 * (self.row_num as i32 - 1)),
        };
        let mut layer_manager = LAYER_MANAGER.lock();
        let window = layer_manager.layer(self.layer_id).window_mut();
        window.r#move(Vector2D::new(0, 0), &mov_src);
        window.fill_rectangle(
            Vector2D::new(0, 16 * (self.row_num as i32 - 1)),
            Vector2D::new(8 * self.column_num as i32, 16),
            self.bg_color,
        );
    }

    /// 行の先頭であるかを返す。
    pub fn is_head(&self) -> bool {
        self.cursor_column == 0
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if !s.is_ascii() {
            return Err(fmt::Error);
        }

        let s = s.as_bytes();
        let lines = (s.len() + self.column_num - 1) / self.column_num;
        for i in 0..lines {
            if i == lines - 1 {
                self.put_string(&s[self.column_num * i..s.len()]);
            } else {
                self.put_string(&s[self.column_num * i..self.column_num * (i + 1)]);
            }
        }

        LAYER_MANAGER.lock().draw_id(self.layer_id);
        Ok(())
    }
}
