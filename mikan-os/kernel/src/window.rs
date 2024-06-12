use core::cmp;

use alloc::{boxed::Box, string::String, vec::Vec};

use crate::{
    font,
    frame_buffer::FrameBuffer,
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{PixelColor, PixelWrite, Rectangle, Vector2D},
    log,
    logger::LogLevel,
};

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

pub enum Window {
    Base(WindowBase),
    Toplevel { base: WindowBase, title: String },
}

impl Window {
    pub const TOP_LEFT_MARGIN: Vector2D<i32> = Vector2D::new(4, 24);
    pub const BOTTOM_RIGHT_MARGIN: Vector2D<i32> = Vector2D::new(4, 4);

    pub fn new_base(width: u32, height: u32, shadow_format: PixelFormat) -> Self {
        WindowBase::new(width, height, shadow_format).into()
    }

    pub fn new_toplevel(
        width: u32,
        height: u32,
        shadow_format: PixelFormat,
        title: impl Into<String>,
    ) -> Self {
        let title = title.into();
        let mut base = WindowBase::new(width, height, shadow_format);
        base.draw_window(&title);

        Self::Toplevel { base, title }
    }

    pub fn activate(&mut self) {
        self.base_mut().activate();
        if matches!(self, Self::Toplevel { .. }) {
            self.draw_window_title(true);
        }
    }

    pub fn deactivate(&mut self) {
        self.base_mut().activate();
        if matches!(self, Self::Toplevel { .. }) {
            self.draw_window_title(false);
        }
    }

    pub fn draw_window_title(&mut self, active: bool) {
        let (base, title) = match self {
            Self::Base(_) => {
                log!(
                    LogLevel::Debug,
                    "Window::draw_window_title() is not supported for Window::Base"
                );
                return;
            }
            Self::Toplevel { base, title } => (base, title),
        };

        let win_w = base.width as i32;
        let bgcolor = if active { 0x000084 } else { 0x848484 };
        base.fill_rectangle(
            Vector2D::new(3, 3),
            Vector2D::new(win_w - 6, 18),
            &PixelColor::to_color(bgcolor),
        );
        font::write_string(
            base,
            Vector2D::new(24, 4),
            title.as_bytes(),
            &PixelColor::to_color(0xffffff),
        );

        for (y, row_data) in CLOSE_BUTTON.iter().enumerate() {
            for (x, &b) in row_data.iter().enumerate() {
                let c = match b {
                    b'@' => 0x000000,
                    b'$' => 0x848484,
                    b':' => 0xc6c6c6,
                    _ => 0xffffff,
                };
                let c = PixelColor::to_color(c);

                let x = x as i32;
                let y = y as i32;
                base.write(
                    Vector2D::new(win_w - 5 - CLOSE_BUTTON_WIDTH as i32 + x, 5 + y),
                    &c,
                );
            }
        }
    }

    /// [PixelWrite] のメソッドを呼ぶと、全体としてのライターが返るとは限らないため、
    /// 全体としての [PixelWriter] が欲しい場合はこちらを呼ぶ。
    pub fn base(&self) -> &WindowBase {
        self.into()
    }

    /// [PixelWrite] のメソッドを呼ぶと、全体としてのライターが返るとは限らないため、
    /// 全体としての [PixelWriter] が欲しい場合はこちらを呼ぶ。
    pub fn base_mut(&mut self) -> &mut WindowBase {
        self.into()
    }
}

/// [WindowBase] としての [Window] 実装。
impl Window {
    /// 通常描画に使われる領域の幅。
    ///
    /// ウィンドウ全体の幅を知りたい場合は `Window::base()` を使用すること。
    pub fn width(&self) -> u32 {
        self.size().x() as _
    }

    /// 通常描画に使われる領域の高さ。
    ///
    /// ウィンドウ全体の高さを知りたい場合は `Window::base()` を使用すること。
    pub fn height(&self) -> u32 {
        self.size().y() as _
    }

    /// 通常描画に使われる領域のサイズ。
    ///
    /// ウィンドウ全体のサイズを知りたい場合は `Window::base()` を使用すること。
    pub fn size(&self) -> Vector2D<i32> {
        match self {
            Self::Base(base) => base.size(),
            Self::Toplevel { base, .. } => {
                base.size() - Self::TOP_LEFT_MARGIN - Self::BOTTOM_RIGHT_MARGIN
            }
        }
    }

    /// 指定された領域のウィンドウの内容を指定されたフレームバッファへ転送する。
    ///
    /// * writer - 描画に用いるライター。
    /// * position - 描画する位置。
    /// * area - 転送する領域。
    pub fn draw_to(&mut self, dst: &mut FrameBuffer, pos: Vector2D<i32>, area: &Rectangle<i32>) {
        self.base_mut().draw_to(dst, pos, area)
    }

