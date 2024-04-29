use alloc::{boxed::Box, vec::Vec};

use crate::{
    error::{Code, Result},
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{
        BgrResv8BitPerColorPixelWriter, PixelWriter, RgbResv8BitPerColorPixelWriter, Vector2D,
    },
    make_error,
};

/// シャドウバッファを実現する構造体。
pub struct FrameBuffer {
    /// ピクセルフォーマット。
    pixel_format: PixelFormat,
    /// バッファ。
    buffer: Box<[u8]>,
    /// ライター。
    writer: Box<dyn PixelWriter + Send>,
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
        let bits_per_pixel = bits_per_pixel(config.pixel_format);
        if bits_per_pixel == 0 {
            return Err(make_error!(Code::UnknownDevice));
        }

        let buffer_len =
            ((bits_per_pixel + 7) / 8) * config.horizontal_resolution * config.vertical_resolution;
        let buffer = {
            if config.frame_buffer != 0 {
                [].into()
            } else {
                let mut buffer = Vec::with_capacity(buffer_len);
                buffer.resize(buffer_len, 0);

                // 必要なところをシャドウのものに変更しておく
                config.frame_buffer = buffer.as_ptr() as _;
                config.pixels_per_scan_line = config.horizontal_resolution;

                buffer.into_boxed_slice()
            }
        };

        let writer: Box<dyn PixelWriter + Send> = match config.pixel_format {
            PixelFormat::Rgb => Box::new(RgbResv8BitPerColorPixelWriter::new(config)),
            PixelFormat::Bgr => Box::new(BgrResv8BitPerColorPixelWriter::new(config)),
        };

        let pixel_format = config.pixel_format;
        Ok(Self {
            pixel_format,
            buffer,
            writer,
        })
    }

    pub fn copy(&mut self, pos: Vector2D<u32>, src: &FrameBuffer) -> Result<()> {
        use core::cmp::{max, min};

        if self.pixel_format != src.pixel_format {
            return Err(make_error!(Code::UnknownPixelFormat));
        }

        let bits_per_pixel = bits_per_pixel(self.pixel_format);
        if bits_per_pixel == 0 {
            return Err(make_error!(Code::UnknownPixelFormat));
        }

        let dst_width = self.horizontal_resolution();
        let dst_height = self.vertical_resolution();
        let src_width = src.horizontal_resolution();
        let src_height = src.vertical_resolution();

        let copy_start_dst_x = max(pos.x(), 0) as usize;
        let copy_start_dst_y = max(pos.y(), 0) as usize;
        let copy_end_dst_x = min(pos.x() as usize + src_width, dst_width);
        let copy_end_dst_y = min(pos.y() as usize + src_height, dst_height);

        let bytes_per_pixel = (bits_per_pixel + 7) / 8;
        let bytes_per_copy_line = bytes_per_pixel * (copy_end_dst_x - copy_start_dst_x);

        let dst_buf = self.frame_buffer()
            + bytes_per_pixel * (self.pixels_per_scan_line() * copy_start_dst_y + copy_start_dst_x);
        let mut dst_buf = dst_buf as *mut u8;
        let mut src_buf = src.frame_buffer() as *const u8;

        for _ in 0..copy_end_dst_y - copy_start_dst_y {
            unsafe {
                core::ptr::copy_nonoverlapping(src_buf, dst_buf, bytes_per_copy_line);
                dst_buf = dst_buf.add(bytes_per_pixel * self.pixels_per_scan_line());
                src_buf = src_buf.add(bytes_per_pixel * src.pixels_per_scan_line());
            }
        }
        Ok(())
    }
}

impl PixelWriter for FrameBuffer {
    fn write(&mut self, pos: crate::graphics::Vector2D<u32>, color: &crate::graphics::PixelColor) {
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
fn bits_per_pixel(format: PixelFormat) -> usize {
    match format {
        PixelFormat::Rgb => 32,
        PixelFormat::Bgr => 32,
    }
}
