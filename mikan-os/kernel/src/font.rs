use crate::{
    font_data::get_font,
    graphics::{PixelColor, PixelWrite, Vector2D},
};

pub fn write_ascii(writer: &mut dyn PixelWrite, pos: Vector2D<i32>, c: u8, color: &PixelColor) {
    let font = get_font(c);
    for (dy, &row) in font.iter().enumerate() {
        for dx in 0..8 {
            if ((row << dx) & 0x80) != 0 {
                writer.write(pos + Vector2D::new(dx, dy as i32), color);
            }
        }
    }
}

pub fn write_string(writer: &mut dyn PixelWrite, pos: Vector2D<i32>, s: &str, color: &PixelColor) {
    let mut x = 0;
    for c in s.chars() {
        write_unicode(
            writer,
            Vector2D::new(pos.x() + 8 * x as i32, pos.y()),
            c,
            color,
        );
        x += if c.is_ascii() { 1 } else { 2 };
    }
}

pub fn write_unicode(writer: &mut dyn PixelWrite, pos: Vector2D<i32>, c: char, color: &PixelColor) {
    if c.is_ascii() {
        write_ascii(writer, pos, c as u8, color);
    } else {
        write_ascii(writer, pos, b'?', color);
        write_ascii(writer, pos + Vector2D::new(8, 0), b'?', color);
    }
}
