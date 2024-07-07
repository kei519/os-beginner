use rusttype::{Font, Point, Scale};

use crate::{
    error::{Code, Result},
    fat,
    font_data::get_font,
    graphics::{PixelColor, PixelWrite, Vector2D},
    make_error,
    util::OnceStatic,
};

const FONT_PATH: &str = "/ipag.ttf";

static FONT: OnceStatic<Font> = OnceStatic::new();

pub fn init() -> Result<()> {
    let (Some(entry), false) = fat::find_file(FONT_PATH, 0) else {
        return Err(make_error!(Code::NoSuchEntry));
    };

    let buf = fat::load_file(entry);
    let Some(font) = Font::try_from_vec_and_index(buf, 0) else {
        return Err(make_error!(Code::FreeTypeError));
    };
    FONT.init(font);
    Ok(())
}

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
    const SCALE: Scale = Scale { x: 16., y: 16. };

    if c.is_ascii() {
        write_ascii(writer, pos, c as u8, color);
    } else {
        let font = FONT.as_ref();
        // フォントに含まれる文字のベースラインからの最高点らしい
        let offset_y = font.v_metrics(SCALE).ascent;
        let glyph = font
            .glyph(c)
            .scaled(SCALE)
            // ここの position というのは恐らく左下の座標
            .positioned(Point { x: 0., y: offset_y });
        if let Some(bb) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y;
                if v >= 0.5 && (0..SCALE.x as _).contains(&x) && (0..SCALE.y as _).contains(&y) {
                    writer.write(pos + Vector2D::new(x, y), color);
                }
            });
        } else {
            write_ascii(writer, pos, b'?', color);
            write_ascii(writer, pos + Vector2D::new(8, 0), b'?', color);
        }
    }
}
