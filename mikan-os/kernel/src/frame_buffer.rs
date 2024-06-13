use alloc::{boxed::Box, vec};

use crate::{
    error::{Code, Result},
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{
        BgrResv8BitPerColorPixelWriter, PixelWrite, Rectangle, RgbResv8BitPerColorPixelWriter,
        Vector2D,
    },
    make_error,
};

/// シャドウバッファを実現する構造体。
pub struct FrameBuffer {
    /// ピクセルフォーマット。
    pixel_format: PixelFormat,
    /// バッファ。
    _buffer: Box<[u8]>,
    /// ライター。
    writer: Box<dyn PixelWrite + Send + Sync>,
}

impl FrameBuffer {
    /// コンストラクタ。
    ///
    /// * config - フレームバッファに関する情報。
    ///
    /// # Returns
    /// 指定されたピクセルフォーマットが不正の場合はエラーを返す。
    ///
    /// # Remarks
    /// シャドウバッファ等、本当のフレームバッファ以外に使う場合は、
    /// `config.frame_buffer` を `0` にしておくこと。
    pub fn new(mut config: FrameBufferConfig) -> Result<Self> {
        let bits_per_pixel = bits_per_pixel(&config.pixel_format);
        if bits_per_pixel == 0 {
            return Err(make_error!(Code::UnknownDevice));
        }

        let buffer_len =
            ((bits_per_pixel + 7) / 8) * config.horizontal_resolution * config.vertical_resolution;
        let buffer = {
            if config.frame_buffer != 0 {
                [].into()
            } else {
                let buffer = vec![0; buffer_len];

                // 必要なところをシャドウのものに変更しておく
                config.frame_buffer = buffer.as_ptr() as _;
                config.pixels_per_scan_line = config.horizontal_resolution;

                buffer.into_boxed_slice()
            }
        };

        let pixel_format = config.pixel_format;
        let writer: Box<dyn PixelWrite + Send + Sync> = match config.pixel_format {
            PixelFormat::Rgb => Box::new(RgbResv8BitPerColorPixelWriter::new(config)),
            PixelFormat::Bgr => Box::new(BgrResv8BitPerColorPixelWriter::new(config)),
        };

        Ok(Self {
            pixel_format,
            _buffer: buffer,
            writer,
        })
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    /// `src` の `src_area` 領域の要素を自身の `dst_pos` にコピーする。
    pub fn copy(
        &mut self,
        dst_pos: Vector2D<i32>,
        src: &FrameBuffer,
        src_area: &Rectangle<i32>,
    ) -> Result<()> {
        if self.pixel_format != src.pixel_format {
            return Err(make_error!(Code::UnknownPixelFormat));
        }

        let bytes_per_pixel = bytes_per_pixel(&self.pixel_format);
        if bytes_per_pixel == 0 {
            return Err(make_error!(Code::UnknownPixelFormat));
        }

        // [FrameBufferConfig] を要求してくるものが多いので、ここで用意しておく
        let dst_config = FrameBufferConfig {
            frame_buffer: self.frame_buffer(),
            pixels_per_scan_line: self.pixels_per_scan_line(),
            horizontal_resolution: self.horizontal_resolution(),
            vertical_resolution: self.vertical_resolution(),
            pixel_format: self.pixel_format,
        };
        let src_config = FrameBufferConfig {
            frame_buffer: src.frame_buffer(),
            pixels_per_scan_line: src.pixels_per_scan_line(),
            horizontal_resolution: src.horizontal_resolution(),
            vertical_resolution: src.vertical_resolution(),
            pixel_format: src.pixel_format,
        };

        // `src_area` を `dst` 上に置いたときの領域
        let src_area_shifted = Rectangle {
            pos: dst_pos,
            size: src_area.size,
        };
        // `src_area` を `dst` 上に置いたときに、同じように `src` を `dst` に置いたときの領域
        let src_outline = Rectangle {
            pos: dst_pos - src_area.pos,
            size: frame_buffer_size(&src_config),
        };
        // `dst` の領域
        let dst_outline = Rectangle {
            pos: Vector2D::new(0, 0),
            size: frame_buffer_size(&dst_config),
        };
        let copy_area = dst_outline & src_outline & src_area_shifted;
        let src_start_pos = copy_area.pos - (dst_pos - src_area.pos);

        let mut dst_buf = frame_addr_at(copy_area.pos, &dst_config);
        let mut src_buf = frame_addr_at(src_start_pos, &src_config);

        for _ in 0..copy_area.size.y() {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    src_buf,
                    dst_buf,
                    bytes_per_pixel * copy_area.size.x() as usize,
                );
                dst_buf = dst_buf.add(bytes_per_scan_line(&dst_config));
                src_buf = src_buf.add(bytes_per_scan_line(&src_config));
            }
        }
        Ok(())
    }

    pub fn r#move(&mut self, dst_pos: Vector2D<i32>, src: &Rectangle<i32>) {
        use core::ptr::copy_nonoverlapping;

        let bytes_per_pixel = bytes_per_pixel(&self.pixel_format);
        let bytes_per_scan_line = bytes_per_pixel * self.pixels_per_scan_line();

        let config = FrameBufferConfig {
            frame_buffer: self.frame_buffer(),
            pixels_per_scan_line: self.pixels_per_scan_line(),
            horizontal_resolution: self.horizontal_resolution(),
            vertical_resolution: self.vertical_resolution(),
            pixel_format: self.pixel_format,
        };

        if dst_pos.y() < src.pos.y() {
            let mut dst_buf = frame_addr_at(dst_pos, &config);
            let mut src_buf = frame_addr_at(src.pos, &config) as *const _;

            for _ in 0..src.size.y() {
                unsafe {
                    copy_nonoverlapping(src_buf, dst_buf, bytes_per_pixel * src.size.x() as usize);
                    dst_buf = dst_buf.add(bytes_per_scan_line);
                    src_buf = src_buf.add(bytes_per_scan_line);
                }
            }
        } else {
            let mut dst_buf = frame_addr_at(dst_pos + Vector2D::new(0, src.size.y() - 1), &config);
            let mut src_buf =
                frame_addr_at(src.pos + Vector2D::new(0, src.size.y() - 1), &config) as *const _;

            for _ in 0..src.size.y() {
                unsafe {
                    copy_nonoverlapping(src_buf, dst_buf, bytes_per_pixel * src.size.x() as usize);
                    dst_buf = dst_buf.sub(bytes_per_scan_line);
                    src_buf = src_buf.sub(bytes_per_scan_line);
                }
            }
        }
    }
}

