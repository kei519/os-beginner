use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use crate::{
    asmfunc,
    error::{Code, Result},
    frame_buffer::FrameBuffer,
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{self, PixelWrite as _, Rectangle, Vector2D, FB_CONFIG},
    make_error,
    message::{LayerOperation, MessageType},
    sync::{Mutex, OnceMutex, SharedLock},
    task,
    window::Window,
};

pub static LAYER_MANAGER: OnceMutex<LayerManager> = OnceMutex::new();

/// 本当のフレームバッファを表す `FrameBuffer`。
pub static SCREEN: OnceMutex<FrameBuffer> = OnceMutex::new();

pub static LAYER_TASK_MAP: Mutex<BTreeMap<u32, u64>> = Mutex::new(BTreeMap::new());

pub fn init() {
    let fb_config = FB_CONFIG.as_ref().clone();
    let frame_width = fb_config.horizontal_resolution as u32;
    let frame_height = fb_config.vertical_resolution as u32;
    let pixel_format = fb_config.pixel_format;

    let screen = match FrameBuffer::new(fb_config) {
        Ok(s) => s,
        Err(e) => {
            panic!(
                "failed to initialize frame buffer: {} as {}:{}",
                e,
                e.file(),
                e.line()
            );
        }
    };
    SCREEN.init(screen);

    let mut layer_manager = LayerManager::new(&SCREEN);

    let bgwindow = Window::new_base(frame_width, frame_height, pixel_format);
    let bglayer_id = layer_manager.new_layer(bgwindow);
    graphics::draw_desktop(&mut *layer_manager.layer(bglayer_id).window().write());
    layer_manager.up_down(bglayer_id, 0);

    LAYER_MANAGER.init(layer_manager);
}

pub fn process_layer_message(
    op: LayerOperation,
    layer_id: u32,
    pos: Vector2D<i32>,
    size: Vector2D<i32>,
) {
    let mut manager = LAYER_MANAGER.lock_wait();
    match op {
        LayerOperation::Move => manager.r#move(layer_id, pos),
        LayerOperation::MoveRelative => manager.move_relative(layer_id, pos),
        LayerOperation::Draw => manager.draw_id(layer_id),
        LayerOperation::DrawArea => manager.draw_area(layer_id, Rectangle { pos, size }),
    }
}

/// 全レイヤーを管理する構造体。
pub struct LayerManager {
    /// レイヤーを描画するライター。
    /// 一般にはフレームバッファに書き込めるライター。
    screen: &'static OnceMutex<FrameBuffer>,
    /// レイヤーを保有するマップ。
    /// キーをそのレイヤーの ID として管理する。
    layers: BTreeMap<u32, Layer>,
    /// 描画順にレイヤー ID を保持する。
    layer_stack: Vec<u32>,
    /// レイヤーに割り振った最新の ID。
    latest_id: u32,
    /// バックバッファ。
    back_buffer: FrameBuffer,
    /// マウスレイヤ ID。
    mouse_layer: u32,
    /// アクティブレイヤ ID。
    active_layer: u32,
}

impl LayerManager {
    /// コンストラクタ。
    ///
    /// * writer - ライター。
    pub fn new(screen: &'static OnceMutex<FrameBuffer>) -> Self {
        // バックバッファの作成
        let config = {
            let screen = screen.lock_wait();

            FrameBufferConfig {
                frame_buffer: 0,
                pixels_per_scan_line: screen.pixels_per_scan_line(),
                horizontal_resolution: screen.horizontal_resolution(),
                vertical_resolution: screen.vertical_resolution(),
                pixel_format: screen.pixel_format(),
            }
        };
        let back_buffer = FrameBuffer::new(config).unwrap();

        Self {
            screen,
            layers: BTreeMap::new(),
            layer_stack: Vec::new(),
            latest_id: 0,
            back_buffer,
            mouse_layer: 0,
            active_layer: 0,
        }
    }

    pub fn screen_size(&self) -> Vector2D<i32> {
        use crate::graphics::PixelWrite as _;

        let screen = self.screen.lock_wait();
        Vector2D::new(
            screen.horizontal_resolution() as i32,
            screen.vertical_resolution() as i32,
        )
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.screen.lock_wait().pixel_format()
    }

    /// 新しいレイヤーを作成し、そのレイヤーの ID を返す。
    ///
    /// * window - 生成するレイヤーに紐づけるウィンドウ。
    pub fn new_layer(&mut self, window: Window) -> u32 {
        self.latest_id += 1;
        self.layers.insert(self.latest_id, Layer::new(window));
        self.latest_id
    }

    /// 指定された ID のレイヤーへの排他参照を返す。
    ///
    /// # Remarks
    ///
    /// 指定された ID のレイヤーが存在しない場合は `panic` する。
    pub fn layer(&mut self, id: u32) -> &mut Layer {
        self.layers.get_mut(&id).unwrap()
    }

