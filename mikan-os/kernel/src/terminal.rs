use alloc::{borrow::ToOwned, sync::Arc, vec::Vec};
use core::str;

// フォーマッターに勝手に vec::self にされてしまうので、マクロは別に読み込む
use alloc::vec;

use crate::{
    asmfunc, font,
    graphics::{PixelColor, PixelWrite, Rectangle, Vector2D, FB_CONFIG},
    layer::{LAYER_MANAGER, LAYER_TASK_MAP},
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
    LAYER_TASK_MAP
        .lock_wait()
        .insert(terminal.layer_id, task_id);
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

        let mut ret = Self {
            layer_id,
            window,
            cursor: Vector2D::new(0, 0),
            cursor_visible: false,
            linebuf_index: 0,
            linebuf: [0u8; LINE_MAX],
        };
        ret.print(">");
        ret
    }

    pub fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.draw_cursor(!self.cursor_visible);

        Rectangle {
            pos: self.calc_curosr_pos(),
            size: Vector2D::new(7, 15),
        }
    }

    pub fn input_key(&mut self, _modifier: u8, _keycode: u8, ascii: u8) -> Rectangle<i32> {
        self.draw_cursor(false);

        let mut draw_area = Rectangle {
            pos: self.calc_curosr_pos(),
            size: Vector2D::new(8 * 2, 16),
        };

        match ascii {
            0 => {}
            b'\n' => {
                self.execute_line();
                self.print(">");
                draw_area.pos = Vector2D::new(0, 0);
                draw_area.size = self.window.read().size();
            }
            0x08 => {
                if self.cursor.x() > 0 && self.linebuf_index > 0 {
                    self.cursor -= Vector2D::new(1, 0);
                    self.window.write().draw_rectangle(
                        self.calc_curosr_pos(),
                        Vector2D::new(8, 16),
                        &PixelColor::new(0, 0, 0),
                    );
                    self.linebuf_index -= 1;
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

        self.draw_cursor(true);

        draw_area
    }

    fn draw_cursor(&mut self, visible: bool) {
        self.cursor_visible = visible;
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

    /// `linebuf` や `linebuf_index` を変更せずに文字列を表示する。
    fn print(&mut self, s: &str) {
        self.draw_cursor(false);

        let newline = |term: &mut Self| {
            term.cursor = if term.cursor.y() < ROWS as i32 - 1 {
                Vector2D::new(0, term.cursor.y() + 1)
            } else {
                term.scroll1();
                Vector2D::new(0, term.cursor.y())
            };
        };

        for &c in s.as_bytes() {
            if c == b'\n' {
                newline(self);
            } else {
                font::write_ascii(
                    &mut *self.window.write(),
                    self.calc_curosr_pos(),
                    c,
                    &PixelColor::new(255, 255, 255),
                );
                if self.cursor.x() == COLUMNS as i32 - 1 {
                    newline(self)
                } else {
                    self.cursor += Vector2D::new(1, 0);
                }
            }
        }

        self.draw_cursor(true);
    }

    fn execute_line(&mut self) {
        let mut command = vec![];
        self.linebuf[..self.linebuf_index].clone_into(&mut command);
        self.linebuf_index = 0;
        self.print("\n");

        let command = str::from_utf8(&command).unwrap();
        let splited: Vec<_> = command.split(' ').filter(|s| !s.is_empty()).collect();

        let Some(&command) = splited.first() else {
            return;
        };
        match command {
            "echo" => {
                if let Some(first_arg) = splited.get(1) {
                    self.print(first_arg);
                }
                self.print("\n");
            }
            "clear" => {
                self.window.write().fill_rectangle(
                    Vector2D::new(4, 4),
                    Vector2D::new(8 * COLUMNS as i32, 16 * ROWS as i32),
                    &PixelColor::new(0, 0, 0),
                );
                self.cursor = Vector2D::new(self.cursor.x(), 0);
            }
            command => {
                self.print("no such command: ");
                self.print(command);
                self.print("\n");
            }
        }
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}