impl PixelWrite for FrameBuffer {
    fn write(&mut self, pos: crate::graphics::Vector2D<i32>, color: &crate::graphics::PixelColor) {
        self.writer.write(pos, color)
    }

    fn frame_buffer(&self) -> usize {
        self.writer.frame_buffer()
    }

    fn vertical_resolution(&self) -> usize {
        self.writer.vertical_resolution()
    }

    fn horizontal_resolution(&self) -> usize {
        self.writer.horizontal_resolution()
    }

    fn pixels_per_scan_line(&self) -> usize {
        self.writer.pixels_per_scan_line()
    }
}

/// 1ピクセルが占めるビット数を返す。
///
/// * format - ピクセルフォーマット。
///
/// # Returns
///
/// 対応していないフォーマットの場合は `0` を返す。
///
/// # Remarks
///
/// 今のところピクセル形式は32ビットしかないが、変わる可能性があることを明示するために
/// 関数にしている。
fn bits_per_pixel(format: &PixelFormat) -> usize {
    match format {
        PixelFormat::Rgb => 32,
        PixelFormat::Bgr => 32,
    }
}

fn bytes_per_pixel(format: &PixelFormat) -> usize {
    match format {
        PixelFormat::Rgb | PixelFormat::Bgr => 4,
    }
}

fn bytes_per_scan_line(config: &FrameBufferConfig) -> usize {
    bytes_per_pixel(&config.pixel_format) * config.pixels_per_scan_line
}

fn frame_addr_at(pos: Vector2D<i32>, config: &FrameBufferConfig) -> *mut u8 {
    (config.frame_buffer
        + bytes_per_pixel(&config.pixel_format)
            * (config.pixels_per_scan_line * pos.y() as usize + pos.x() as usize)) as *mut u8
}

fn frame_buffer_size(config: &FrameBufferConfig) -> Vector2D<i32> {
    Vector2D::new(
        config.horizontal_resolution as _,
        config.vertical_resolution as _,
    )
}
