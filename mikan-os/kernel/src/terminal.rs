use alloc::{
    collections::VecDeque,
    format,
    string::{String, ToString as _},
    sync::Arc,
    vec::Vec,
};
use core::{
    cmp,
    ffi::c_char,
    mem,
    ops::{Deref, DerefMut},
    ptr, slice, str,
};

use crate::{
    asmfunc,
    collections::HashMap,
    elf::{Elf64Ehdr, Elf64Phdr, ExecuteType, ProgramType},
    error::{Code, Result},
    fat::{self, DirectoryEntry, BYTES_PER_CLUSTER},
    file::FileDescriptor,
    font,
    graphics::{PixelColor, PixelWrite, Rectangle, Vector2D, FB_CONFIG},
    layer::{LAYER_MANAGER, LAYER_TASK_MAP},
    make_error,
    memory_manager::{BYTES_PER_FRAME, MEMORY_MANAGER},
    message::{Message, MessageType},
    paging::{self, LinearAddress4Level, PageMapEntry},
    pci,
    sync::{Mutex, SharedLock},
    task::{self, Task},
    timer::{Timer, TIMER_FREQ, TIMER_MANAGER},
    window::Window,
};
pub const APP_STACK_ADDR: u64 = 0xffff_ffff_ffff_e000;
pub const DEFAULT_APP_STACK_SIZE: u64 = 8 << 20;

pub const FILE_MAP_END: u64 = 0xffff_c000_0000_0000;

static APP_LOADS: Mutex<HashMap<&'static DirectoryEntry, AppLoadInfoTemplate>> =
    Mutex::new(HashMap::new());

/// [Terminal] のアドレスを保持し、参照を得るための構造体。
#[derive(Debug, Clone, Copy)]
pub struct TerminalRef(usize);

impl Deref for TerminalRef {
    type Target = Terminal;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self.0) }
    }
}

impl DerefMut for TerminalRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { mem::transmute(self.0) }
    }
}

impl From<&Terminal> for TerminalRef {
    fn from(value: &Terminal) -> Self {
        Self(value as *const _ as _)
    }
}

impl From<&mut Terminal> for TerminalRef {
    fn from(value: &mut Terminal) -> Self {
        (&*value).into()
    }
}

