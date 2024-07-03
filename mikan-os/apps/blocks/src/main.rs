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

extern crate app_lib;

const NUM_BLOCKS_X: usize = 10;
const NUM_BLOCKS_Y: usize = 5;

const BLOCK_WIDTH: i32 = 20;
const BLOCK_HEIGHT: i32 = 10;

const BAR_WIDTH: i32 = 30;
const BAR_HEIGHT: i32 = 5;
const BALL_RADIUS: i32 = 5;

const GAP_WIDTH: i32 = 30;
const GAP_HEIGHT: i32 = 30;
const GAP_BAR: i32 = 80;
const BAR_FLOAT: i32 = 10;

const CANVAS_WIDTH: i32 = NUM_BLOCKS_X as i32 * BLOCK_WIDTH + 2 * GAP_WIDTH;
const CANVAS_HEIGHT: i32 =
    GAP_HEIGHT + NUM_BLOCKS_Y as i32 * BLOCK_HEIGHT + GAP_BAR + BAR_HEIGHT + BAR_FLOAT;
const BAR_Y: i32 = CANVAS_HEIGHT - BAR_FLOAT - BAR_HEIGHT;

const FRAME_RATE: u64 = 60;
const BAR_SPPED: i32 = CANVAS_WIDTH / 2;
const BALL_SPPED: i32 = BAR_SPPED;

pub struct Blocks {
    data: [[u8; (NUM_BLOCKS_X + 7) / 8]; NUM_BLOCKS_Y],
}

impl Blocks {
    const fn new() -> Self {
        Self {
            data: [[0xff; (NUM_BLOCKS_X + 7) / 8]; NUM_BLOCKS_Y],
        }
    }

    fn is_at(&self, x: i32, y: i32) -> bool {
        let (offset, index) = Self::get_index_offset(x);
        self.data[y as usize][offset] & (1 << index) != 0
    }

    fn set_at(&mut self, x: i32, y: i32, value: bool) {
        let (index, offset) = Self::get_index_offset(x);
        if value {
            self.data[y as usize][index] |= 1 << offset;
        } else {
            self.data[y as usize][index] &= !(1 << offset);
        }
    }

    fn get_index_offset(x: i32) -> (usize, u8) {
        let index = x >> 3;
        let offset = x & 7;
        (index as _, offset as _)
    }
}

fn draw_blocks(layer_id: u32, flags: LayerFlags, blocks: &mut Blocks) {
    for by in 0..NUM_BLOCKS_Y as _ {
        let y = 24 + GAP_HEIGHT + by * BLOCK_HEIGHT;
        let color = 0xff << (by % 3) * 8;

        for bx in 0..NUM_BLOCKS_X as _ {
            if blocks.is_at(bx, by) {
                let x = 4 + GAP_WIDTH + bx * BLOCK_WIDTH;
                let c = color | (0xff << ((bx + by) % 3) * 8);
                graphics::win_fill_rectangle_with_flags(
                    layer_id,
                    x,
                    y,
                    BLOCK_WIDTH,
                    BLOCK_HEIGHT,
                    c,
                    flags,
                );
            }
        }
    }
}

fn draw_bar(layer_id: u32, flags: LayerFlags, bar_x: i32) {
    graphics::win_fill_rectangle_with_flags(
        layer_id,
        4 + bar_x,
        24 + BAR_Y,
        BAR_WIDTH,
        BAR_HEIGHT,
        0xffffff,
        flags,
    );
}

fn draw_ball(layer_id: u32, flags: LayerFlags, x: i32, y: i32) {
    graphics::win_fill_rectangle_with_flags(
        layer_id,
        4 + x - BALL_RADIUS,
        24 + y - BALL_RADIUS,
        2 * BALL_RADIUS,
        2 * BALL_RADIUS,
        0x007f00,
        flags,
    );
    graphics::win_fill_rectangle_with_flags(
        layer_id,
        4 + x - BALL_RADIUS / 2,
        24 + y - BALL_RADIUS / 2,
        BALL_RADIUS,
        BALL_RADIUS,
        0x00ff00,
        flags,
    );
}