    /// レイヤーを指定位置に移動させて描画する。
    ///
    /// # Remarks
    ///
    /// 有効な ID を指定していない場合は `panic` する。
    pub fn r#move(&mut self, id: u32, new_position: Vector2D<i32>) {
        let layer = self.layer(id);
        let window_size = layer.window().read().base().size();
        let old_pos = layer.pos;

        layer.r#move(new_position);

        // 過去いた領域を消すために上書きする
        self.draw(&Rectangle {
            pos: old_pos,
            size: window_size,
        });

        self.draw_id(id);
    }

    /// レイヤーを指定位置に相対的に移動させる。
    ///
    /// # Remarks
    ///
    /// 有効な ID を指定していない場合は `panic` する。
    pub fn move_relative(&mut self, id: u32, pos_diff: Vector2D<i32>) {
        let layer = self.find_layer_mut(id).unwrap();
        let window_size = layer.window().read().base().size();
        let old_pos = layer.pos;
        layer.move_relative(pos_diff);

        // 過去いた領域を消すために上書きする
        self.draw(&Rectangle {
            pos: old_pos,
            size: window_size,
        });

        self.draw_id(id);
    }

    /// 指定領域のレイヤーを画面に描画する。
    pub fn draw(&mut self, area: &Rectangle<i32>) {
        for layer_id in &self.layer_stack {
            self.layers
                .get_mut(layer_id)
                .unwrap()
                .draw_to(&mut self.back_buffer, area)
        }

        // バックバッファをフレームバッファにコピー
        self.screen
            .lock_wait()
            .copy(area.pos, &self.back_buffer, area)
            .unwrap();
    }

    /// 指定されたレイヤーより上のレイヤーを描画する。
    ///
    /// # Remarks
    /// 有効な ID を指定していない場合は `panic` する。
    pub fn draw_id(&mut self, id: u32) {
        self.draw_area(
            id,
            Rectangle {
                pos: Vector2D::new(0, 0),
                size: Vector2D::new(-1, -1),
            },
        )
    }

    pub fn draw_area(&mut self, id: u32, mut area: Rectangle<i32>) {
        let mut window_area = None;
        for layer_id in &self.layer_stack {
            // layer_stack の中に入っているのは layers の中に入っているものに限られるため、
            // unwrap は必ず成功する
            let layer = self.layers.get_mut(layer_id).unwrap();
            if *layer_id == id {
                let mut wnd_area = Rectangle {
                    pos: layer.pos,
                    size: layer.window.read().base().size(),
                };
                // area.size が正の場合は area.pos から area.size 分だけ描画する
                if area.size.x() >= 0 || area.size.y() >= 0 {
                    area.pos += wnd_area.pos;
                    wnd_area = wnd_area & area;
                }
                window_area = Some(wnd_area);
            }
            // layer_id より上のレイヤーでは window_area が Some になっているはずなので、
            // 上のレイヤーは全て描画される
            if let Some(area) = window_area {
                layer.draw_to(&mut self.back_buffer, &area);
            }
        }

        if let Some(area) = window_area {
            self.screen
                .lock_wait()
                .copy(area.pos, &self.back_buffer, &area)
                .unwrap();
        }
    }

    /// 指定されたレイヤを非表示にする。
    ///
    /// # Remarks
    ///
    /// 無効な ID を指定された場合はなにもしない。
    pub fn hide(&mut self, id: u32) {
        if let Some(pos) = self.layer_stack.iter().position(|item| *item == id) {
            self.layer_stack.remove(pos);
        }
    }

    /// 指定された ID のレイヤーの高さを変更する。
    ///
    /// * id - レイヤーの ID。
    /// * new_height - 新しい高さ。マイナス値の場合は非表示にする。
    /// また、現在表示されているレイヤー数以上を指定した場合、最前面に配置される。
    pub fn up_down(&mut self, id: u32, new_height: i32) {
        // 負値は非表示
        if new_height < 0 {
            self.hide(id);
            return;
        }

        // 表示されているレイヤー数以上は最前面
        let new_pos = if new_height > self.layer_stack.len() as i32 {
            self.layer_stack.len()
        } else {
            new_height as usize
        };

        let old_pos = self.layer_stack.iter().position(|item| *item == id);
        // 元々表示されていなかった場合は挿入するだけ
        let Some(old_pos) = old_pos else {
            self.layer_stack.insert(new_pos, id);
            return;
        };

        // 元々表示されていた場合は、自分が抜ける分位置を下げる必要がある
        let new_pos = if new_pos == self.layer_stack.len() {
            new_pos - 1
        } else {
            new_pos
        };
        self.layer_stack.remove(old_pos);
        self.layer_stack.insert(new_pos, id);
    }

    /// 指定された ID のレイヤーを探し、見つかったら共有参照を返す。
    pub fn find_layer(&self, id: u32) -> Option<&Layer> {
        self.layers.get(&id)
    }

    /// 指定された ID のレイヤーを探し、見つかったら排他参照を返す。
    fn find_layer_mut(&mut self, id: u32) -> Option<&mut Layer> {
        self.layers.get_mut(&id)
    }

    /// 指定された位置のレイヤー ID を取得する。
    pub fn find_layer_by_position(&self, pos: &Vector2D<i32>, exclude_id: u32) -> Option<u32> {
        let pred = |&id| {
            if id == exclude_id {
                return None;
            }
            let layer = &self.layers[&id];
            let win_pos = layer.pos();
            let win_end_pos = win_pos + layer.window().read().base().size();

            if (win_pos.x() <= pos.x() && pos.x() < win_end_pos.x())
                && (win_pos.y() <= pos.y() && pos.y() < win_end_pos.y())
            {
                Some(id)
            } else {
                None
            }
        };
        self.layer_stack.iter().rev().find_map(pred)
    }

    /// 非表示の場合は `-1` を返す。
    pub fn get_height(&self, id: u32) -> i32 {
        match self
            .layer_stack
            .iter()
            .enumerate()
            .find(|(_, &layer)| layer == id)
            .map(|(i, _)| i)
        {
            Some(height) => height as i32,
            None => -1,
        }
    }

    pub fn set_mouse_layer(&mut self, id: u32) {
        self.mouse_layer = id;
    }

    pub fn get_active(&self) -> u32 {
        self.active_layer
    }

    /// `id` が `0` の場合は全てのウィンドウを非アクティブにする。
    pub fn activate(&mut self, id: u32) {
        if self.active_layer == id {
            return;
        }

        if self.active_layer > 0 {
            self.layer(self.active_layer).window().write().deactivate();
            self.draw_id(self.active_layer);
            let _ = Self::send_window_active_message(self.active_layer, false);
        }

        self.active_layer = id;
        if id > 0 {
            self.layer(id).window().write().activate();
            if self.layer_stack.iter().any(|&layer| layer == id) {
                self.up_down(id, self.get_height(self.mouse_layer) - 1);
            } else {
                self.up_down(id, self.get_height(self.mouse_layer));
            }
            self.draw_id(id);
            let _ = Self::send_window_active_message(id, true);
        }
    }

    pub fn remove_layer(&mut self, id: u32) {
        self.hide(id);
        self.layers.remove(&id);
    }

    fn send_window_active_message(layer_id: u32, activate: bool) -> Result<()> {
        let Some(&task_id) = LAYER_TASK_MAP.lock_wait().get(&layer_id) else {
            return Err(make_error!(Code::NoSuchTask));
        };

        let msg = MessageType::WindowActive { activate }.into();
        asmfunc::cli();
        let ret = task::send_message(task_id, msg);
        asmfunc::sti();
        ret
    }
}

