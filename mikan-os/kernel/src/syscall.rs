use core::{ffi::CStr, mem, slice};

use alloc::sync::Arc;

use crate::{
    app_event::AppEvent,
    asmfunc,
    bitfield::BitField,
    errno::ErrNo,
    font,
    graphics::{PixelColor, PixelWrite as _, Rectangle, Vector2D, FB_CONFIG},
    keyboard::{LCONTROL_BIT, RCONTROL_BIT},
    layer::{LAYER_MANAGER, LAYER_TASK_MAP},
    log,
    logger::LogLevel,
    message::MessageType,
    msr::{IA32_EFER, IA32_FMASK, IA32_LSTAR, IA32_STAR},
    sync::SharedLock,
    task, terminal,
    timer::{TIMER_FREQ, TIMER_MANAGER},
    window::Window,
};

pub type SyscallFuncType = extern "sysv64" fn(u64, u64, u64, u64, u64, u64) -> Result;

#[no_mangle]
pub static SYSCALL_TABLE: [SyscallFuncType; 11] = [
    log_string,
    put_string,
    exit,
    open_window,
    win_write_string,
    win_fill_rectangle,
    get_current_tick,
    win_redraw,
    win_draw_line,
    close_window,
    read_event,
];

pub fn init() {
    asmfunc::write_msr(IA32_EFER, 0x0501);
    asmfunc::write_msr(IA32_LSTAR, asmfunc::syscall_entry as usize as _);
    // [47:32] が syscall 時に設定されるセグメント
    // [64:48] が sysret 時に設定されるセグメント を決める
    asmfunc::write_msr(IA32_STAR, 8 << 32 | (16 | 3) << 48);
    asmfunc::write_msr(IA32_FMASK, 0);
}

#[repr(C)]
pub struct Result {
    value: u64,
    error: i32,
}

impl Result {
    fn new(value: u64, error: impl Into<i32>) -> Self {
        Self {
            value,
            error: error.into(),
        }
    }

    fn value(value: u64) -> Self {
        Self::new(value, 0)
    }

    fn error(error: impl Into<i32>) -> Self {
        Self::new(0, error)
    }
}

impl From<ErrNo> for Result {
    fn from(value: ErrNo) -> Self {
        Self::error(value)
    }
}

extern "sysv64" fn log_string(arg1: u64, arg2: u64, _: u64, _: u64, _: u64, _: u64) -> Result {
    let log_level: LogLevel = match arg1.try_into() {
        Ok(level) => level,
        Err(_) => return ErrNo::EPERM.into(),
    };

    let s = match unsafe { CStr::from_ptr(arg2 as _) }.to_str() {
        Ok(s) => s,
        Err(_) => return ErrNo::EINVAL.into(),
    };

    log!(log_level, "{}", s);
    Result::value(s.len() as _)
}

extern "sysv64" fn put_string(arg1: u64, arg2: u64, arg3: u64, _: u64, _: u64, _: u64) -> Result {
    let fd = arg1;
    let s: &[u8] = unsafe { slice::from_raw_parts(arg2 as _, arg3 as _) };

    if fd == 1 {
        let task_id = task::current_task().id();
        // システムコールを呼び出す可能性があるのは、ターミナル上で起動したアプリだけなので、
        // そのターミナルは必ず存在するため、unwrap は必ず成功する
        let mut terminal = terminal::get_term(task_id).unwrap();
        terminal.print(s);
        Result::value(s.len() as _)
    } else {
        ErrNo::EBADF.into()
    }
}

extern "sysv64" fn exit(arg1: u64, _: u64, _: u64, _: u64, _: u64, _: u64) -> Result {
    asmfunc::cli();
    let task = task::current_task();
    asmfunc::sti();
    Result::new(*task.os_stack_ptr(), arg1 as i32)
}

extern "sysv64" fn open_window(w: u64, h: u64, x: u64, y: u64, title: u64, _: u64) -> Result {
    let w = w as u32;
    let h = h as u32;
    let x = x as i32;
    let y = y as i32;
    let title = match unsafe { CStr::from_ptr(title as _) }.to_str() {
        Ok(s) => s,
        Err(_) => return ErrNo::EINVAL.into(),
    };

    let win = Window::new_toplevel(w, h, FB_CONFIG.as_ref().pixel_format, title);

    let mut manager = LAYER_MANAGER.lock_wait();
    let layer_id = manager.new_layer(win);
    manager
        .layer(layer_id)
        .set_draggable(true)
        .r#move(Vector2D::new(x, y));
    manager.activate(layer_id);

    asmfunc::cli();
    let task_id = task::current_task().id();
    asmfunc::sti();
    LAYER_TASK_MAP.lock_wait().insert(layer_id, task_id);

    Result::value(layer_id as _)
}

extern "sysv64" fn win_write_string(
    layer_id_flags: u64,
    x: u64,
    y: u64,
    color: u64,
    s: u64,
    _: u64,
) -> Result {
    let s = match unsafe { CStr::from_ptr(s as _) }.to_str() {
        Ok(s) => s,
        Err(_) => return ErrNo::EINVAL.into(),
    };
    do_win_func(
        |win| {
            font::write_string(
                win.write().base_mut(),
                Vector2D::new(x as _, y as _),
                s.as_bytes(),
                &PixelColor::to_color(color as _),
            );
            Result::value(0)
        },
        layer_id_flags,
    )
}

