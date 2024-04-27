use alloc::{boxed::Box, vec::Vec};

use crate::graphics::{PixelColor, PixelWriter, Vector2D};

/// ウィンドウを管理する構造体。
#[derive(Default)]
pub struct Window {
    /// 幅。
    width: u32,
    /// 高さ。
    height: u32,
    /// ウィンドウのバッファ。
    data: Box<[PixelColor]>,
    /// 透明色の有無と、透明にする色。
    transparent_color: Option<PixelColor>,
}

impl Window {
    /// コンストラクタ。
    ///
    /// * width - ウィンドウの幅。
    /// * height - ウィンドウの高さ
    pub fn new(width: u32, height: u32) -> Self {
        let mut data = Vec::with_capacity((width * height) as usize);
        data.resize((width * height) as usize, Default::default());
        let data = data.into_boxed_slice();

        Self {
            width,
            height,
            data,
            transparent_color: None,
        }
    }

    /// ウィンドウの内容を指定された描画先へ描画する。
    ///
    /// * writer - 描画に用いるライター。
    /// * position - 描画する位置。
    pub fn draw_to(&mut self, writer: &mut dyn PixelWriter, position: Vector2D<u32>) {
        // 透明色が設定されていない場合はそのまま描画する
        if self.transparent_color.is_none() {
            for y in 0..self.height {
                for x in 0..self.width {
                    let pos_relative = Vector2D::new(x, y);
                    writer.write(position + pos_relative, self.at(pos_relative));
                }
            }
            return;
        }

        // 透明色が設定されている場合は、その色のピクセルは描画しない
        let tc = self.transparent_color.unwrap();
        for y in 0..self.height {
            for x in 0..self.width {
                let pos_relative = Vector2D::new(x, y);
                let c = self.at(pos_relative);
                if *c != tc {
                    writer.write(position + pos_relative, c);
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
    fn write(&mut self, pos: Vector2D<u32>, color: &PixelColor) {
        *self.at(pos) = *color;
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