/// レイヤーを表す構造体。
pub struct Layer {
    /// 位置。
    pos: Vector2D<i32>,
    /// 設定されているウィンドウ。
    window: Arc<SharedLock<Window>>,
    /// ドラッグ可能かどうかを表すフラグ。
    /// デフォルトは `false`。
    dragable: bool,
}

impl Layer {
    /// コンストラクタ。
    ///
    /// * window - 紐づけるウィンドウ。
    pub fn new(window: Window) -> Self {
        Self {
            window: Arc::new(SharedLock::new(window)),
            pos: Default::default(),
            dragable: false,
        }
    }

    /// 紐づいているウィンドウを返す。
    pub fn window(&self) -> Arc<SharedLock<Window>> {
        self.window.clone()
    }

    /// レイヤーの場所を返す。
    pub fn pos(&self) -> Vector2D<i32> {
        self.pos
    }

    /// レイヤーを指定された位置に動かす。
    pub fn r#move(&mut self, pos: Vector2D<i32>) -> &mut Self {
        self.pos = pos;
        self
    }

    /// レイヤーを指定された分だけ動かす。
    pub fn move_relative(&mut self, pos_diff: Vector2D<i32>) -> &mut Self {
        self.pos += pos_diff;
        self
    }

    /// レイヤーを設定された位置に描画する。
    pub fn draw_to(&self, screen: &mut FrameBuffer, area: &Rectangle<i32>) {
        self.window.read().base().draw_to(screen, self.pos, area);
    }

    /// ドラッグ可能かどうかを設定する。
    pub fn set_draggable(&mut self, dragable: bool) -> &mut Self {
        self.dragable = dragable;
        self
    }

    /// ドラッグ可能かどうかを返す。
    pub fn is_draggable(&self) -> bool {
        self.dragable
    }
}
