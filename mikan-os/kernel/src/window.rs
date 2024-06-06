use core::cmp;

use alloc::{boxed::Box, vec::Vec};

use crate::{
    font,
    frame_buffer::FrameBuffer,
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{PixelColor, PixelWrite, Rectangle, Vector2D},
};

/// ウィンドウを管理する構造体。
pub struct Window {
    /// 幅。
    width: u32,
    /// 高さ。
    height: u32,
    /// ウィンドウのバッファ。
    data: Box<[PixelColor]>,
    /// 透明色の有無と、透明にする色。
    transparent_color: Option<PixelColor>,
    /// シャドウバッファ。
    shadow_buffer: FrameBuffer,
}

impl Window {
    /// コンストラクタ。
    ///
    /// * width - ウィンドウの幅。
    /// * height - ウィンドウの高さ
    pub fn new(width: u32, height: u32, shadow_fomrat: PixelFormat) -> Self {
        let mut data = Vec::with_capacity((width * height) as usize);
        data.resize((width * height) as usize, Default::default());
        let data = data.into_boxed_slice();

        let config = FrameBufferConfig {
            frame_buffer: 0,
            pixels_per_scan_line: 0,
            horizontal_resolution: width as _,
            vertical_resolution: height as _,
            pixel_format: shadow_fomrat,
        };

        Self {
            width,
            height,
            data,
            transparent_color: None,
            shadow_buffer: FrameBuffer::new(config).unwrap(),
        }
    }

    /// 指定された領域のウィンドウの内容を指定されたフレームバッファへ転送する。
    ///
    /// * writer - 描画に用いるライター。
    /// * position - 描画する位置。
    /// * area - 転送する領域。
    pub fn draw_to(&mut self, dst: &mut FrameBuffer, pos: Vector2D<i32>, area: &Rectangle<i32>) {
        // 自身の領域
        let window_area = Rectangle {
            pos,
            size: self.size(),
        };
        // `window_area` のこの部分だけ転送する
        let intersection = *area & window_area;

        // 透明色が設定されていない場合はそのまま描画する
        if self.transparent_color.is_none() {
            dst.copy(
                intersection.pos,
                &self.shadow_buffer,
                &Rectangle {
                    pos: intersection.pos - pos,
                    size: intersection.size,
                },
            )
            .unwrap();
            return;
        }

        // 透明色が設定されている場合は、その色のピクセルは描画しない
        let tc = self.transparent_color.unwrap();
        // 描き込むフレームバッファからはみ出る分は描画しない
        for y in cmp::max(0, 0 - intersection.pos.y())
            ..cmp::min(
                intersection.size.y(),
                dst.vertical_resolution() as i32 - intersection.pos.y(),
            )
        {
            for x in cmp::max(0, 0 - intersection.pos.x())
                ..cmp::min(
                    intersection.size.x(),
                    dst.horizontal_resolution() as i32 - intersection.pos.x(),
                )
            {
                let pos_relative = Vector2D::new(x, y);
                let c = self.at(pos_relative);
                if *c != tc {
                    dst.write(pos + pos_relative, c);
                }
            }
        }
    }

    pub fn r#move(&mut self, dst_pos: Vector2D<i32>, src: &Rectangle<i32>) {
        self.shadow_buffer.r#move(dst_pos, src)
    }

    /// ウィンドウの透過色を設定する。
    pub fn set_transparent_color(&mut self, c: Option<PixelColor>) {
        self.transparent_color = c;
    }

    /// 指定された位置のピクセルへの排他参照を返す。
    pub fn at(&mut self, pos: Vector2D<i32>) -> &mut PixelColor {
        &mut self.data[(pos.y() * self.width as i32 + pos.x()) as usize]
    }

    pub fn size(&self) -> Vector2D<i32> {
        Vector2D::new(self.width as i32, self.height as i32)
    }
}

impl PixelWrite for Window {
    /// 保持しているバッファ内に書き込みを行う。
    ///
    /// # Remarks
    ///
    /// 描画が必要な場合は `Window::draw_to()` を呼ぶこと。
    fn write(&mut self, pos: Vector2D<i32>, color: &PixelColor) {
        *self.at(pos) = *color;
        self.shadow_buffer.write(pos, color);
    }

    fn frame_buffer(&self) -> usize {
        self.data.as_ptr() as usize
    }

    fn pixels_per_scan_line(&self) -> usize {
        self.width as usize
    }

    fn horizontal_resolution(&self) -> usize {
        self.width as usize
    }

    fn vertical_resolution(&self) -> usize {
        self.height as usize
    }
}

const CLOSE_BUTTON_WIDTH: usize = 16;
const CLOSE_BUTTON_HEIGHT: usize = 14;

const CLOSE_BUTTON: [&[u8; CLOSE_BUTTON_WIDTH]; CLOSE_BUTTON_HEIGHT] = [
    b"...............@",
    b".:::::::::::::$@",
    b".:::::::::::::$@",
    b".:::@@::::@@::$@",
    b".::::@@::@@:::$@",
    b".::::::@@:::::$@",
    b".::::::@@:::::$@",
    b".::::@@::@@:::$@",
    b".:::@@::::@@::$@",
    b".:::::::::::::$@",
    b".:::::::::::::$@",
    b".:::::::::::::$@",
    b".$$$$$$$$$$$$$$@",
    b"@@@@@@@@@@@@@@@@",
];

const fn to_color(c: u32) -> PixelColor {
    PixelColor::new((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

impl Window {
    pub fn draw_window(&mut self, title: &[u8]) {
        let win_w = self.width as i32;
        let win_h = self.height as i32;

        {
            let mut fill_rect = |pos, size, c| {
                self.fill_rectangle(pos, size, &to_color(c));
            };

            fill_rect(Vector2D::new(0, 0), Vector2D::new(win_w, 1), 0xc6c6c6);
            fill_rect(Vector2D::new(1, 1), Vector2D::new(win_w - 2, 1), 0xffffff);
            fill_rect(Vector2D::new(0, 0), Vector2D::new(1, win_h), 0xc6c6c6);
            fill_rect(Vector2D::new(1, 1), Vector2D::new(1, win_h - 2), 0xffffff);
            fill_rect(
                Vector2D::new(win_w - 2, 1),
                Vector2D::new(1, win_h - 2),
                0x848484,
            );
            fill_rect(
                Vector2D::new(win_w - 1, 0),
                Vector2D::new(1, win_h),
                0x000000,
            );
            fill_rect(
                Vector2D::new(2, 2),
                Vector2D::new(win_w - 4, win_h - 4),
                0xc6c6c6,
            );
            fill_rect(Vector2D::new(3, 3), Vector2D::new(win_w - 6, 18), 0x000084);
            fill_rect(
                Vector2D::new(1, win_h - 2),
                Vector2D::new(win_w - 2, 1),
                0x848484,
            );
            fill_rect(
                Vector2D::new(0, win_h - 1),
                Vector2D::new(win_w, 1),
                0x000000,
            );
        }

        font::write_string(self, Vector2D::new(24, 4), title, &to_color(0xffffff));

        for (y, row) in CLOSE_BUTTON.iter().enumerate() {
            for (x, &b) in row.iter().enumerate() {
                let c = to_color(match b {
                    b'@' => 0x000000,
                    b'$' => 0x848484,
                    b':' => 0xc6c6c6,
                    _ => 0xffffff,
                });
                self.write(
                    Vector2D::new(win_w - 5 - (CLOSE_BUTTON_WIDTH + x) as i32, 5 + y as i32),
                    &c,
                )
            }
        }
    }
}