    pub fn r#move(&mut self, dst_pos: Vector2D<i32>, src: &Rectangle<i32>) {
        self.base_mut().r#move(dst_pos, src)
    }

    /// ウィンドウの透過色を設定する。
    pub fn set_transparent_color(&mut self, c: Option<PixelColor>) {
        self.base_mut().set_transparent_color(c)
    }

    /// 指定された位置のピクセルへの排他参照を返す。
    pub fn at(&mut self, pos: Vector2D<i32>) -> &mut PixelColor {
        self.base_mut().at(pos)
    }
}

impl From<WindowBase> for Window {
    fn from(value: WindowBase) -> Self {
        Self::Base(value)
    }
}

impl From<Window> for WindowBase {
    fn from(value: Window) -> Self {
        match value {
            Window::Base(base) | Window::Toplevel { base, .. } => base,
        }
    }
}

impl<'a> From<&'a Window> for &'a WindowBase {
    fn from(value: &'a Window) -> Self {
        match value {
            Window::Base(base) | Window::Toplevel { base, .. } => base,
        }
    }
}

impl<'a> From<&'a mut Window> for &'a mut WindowBase {
    fn from(value: &'a mut Window) -> Self {
        match value {
            Window::Base(base) | Window::Toplevel { base, .. } => base,
        }
    }
}

/// 普段描画に使う部分だけを返すので、
/// [WindowBase] に描画したい場合は `Self::base_mut()` を呼んだほうが良い。
impl PixelWrite for Window {
    fn write(&mut self, pos: Vector2D<i32>, color: &PixelColor) {
        match self {
            Self::Base(base) => base.write(pos, color),
            Self::Toplevel { base, .. } => base.write(pos + Self::TOP_LEFT_MARGIN, color),
        }
    }

    fn frame_buffer(&self) -> usize {
        match self {
            Self::Base(base) => base.frame_buffer(),
            Self::Toplevel { base, .. } => {
                base.frame_buffer()
                    + (Self::TOP_LEFT_MARGIN.y() as usize * base.pixels_per_scan_line()
                        + Self::TOP_LEFT_MARGIN.x() as usize)
                        * 8
            }
        }
    }

    fn pixels_per_scan_line(&self) -> usize {
        self.base().pixels_per_scan_line()
    }

    fn horizontal_resolution(&self) -> usize {
        self.size().x() as _
    }

    fn vertical_resolution(&self) -> usize {
        self.size().y() as _
    }
}

/// ウィンドウを管理する構造体。
pub struct WindowBase {
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

impl WindowBase {
    /// コンストラクタ。
    ///
    /// * width - ウィンドウの幅。
    /// * height - ウィンドウの高さ
    pub fn new(width: u32, height: u32, shadow_format: PixelFormat) -> Self {
        let mut data = Vec::with_capacity((width * height) as usize);
        data.resize((width * height) as usize, Default::default());
        let data = data.into_boxed_slice();

        let config = FrameBufferConfig {
            frame_buffer: 0,
            pixels_per_scan_line: 0,
            horizontal_resolution: width as _,
            vertical_resolution: height as _,
            pixel_format: shadow_format,
        };

        Self {
            width,
            height,
            data,
            transparent_color: None,
            shadow_buffer: FrameBuffer::new(config).unwrap(),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
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

    pub fn activate(&mut self) {}
    pub fn deactivate(&mut self) {}
}

impl PixelWrite for WindowBase {
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

impl WindowBase {
    pub fn draw_window(&mut self, title: &str) {
        let win_w = self.width as i32;
        let win_h = self.height as i32;

        {
            let mut fill_rect = |pos, size, c| {
                self.fill_rectangle(pos, size, &PixelColor::to_color(c));
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

        font::write_string(
            self,
            Vector2D::new(24, 4),
            title.as_bytes(),
            &PixelColor::to_color(0xffffff),
        );

        for (y, row) in CLOSE_BUTTON.iter().enumerate() {
            for (x, &b) in row.iter().enumerate() {
                let c = PixelColor::to_color(match b {
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

    /// ウィンドウの中にテキスト描画用のスペースを描画する。
    pub fn draw_text_box(&mut self, pos: Vector2D<i32>, size: Vector2D<i32>) {
        let mut fill_rect = |pos, size, c| self.fill_rectangle(pos, size, &PixelColor::to_color(c));

        // fill main box
        fill_rect(
            pos + Vector2D::new(1, 1),
            size - Vector2D::new(2, 2),
            0xffffff,
        );

        // draw border lines
        fill_rect(pos, Vector2D::new(size.x(), 1), 0x848484);
        fill_rect(pos, Vector2D::new(1, size.y()), 0x848484);
        fill_rect(
            pos + Vector2D::new(0, size.y()),
            Vector2D::new(size.x(), 1),
            0xc6c6c6,
        );
        fill_rect(
            pos + Vector2D::new(size.x(), 0),
            Vector2D::new(1, size.y()),
            0xc6c6c6,
        );
    }
}
