use core::{
    alloc::{GlobalAlloc, Layout},
    borrow::Borrow,
    hash::{BuildHasher, Hash, Hasher},
    mem,
};

use alloc::{vec, vec::Vec};

use crate::{asmfunc, log, logger::LogLevel, memory_manager::GLOBAL};

/// [FNV hash](https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function) で
/// ハッシュを作成する。
#[derive(Debug, Clone)]
pub struct FnvHasher {
    hash: u64,
}

impl FnvHasher {
    pub const fn new() -> Self {
        Self {
            hash: 0xcbf29ce484222325,
        }
    }
}

impl Default for FnvHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for FnvHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.hash *= 0x100000001b3;
            self.hash ^= byte as u64;
        }
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}

/// [FnvHasher] 用の [BuildHasher]。
#[derive(Debug, Default, Clone, Copy)]
pub struct FnvBuilder;

impl BuildHasher for FnvBuilder {
    type Hasher = FnvHasher;

    fn hash_one<T: Hash>(&self, x: T) -> u64
    where
        Self: Sized,
        Self::Hasher: Hasher,
    {
        let mut hasher = FnvHasher::new();
        x.hash(&mut hasher);
        hasher.finish()
    }

    fn build_hasher(&self) -> Self::Hasher {
        FnvHasher::new()
    }
}

/// [HashMap] の内部で保持するエントリ。
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum HashEntry<K, V> {
    None = 0,
    Some { key: K, value: V },
    TombStone,
}

impl<K, V> Default for HashEntry<K, V> {
    fn default() -> Self {
        Self::None
    }
}

/// [FnvHasher] をハッシュに用いるハッシュマップ。
#[derive(Debug, Default)]
pub struct HashMap<K, V> {
    buckets: Vec<HashEntry<K, V>>,
    used: usize,
}

// HashMap 用定数
impl<K, V> HashMap<K, V> {
    /// 初期化時のサイズ。
    const INIT_SIZE: usize = 16;

    /// [HasMap::rehash()] 時に使用率がこれを上回っていたら、ハッシュテーブルのサイズを倍にする。
    const LOW_WATERMARK: usize = 50;

    /// 使用率がこれを超えていたら、[HashMap::rehash()] を呼び出す。
    const HIGH_WATERMARK: usize = 70;
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    /// 空のマップで初期化する。
    pub const fn new() -> Self {
        Self {
            buckets: vec![],
            used: 0,
        }
    }

    /// 値を挿入する。
    /// 以前に同じキーで値が挿入されていた場合は、その値を取り出して返す。
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.cap() == 0 {
            // もし初期化されていなければ初期化する
            self.buckets = vec_with_none(Self::INIT_SIZE);
        } else if self.usage() >= Self::HIGH_WATERMARK {
            // 使用率が規定値を超えていたら再配置する
            self.rehash();
        }

        let hash = FnvBuilder.hash_one(key.borrow()) as usize;

