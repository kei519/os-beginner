use alloc::{boxed::Box, vec::Vec};

use crate::{
    frame_buffer::FrameBuffer,
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{PixelColor, PixelWriter, Vector2D},
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

    /// ウィンドウの内容を指定されたフレームバッファへ転送する。
    ///
    /// * writer - 描画に用いるライター。
    /// * position - 描画する位置。
    pub fn draw_to(&mut self, dst: &mut FrameBuffer, position: Vector2D<u32>) {
        // 透明色が設定されていない場合はそのまま描画する
        if self.transparent_color.is_none() {
            dst.copy(position, &self.shadow_buffer).unwrap();
            return;
        }

        // 透明色が設定されている場合は、その色のピクセルは描画しない
        let tc = self.transparent_color.unwrap();
        for y in 0..self.height {
            for x in 0..self.width {
                let pos_relative = Vector2D::new(x, y);
                let c = self.at(pos_relative);
                if *c != tc {
                    dst.write(position + pos_relative, c);
                }
            }
        }
    }

    /// ウィンドウの透過色を設定する。
    pub fn set_transparent_color(&mut self, c: Option<PixelColor>) {
        self.transparent_color = c;
    }

    /// 指定された位置のピクセルへの排他参照を返す。
    pub fn at(&mut self, pos: Vector2D<u32>) -> &mut PixelColor {
        &mut self.data[(pos.y() * self.width + pos.x()) as usize]
    }
}

impl PixelWriter for Window {
    /// 保持しているバッファ内に書き込みを行う。
    ///
    /// # Remarks
    ///
    /// 描画が必要な場合は `Window::draw_to()` を呼ぶこと。
    fn write(&mut self, pos: Vector2D<u32>, color: &PixelColor) {
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
