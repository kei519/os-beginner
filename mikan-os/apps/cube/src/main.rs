#![no_std]
#![no_main]

use core::{f64::consts, panic::PanicInfo, sync::atomic::Ordering};

use app_lib::{
    args::Args,
    events::{self, AppEvent},
    graphics::{self, LayerFlags},
    kernel_log,
    logger::LogLevel,
    main,
    time::{self, TimerMode},
    ERRNO,
};
use libm::{cos, sin};

extern crate app_lib;

const SCALE: i32 = 50;
const MARGIN: i32 = 10;
const CANVAS_SIZE: i32 = 3 * SCALE + MARGIN;

const CUBE: [Vector3D<i32>; 8] = [
    Vector3D::new(1, 1, 1),
    Vector3D::new(1, 1, -1),
    Vector3D::new(1, -1, 1),
    Vector3D::new(1, -1, -1),
    Vector3D::new(-1, 1, 1),
    Vector3D::new(-1, 1, -1),
    Vector3D::new(-1, -1, 1),
    Vector3D::new(-1, -1, -1),
];

const SURFACE: [[usize; 4]; 6] = [
    [0, 4, 6, 2],
    [1, 3, 7, 5],
    [0, 2, 3, 1],
    [0, 1, 5, 4],
    [4, 5, 7, 6],
    [6, 7, 3, 2],
];

