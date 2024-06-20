use alloc::{collections::VecDeque, format, string::String, sync::Arc, vec::Vec};
use core::{cmp, mem, str};

use crate::{
    asmfunc,
    fat::{self, DirectoryEntry, BYTES_PER_CLUSTER, END_OF_CLUSTER_CHAIN},
    font,
    graphics::{PixelColor, PixelWrite, Rectangle, Vector2D, FB_CONFIG},
    layer::{LAYER_MANAGER, LAYER_TASK_MAP},
    message::{Message, MessageType},
    pci,
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
    cmd_history: VecDeque<String>,
    cmd_history_index: i32,
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

        let mut cmd_history = VecDeque::new();
        cmd_history.resize(8, String::new());

        let mut ret = Self {
            layer_id,
            window,
            cursor: Vector2D::new(0, 0),
            cursor_visible: false,
            linebuf_index: 0,
            linebuf: [0u8; LINE_MAX],
            cmd_history,
            cmd_history_index: -1,
        };
        ret.print(b">");
        ret
    }

    pub fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.draw_cursor(!self.cursor_visible);

        Rectangle {
            pos: self.calc_curosr_pos(),
            size: Vector2D::new(7, 15),
        }
    }

    fn input_key(&mut self, _modifier: u8, keycode: u8, ascii: u8) -> Rectangle<i32> {
        self.draw_cursor(false);

        let mut draw_area = Rectangle {
            pos: self.calc_curosr_pos(),
            size: Vector2D::new(8 * 2, 16),
        };

        match ascii {
            0 => {
                draw_area = match keycode {
                    // down arrow
                    0x51 => self.history_up_down(-1),
                    // up arrow
                    0x52 => self.history_up_down(1),
                    _ => draw_area,
                }
            }
            b'\n' => {
                let command =
                    String::from(str::from_utf8(&self.linebuf[..self.linebuf_index]).unwrap());
                if self.linebuf_index > 0 {
                    self.cmd_history.pop_back();
                    self.cmd_history.push_front(command.clone());
                }
                self.linebuf_index = 0;
                self.cmd_history_index = -1;

                self.cursor = if self.cursor.y() < ROWS as i32 - 1 {
                    Vector2D::new(0, self.cursor.y() + 1)
                } else {
                    self.scroll1();
                    Vector2D::new(0, self.cursor.y())
                };

                self.execute_line(command);
                self.print(b">");
                draw_area.pos = Vector2D::new(0, 0);
                draw_area.size = self.window.read().size();
            }
            0x08 => {
                // backspace
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
    fn print(&mut self, s: &[u8]) {
        self.draw_cursor(false);

        let newline = |term: &mut Self| {
            term.cursor = if term.cursor.y() < ROWS as i32 - 1 {
                Vector2D::new(0, term.cursor.y() + 1)
            } else {
                term.scroll1();
                Vector2D::new(0, term.cursor.y())
            };
        };

        for &c in s {
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

    fn execute_line(&mut self, command: String) {
        let args: Vec<_> = command.split(' ').filter(|s| !s.is_empty()).collect();

        let Some(&command) = args.first() else {
            return;
        };
        match command {
            "echo" => {
                if let Some(first_arg) = args.get(1) {
                    self.print(first_arg.as_bytes());
                }
                self.print(b"\n");
            }
            "clear" => {
                self.window.write().fill_rectangle(
                    Vector2D::new(4, 4),
                    Vector2D::new(8 * COLUMNS as i32, 16 * ROWS as i32),
                    &PixelColor::new(0, 0, 0),
                );
                self.cursor = Vector2D::new(self.cursor.x(), 0);
            }
            "lspci" => {
                for dev in pci::DEVICES.read().iter() {
                    let vendor_id = dev.read_vendor_id();
                    let s = format!(
                        "{:02x}:{:02x}.{} vend={:04x} head={:02x} class={:02x}.{:02x}:{:02x}\n",
                        dev.bus(),
                        dev.device(),
                        dev.function(),
                        vendor_id,
                        dev.header_type(),
                        dev.class_code().base(),
                        dev.class_code().sub(),
                        dev.class_code().interface(),
                    );
                    self.print(s.as_bytes());
                }
            }
            "ls" => {
                let image = fat::BOOT_VOLUME_IMAGE.get();
                let entries_per_cluster =
                    image.byts_per_sec() as usize / mem::size_of::<fat::DirectoryEntry>();
                let root_dir_entries = fat::get_sector_by_cluster::<fat::DirectoryEntry>(
                    image.root_clus() as u64,
                    entries_per_cluster,
                );

                for entry in root_dir_entries {
                    let (base, ext) = fat::read_name(entry);
                    if base[0] == 0x00 {
                        break;
                    } else if base[0] == 0x5e {
                        continue;
                    } else if entry.attr == fat::Attribute::LongName as u8 {
                        continue;
                    }

                    let s = if !ext.is_empty() {
                        format!(
                            "{}.{}\n",
                            str::from_utf8(base).unwrap(),
                            str::from_utf8(ext).unwrap()
                        )
                    } else {
                        format!("{}\n", str::from_utf8(base).unwrap())
                    };
                    self.print(s.as_bytes());
                }
            }
            "cat" => {
                let Some(file_name) = args.get(1) else {
                    self.print(b"Usage: cat <file>\n");
                    return;
                };
                let Some(file_entry) = fat::find_file(file_name, 0) else {
                    self.print(format!("no such file: {}\n", file_name).as_bytes());
                    return;
                };

                let mut cluster = file_entry.first_cluster() as u64;
                let mut remain_bytes = file_entry.file_size;

                self.draw_cursor(false);
                let bytes_per_cluster = fat::BYTES_PER_CLUSTER.get();
                while cluster != 0 && cluster != fat::END_OF_CLUSTER_CHAIN {
                    let s = fat::get_sector_by_cluster::<u8>(
                        cluster,
                        cmp::min(bytes_per_cluster as _, remain_bytes as _),
                    );

                    self.print(s);
                    remain_bytes -= s.len() as u32;
                    cluster = fat::next_cluster(cluster);
                }
            }
            command => match fat::find_file(command, 0) {
                Some(file_entry) => self.execute_file(file_entry),
                None => {
                    self.print(b"no such command: ");
                    self.print(command.as_bytes());
                    self.print(b"\n");
                }
            },
        }
    }

    fn history_up_down(&mut self, direction: i32) -> Rectangle<i32> {
        if direction == -1 && self.cmd_history_index >= 0 {
            self.cmd_history_index -= 1;
        } else if direction == 1 && self.cmd_history_index + 1 < self.cmd_history.len() as i32 {
            self.cmd_history_index += 1;
        }

        // プロンプト分の1
        self.cursor = Vector2D::new(1, self.cursor.y());
        let first_pos = self.calc_curosr_pos();

        let draw_area = Rectangle {
            pos: first_pos,
            size: Vector2D::new(8 * (COLUMNS as i32 - 1), 16),
        };
        self.window.write().fill_rectangle(
            draw_area.pos,
            draw_area.size,
            &PixelColor::new(0, 0, 0),
        );

        let history = if self.cmd_history_index >= 0 {
            self.cmd_history[self.cmd_history_index as usize].as_bytes()
        } else {
            b""
        };

        self.linebuf[..history.len()].copy_from_slice(history);
        self.linebuf_index = history.len();

        font::write_string(
            &mut *self.window.write(),
            first_pos,
            history,
            &PixelColor::new(255, 255, 255),
        );
        self.cursor += Vector2D::new(history.len() as i32, 0);
        draw_area
    }

    fn execute_file(&mut self, file_entry: &DirectoryEntry) {
        let mut cluster = file_entry.first_cluster() as u64;
        let mut remain_bytes = file_entry.file_size as _;

        let mut file_buf = Vec::<u8>::with_capacity(remain_bytes);

        while cluster != 0 && cluster != END_OF_CLUSTER_CHAIN {
            let copy_bytes = cmp::min(BYTES_PER_CLUSTER.get() as _, remain_bytes);
            file_buf.extend_from_slice(fat::get_sector_by_cluster(cluster, copy_bytes));

            remain_bytes -= copy_bytes;
            cluster = fat::next_cluster(cluster);
        }

        type Func = fn();
        let f: Func = unsafe { mem::transmute(file_buf.as_ptr()) };
        f();
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}
