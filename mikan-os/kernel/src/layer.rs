use alloc::{collections::BTreeMap, vec::Vec};

use crate::{
    frame_buffer::FrameBuffer,
    frame_buffer_config::FrameBufferConfig,
    graphics::{PixelWriter as _, Rectangle, Vector2D},
    sync::OnceMutex,
    window::Window,
};

pub static LAYER_MANAGER: OnceMutex<LayerManager> = OnceMutex::new();

/// 本当のフレームバッファを表す `FrameBuffer`。
pub static SCREEN: OnceMutex<FrameBuffer> = OnceMutex::new();

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
}

impl LayerManager {
    /// コンストラクタ。
    ///
    /// * writer - ライター。
    pub fn new(screen: &'static OnceMutex<FrameBuffer>) -> Self {
        // バックバッファの作成
        let config = {
            let screen = screen.lock();

            FrameBufferConfig {
                frame_buffer: 0,
                pixels_per_scan_line: screen.pixels_per_scan_line(),
                horizontal_resolution: screen.horizontal_resolution(),
                vertical_resolution: screen.vertical_resolution(),
                pixel_format: screen.pixef_format(),
            }
        };
        let back_buffer = FrameBuffer::new(config).unwrap();

        Self {
            screen,
            layers: BTreeMap::new(),
            layer_stack: Vec::new(),
            latest_id: 0,
            back_buffer,
        }
    }

    pub fn screen_size(&self) -> Vector2D<i32> {
        use crate::graphics::PixelWriter as _;

        let screen = self.screen.lock();
        Vector2D::new(
            screen.horizontal_resolution() as i32,
            screen.vertical_resolution() as i32,
        )
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
        let window_size = layer.window().size();
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
        let window_size = layer.window().size();
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
            .lock()
            .copy(area.pos, &self.back_buffer, area)
            .unwrap();
    }

    /// 指定されたレイヤーより上のレイヤーを描画する。
    ///
    /// # Remarks
    /// 有効な ID を指定していない場合は `panic` する。
    pub fn draw_id(&mut self, id: u32) {
        let mut draw = false;
        // 借用の問題で、最初に指定領域を用意しておく
        let area = Rectangle {
            pos: self.layer(id).pos,
            size: self.layer(id).window().size(),
        };

        for layer_id in &self.layer_stack {
            // 指定された ID 以降は、指定された ID と重なる領域を全て描画する
            if *layer_id == id {
                draw = true;
            }
            if draw {
                self.layers
                    .get_mut(layer_id)
                    .unwrap()
                    .draw_to(&mut self.back_buffer, &area)
            }
        }

        // バックバッファをフレームバッファにコピー
        self.screen
            .lock()
            .copy(area.pos, &self.back_buffer, &area)
            .unwrap();
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
    pub fn up_down(&mut self, id: u32, mut new_height: i32) {
        // 負値は非表示
        if new_height < 0 {
            self.hide(id);
            return;
        }

        // 表示されているレイヤー数以上は最前面
        if new_height > self.layer_stack.len() as i32 {
            new_height = self.layer_stack.len() as i32;
        }
        let mut new_pos = new_height as usize;

        let old_pos = self.layer_stack.iter().position(|item| *item == id);
        // 元々表示されていなかった場合は挿入するだけ
        let Some(old_pos) = old_pos else {
            self.layer_stack.insert(new_pos, id);
            return;
        };

        // 元々表示されていた場合は、自分が抜ける分位置を下げる必要がある
        if new_pos == self.layer_stack.len() {
            new_pos -= 1;
        }
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
            let win_end_pos = win_pos + layer.window().size();

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
}

/// レイヤーを表す構造体。
pub struct Layer {
    /// 位置。
    pos: Vector2D<i32>,
    /// 設定されているウィンドウ。
    window: Window,
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
            window,
            pos: Default::default(),
            dragable: false,
        }
    }

    /// 紐づいているウィンドウへの排他参照を返す。
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    /// 紐づいているウィンドウへの共有参照を返す。
    pub fn window(&self) -> &Window {
        &self.window
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
    pub fn draw_to(&mut self, screen: &mut FrameBuffer, area: &Rectangle<i32>) {
        self.window.draw_to(screen, self.pos, area);
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