#[main]
fn main(_: Args) -> i32 {
    let layer_id = graphics::open_window(CANVAS_WIDTH + 8, CANVAS_HEIGHT + 28, 10, 10, "blocks");
    if layer_id == 0 {
        return ERRNO.load(Ordering::Relaxed);
    }

    let mut blocks = Blocks::new();

    const BALL_X: i32 = CANVAS_WIDTH / 2 - BALL_RADIUS - 20;
    const BALL_Y: i32 = CANVAS_HEIGHT - BAR_FLOAT - BAR_HEIGHT - BALL_RADIUS - 20;

    let mut bar_x = CANVAS_WIDTH / 2 - BAR_WIDTH / 2;
    let mut ball_x = BALL_X;
    let mut ball_y = BALL_Y;
    let mut move_dir = 0; // -1: left, 1: right
    let mut ball_dir = 0; // degree
    let mut ball_dx = 0;
    let mut ball_dy = 0;

    let mut prev_timeout = 0;

    'outer: loop {
        // 画面を一旦クリアし、各種オブジェクトを描画
        let flags = LayerFlags::new().set_redraw(false);
        graphics::win_fill_rectangle_with_flags(
            layer_id,
            4,
            24,
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            0,
            flags,
        );
        draw_blocks(layer_id, flags, &mut blocks);
        draw_bar(layer_id, flags, bar_x);
        if ball_y >= 0 {
            draw_ball(layer_id, flags, ball_x, ball_y);
        }
        graphics::win_redraw(layer_id);

        if prev_timeout == 0 {
            let mode = TimerMode::new().set_relative(true);
            prev_timeout = time::create_timer(mode, 1, 1000 / FRAME_RATE);
        } else {
            prev_timeout += 1000 / FRAME_RATE;
            time::create_timer(TimerMode::new(), 1, prev_timeout);
        }

        let mut events = [AppEvent::Null; 1];
        loop {
            events::read_event(&mut events);
            match events[0] {
                AppEvent::Timer { .. } => break,
                AppEvent::Quit => break 'outer,
                AppEvent::KeyPush { keycode, press, .. } => {
                    if !press {
                        // 離した
                        move_dir = 0;
                    } else {
                        match keycode {
                            79 /* 右矢印 */ => move_dir = 1,
                            80 /* 左矢印 */ => move_dir = -1,
                            44 /* スペース */ => {
                                if ball_dir == 0 && ball_y < 0 {
                                ball_x = BALL_X;
                                ball_y = BALL_Y;
                                } else if ball_dir == 0 {
                                    ball_dir = 45;
                                }
                            }
                            _ => {}
                        }
                        if bar_x == 0 && move_dir < 0 {
                            move_dir = 0;
                        } else if bar_x + BAR_WIDTH == CANVAS_WIDTH - 1 && move_dir > 0 {
                            move_dir = 0;
                        }
                    }
                }
                _ => {}
            }
        }

        bar_x += move_dir * BAR_SPPED / FRAME_RATE as i32;
        bar_x = bar_x.clamp(0, CANVAS_WIDTH - BAR_WIDTH - 1);

        if ball_dir == 0 {
            continue;
        }

        let ball_x_ = ball_x + ball_dx;
        let ball_y_ = ball_y + ball_dy;
        if (ball_dx < 0 && ball_x_ < BALL_RADIUS)
            || (ball_dx > 0 && CANVAS_WIDTH - BALL_RADIUS <= ball_x_)
        {
            // 壁
            ball_dir = 180 - ball_dir;
        }
        if ball_dy < 0 && ball_y_ < BALL_RADIUS {
            // 天井
            ball_dir = -ball_dir;
        } else if (bar_x..bar_x + BAR_WIDTH).contains(&ball_x_)
            && ball_dy > 0
            && BAR_Y - BALL_RADIUS <= ball_y_
        {
            // バー
            ball_dir = -ball_dir
        } else if ball_dy > 0 && CANVAS_HEIGHT - BALL_RADIUS <= ball_y_ {
            // 落下
            ball_dir = 0;
            ball_y = -1;
            continue;
        }

        'bl: {
            if !(GAP_WIDTH..CANVAS_WIDTH - GAP_WIDTH).contains(&ball_x_)
                || !(GAP_HEIGHT..GAP_HEIGHT + NUM_BLOCKS_Y as i32 * BLOCK_HEIGHT).contains(&ball_y_)
            {
                break 'bl;
            }

            let index_x = (ball_x_ - GAP_WIDTH) / BLOCK_WIDTH;
            let index_y = (ball_y_ - GAP_HEIGHT) / BLOCK_HEIGHT;
            if !blocks.is_at(index_x, index_y) {
                // ブロックがない
                break 'bl;
            }

            // ブロックがある
            blocks.set_at(index_x, index_y, false);

            let block_left = GAP_WIDTH + index_x * BLOCK_WIDTH;
            let block_right = GAP_WIDTH + (index_x + 1) * BLOCK_WIDTH;
            let block_top = GAP_HEIGHT + index_y * BLOCK_HEIGHT;
            let block_bottom = GAP_HEIGHT + (index_y + 1) * BLOCK_HEIGHT;
            if (ball_x < block_left && block_left <= ball_x_)
                || (block_right < ball_x && ball_x_ <= block_right)
            {
                ball_dir = 180 - ball_dir;
            }
            if (ball_y < block_top && block_top <= ball_y_)
                || (block_bottom < ball_y && ball_y_ <= block_bottom)
            {
                ball_dir = -ball_dir;
            }
        }

        ball_dx = libm::round(
            BALL_SPPED as f64 * libm::cos(consts::PI * ball_dir as f64 / 180.) / FRAME_RATE as f64,
        ) as _;
        ball_dy = libm::round(
            BALL_SPPED as f64 * libm::sin(consts::PI * ball_dir as f64 / 180.) / FRAME_RATE as f64,
        ) as _;
        ball_x += ball_dx;
        ball_y += ball_dy;
    }

    graphics::close_window(layer_id);
    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel_log!(LogLevel::Error, "paniced in rpn: {}", info);
    loop {}
}
