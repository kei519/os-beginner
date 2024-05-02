#![allow(unused)]

use core::{
    fmt::{self, Write},
    ptr::{copy_nonoverlapping, write},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    font::{write_ascii, write_string},
    graphics::{PixelColor, PixelWriter, Rectangle, Vector2D},
    sync::OnceMutex,
    LAYER_MANAGER,
};

pub struct Console {
    /// ピクセル描画用。
    writer: &'static OnceMutex<Box<dyn PixelWriter + Send>>,
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
    /// コンソールをレイヤー上に持つときのレイヤー ID。
    layer_id: u32,
}

impl Console {
    pub fn new(
        writer: &'static OnceMutex<Box<dyn PixelWriter + Send>>,
        fg_color: &'static PixelColor,
        bg_color: &'static PixelColor,
        row_num: usize,
        column_num: usize,
    ) -> Self {
        let mut buf = Box::new(Vec::with_capacity(row_num * column_num));
        buf.resize(row_num * column_num, 0u8);
        Self {
            writer,
            fg_color,
            bg_color,
            buffer: buf.into_boxed_slice(),
            cursor_row: 0,
            cursor_column: 0,
            row_num,
            column_num,
            layer_id: 0,
        }
    }

    pub fn put_string(&mut self, s: &[u8]) {
        for &c in s {
            if c == b'\n' {
                self.new_line();
            } else if (self.cursor_column < self.column_num - 1) {
                let pos = Vector2D::new(8 * self.cursor_column as u32, 16 * self.cursor_row as u32);
                if self.layer_id == 0 {
                    write_ascii(&mut **self.writer.lock(), pos, c, &self.fg_color);
                } else {
                    write_ascii(
                        LAYER_MANAGER.lock().layer(self.layer_id).widow(),
                        pos,
                        c,
                        &self.fg_color,
                    )
                }
                self.buffer[self.cursor_row * self.column_num + self.cursor_column] = c;
                self.cursor_column += 1;
            }
        }

        if LAYER_MANAGER.is_initialized() {
            LAYER_MANAGER.lock().draw();
        }
    }

    pub fn new_line(&mut self) {
        self.cursor_column = 0;

        if self.cursor_row < self.row_num - 1 {
            self.cursor_row += 1;
            return;
        }

        // 背景の描画
        if self.layer_id == 0 {
            let mut writer = self.writer.lock();
            writer.fill_rectangle(
                Vector2D::new(0, 0),
                Vector2D::new(8 * self.column_num as u32, 16 * self.row_num as u32),
                &self.bg_color,
            );

            for row in 0..self.row_num - 1 {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        self.buffer[(row + 1) * self.column_num..].as_ptr(),
                        self.buffer[row * self.column_num..].as_mut_ptr(),
                        self.column_num,
                    )
                };
                write_string(
                    &mut **writer,
                    Vector2D::new(0, 16 * row as u32),
                    &self.buffer[row * self.column_num..(row + 1) * self.column_num],
                    &self.fg_color,
                );
            }
        } else {
            let mov_src = Rectangle {
                pos: Vector2D::new(0, 16),
                size: Vector2D::new(8 * self.column_num as u32, 16 * (self.row_num as u32 - 1)),
            };
            let mut layer_manager = LAYER_MANAGER.lock();
            let window = layer_manager.layer(self.layer_id).widow();
            window.r#move(Vector2D::new(0, 0), &mov_src);
            window.fill_rectangle(
                Vector2D::new(0, 16 * (self.row_num as u32 - 1)),
                Vector2D::new(8 * self.column_num as u32, 16),
                &self.bg_color,
            );
        }
    }

    /// 行の先頭であるかを返す。
    pub fn is_head(&self) -> bool {
        self.cursor_column == 0
    }

    pub fn set_layer(&mut self, layer_id: u32) {
        self.layer_id = layer_id;
        self.refresh();
    }

    pub fn refresh(&mut self) {
        for row in 0..self.row_num {
            write_string(
                LAYER_MANAGER.lock().layer(self.layer_id).widow(),
                Vector2D::new(0, 16 * row as u32),
                &self.buffer[row * self.column_num..(row + 1) * self.column_num],
                &self.fg_color,
            );
        }
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

        if self.layer_id != 0 {
            LAYER_MANAGER.lock().draw();
        }
        Ok(())
    }
}
