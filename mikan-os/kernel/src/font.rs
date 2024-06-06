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

pub fn write_string(writer: &mut dyn PixelWrite, pos: Vector2D<i32>, s: &[u8], color: &PixelColor) {
    for (i, &c) in s.iter().enumerate() {
        write_ascii(
            writer,
            Vector2D::new(pos.x() + 8 * i as i32, pos.y()),
            c,
            color,
        );
    }
}
