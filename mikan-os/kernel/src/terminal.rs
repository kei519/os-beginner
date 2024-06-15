use core::str;

use alloc::sync::Arc;

use crate::{
    asmfunc, font,
    graphics::{PixelColor, PixelWrite, Rectangle, Vector2D, FB_CONFIG},
    layer::LAYER_MANAGER,
    log,
    logger::LogLevel,
    message::{Message, MessageType},
    sync::SharedLock,
    task,
    window::Window,
};

pub fn task_terminal(task_id: u64, _: i64, _: u32) {
    let mut terminal = Terminal::new();
    asmfunc::cli();
    let task = task::current_task();
    {
        let mut manager = LAYER_MANAGER.lock_wait();
        manager.r#move(terminal.layer_id, Vector2D::new(100, 200));
        manager.activate(terminal.layer_id);
    }
    asmfunc::sti();

    loop {
        // task.msgs は Mutex のため、cli は必要ない
        let msg = match task.receive_message() {
            Some(msg) => msg,
            None => {
                task.sleep();
                continue;
            }
        };

        match msg.ty {
            MessageType::TimerTimeout(_) => {
                let mut area = terminal.blink_cursor();
                area.pos += Window::TOP_LEFT_MARGIN;

                let msg = Message::from_draw_area(task_id, terminal.layer_id, area);
                asmfunc::cli();
                task::send_message(1, msg).unwrap();
                asmfunc::sti();
            }
            MessageType::KeyPush {
                modifier,
                keycode,
                ascii,
            } => {
                let mut area = terminal.input_key(modifier, keycode, ascii);
                area.pos += Window::TOP_LEFT_MARGIN;
                let msg = Message::from_draw_area(task_id, terminal.layer_id, area);
                asmfunc::cli();
                task::send_message(1, msg).unwrap();
                asmfunc::sti();
            }
            _ => {}
        }
    }
}

const ROWS: usize = 15;
const COLUMNS: usize = 60;
const LINE_MAX: usize = 128;

pub struct Terminal {
    layer_id: u32,
    window: Arc<SharedLock<Window>>,
    cursor: Vector2D<i32>,
    cursor_visible: bool,
    linebuf_index: usize,
    linebuf: [u8; LINE_MAX],
}

impl Terminal {
    pub fn new() -> Self {
        let mut window = Window::new_toplevel(
            COLUMNS as u32 * 8 + 8 + Window::MARGIN_X,
            ROWS as u32 * 16 + 8 + Window::MARGIN_Y,
            FB_CONFIG.as_ref().pixel_format,
            "MikanTerm",
        );
        let size = window.size();
        window.draw_terminal(Vector2D::new(0, 0), size);

        let (layer_id, window) = {
            let mut manager = LAYER_MANAGER.lock_wait();
            let id = manager.new_layer(window);
            manager.layer(id).set_draggable(true);
            let window = manager.layer(id).window();

            (id, window)
        };

        Self {
            layer_id,
            window,
            cursor: Vector2D::new(0, 0),
            cursor_visible: false,
            linebuf_index: 0,
            linebuf: [0u8; LINE_MAX],
        }
    }

    pub fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.cursor_visible = !self.cursor_visible;
        self.draw_cursor();

        Rectangle {
            pos: self.calc_curosr_pos(),
            size: Vector2D::new(7, 15),
        }
    }

    pub fn input_key(&mut self, _modifier: u8, _keycode: u8, ascii: u8) -> Rectangle<i32> {
        self.cursor_visible = false;
        self.draw_cursor();

        let mut draw_area = Rectangle {
            pos: self.calc_curosr_pos(),
            size: Vector2D::new(8 * 2, 16),
        };

        match ascii {
            0 => {}
            b'\n' => {
                log!(
                    LogLevel::Warn,
                    "line: {:}",
                    str::from_utf8(&self.linebuf[..self.linebuf_index]).unwrap(),
                );
                self.linebuf_index = 0;
                if self.cursor.y() < ROWS as i32 - 1 {
                    self.cursor = Vector2D::new(0, self.cursor.y() + 1)
                } else {
                    self.cursor = Vector2D::new(0, self.cursor.y());
                    self.scroll1();
                }
                draw_area.pos = Window::TOP_LEFT_MARGIN;
                draw_area.size = self.window.read().size();
            }
            0x08 => {
                if self.cursor.x() > 0 {
                    self.cursor -= Vector2D::new(1, 0);
                    self.window.write().draw_rectangle(
                        self.calc_curosr_pos(),
                        Vector2D::new(8, 16),
                        &PixelColor::new(0, 0, 0),
                    );
                    if self.linebuf_index > 0 {
                        self.linebuf_index -= 1;
                    }
                }
            }
            ascii => {
                if self.cursor.x() < COLUMNS as i32 - 1 && self.linebuf_index < LINE_MAX - 1 {
                    self.linebuf[self.linebuf_index] = ascii;
                    self.linebuf_index += 1;
                    font::write_ascii(
                        &mut *self.window.write(),
                        self.calc_curosr_pos(),
                        ascii,
                        &PixelColor::new(255, 255, 255),
                    );
                    self.cursor += Vector2D::new(1, 0);
                }
            }
        }

        self.cursor_visible = true;
        self.draw_cursor();

        draw_area
    }

    fn draw_cursor(&mut self) {
        let color = if self.cursor_visible { 0xffffff } else { 0 };
        let color = PixelColor::to_color(color);
        let pos = Vector2D::new(4 + 8 * self.cursor.x(), 5 + 16 * self.cursor.y());

        self.window
            .write()
            .fill_rectangle(pos, Vector2D::new(7, 15), &color);
    }

    fn calc_curosr_pos(&self) -> Vector2D<i32> {
        Vector2D::new(4 + 8 * self.cursor.x(), 4 + 16 * self.cursor.y())
    }

    fn scroll1(&mut self) {
        let move_src = Rectangle {
            pos: Vector2D::new(4, 4 + 16),
            size: Vector2D::new(8 * COLUMNS as i32, 16 * (ROWS as i32 - 1)),
        };
        let mut window = self.window.write();
        window.r#move(Vector2D::new(4, 4), &move_src);
        window.fill_rectangle(
            Vector2D::new(4, 4 + 16 * self.cursor.y()),
            Vector2D::new(8 * COLUMNS as i32, 16),
            &PixelColor::new(0, 0, 0),
        );
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}