        self.used += 1;
        for i in 0..self.cap() {
            let cap = self.cap();
            // self.cap() で剰余を取っているので、中身はかならずある
            match self.buckets.get_mut((hash + i) % cap).unwrap() {
                &mut HashEntry::TombStone => continue,
                entry @ &mut HashEntry::None => {
                    // None が入っているだけなので捨てる
                    let _ = mem::replace(entry, HashEntry::Some { key, value });
                    return None;
                }
                HashEntry::Some {
                    key: cur_key,
                    value: cur_val,
                } => {
                    if *cur_key == key {
                        let prev_val = mem::replace(cur_val, value);
                        return Some(prev_val);
                    }
                }
            }
        }
        unreachable!("each buckets item must be one of HashEntry")
    }

    /// キーが `k` と一致するものがあれば、その値への共有参照を返す。
    pub fn get<Q: Hash + Eq + ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        // 初期化されていない場合はなにも含まれていない
        if self.cap() == 0 {
            return None;
        }

        let hash = FnvBuilder.hash_one(k) as usize;

        let cap = self.cap();
        for i in 0..cap {
            match self.buckets.get((hash + i) % cap).unwrap() {
                &HashEntry::None => return None,
                &HashEntry::TombStone => continue,
                HashEntry::Some { key, value } => {
                    if key.borrow() == k {
                        return Some(value);
                    }
                }
            }
        }
        unreachable!("each buckets item must be one of HashEntry")
    }

    /// キーが 'k' と一致するものがあれば、その値への排他参照を返す。
    pub fn get_mut<Q: Hash + Eq + ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        // 初期化されていない場合はなにも含まれていない
        if self.cap() == 0 {
            return None;
        }

        let hash = FnvBuilder.hash_one(k) as usize;

        let cap = self.cap();
        let (seconde, first) = self.buckets.split_at_mut(hash % cap);
        for entry in first.iter_mut().chain(seconde.iter_mut()) {
            match entry {
                &mut HashEntry::None => return None,
                &mut HashEntry::TombStone => continue,
                HashEntry::Some { key, value } => {
                    if <K as Borrow<Q>>::borrow(key) == k {
                        return Some(value);
                    }
                }
            }
        }
        unreachable!("each buckets item must be one of HashEntry")
    }

    /// キーが `k` と一致するものがあれば削除し、その値を返す。
    pub fn remove<Q: Hash + Eq + ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        // 初期化されていない場合はなにも含まれていない
        if self.cap() == 0 {
            return None;
        }

        let hash = FnvBuilder.hash_one(k) as usize;

        let cap = self.cap();
        let index = 'l: {
            for i in 0..cap {
                match self.buckets.get((hash + i) % cap).unwrap() {
                    &HashEntry::None => return None,
                    &HashEntry::TombStone => continue,
                    HashEntry::Some { key, .. } => {
                        if key.borrow() == k {
                            break 'l (hash + i) % cap;
                        }
                    }
                }
            }
            unreachable!("each buckets item must be one of HashEntry")
        };

        self.used -= 1;
        match mem::replace(self.buckets.get_mut(index).unwrap(), HashEntry::TombStone) {
            HashEntry::Some { value, .. } => Some(value),
            _ => unreachable!(),
        }
    }

    pub fn clear(&mut self) {
        for item in &mut self.buckets {
            *item = HashEntry::None;
        }
        self.used = 0;
    }

    /// 現在の容量。
    pub fn cap(&self) -> usize {
        self.buckets.len()
    }

    /// 現在の使用率（%）。
    fn usage(&self) -> usize {
        self.used * 100 / self.cap()
    }

    /// ハッシュテーブルの再配置を行う。
    /// 容量が不十分な場合はサイズの変更も行う。
    fn rehash(&mut self) {
        let used = self.used;
        self.used = 0;

        let mut cap = self.cap();
        while (used * 100) / cap >= Self::LOW_WATERMARK {
            cap *= 2;
        }

        let old_buckets = mem::replace(&mut self.buckets, vec_with_none(cap));
        for entry in old_buckets {
            if let HashEntry::Some { key, value } = entry {
                self.insert(key, value);
            }
        }
        assert_eq!(self.used, used);
    }
}

/// `capacity` 分 [HashEntry::None] で埋められたベクトルを返す。
fn vec_with_none<K, V>(capacity: usize) -> Vec<HashEntry<K, V>> {
    // `capacity` は `isize::MAX` 以下でないといけない
    if capacity > isize::MAX as usize {
        log!(LogLevel::Error, "too large capacity: {}", capacity);
        asmfunc::halt();
    }

    let layout = Layout::new::<HashEntry<K, V>>();
    unsafe {
        let layout = Layout::from_size_align_unchecked(layout.size() * capacity, layout.align());
        // `HashEntry::None` は 0 だから、全て 0 埋めしておけば `None` で初期化したことになる
        let ptr = GLOBAL.alloc_zeroed(layout) as *mut HashEntry<K, V>;

        Vec::from_raw_parts(ptr, capacity, capacity)
    }
}
