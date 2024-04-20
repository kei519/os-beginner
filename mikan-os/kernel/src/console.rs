#![allow(unused)]

use core::{
    fmt::{self, Write},
    ptr::{copy_nonoverlapping, write},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    font::{write_ascii, write_string},
    graphics::{PixelColor, PixelWriter, Vector2D},
    sync::OnceMutex,
    LAYER_MANAGER,
};

pub(crate) struct Console {
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
    pub(crate) fn new(
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

    pub(crate) fn put_string(&mut self, s: &[u8]) {
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

    pub(crate) fn new_line(&mut self) {
        self.cursor_column = 0;

        if self.cursor_row < self.row_num - 1 {
            self.cursor_row += 1;
        } else {
            // 背景の描画
            for y in 0..16 * self.row_num {
                for x in 0..8 * self.column_num {
                    self.writer
                        .lock()
                        .write(Vector2D::new(x as u32, y as u32), &self.bg_color);
                }
            }

            // バッファの移動と描画
            for row in 0..self.row_num - 1 {
                unsafe {
                    copy_nonoverlapping(
                        self.buffer.as_ptr().add((row + 1) * self.column_num),
                        self.buffer.as_mut_ptr().add(row * self.column_num),
                        self.column_num,
                    );
                }
                write_string(
                    &mut **self.writer.lock(),
                    Vector2D::new(0, 16 * row as u32),
                    &self.buffer[row * self.column_num..(row + 1) * self.column_num],
                    &self.fg_color,
                );
            }

            for column in 0..self.column_num {
                unsafe {
                    write(
                        self.buffer
                            .as_mut_ptr()
                            .add((self.row_num - 1) * self.column_num + column),
                        0,
                    );
                }
            }
        }
    }

    /// 行の先頭であるかを返す。
    pub(crate) fn is_head(&self) -> bool {
        self.cursor_column == 0
    }

    pub(crate) fn set_layer(&mut self, layer_id: u32) {
        self.layer_id = layer_id;
    }

    pub(crate) fn refresh(&mut self) {
        for row in 0..self.row_num {
            write_string(
                &mut **self.writer.lock(),
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
        Ok(())
    }
}