/// 通常タスクに渡される `data`, `layer_id` だが、ターミナルは両者を必要としないので、
/// `data` はターミナルを表示せずに実行するアプリのパス文字列の先頭へのポインタ、
/// `layer_id` はそのパス文字列の長さとして使うことにする。
/// `data` が `0` の場合は通常通りターミナルを表示する。
pub fn task_terminal(task_id: u64, s_ptr: i64, s_len: u32) {
    let show_window = s_ptr == 0;

    let mut terminal = Terminal::new(task_id, show_window);
    asmfunc::cli();
    let task = task::current_task();
    asmfunc::sti();
    if show_window {
        let mut manager = LAYER_MANAGER.lock_wait();
        manager.r#move(terminal.layer_id, Vector2D::new(100, 200));
        manager.activate(terminal.layer_id);
        LAYER_TASK_MAP
            .lock_wait()
            .insert(terminal.layer_id, task_id);
    }

    // ウィンドウを表示しないということは、外から実行パスが与えられているということ
    if !show_window {
        let command_line = unsafe { slice::from_raw_parts(s_ptr as *const u8, s_len as _) };
        for &b in command_line {
            terminal.input_key(0, 0, b);
        }
        terminal.input_key(0, 0, b'\n');
    }

    let add_blink_timer = |t| {
        TIMER_MANAGER.lock_wait().add_timer(Timer::new(
            t + (TIMER_FREQ as f64 * 0.5) as u64,
            1,
            task_id,
        ));
    };
    let current_tick = TIMER_MANAGER.lock_wait().current_tick();
    if show_window {
        add_blink_timer(current_tick);
    }

    let mut window_isactive = true;

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
            MessageType::TimerTimeout { timeout, .. } => {
                if show_window && window_isactive {
                    add_blink_timer(timeout);
                    let mut area = terminal.blink_cursor();
                    area.pos += Window::TOP_LEFT_MARGIN;

                    let msg = Message::from_draw_area(task_id, terminal.layer_id, area);
                    asmfunc::cli();
                    task::send_message(1, msg).unwrap();
                    asmfunc::sti();
                }
            }
            MessageType::KeyPush {
                modifier,
                keycode,
                ascii,
                press,
            } => {
                if press {
                    let mut area = terminal.input_key(modifier, keycode, ascii);
                    if show_window {
                        area.pos += Window::TOP_LEFT_MARGIN;
                        let msg = Message::from_draw_area(task_id, terminal.layer_id, area);
                        asmfunc::cli();
                        task::send_message(1, msg).unwrap();
                        asmfunc::sti();
                    }
                }
            }
            MessageType::WindowActive { activate } => {
                window_isactive = activate;
                let current_time = TIMER_MANAGER.lock_wait().current_tick();
                add_blink_timer(current_time);
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
    task_id: u64,
    /// `window` が `None` の場合は表ターミナルを表示しないことを表す。
    window: Option<Arc<SharedLock<Window>>>,
    cursor: Vector2D<i32>,
    cursor_visible: bool,
    linebuf_index: usize,
    linebuf: [u8; LINE_MAX],
    cmd_history: VecDeque<String>,
    cmd_history_index: i32,
}

impl Terminal {
    pub fn new(task_id: u64, show_window: bool) -> Self {
        let (layer_id, window) = if show_window {
            let mut window = Window::new_toplevel(
                COLUMNS as u32 * 8 + 8 + Window::MARGIN_X,
                ROWS as u32 * 16 + 8 + Window::MARGIN_Y,
                FB_CONFIG.as_ref().pixel_format,
                "MikanTerm",
            );
            let size = window.size();
            window.draw_terminal(Vector2D::new(0, 0), size);

            let mut manager = LAYER_MANAGER.lock_wait();
            let id = manager.new_layer(window);
            manager.layer(id).set_draggable(true);
            let window = manager.layer(id).window();

            (id, Some(window))
        } else {
            (0, None)
        };

        let mut cmd_history = VecDeque::new();
        cmd_history.resize(8, String::new());

        let mut ret = Self {
            layer_id,
            task_id,
            window,
            cursor: Vector2D::new(0, 0),
            cursor_visible: false,
            linebuf_index: 0,
            linebuf: [0u8; LINE_MAX],
            cmd_history,
            cmd_history_index: -1,
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

    /// `linebuf` や `linebuf_index` を変更せずに文字列を表示する。
    pub fn print(&mut self, s: &str) {
        let Some(window) = self.window.clone() else {
            return;
        };

        let cursor_before = self.calc_curosr_pos();
        self.draw_cursor(false);

        let newline = |term: &mut Self| {
            term.cursor = if term.cursor.y() < ROWS as i32 - 1 {
                Vector2D::new(0, term.cursor.y() + 1)
            } else {
                term.scroll1();
                Vector2D::new(0, term.cursor.y())
            };
        };

        for c in s.chars() {
            if c == '\n' {
                newline(self);
            } else if c.is_ascii() {
                if self.cursor.x() == COLUMNS as i32 {
                    newline(self)
                }
                font::write_unicode(
                    &mut *window.write(),
                    self.calc_curosr_pos(),
                    c,
                    &PixelColor::new(255, 255, 255),
                );
                self.cursor += Vector2D::new(1, 0);
            } else {
                if self.cursor.x() >= COLUMNS as i32 - 1 {
                    newline(self);
                }
                font::write_unicode(
                    &mut *window.write(),
                    self.calc_curosr_pos(),
                    c,
                    &PixelColor::new(255, 255, 255),
                );
                self.cursor += Vector2D::new(2, 0);
            }
        }

        self.draw_cursor(true);
        let cursor_after = self.calc_curosr_pos();

        let draw_pos = Window::TOP_LEFT_MARGIN + Vector2D::new(0, cursor_before.y());
        let draw_size = Vector2D::new(
            window.read().width() as _,
            cursor_after.y() - cursor_before.y() + 16,
        );
        let draw_area = Rectangle {
            pos: draw_pos,
            size: draw_size,
        };

        let msg = Message::from_draw_area(self.task_id, self.layer_id, draw_area);
        asmfunc::cli();
        task::send_message(1, msg).unwrap();
        asmfunc::sti();
    }

    pub fn input_key(&mut self, _modifier: u8, keycode: u8, ascii: u8) -> Rectangle<i32> {
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
                self.print(">");
                if let Some(ref window) = self.window {
                    draw_area.pos = Vector2D::new(0, 0);
                    draw_area.size = window.read().size();
                }
            }
            0x08 => {
                // backspace
                if self.cursor.x() > 0 && self.linebuf_index > 0 {
                    self.cursor -= Vector2D::new(1, 0);
                    if let Some(ref window) = self.window {
                        window.write().draw_rectangle(
                            self.calc_curosr_pos(),
                            Vector2D::new(8, 16),
                            &PixelColor::new(0, 0, 0),
                        );
                    }
                    self.linebuf_index -= 1;
                }
            }
            ascii => {
                if self.cursor.x() < COLUMNS as i32 - 1 && self.linebuf_index < LINE_MAX - 1 {
                    self.linebuf[self.linebuf_index] = ascii;
                    self.linebuf_index += 1;
                    if let Some(ref window) = self.window {
                        font::write_ascii(
                            &mut *window.write(),
                            self.calc_curosr_pos(),
                            ascii,
                            &PixelColor::new(255, 255, 255),
                        );
                    }
                    self.cursor += Vector2D::new(1, 0);
                }
            }
        }

        self.draw_cursor(true);

        draw_area
    }

    fn draw_cursor(&mut self, visible: bool) {
        let Some(ref window) = self.window else {
            return;
        };

        self.cursor_visible = visible;
        let color = if self.cursor_visible { 0xffffff } else { 0 };
        let color = PixelColor::to_color(color);
        let pos = Vector2D::new(4 + 8 * self.cursor.x(), 5 + 16 * self.cursor.y());

        window
            .write()
            .fill_rectangle(pos, Vector2D::new(7, 15), &color);
    }

    fn calc_curosr_pos(&self) -> Vector2D<i32> {
        Vector2D::new(4 + 8 * self.cursor.x(), 4 + 16 * self.cursor.y())
    }

    fn scroll1(&mut self) {
        let Some(ref window) = self.window else {
            return;
        };
        let move_src = Rectangle {
            pos: Vector2D::new(4, 4 + 16),
            size: Vector2D::new(8 * COLUMNS as i32, 16 * (ROWS as i32 - 1)),
        };
        let mut window = window.write();
        window.r#move(Vector2D::new(4, 4), &move_src);
        window.fill_rectangle(
            Vector2D::new(4, 4 + 16 * self.cursor.y()),
            Vector2D::new(8 * COLUMNS as i32, 16),
            &PixelColor::new(0, 0, 0),
        );
    }

    fn execute_line(&mut self, command: String) {
        let args: Vec<_> = command.split(' ').filter(|s| !s.is_empty()).collect();

        let Some(&command) = args.first() else {
            return;
        };
        match command {
            "echo" => {
                if let Some(first_arg) = args.get(1) {
                    self.print(first_arg);
                }
                self.print("\n");
            }
            "clear" => {
                if let Some(ref window) = self.window {
                    window.write().fill_rectangle(
                        Vector2D::new(4, 4),
                        Vector2D::new(8 * COLUMNS as i32, 16 * ROWS as i32),
                        &PixelColor::new(0, 0, 0),
                    );
                    self.cursor = Vector2D::new(self.cursor.x(), 0);
                }
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
                    self.print(&s);
                }
            }
            "ls" => {
                let Some(&first_arg) = args.get(1) else {
                    self.list_all_entries(fat::BOOT_VOLUME_IMAGE.get().root_clus());
                    return;
                };

                let (Some(dir), post_slash) = fat::find_file(first_arg, 0) else {
                    self.print("No such file or directory: ");
                    self.print(first_arg);
                    self.print("\n");
                    return;
                };
                if dir.attr == fat::Attribute::Directory as _ {
                    self.list_all_entries(dir.first_cluster());
                } else {
                    let (base, ext) = fat::read_name(dir);
                    // Safety: ASCII 文字列だけが格納されている
                    let name = if ext.is_empty() {
                        base.to_string()
                    } else {
                        format!("{}.{}", base, ext)
                    };
                    if post_slash {
                        self.print(&name);
                        self.print(" is not a directory\n");
                    } else {
                        self.print(&name);
                        self.print("\n");
                    }
                }
            }
            "cat" => {
                let Some(file_path) = args.get(1) else {
                    self.print("Usage: cat <file>\n");
                    return;
                };
                let (Some(file_entry), post_slash) = fat::find_file(file_path, 0) else {
                    self.print(&format!("no such file: {}\n", file_path));
                    return;
                };
                if file_entry.attr != fat::Attribute::Directory as _ && post_slash {
                    self.print(file_path);
                    self.print(" is not a directory\n");
                    return;
                }

                let mut cluster = file_entry.first_cluster() as u64;
                let mut remain_bytes = file_entry.file_size;

                self.draw_cursor(false);
                let bytes_per_cluster = fat::BYTES_PER_CLUSTER.get();
                while cluster != 0 && cluster != fat::END_OF_CLUSTER_CHAIN {
                    let s = fat::get_sector_by_cluster::<u8>(
                        cluster,
                        cmp::min(bytes_per_cluster as _, remain_bytes as _),
                    );
                    let s = String::from_utf8_lossy(s);

                    self.print(&s);
                    remain_bytes -= s.len() as u32;
                    cluster = fat::next_cluster(cluster);
                }
            }
            "noterm" => {
                if let Some(&command) = args.get(1) {
                    asmfunc::cli();
                    task::new_task()
                        .init_context(task_terminal, command.as_ptr() as _, command.len() as _)
                        .wake_up(-1);
                    asmfunc::sti();
                }
            }
            "ulimit" => {
                if args.len() >= 3 {
                    if "-s" == args[1] {
                        if let Ok(size) = args[2].parse::<u64>() {
                            asmfunc::cli();
                            let task = task::current_task();
                            asmfunc::sti();
                            task.set_app_stack_size(size << 10);
                        } else {
                            self.print("Usage: ulimit -s <size (KiB)>\n");
                        }
                    }
                } else {
                    asmfunc::cli();
                    let task = task::current_task();
                    asmfunc::sti();
                    let s = format!("stack_size: {} KiB\n", task.app_stack_size() >> 10);
                    self.print(&s);
                }
            }
            "memstat" => {
                let stat = MEMORY_MANAGER.stat();
                let s = format!(
                    "Phys used : {} frames ({} MiB)\n\
                    Phys total: {} frames ({} MiB)\n",
                    stat.allocated_frames,
                    (stat.allocated_frames * BYTES_PER_FRAME) >> 20,
                    stat.total_frames,
                    (stat.total_frames * BYTES_PER_FRAME) >> 20,
                );
                self.print(&s);
            }
            command => match fat::find_file(command, 0) {
                (Some(file_entry), post_slash) => {
                    if file_entry.attr != fat::Attribute::Directory as _ && post_slash {
                        self.print(command);
                        self.print(" is not a directory\n");
                        return;
                    }
                    match self.execute_file(file_entry, args) {
                        Err(e) => self.print(&format!("failed to exec file: {}\n", e)),
                        Ok(code) => {
                            self.print(&format!("app exited. ret = {}\n", code));
                        }
                    }
                }
                (None, _) => {
                    self.print("no such command: ");
                    self.print(command);
                    self.print("\n");
                }
            },
        }
    }

    fn history_up_down(&mut self, direction: i32) -> Rectangle<i32> {
        let Some(ref window) = self.window else {
            return Rectangle {
                pos: Vector2D::default(),
                size: Vector2D::default(),
            };
        };

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
        window
            .write()
            .fill_rectangle(draw_area.pos, draw_area.size, &PixelColor::new(0, 0, 0));

        let history = if self.cmd_history_index >= 0 {
            self.cmd_history[self.cmd_history_index as usize].as_bytes()
        } else {
            b""
        };

        self.linebuf[..history.len()].copy_from_slice(history);
        self.linebuf_index = history.len();

        // Safety: 入力できる文字は ASCII に限られている
        let history = unsafe { core::str::from_utf8_unchecked(history) };
        font::write_string(
            &mut *window.write(),
            first_pos,
            history,
            &PixelColor::new(255, 255, 255),
        );
        self.cursor += Vector2D::new(history.len() as i32, 0);
        draw_area
    }

    fn execute_file(
        &mut self,
        file_entry: &'static DirectoryEntry,
        args: Vec<&str>,
    ) -> Result<i32> {
        asmfunc::cli();
        let task = task::current_task();
        asmfunc::sti();

        paging::setup_pml4(&task)?;

        let app_load = load_app(file_entry, &task)?;

        // デマンドページを ELF バイナリの最後から割り当てる
        let elf_next_page = (app_load.vaddr_end + 4095) & !0xfff;
        task.set_dpaging_begin(elf_next_page);
        task.set_dpaging_end(elf_next_page);

        let stack_frame_addr = LinearAddress4Level {
            addr: APP_STACK_ADDR,
        };
        paging::setup_page_maps(stack_frame_addr, 1, true)?;

        let args_frame_addr = LinearAddress4Level {
            addr: 0xffff_ffff_ffff_f000,
        };
        paging::setup_page_maps(args_frame_addr, 1, true)?;
        let arg_buf =
            unsafe { slice::from_raw_parts_mut(args_frame_addr.addr as *mut u8, BYTES_PER_FRAME) };
        let argc = make_arg_vector(args, arg_buf)?;

        asmfunc::cli();
        let task = task::current_task();
        asmfunc::sti();

        // 標準入出力（現在は標準入力のみ）の設定
        {
            let mut files = task.files().lock_wait();
            // Safety: TerminalRef が持たれるのは、
            //         ターミナルの制御がアプリに移っている間だけであり、
            //         その間そのターミナルではアプリ以外のことができないので問題ない
            for i in 0..3 {
                files.insert(i, FileDescriptor::new_term(task.clone(), self.into()));
            }
        }

        let ret = asmfunc::call_app(
            argc as _,
            args_frame_addr.addr as _,
            3 << 3 | 3,
            app_load.entry as _,
            stack_frame_addr.addr + BYTES_PER_FRAME as u64 * 2 - 8,
            task.os_stack_ptr(),
        );

        // アプリの実行が終了したら、現在のファイルディスクリプタを全削除
        {
            let mut files = task.files().lock_wait();
            files.clear();
            let mut file_maps = task.file_maps().lock_wait();
            file_maps.clear();
        }

        paging::clean_page_maps(LinearAddress4Level {
            addr: 0xffff_8000_0000_0000,
        });

        paging::free_pml4(&task);

        Ok(ret)
    }

    fn list_all_entries(&mut self, mut dir_cluster: u32) {
        let entries_per_cluster =
            BYTES_PER_CLUSTER.get() as usize / mem::size_of::<fat::DirectoryEntry>();

        while dir_cluster != fat::END_OF_CLUSTER_CHAIN as _ {
            let dir = fat::get_sector_by_cluster::<fat::DirectoryEntry>(
                dir_cluster as _,
                entries_per_cluster,
            );

            for entry in dir {
                let (base, ext) = fat::read_name(entry);
                // ファイル終了
                if base.as_bytes()[0] == 0x00 {
                    return;
                } else if base.as_bytes()[0] == 0x5e || entry.attr == fat::Attribute::LongName as u8
                {
                    continue;
                }

                let s = if !ext.is_empty() {
                    format!("{}.{}\n", base, ext)
                } else {
                    format!("{}\n", base)
                };
                self.print(&s);
            }

            dir_cluster = fat::next_cluster(dir_cluster as _) as _;
        }
    }
}