const COLOR: [u32; SURFACE.len()] = [0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vector3D<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vector3D<T> {
    const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vector2D<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vector2D<T> {
    const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

#[main]
fn main(_: Args) -> i32 {
    let layer_id = graphics::open_window(CANVAS_SIZE + 8, CANVAS_SIZE + 28, 10, 10, "cube");
    if layer_id == 0 {
        return ERRNO.load(Ordering::Relaxed);
    }

    let mut vert = [Vector3D::new(0., 0., 0.); CUBE.len()];
    let mut centerz4 = [0.; SURFACE.len()];
    let mut scr = [Vector2D::new(0, 0); CUBE.len()];

    let (mut thx, mut thy, mut thz) = (0, 0, 0);
    let to_rad = consts::PI / 0x8000 as f64;
    loop {
        // 立方体を X, Y, Z 軸周りに回転
        thx = (thx + 182) & 0xffff;
        thy = (thy + 273) & 0xffff;
        thz = (thz + 364) & 0xffff;
        let (xp, xa) = (cos(thx as f64 * to_rad), sin(thx as f64 * to_rad));
        let (yp, ya) = (cos(thy as f64 * to_rad), sin(thy as f64 * to_rad));
        let (zp, za) = (cos(thz as f64 * to_rad), sin(thz as f64 * to_rad));
        for (i, cv) in CUBE.iter().enumerate() {
            let zt = (SCALE * cv.z) as f64 * xp + (SCALE * cv.y) as f64 * xa;
            let yt = (SCALE * cv.y) as f64 * xp - (SCALE * cv.z) as f64 * xa;
            let xt = (SCALE * cv.x) as f64 * yp + zt * ya;
            vert[i].z = (zt * yp - (SCALE * cv.x) as f64 * ya) as _;
            vert[i].x = (xt * zp - yt * za) as _;
            vert[i].y = (yt * zp + xt * za) as _;
        }

        // 面中心の Z 座標（を4倍した値）を6面について計算
        for sur in 0..SURFACE.len() {
            centerz4[sur] = 0.;
            for i in 0..SURFACE[sur].len() {
                centerz4[sur] += vert[SURFACE[sur][i]].z;
            }
        }

        // 画面を一旦クリアし、立方体を描画
        let flags = LayerFlags::new().set_redraw(false);
        graphics::win_fill_rectangle_with_flags(
            layer_id,
            4,
            24,
            CANVAS_SIZE,
            CANVAS_SIZE,
            0,
            flags,
        );
        draw_obj(layer_id, &mut vert, &mut centerz4, &mut scr);
        graphics::win_redraw(layer_id);

        if unsafe { sleep(50) } {
            break;
        }
    }

    graphics::close_window(layer_id);
    0
}

fn draw_obj(
    layer_id: u32,
    vert: &mut [Vector3D<f64>],
    centerz4: &mut [f64],
    scr: &mut [Vector2D<i32>],
) {
    // オブジェクト座標 vert を スクリーン座標 scr に変換（画面奥が z+）
    for i in 0..CUBE.len() {
        let t = 6. * SCALE as f64 / (vert[i].z + 8. * SCALE as f64);
        scr[i].x = ((vert[i].x * t) + CANVAS_SIZE as f64 / 2.) as _;
        scr[i].y = ((vert[i].y * t) + CANVAS_SIZE as f64 / 2.) as _;
    }

    loop {
        // 奥にある（= Z 座標が大きい）オブジェクトから順に描画
        let sur = centerz4
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
            .unwrap();
        if centerz4[sur] == f64::MIN {
            break;
        }
        centerz4[sur] = f64::MIN;

        // 法線ベクトルがこっちを向いている面だけ描画
        let v0 = vert[SURFACE[sur][0]];
        let v1 = vert[SURFACE[sur][1]];
        let v2 = vert[SURFACE[sur][2]];

        // v0 --> v1
        let e0x = v1.x - v0.x;
        let e0y = v1.y - v0.y;
        // v1 --> v2
        let e1x = v2.x - v1.x;
        let e1y = v2.y - v1.y;
        if e0x * e1y <= e0y * e1x {
            draw_surface(layer_id, sur, scr);
        }
    }
}

fn draw_surface(layer_id: u32, sur: usize, scr: &mut [Vector2D<i32>]) {
    let surface = SURFACE[sur]; // 描画する面
    let (mut ymin, mut ymax) = (CANVAS_SIZE, 0); // 画面の描画範囲 [ymin, ymax]

    // Y, X 座標の組
    let mut y2x_up = [0; CANVAS_SIZE as _];
    let mut y2x_down = [0; CANVAS_SIZE as _];
    for i in 0..surface.len() {
        let p0 = scr[surface[(i + 3) % 4]];
        let p1 = scr[surface[i]];
        ymin = ymin.min(p1.y);
        ymax = ymax.max(p1.y);
        if p0.y == p1.y {
            continue;
        }

        let (y2x, x0, y0, y1, dx) = if p0.y < p1.y {
            // p0 --> p1 は上る方向
            (&mut y2x_up, p0.x, p0.y, p1.y, p1.x - p0.x)
        } else {
            // po --> p1 は下る方向
            (&mut y2x_down, p1.x, p1.y, p0.y, p0.x - p1.x)
        };

        let m = dx as f64 / (y1 - y0) as f64;
        let roundish = if dx >= 0 { libm::floor } else { libm::ceil };
        for y in y0..=y1 {
            y2x[y as usize] = roundish(m * (y - y0) as f64 + x0 as f64) as _;
        }
    }

    for y in ymin..=ymax {
        let y = y as usize;
        let p0x = y2x_up[y].min(y2x_down[y]);
        let p1x = y2x_up[y].max(y2x_down[y]);
        let flags = LayerFlags::new().set_redraw(false);
        graphics::win_fill_rectangle_with_flags(
            layer_id,
            4 + p0x,
            24 + y as i32,
            p1x - p0x + 1,
            1,
            COLOR[sur],
            flags,
        );
    }
}

/// # Safety
///
/// シングルスレッドで呼ぶ。
unsafe fn sleep(ms: u64) -> bool {
    static mut PREV_TIMEOUT: u64 = 0;
    if PREV_TIMEOUT == 0 {
        let mode = TimerMode::new().set_relative(true);
        PREV_TIMEOUT = time::create_timer(mode, 1, ms);
    } else {
        PREV_TIMEOUT += ms;
        time::create_timer(TimerMode::new(), 1, PREV_TIMEOUT);
    }

    let mut events = [AppEvent::Null; 1];
    loop {
        events::read_event(&mut events);
        match events[0] {
            AppEvent::Timer { .. } => return false,
            AppEvent::Quit => return true,
            _ => {}
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
