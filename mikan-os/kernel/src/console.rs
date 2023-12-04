#![allow(unused)]

use core::{
    fmt::{self, Write},
    ptr::copy_nonoverlapping,
};

use crate::{
    font::{write_ascii, write_string},
    graphics::{PixelColor, PixelWriter, Vector2D},
};

const ROW_NUM: usize = 25;
const COLUMN_NUM: usize = 80;

pub(crate) struct Console<'a> {
    writer: &'a dyn PixelWriter,
    fg_color: &'a PixelColor,
    bg_color: &'a PixelColor,
    buffer: [[u8; COLUMN_NUM]; ROW_NUM],
    cursor_row: usize,
    cursor_column: usize,
}

impl<'a> Console<'a> {
    pub(crate) fn new(
        writer: &'a dyn PixelWriter,
        fg_color: &'a PixelColor,
        bg_color: &'a PixelColor,
    ) -> Self {
        Self {
            writer,
            fg_color,
            bg_color,
            buffer: [[0u8; COLUMN_NUM]; ROW_NUM],
            cursor_row: 0,
            cursor_column: 0,
        }
    }

    pub(crate) fn put_string(&mut self, s: &[u8]) {
        for &c in s {
            if c == b'\n' {
                self.new_line();
            } else if (self.cursor_column < COLUMN_NUM) {
                write_ascii(
                    self.writer,
                    Vector2D::new(8 * self.cursor_column as u32, 16 * self.cursor_row as u32),
                    c,
                    &self.fg_color,
                );
                self.buffer[self.cursor_row][self.cursor_column] = c;
                self.cursor_column += 1;
            }
        }
    }

    pub(crate) fn new_line(&mut self) {
        self.cursor_column = 0;

        if self.cursor_row < ROW_NUM - 1 {
            self.cursor_row += 1;
        } else {
            // 背景の描画
            for y in 0..16 * ROW_NUM {
                for x in 0..8 * COLUMN_NUM {
                    self.writer
                        .write(Vector2D::new(x as u32, y as u32), &self.bg_color);
                }
            }

            // バッファの移動と描画
            for row in 0..ROW_NUM - 1 {
                unsafe {
                    copy_nonoverlapping(
                        self.buffer[row + 1].as_ptr(),
                        self.buffer[row].as_mut_ptr(),
                        COLUMN_NUM,
                    );
                }
                write_string(
                    self.writer,
                    Vector2D::new(0, 16 * row as u32),
                    &self.buffer[row],
                    &self.fg_color,
                );
            }
            self.buffer[ROW_NUM - 1] = [0u8; COLUMN_NUM];
        }
    }
}

impl<'a> Write for Console<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if !s.is_ascii() {
            return Err(fmt::Error);
        }

        let s = s.as_bytes();
        let lines = (s.len() + COLUMN_NUM - 1) / COLUMN_NUM;
        for i in 0..lines {
            if i == lines - 1 {
                self.put_string(&s[COLUMN_NUM * i..s.len()]);
            } else {
                self.put_string(&s[COLUMN_NUM * i..COLUMN_NUM * (i + 1)]);
            }
        }
        Ok(())
    }
}