extern "sysv64" fn win_fill_rectangle(
    layer_id_flags: u64,
    x: u64,
    y: u64,
    w: u64,
    h: u64,
    color: u64,
) -> Result {
    do_win_func(
        |win| {
            win.write().base_mut().fill_rectangle(
                Vector2D::new(x as _, y as _),
                Vector2D::new(w as _, h as _),
                &PixelColor::to_color(color as _),
            );
            Result::value(0)
        },
        layer_id_flags,
    )
}

fn do_win_func(f: impl Fn(Arc<SharedLock<Window>>) -> Result, layer_id_flags: u64) -> Result {
    let layer_flags = layer_id_flags.get_bits(32..) as u32;
    let layer_id = layer_id_flags.get_bits(..32) as u32;

    let window = match LAYER_MANAGER.lock_wait().find_layer(layer_id) {
        Some(layer) => layer.window(),
        None => return Result::error(ErrNo::EBADF),
    };

    let res = f(window);
    if res.error != 0 {
        return res;
    }

    // layer_flags の 0 ビット目が立っていたら再描画しない
    // つまり特に指定がなければ再描画する
    if !layer_flags.get_bit(0) {
        LAYER_MANAGER.lock_wait().draw_id(layer_id);
    }

    res
}

extern "sysv64" fn get_current_tick(_: u64, _: u64, _: u64, _: u64, _: u64, _: u64) -> Result {
    Result::new(TIMER_MANAGER.lock_wait().current_tick(), TIMER_FREQ as i32)
}

extern "sysv64" fn win_redraw(
    layer_id_flags: u64,
    _: u64,
    _: u64,
    _: u64,
    _: u64,
    _: u64,
) -> Result {
    do_win_func(|_| Result::value(0), layer_id_flags)
}

extern "sysv64" fn win_draw_line(
    layer_id_flags: u64,
    x0: u64,
    y0: u64,
    x1: u64,
    y1: u64,
    color: u64,
) -> Result {
    do_win_func(
        move |win| {
            let (mut x0, mut y0, mut x1, mut y1) = (x0 as i32, y0 as i32, x1 as i32, y1 as i32);
            let color = PixelColor::to_color(color as u32);

            let dx = x1 - x0 + (x1 - x0).signum();
            let dy = y1 - y0 + (y1 - y0).signum();

            if dx == 0 && dy == 0 {
                win.write().base_mut().write(Vector2D::new(x0, y0), &color);
                return Result::value(0);
            }

            if dx.abs() >= dy.abs() {
                if dx < 0 {
                    mem::swap(&mut x0, &mut x1);
                    mem::swap(&mut y0, &mut y1);
                }
                let roundish = if y1 >= y0 { libm::floor } else { libm::ceil };
                let m = dy as f64 / dx as f64;
                for x in x0..=x1 {
                    let y = roundish(m * (x - x0) as f64 + y0 as f64) as i32;
                    win.write().base_mut().write(Vector2D::new(x, y), &color);
                }
            } else {
                if dy < 0 {
                    mem::swap(&mut x0, &mut x1);
                    mem::swap(&mut y0, &mut y1);
                }
                let roundish = if x1 >= 0 { libm::floor } else { libm::ceil };
                let m = dx as f64 / dy as f64;
                for y in y0..=y1 {
                    let x = roundish(m * (y - y0) as f64 + x0 as f64) as i32;
                    win.write().base_mut().write(Vector2D::new(x, y), &color);
                }
            }

            Result::value(0)
        },
        layer_id_flags,
    )
}

extern "sysv64" fn close_window(
    layer_id_flags: u64,
    _: u64,
    _: u64,
    _: u64,
    _: u64,
    _: u64,
) -> Result {
    let layer_id = layer_id_flags.get_bits(..32) as u32;

    let mut manager = LAYER_MANAGER.lock_wait();
    let layer = manager.layer(layer_id);

    let pos = layer.pos();
    let size = layer.window().read().base().size();

    manager.activate(0);
    manager.remove_layer(layer_id);
    manager.draw(&Rectangle { pos, size });
    LAYER_TASK_MAP.lock_wait().remove(&layer_id);

    Result::value(0)
}

extern "sysv64" fn read_event(events: u64, len: u64, _: u64, _: u64, _: u64, _: u64) -> Result {
    if events < 0x8000_0000_0000_0000 {
        return ErrNo::EFAULT.into();
    }

    // ここで良くわからない配列（へのポインタ）を &[AppEvents] に変換しているが、
    // 読み込みは行わないので UB は起きないはず
    let app_events = unsafe { slice::from_raw_parts_mut(events as *mut AppEvent, len as _) };

    asmfunc::cli();
    let task = task::current_task();
    asmfunc::sti();

    let mut i = 0;
    while i < app_events.len() {
        // receive_message はロックを取得してから処理するから、cli は必要ない
        let msg = match task.receive_message() {
            Some(msg) => msg,
            None => {
                if i == 0 {
                    task.sleep();
                    continue;
                } else {
                    break;
                }
            }
        };

        match msg.ty {
            MessageType::KeyPush {
                modifier, keycode, ..
            } => {
                if keycode == 20 /* Q キー */
                    && (modifier.get_bit(LCONTROL_BIT) || modifier.get_bit(RCONTROL_BIT))
                {
                    app_events[i] = AppEvent::Quit;
                    i += 1;
                }
            }
            MessageType::MouseMove {
                x,
                y,
                dx,
                dy,
                buttons,
            } => {
                app_events[i] = AppEvent::MouseMove {
                    x,
                    y,
                    dx,
                    dy,
                    buttons,
                };
                i += 1;
            }
            ty => log!(LogLevel::Info, "uncaught event type: {:?}", ty),
        }
    }

    Result::value(i as _)
}
