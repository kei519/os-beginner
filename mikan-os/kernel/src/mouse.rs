use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use crate::{
    bitfield::BitField as _,
    graphics::{PixelColor, PixelWrite, Vector2D},
    layer::{LAYER_MANAGER, LAYER_TASK_MAP, SCREEN},
    message::{Message, MessageType},
    task,
    usb::HIDMouseDriver,
    window::Window,
};

pub static MOUSE_LAYER_ID: AtomicU32 = AtomicU32::new(0);

pub fn init() {
    let mut mouse_window = Window::new_base(
        MOUSE_CURSOR_WIDTH as u32,
        MOUSE_CURSOR_HEIGHT as u32,
        SCREEN.lock_wait().pixel_format(),
    );

    let mut layer_manager = LAYER_MANAGER.lock_wait();
    mouse_window.set_transparent_color(Some(MOUSE_TRANSPARENT_COLOR));
    draw_mouse_cursor(&mut mouse_window, &Vector2D::new(0, 0));
    let mouse_layer_id = layer_manager.new_layer(mouse_window);
    layer_manager.set_mouse_layer(mouse_layer_id);
    layer_manager
        .layer(mouse_layer_id)
        .move_relative(Vector2D::new(200, 200));
    layer_manager.up_down(mouse_layer_id, i32::MAX);
    MOUSE_LAYER_ID.store(mouse_layer_id, Ordering::Release);

    HIDMouseDriver::set_default_observer(mouse_observer);
}

pub fn mouse_observer(buttons: u8, displacement_x: i8, displacement_y: i8) {
    static MOUSE_DRAG_LAYER_ID: AtomicU32 = AtomicU32::new(0);
    static PREVIOUS_BUTTONS: AtomicU8 = AtomicU8::new(0);

    let mut layer_manager = LAYER_MANAGER.lock_wait();
    let layer_id = MOUSE_LAYER_ID.load(Ordering::Acquire);

    let oldpos = layer_manager.layer(layer_id).pos();
    let newpos = oldpos + Vector2D::new(displacement_x as i32, displacement_y as i32);
    let newpos = Vector2D::element_min(&newpos, &layer_manager.screen_size());
    let mouse_position = Vector2D::element_max(&newpos, &Vector2D::new(0, 0));

    let posdiff = mouse_position - oldpos;

    layer_manager.r#move(layer_id, mouse_position);

    let previous_left_pressed = PREVIOUS_BUTTONS.load(Ordering::Acquire).get_bit(0);
    let left_pressed = buttons.get_bit(0);
    if !previous_left_pressed && left_pressed {
        if let Some(id) = layer_manager.find_layer_by_position(&mouse_position, layer_id) {
            if layer_manager.layer(id).is_draggable() {
                // y 座標がタイトルバーの中のときだけドラッグモードに入る
                let y_layer = mouse_position.y() - layer_manager.layer(id).pos().y();
                if y_layer < Window::TOP_LEFT_MARGIN.y() {
                    MOUSE_DRAG_LAYER_ID.store(id, Ordering::Release);
                }
                layer_manager.activate(id);
            } else {
                layer_manager.activate(0);
            }
        }
    } else if previous_left_pressed && left_pressed {
        let mouse_drag_layer_id = MOUSE_DRAG_LAYER_ID.load(Ordering::Acquire);
        if mouse_drag_layer_id != 0 {
            layer_manager.move_relative(mouse_drag_layer_id, posdiff)
        }
    } else if previous_left_pressed && !left_pressed {
        MOUSE_DRAG_LAYER_ID.store(0, Ordering::Release);
    }
    drop(layer_manager);

    // ウィンドウのドラッグを行っているときは、
    // アクティブウィンドウの相対位置が動かないため送らない
    if MOUSE_DRAG_LAYER_ID.load(Ordering::Acquire) == 0 {
        send_mouse_message(newpos, posdiff, buttons);
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

fn send_mouse_message(newpos: Vector2D<i32>, posdiff: Vector2D<i32>, buttons: u8) {
    let manager = match LAYER_MANAGER.lock() {
        Some(m) => m,
        None => return,
    };
    let act = manager.get_active();
    if act == 0 {
        return;
    }
    // アクティブウィンドウなので必ず見つかる
    let layer = manager.find_layer(act).unwrap();

    let Some(task_id) = LAYER_TASK_MAP.lock_wait().get(&act).copied() else {
        return;
    };

    if posdiff != Vector2D::new(0, 0) {
        let relpos = newpos - layer.pos();
        let msg = Message {
            src_task: 0,
            ty: MessageType::MouseMove {
                x: relpos.x(),
                y: relpos.y(),
                dx: posdiff.x(),
                dy: posdiff.y(),
                buttons,
            },
        };
        let _ = task::send_message(task_id, msg);
    }
}
