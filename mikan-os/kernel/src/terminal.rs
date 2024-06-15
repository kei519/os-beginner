use alloc::sync::Arc;

use crate::{
    asmfunc,
    graphics::{PixelColor, PixelWrite, Rectangle, Vector2D, FB_CONFIG},
    layer::LAYER_MANAGER,
    message::{Message, MessageType},
    sync::SharedLock,
    task,
    window::Window,
};

const ROWS: usize = 15;
const COLUMNS: usize = 60;

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

        if let MessageType::TimerTimeout(_) = msg.ty {
            let area = terminal.blink_cursor();

            let msg = Message::from_draw_area(task_id, terminal.layer_id, area);
            asmfunc::cli();
            task::send_message(1, msg).unwrap();
            asmfunc::sti();
        }
    }
}

pub struct Terminal {
    layer_id: u32,
    window: Arc<SharedLock<Window>>,
    cursor: Vector2D<i32>,
    cursor_visible: bool,
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
        }
    }

    pub fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.cursor_visible = !self.cursor_visible;
        self.draw_cursor();

        Rectangle {
            pos: Window::TOP_LEFT_MARGIN
                + Vector2D::new(4 + 8 * self.cursor.x(), 5 + 16 * self.cursor.y()),
            size: Vector2D::new(7, 15),
        }
    }

    fn draw_cursor(&mut self) {
        let color = if self.cursor_visible { 0xffffff } else { 0 };
        let color = PixelColor::to_color(color);
        let pos = Vector2D::new(4 + 8 * self.cursor.x(), 5 + 16 * self.cursor.y());

        self.window
            .write()
            .fill_rectangle(pos, Vector2D::new(7, 15), &color);
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}
