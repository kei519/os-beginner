#![allow(unused)]

use core::{
    ops::{Add, AddAssign, Sub, SubAssign},
    slice,
};

use crate::{frame_buffer_config::FrameBufferConfig, DESKTOP_BG_COLOR};

#[derive(PartialEq, Eq, Clone, Default, Copy)]
pub struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

/// ピクセルの色情報を持つ。
impl PixelColor {
    /// 初期化。
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// 32 bit 情報から [PixelColor] へ変換する。
    pub fn to_color(c: u32) -> Self {
        Self {
            r: (c >> 16) as u8 & 0xff,
            g: (c >> 8) as u8 & 0xff,
            b: c as u8 & 0xff,
        }
    }
}

/// ピクセルを塗るための色々を提供する。
pub trait PixelWriter {
    /// ピクセルを塗る手段を提供する。
    fn write(&mut self, pos: Vector2D<u32>, color: &PixelColor);

    /// フレームバッファの先頭アドレスを表す。
    fn frame_buffer(&self) -> usize;

    /// 1行あたりのピクセル数を表す。
    fn pixels_per_scan_line(&self) -> usize;

    /// 横方向の解像度を表す。
    fn horizontal_resolution(&self) -> usize;

    /// 縦方向の解像度を表す。
    fn vertical_resolution(&self) -> usize;

    /// 長方形の枠を指定された色で塗る。
    fn draw_rectangle(&mut self, pos: Vector2D<u32>, size: Vector2D<u32>, c: &PixelColor) {
        // 横線
        for dx in 0..size.x {
            self.write(pos + Vector2D::new(dx, 0), c);
            self.write(pos + Vector2D::new(dx, size.y - 1), c);
        }

        // 縦線
        for dy in 0..size.y {
            self.write(pos + Vector2D::new(0, dy), c);
            self.write(pos + Vector2D::new(size.x - 1, dy), c);
        }
    }

    // 長方形を指定された色で塗る。
    fn fill_rectangle(&mut self, pos: Vector2D<u32>, size: Vector2D<u32>, c: &PixelColor) {
        for dy in 0..size.y {
            for dx in 0..size.x {
                self.write(pos + Vector2D::new(dx, dy), c);
            }
        }
    }
}

/// フレームバッファのピクセルの持ち方が RGB のときのクラス。
pub struct RgbResv8BitPerColorPixelWriter {
    config: FrameBufferConfig,
}

impl RgbResv8BitPerColorPixelWriter {
    pub fn new(config: FrameBufferConfig) -> Self {
        Self { config }
    }

    fn pixel_at(&mut self, pos: Vector2D<u32>) -> &mut [u8; 3] {
        unsafe {
            slice::from_raw_parts_mut(
                (self.frame_buffer()
                    + 4 * (self.pixels_per_scan_line() * pos.y as usize + pos.x as usize))
                    as *mut u8,
                3,
            )
            .try_into()
            .unwrap()
        }
    }
}

impl PixelWriter for RgbResv8BitPerColorPixelWriter {
    fn write(&mut self, pos: Vector2D<u32>, color: &PixelColor) {
        let pixel = self.pixel_at(pos);
        pixel[0] = color.r;
        pixel[1] = color.g;
        pixel[2] = color.b;
    }

    fn frame_buffer(&self) -> usize {
        self.config.frame_buffer
    }

    fn pixels_per_scan_line(&self) -> usize {
        self.config.pixels_per_scan_line
    }

    fn horizontal_resolution(&self) -> usize {
        self.config.horizontal_resolution
    }

    fn vertical_resolution(&self) -> usize {
        self.config.vertical_resolution
    }
}

/// フレームバッファのピクセルの持ち方が BGR のときのクラス。
pub struct BgrResv8BitPerColorPixelWriter {
    config: FrameBufferConfig,
}

impl BgrResv8BitPerColorPixelWriter {
    /// 初期化。
    pub fn new(config: FrameBufferConfig) -> Self {
        Self { config }
    }

    fn pixel_at(&mut self, pos: Vector2D<u32>) -> &mut [u8; 3] {
        unsafe {
            slice::from_raw_parts_mut(
                (self.frame_buffer()
                    + 4 * (self.pixels_per_scan_line() * pos.y as usize + pos.x as usize))
                    as *mut u8,
                3,
            )
        }
        .try_into()
        .unwrap()
    }
}

impl PixelWriter for BgrResv8BitPerColorPixelWriter {
    fn write(&mut self, pos: Vector2D<u32>, color: &PixelColor) {
        let pixel = self.pixel_at(pos);
        pixel[0] = color.b;
        pixel[1] = color.g;
        pixel[2] = color.r;
    }

    fn frame_buffer(&self) -> usize {
        self.config.frame_buffer
    }

    fn pixels_per_scan_line(&self) -> usize {
        self.config.pixels_per_scan_line
    }

    fn horizontal_resolution(&self) -> usize {
        self.config.horizontal_resolution
    }

    fn vertical_resolution(&self) -> usize {
        self.config.vertical_resolution
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
/// 2次元のベクトル情報を保持するクラス。
pub struct Vector2D<T> {
    x: T,
    y: T,
}

impl<T: Copy + Add + AddAssign + Sub + SubAssign> Vector2D<T> {
    /// 初期化。
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    /// x 成分を返す。
    pub const fn x(&self) -> T {
        self.x
    }

    /// y 成分を返す。
    pub const fn y(&self) -> T {
        self.y
    }
}

/// 成分の加算を加算として定義する。
impl<T> Add for Vector2D<T>
where
    T: Add<Output = T>,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

/// 成分の加算を加算として定義する。
impl<T: AddAssign> AddAssign for Vector2D<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

/// 成分の減算を減算として定義する。
impl<T> Sub for Vector2D<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

/// 成分の減算を減算として定義する。
impl<T: SubAssign> SubAssign for Vector2D<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

pub fn draw_desktop(writer: &mut dyn PixelWriter) {
    let frame_width = writer.horizontal_resolution() as u32;
    let frame_height = writer.vertical_resolution() as u32;

    // デスクトップ背景の描画
    writer.fill_rectangle(
        Vector2D::new(0, 0),
        Vector2D::new(frame_width, frame_height - 50),
        &DESKTOP_BG_COLOR,
    );
    // タスクバーの表示
    writer.fill_rectangle(
        Vector2D::new(0, frame_height - 50),
        Vector2D::new(frame_width, 50),
        &PixelColor::new(1, 8, 17),
    );
    // （多分）Windows の検索窓
    writer.fill_rectangle(
        Vector2D::new(0, frame_height - 50),
        Vector2D::new(frame_width / 5, 50),
        &PixelColor::new(80, 80, 80),
    );
    // （多分）Windows のスタートボタン
    writer.fill_rectangle(
        Vector2D::new(10, frame_height - 40),
        Vector2D::new(30, 30),
        &PixelColor::new(160, 160, 160),
    );
}