/// アプリの情報と、コピーオンライトの雛形になっているページディレクトリの情報を保持する。
#[derive(Debug, Clone)]
struct AppLoadInfoTemplate {
    entry: u64,
    vaddr_end: u64,
    pml4: &'static [PageMapEntry],
}

/// アプリの情報（ページディレクトリを含む）を保持する。
#[derive(Debug)]
struct AppLoadInfo {
    entry: u64,
    vaddr_end: u64,
    pml4: &'static mut [PageMapEntry],
}

impl AppLoadInfo {
    fn new(template: &AppLoadInfoTemplate, pml4: &'static mut [PageMapEntry]) -> Self {
        Self {
            entry: template.entry,
            vaddr_end: template.vaddr_end,
            pml4,
        }
    }
}

/// ロードした ELF バイナリの最終アドレスを返す。
fn load_elf(ehdr: &Elf64Ehdr) -> Result<u64> {
    if ehdr.r#type != ExecuteType::Exec {
        return Err(make_error!(Code::InvalidFormat));
    }

    let addr_first = get_first_load_address(ehdr);
    if addr_first < 0xffff_8000_0000_0000 {
        return Err(make_error!(Code::InvalidFormat));
    }

    copy_load_segments(ehdr)
}

/// アプリがロードされていなければ読み取り専用でロードし、
/// 既にどこかにロードされている場合は PT（ページテーブル）ごとその浅いコピーを返す。
fn load_app(file_entry: &'static DirectoryEntry, task: &Arc<Task>) -> Result<AppLoadInfo> {
    let temp_pml4 = paging::setup_pml4(task)?;

    let mut app_loads = APP_LOADS.lock_wait();
    if let Some(app_load) = app_loads.get(&file_entry).cloned() {
        paging::copy_page_maps(temp_pml4, app_load.pml4, 4, 256)?;
        return Ok(AppLoadInfo::new(&app_load, temp_pml4));
    }

    let file_buf = fat::load_file(file_entry);

    let elf_header: &Elf64Ehdr = unsafe { &*(file_buf.as_ptr() as *const _) };
    if &elf_header.ident[..4] != b"\x7fELF" {
        return Err(make_error!(Code::InvalidFile));
    }

    let last_addr = load_elf(elf_header)?;

    let app_load_temp = AppLoadInfoTemplate {
        entry: elf_header.entry as _,
        vaddr_end: last_addr,
        pml4: &*temp_pml4,
    };

    app_loads.insert(file_entry, app_load_temp.clone());

    let app_load = AppLoadInfo::new(&app_load_temp, paging::setup_pml4(task)?);
    paging::copy_page_maps(app_load.pml4, app_load_temp.pml4, 4, 256)?;
    Ok(app_load)
}

