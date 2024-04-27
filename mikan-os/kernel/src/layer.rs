use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

use crate::{
    graphics::{PixelWriter, Vector2D},
    sync::OnceMutex,
    window::Window,
};

/// 全レイヤーを管理する構造体。
pub struct LayerManager {
    /// レイヤーを描画するライター。
    /// 一般にはフレームバッファに書き込めるライター。
    writer: &'static OnceMutex<Box<dyn PixelWriter + Send>>,
    /// レイヤーを保有するマップ。
    /// キーをそのレイヤーの ID として管理する。
    layers: BTreeMap<u32, Layer>,
    /// 描画順にレイヤー ID を保持する。
    layer_stack: Vec<u32>,
    /// レイヤーに割り振った最新の ID。
    latest_id: u32,
}

impl LayerManager {
    /// コンストラクタ。
    ///
    /// * writer - ライター。
    pub fn new(writer: &'static OnceMutex<Box<dyn PixelWriter + Send>>) -> Self {
        Self {
            writer,
            layers: BTreeMap::new(),
            layer_stack: Vec::new(),
            latest_id: 0,
        }
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

    /// レイヤーを指定位置に移動させる。
    ///
    /// # Remarks
    ///
    /// 有効な ID を指定していない場合は `panic` する。
    pub fn r#move(&mut self, id: u32, new_position: Vector2D<u32>) {
        self.find_layer_mut(id).unwrap().r#move(new_position);
    }

    /// レイヤーを指定位置に相対的に移動させる。
    ///
    /// # Remarks
    ///
    /// 有効な ID を指定していない場合は `panic` する。
    pub fn move_relative(&mut self, id: u32, pos_diff: Vector2D<u32>) {
        self.find_layer_mut(id).unwrap().move_relative(pos_diff);
    }

    /// レイヤーを画面に描画する。
    pub fn draw(&mut self) {
        for layer_id in &self.layer_stack {
            self.layers
                .get_mut(layer_id)
                .unwrap()
                .draw_to(&mut **self.writer.lock())
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
    fn find_layer(&self, id: u32) -> Option<&Layer> {
        self.layers.get(&id)
    }

    /// 指定された ID のレイヤーを探し、見つかったら排他参照を返す。
    fn find_layer_mut(&mut self, id: u32) -> Option<&mut Layer> {
        self.layers.get_mut(&id)
    }
}

/// レイヤーを表す構造体。
#[derive(Default)]
pub struct Layer {
    /// 位置。
    pos: Vector2D<u32>,
    /// 設定されているウィンドウ。
    window: Window,
}

impl Layer {
    /// コンストラクタ。
    ///
    /// * window - 紐づけるウィンドウ。
    pub fn new(window: Window) -> Self {
        Self {
            window,
            ..Default::default()
        }
    }

    /// 紐づいているウィンドウへの排他参照を返す。
    pub fn widow(&mut self) -> &mut Window {
        &mut self.window
    }

    /// レイヤーを指定された位置に動かす。
    pub fn r#move(&mut self, pos: Vector2D<u32>) -> &mut Self {
        self.pos = pos;
        self
    }

    /// レイヤーを指定された分だけ動かす。
    pub fn move_relative(&mut self, pos_diff: Vector2D<u32>) -> &mut Self {
        self.pos += pos_diff;
        self
    }

    /// レイヤーを設定された位置に描画する。
    pub fn draw_to(&mut self, writer: &mut dyn PixelWriter) {
        self.window.draw_to(writer, self.pos);
    }
}
