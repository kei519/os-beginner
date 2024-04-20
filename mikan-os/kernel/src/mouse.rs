use alloc::boxed::Box;

use crate::{
    graphics::{PixelColor, PixelWriter, Vector2D},
    sync::OnceMutex,
};

/// マウスカーソルの横幅
pub(crate) const MOUSE_CURSOR_WIDTH: usize = 15;
/// マウスカーソルの高さ
pub(crate) const MOUSE_CURSOR_HEIGHT: usize = 24;
/// マウスの透明色
pub(crate) const MOUSE_TRANSPARENT_COLOR: PixelColor = PixelColor::new(0, 0, 1);
/// マウスカーソルの形
const MOUSE_CURSOR_SHAPE: [&[u8; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] = [
    b"@              ",
    b"@@             ",
    b"@.@            ",
    b"@..@           ",
    b"@...@          ",
    b"@....@         ",
    b"@.....@        ",
    b"@......@       ",
    b"@.......@      ",
    b"@........@     ",
    b"@.........@    ",
    b"@..........@   ",
    b"@...........@  ",
    b"@............@ ",
    b"@......@@@@@@@@",
    b"@......@       ",
    b"@....@@.@      ",
    b"@...@ @.@      ",
    b"@..@   @.@     ",
    b"@.@    @.@     ",
    b"@@      @.@    ",
    b"@       @.@    ",
    b"         @.@   ",
    b"         @@@   ",
];

pub(crate) struct MouseCursor {
    pixel_writer: &'static OnceMutex<Box<dyn PixelWriter + Send>>,
    erase_color: PixelColor,
    position: Vector2D<u32>,
}

impl MouseCursor {
    pub(crate) fn new(
        writer: &'static OnceMutex<Box<dyn PixelWriter + Send>>,
        erase_color: PixelColor,
        initial_position: Vector2D<u32>,
    ) -> Self {
        let mut ret = Self {
            pixel_writer: writer,
            erase_color,
            position: initial_position,
        };
        ret.draw_mouse_cursor();
        ret
    }

    pub(crate) fn move_relative(&mut self, displacement: Vector2D<u32>) {
        self.erase_mouse_cursor();
        self.position += displacement;
        self.draw_mouse_cursor();
    }

    fn draw_mouse_cursor(&mut self) {
        draw_mouse_cursor(&mut **self.pixel_writer.lock(), &self.position)
    }

    fn erase_mouse_cursor(&mut self) {
        for dy in 0..MOUSE_CURSOR_HEIGHT {
            for dx in 0..MOUSE_CURSOR_WIDTH {
                if MOUSE_CURSOR_SHAPE[dy][dx] != b' ' {
                    self.pixel_writer.lock().write(
                        self.position + Vector2D::new(dx as u32, dy as u32),
                        &self.erase_color,
                    )
                }
            }
        }
    }
}

pub(crate) fn draw_mouse_cursor(writer: &mut dyn PixelWriter, pos: &Vector2D<u32>) {
    for dy in 0..MOUSE_CURSOR_HEIGHT {
        for dx in 0..MOUSE_CURSOR_WIDTH {
            let pos = *pos + Vector2D::new(dx as u32, dy as u32);
            if MOUSE_CURSOR_SHAPE[dy][dx] == b'@' {
                writer.write(pos, &PixelColor::new(0, 0, 0));
            } else if MOUSE_CURSOR_SHAPE[dy][dx] == b'.' {
                writer.write(pos, &PixelColor::new(255, 255, 255));
            } else {
                writer.write(pos, &MOUSE_TRANSPARENT_COLOR);
            }
        }
    }
}
