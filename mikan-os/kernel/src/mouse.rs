use alloc::boxed::Box;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use crate::{
    bitfield::BitField as _,
    console::DESKTOP_BG_COLOR,
    graphics::{PixelColor, PixelWrite, Vector2D, PIXEL_WRITER},
    layer::{LAYER_MANAGER, SCREEN},
    sync::OnceMutex,
    usb::HIDMouseDriver,
    window::Window,
};

pub static MOUSE_CURSOR: OnceMutex<MouseCursor> = OnceMutex::new();
pub static MOUSE_LAYER_ID: AtomicU32 = AtomicU32::new(0);

pub fn init() {
    // マウスカーソルの生成
    MOUSE_CURSOR.init(MouseCursor::new(
        &PIXEL_WRITER,
        DESKTOP_BG_COLOR,
        Vector2D::new(300, 200),
    ));

    HIDMouseDriver::set_default_observer(mouse_observer);

    let mut mouse_window = Window::new(
        MOUSE_CURSOR_WIDTH as u32,
        MOUSE_CURSOR_HEIGHT as u32,
        SCREEN.lock().pixel_format(),
    );

    let mut layer_manager = LAYER_MANAGER.lock();
    mouse_window.set_transparent_color(Some(MOUSE_TRANSPARENT_COLOR));
    draw_mouse_cursor(&mut mouse_window, &Vector2D::new(0, 0));
    let mouse_layer_id = layer_manager.new_layer(mouse_window);
    layer_manager
        .layer(mouse_layer_id)
        .move_relative(Vector2D::new(200, 200));
    layer_manager.up_down(mouse_layer_id, 3);
    MOUSE_LAYER_ID.store(mouse_layer_id, Ordering::Release);
}

pub fn mouse_observer(buttons: u8, displacement_x: i8, displacement_y: i8) {
    static MOUSE_DRAG_LAYER_ID: AtomicU32 = AtomicU32::new(0);
    static PREVIOUS_BUTTONS: AtomicU8 = AtomicU8::new(0);

    let mut layer_maneger = LAYER_MANAGER.lock();
    let layer_id = MOUSE_LAYER_ID.load(Ordering::Acquire);

    let oldpos = layer_maneger.layer(layer_id).pos();
    let newpos = oldpos + Vector2D::new(displacement_x as i32, displacement_y as i32);
    let newpos = Vector2D::element_min(&newpos, &layer_maneger.screen_size());
    let mouse_position = Vector2D::element_max(&newpos, &Vector2D::new(0, 0));

    let posdiff = mouse_position - oldpos;

    layer_maneger.r#move(layer_id, mouse_position);

    let previous_left_pressed = PREVIOUS_BUTTONS.load(Ordering::Acquire).get_bit(0);
    let left_pressed = buttons.get_bit(0);
    if !previous_left_pressed && left_pressed {
        if let Some(id) = layer_maneger.find_layer_by_position(&mouse_position, layer_id) {
            if layer_maneger.layer(id).is_draggable() {
                MOUSE_DRAG_LAYER_ID.store(id, Ordering::Release);
            }
        }
    } else if previous_left_pressed && left_pressed {
        let mouse_drag_layer_id = MOUSE_DRAG_LAYER_ID.load(Ordering::Acquire);
        if mouse_drag_layer_id != 0 {
            layer_maneger.move_relative(mouse_drag_layer_id, posdiff)
        }
    } else if previous_left_pressed && !left_pressed {
        MOUSE_DRAG_LAYER_ID.store(0, Ordering::Release);
    }

    PREVIOUS_BUTTONS.store(buttons, Ordering::Release);
}

/// マウスカーソルの横幅
pub const MOUSE_CURSOR_WIDTH: usize = 15;
/// マウスカーソルの高さ
pub const MOUSE_CURSOR_HEIGHT: usize = 24;
/// マウスの透明色
pub const MOUSE_TRANSPARENT_COLOR: PixelColor = PixelColor::new(0, 0, 1);
/// マウスカーソルの形
const MOUSE_CURSOR_SHAPE: [&[u8; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] = [
    b"@              ",
    b"@@             ",
    b"@.@            ",
    b"@..@           ",
    b"@...@          ",
    b"@....@         ",
    b"@.....@        ",
    b"@......@       ",
    b"@.......@      ",
    b"@........@     ",
    b"@.........@    ",
    b"@..........@   ",
    b"@...........@  ",
    b"@............@ ",
    b"@......@@@@@@@@",
    b"@......@       ",
    b"@....@@.@      ",
    b"@...@ @.@      ",
    b"@..@   @.@     ",
    b"@.@    @.@     ",
    b"@@      @.@    ",
    b"@       @.@    ",
    b"         @.@   ",
    b"         @@@   ",
];

pub struct MouseCursor {
    pixel_writer: &'static OnceMutex<Box<dyn PixelWrite + Send>>,
    erase_color: PixelColor,
    position: Vector2D<i32>,
}

impl MouseCursor {
    pub fn new(
        writer: &'static OnceMutex<Box<dyn PixelWrite + Send>>,
        erase_color: PixelColor,
        initial_position: Vector2D<i32>,
    ) -> Self {
        let mut ret = Self {
            pixel_writer: writer,
            erase_color,
            position: initial_position,
        };
        ret.draw_mouse_cursor();
        ret
    }

    pub fn move_relative(&mut self, displacement: Vector2D<i32>) {
        self.erase_mouse_cursor();
        self.position += displacement;
        self.draw_mouse_cursor();
    }

    fn draw_mouse_cursor(&mut self) {
        draw_mouse_cursor(&mut **self.pixel_writer.lock(), &self.position)
    }

    fn erase_mouse_cursor(&mut self) {
        for (dy, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
            for (dx, &b) in row.iter().enumerate() {
                if b != b' ' {
                    self.pixel_writer.lock().write(
                        self.position + Vector2D::new(dx as i32, dy as i32),
                        &self.erase_color,
                    )
                }
            }
        }
    }
}

pub fn draw_mouse_cursor(writer: &mut dyn PixelWrite, pos: &Vector2D<i32>) {
    for (dy, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, &b) in row.iter().enumerate() {
            let pos = *pos + Vector2D::new(dx as i32, dy as i32);
            match b {
                b'@' => writer.write(pos, &PixelColor::new(0, 0, 0)),
                b'.' => writer.write(pos, &PixelColor::new(255, 255, 255)),
                _ => writer.write(pos, &MOUSE_TRANSPARENT_COLOR),
            }
        }
    }
}