fn get_first_load_address(ehdr: &Elf64Ehdr) -> usize {
    for phdr in get_program_headers(ehdr) {
        if phdr.r#type != ProgramType::Load as _ {
            continue;
        }
        return phdr.vaddr;
    }
    0
}

fn get_program_headers(ehdr: &Elf64Ehdr) -> &[Elf64Phdr] {
    unsafe {
        slice::from_raw_parts(
            (ehdr as *const Elf64Ehdr).byte_add(ehdr.phoff as usize) as *const _,
            ehdr.phnum as usize,
        )
    }
}

/// ロードした ELF バイナリの最終アドレスを返す。
fn copy_load_segments(ehdr: &Elf64Ehdr) -> Result<u64> {
    let mut elf_last_addr = 0;

    for phdr in get_program_headers(ehdr) {
        if phdr.r#type != ProgramType::Load as _ {
            continue;
        }

        let dest_addr = LinearAddress4Level {
            addr: phdr.vaddr as _,
        };

        let seg_last_addr = phdr.vaddr + phdr.memsz as usize;
        elf_last_addr = elf_last_addr.max(seg_last_addr as _);
        // `phdr.vaddr` が 4 KB アラインされているわけではないので、
        // 4 KB アラインの先頭から数える必要がある
        let num_4kpages = ((phdr.vaddr & 0xfff) + phdr.memsz as usize + 4095) / 4096;

        paging::setup_page_maps(dest_addr, num_4kpages, false)?;

        unsafe {
            let src = (ehdr as *const _ as *const u8).add(phdr.offset as usize);
            let dst = phdr.vaddr as *mut u8;
            ptr::copy_nonoverlapping(src, dst, phdr.filesz as _);
            ptr::write_bytes(
                dst.byte_add(phdr.filesz as _),
                0,
                (phdr.memsz - phdr.filesz) as _,
            );
        }
    }

    Ok(elf_last_addr)
}

/// 引数を配置し、引数の数を返す。
/// ただし `args` は32個まで。
fn make_arg_vector(args: Vec<&str>, buf: &mut [u8]) -> Result<usize> {
    let len = args.len();
    if len >= 32 {
        return Err(make_error!(Code::InvalidFormat, "too many args"));
    }

    if buf.len() < 32 * mem::size_of::<*const c_char>() {
        return Err(make_error!(Code::BufferTooSmall));
    }

    let mut cur = 32 * mem::size_of::<*const c_char>();
    for (i, arg) in args.into_iter().enumerate() {
        // null 文字分多く必要
        if cur + arg.len() + 1 >= buf.len() {
            return Err(make_error!(Code::BufferTooSmall));
        }

        unsafe {
            *(buf.as_ptr() as *mut *const c_char).add(i) =
                buf.as_ptr().byte_add(cur) as *const c_char;
        }
        buf[cur..cur + arg.len()].clone_from_slice(arg.as_bytes());
        cur += arg.len() + 1;
        buf[cur - 1] = 0;
    }
    Ok(len)
}
