use std::{alloc::{alloc, dealloc, realloc, Layout}, collections::HashMap, fmt::{self, Debug}, marker::PhantomData, mem, ops::{Index, IndexMut}, ptr::{self, NonNull}};

use num::Num;

use super::{normal_vec_trait::NormalVecMethods, vec_trait::Math};

/// <T> のdefault値をスパースするSparseVectorの実装
/// Vecの実装を参考にします
/// src : https://doc.rust-jp.rs/rust-nomicon-ja/vec.html
///     : https://doc.rust-lang.org/std/vec/struct.Vec.html

#[derive(Clone)]
pub struct DefaultSparseVec<T: Default + PartialEq + Clone> {
    buf: RawDefaultSparseVec<T>,
    raw_len: usize,
    len: usize,
    default: T,
}

impl<T: Default + PartialEq + Clone> DefaultSparseVec<T> {
    #[inline(always)]
    fn val_ptr(&self) -> *mut T { self.buf.val_ptr.as_ptr() }

    #[inline(always)]
    fn ind_ptr(&self) -> *mut usize { self.buf.ind_ptr.as_ptr() }

    #[inline(always)]
    fn cap(&self) -> usize { self.buf.cap }

    /// ind_binary_searchメソッドの実装
    /// 返り値は「該当indexが見つかったら Ok(要素位置)、
    ///  見つからなければ Err(挿入すべき要素位置)」
    #[inline(always)]
    fn ind_binary_search(&self, index: &usize) -> Result<usize, usize> {
        // 要素が無い場合は「まだどこにも挿入されていない」ので Err(0)
        if self.raw_len == 0 {
            return Err(0);
        }

        let mut left = 0;
        let mut right = self.raw_len - 1;
        while left < right {
            let mid = left + (right - left) / 2;
            let mid_index = unsafe { ptr::read(self.ind_ptr().add(mid)) };
            if mid_index == *index {
                return Ok(mid);
            } else if mid_index < *index {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        // ループ終了後 left == right の位置になっている
        let final_index = unsafe { ptr::read(self.ind_ptr().add(left)) };
        if final_index == *index {
            Ok(left)
        } else if final_index < *index {
            Err(left + 1)
        } else {
            Err(left)
        }
    }

    /// newメソッドの実装
    #[inline(always)]
    pub fn new() -> Self {
        DefaultSparseVec {
            buf: RawDefaultSparseVec::new(),
            raw_len: 0,
            len: 0,
            default: Default::default(),
        }
    }

    #[inline(always)]
    pub fn with_capacity(cap: usize) -> Self {
        let mut vec = DefaultSparseVec {
            buf: RawDefaultSparseVec::new(),
            raw_len: 0,
            len: 0,
            default: Default::default(),
        };
        vec.buf.cap = cap;
        vec.buf.cap_set();
        vec
    }

    // is_emptyメソッドの実装
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// capacityメソッドの実装
    /// スパースベクトルの現在の容量を取得
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.cap()
    }

    /// reserveメソッドの実装
    /// スパースベクトルの容量を増やす
    /// 既に確保されている容量よりも小さい場合は何もしない
    /// 既に確保されている容量よりも大きい場合は、新しい容量に再確保する
    #[inline(always)]
    pub fn reserve(&mut self, additional: usize) {
        let new_cap = self.raw_len + additional;
        if new_cap > self.cap() {
            self.buf.cap = new_cap;
            self.buf.re_cap_set();
        }
    }

    /// shrink_to_fitメソッドの実装
    /// スパースベクトルの容量を現在の長さに合わせる
    /// 既に確保されている容量と現在の長さが同じ場合は何もしない
    #[inline(always)]
    pub fn shrink_to_fit(&mut self) {
        if self.raw_len < self.cap() {
            self.buf.cap = self.raw_len;
            self.buf.re_cap_set();
        }
    }

    /// nnzメソッドの実装
    /// スパースベクトル長の取得
    #[inline(always)]
    pub fn nnz(&self) -> usize {
        self.raw_len
    }

    /// lenメソッドの実装
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// clearメソッドの実装
    #[inline(always)]
    pub fn clear(&mut self) {
        while let Some(_) = self.pop() {}
    }

    /// pushメソッドの実装
    #[inline(always)]
    pub fn push(&mut self, elem: T) {
        if self.raw_len == self.cap() {
            self.buf.grow();
        }
        if self.default != elem {
            unsafe {
                ptr::write(self.val_ptr().offset(self.raw_len as isize), elem);
                ptr::write(self.ind_ptr().offset(self.raw_len as isize), self.len);
            }
            self.raw_len += 1;
        }
        self.len += 1;
    }

    /// popメソッドの実装
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        // 空らずraw_len =< len であることが保証されている
        let pop_elem = 
            if self.raw_len == self.len {
                self.raw_len -= 1;
                unsafe {
                    Some(ptr::read(self.val_ptr().offset(self.raw_len as isize)))
                }
            } else {
                Some(self.default.clone())
            };
        self.len -= 1;
        pop_elem
    }

    /// getメソッドの実装
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        match self.ind_binary_search(&index) {
            Ok(i) => {
                let val = unsafe { &*self.val_ptr().offset(i as isize) };
                Some(val)
            }
            Err(_) => Some(&self.default),
        }
    }

    // get_mutメソッドの実装
    // このメソッドは、指定されたインデックスの要素を変更するために使用されます。
    // ! : スパース分部の要素をわたすためにわざと値を生成します
    // ! : 無駄にデフォルト値を生成するので、このメソッドは避けるべきです
    #[deprecated(note = "このメソッドは避けるべきです. 
                        スパース分部の実値を渡すため、スパース分部の値を無駄に生成します.
                        default値以外を代入する場合は問題ありません.")]
    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        match self.ind_binary_search(&index) {
            Ok(i) => {
                let val = unsafe { &mut *self.val_ptr().offset(i as isize) };
                Some(val)
            }
            Err(i) => {
                if self.raw_len == self.cap() {
                    self.buf.grow();
                }
                unsafe {
                    let src = i as isize;
                    let dst = src + 1;
                    let count = self.raw_len - i;
                    ptr::copy(
                        self.val_ptr().offset(src),
                        self.val_ptr().offset(dst),
                        count,
                    );
                    ptr::copy(
                        self.ind_ptr().offset(src),
                        self.ind_ptr().offset(dst),
                        count,
                    );
                    ptr::write(self.val_ptr().offset(i as isize), self.default.clone());
                    ptr::write(self.ind_ptr().offset(i as isize), index);
                }
                self.raw_len += 1;
                let val = unsafe { &mut *self.val_ptr().offset(i as isize) };
                Some(val)
            },
        }
    }

    /// insertメソッド
    /// 「index 番目に新しい要素を割り込む」という動作
    /// - 後続要素のインデックスは常に +1 シフト
    /// - `elem` が非デフォルト値なら物理領域に書き込む (raw_len += 1)
    /// - `elem` がデフォルト値なら物理領域には書き込まない（スパース化）
    ///
    #[inline(always)]
    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "index out of bounds");

        // 挿入により論理的な長さは常に +1
        self.len += 1;

        // シフト時に書き込み先が必要なので、raw_len == cap なら grow する
        if self.raw_len == self.cap() {
            self.buf.grow();
        }

        // ind_binary_search で挿入ポイント i を特定
        // (すでに同じ index があっても、そこに割り込む)
        let i = match self.ind_binary_search(&index) {
            Ok(pos) => pos,
            Err(pos) => pos,
        };

        unsafe {
            // まず後ろの要素をまとめて1つ後ろへシフト
            let src = i as isize;
            let dst = src + 1;
            let count = self.raw_len - i;

            // 値をコピー (memmove 相当)
            ptr::copy(
                self.val_ptr().offset(src),
                self.val_ptr().offset(dst),
                count,
            );
            // インデックスをコピー
            ptr::copy(
                self.ind_ptr().offset(src),
                self.ind_ptr().offset(dst),
                count,
            );

            // シフトされた要素のインデックス値を +1
            for offset in (i + 1)..(self.raw_len + 1) {
                *self.ind_ptr().offset(offset as isize) += 1;
            }
        }

        // `elem` がデフォルト値なら物理的には書き込まずスパース化
        if elem != self.default {
            unsafe {
                // シフトしたスロット i に書き込み
                ptr::write(self.val_ptr().offset(i as isize), elem);
                ptr::write(self.ind_ptr().offset(i as isize), index);
            }
            // 非デフォルト値なので raw_len も増やす
            self.raw_len += 1;
        }
    }

    /// removeメソッド
    /// 
    /// `index` 番目の要素を削除し、削除した要素を返します。
    /// - 論理インデックス `index` が物理的に存在すれば、その値を返す
    /// - 物理的になければ（= デフォルト扱いだった）デフォルト値を返す
    /// 
    /// いずれにせよ後ろの要素（論理インデックスが `index` より大きい要素）は
    /// インデックスを 1 つ前にシフトします。
    #[inline(always)]
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");
        
        // 論理的な要素数は常に1つ減る
        self.len -= 1;

        match self.ind_binary_search(&index) {
            Ok(i) => {
                // 今回削除する要素を読みだす
                let removed_val = unsafe {
                    ptr::read(self.val_ptr().offset(i as isize))
                };

                // `i` 番目を削除するので、後ろを前にシフト
                let count = self.raw_len - i - 1;
                if count > 0 {
                    unsafe {
                        // 値をコピーして前につめる
                        ptr::copy(
                            self.val_ptr().offset(i as isize + 1),
                            self.val_ptr().offset(i as isize),
                            count
                        );
                        // インデックスもコピーして前につめる
                        ptr::copy(
                            self.ind_ptr().offset(i as isize + 1),
                            self.ind_ptr().offset(i as isize),
                            count
                        );
                        // シフトした後のインデックスは全て -1 (1つ前に詰める)
                        for offset in i..(self.raw_len - 1) {
                            *self.ind_ptr().offset(offset as isize) -= 1;
                        }
                    }
                }
                // 物理的な要素数は 1 減
                self.raw_len -= 1;

                // 取り除いた要素を返す
                removed_val
            }
            Err(i) => {
                // index は詰める必要があるので、i 以降の要素のインデックスを -1
                // （たとえば “要素自体は無い” けど、後ろにある要素は
                //  論理インデックスが 1 つ前になる）
                if i < self.raw_len {
                    unsafe {
                        for offset in i..self.raw_len {
                            *self.ind_ptr().offset(offset as isize) -= 1;
                        }
                    }
                }

                // “もともと物理要素が無い” のだから、デフォルト値を返す
                self.default.clone()
            }
        }
    }

    /// 2つのスパースベクタを “連結” する append 実装例
    /// - `other` は消費 (ムーブ) して、自分に要素をつけ足す
    /// - `other` のインデックスは自分の `len` 分だけシフト
    #[inline(always)]
    pub fn append(&mut self, other: Self) {
        let other_len = other.len();
        let other_raw_len = other.nnz();
        let other_default = other.default.clone();

        // 1) 相手が空なら何もしないで終了
        if other_len == 0 {
            return;
        }

        // 2) デフォルト値が異なる場合はエラーとする
        //    (同じスパース化の基準でなければ連結できない)
        assert!(
            self.default == other_default,
            "default value mismatch"
        );

        // 3) “論理インデックス” の連結位置を決める (ここでは self.len)
        let offset = self.len;

        // 4) 自分の長さ (論理) は other 分だけ伸びる
        self.len += other_len;

        // 5) キャパが足りなければ拡張
        //    raw_len + other_raw_len 分必要
        if self.raw_len + other_raw_len > self.cap() {
            // 例えば reserve は “cap が足りない分だけ” 確保するようにする
            self.reserve(self.raw_len + other_raw_len - self.cap());
        }

        // 6) 相手が物理的にも空でなければ(= other_raw_len>0) コピーする
        if other_raw_len > 0 {
            unsafe {
                // まず、相手の ind_ptr / val_ptr からコピー
                //   - コピー先は self.ind_ptr().add(self.raw_len)
                //   - コピー元は other.ind_ptr()
                ptr::copy(
                    other.ind_ptr(),
                    self.ind_ptr().add(self.raw_len),
                    other_raw_len,
                );
                ptr::copy(
                    other.val_ptr(),
                    self.val_ptr().add(self.raw_len),
                    other_raw_len,
                );

                // コピーされたインデックスをすべて offset だけ + する
                for i in 0..other_raw_len {
                    *self.ind_ptr().add(self.raw_len + i) += offset;
                }
            }

            // raw_len も伸ばす
            self.raw_len += other_raw_len;
        }
    }

    /// extendメソッドの実装
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for elem in iter {
            self.push(elem);
        }
    }

    /// iterメソッドの実装(仮)
    /// スパース分部を含みません
    /// スパース分部が必要な場合はNormalVecMethods trait実装
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (&usize, &T)> {
        (0..self.raw_len).map(move |i| {
            let val: &T = unsafe { &*self.val_ptr().offset(i as isize) };
            let ind: &usize = unsafe { &*self.ind_ptr().offset(i as isize) };
            (ind, val)
        })
    }

    /// iter_mutメソッドの実装(仮)
    /// スパース分部を含みません
    /// スパース分部が必要な場合はNormalVecMethods trait実装
    #[inline(always)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut usize, &mut T)> {
        (0..self.raw_len).map(move |i| {
            let val: &mut T = unsafe { &mut *self.val_ptr().offset(i as isize) };
            let ind: &mut usize = unsafe { &mut *self.ind_ptr().offset(i as isize) };
            (ind, val)
        })
    }

    //// as_sliceメソッドの実装
    #[inline(always)]
    pub fn as_slice_val(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.val_ptr(), self.raw_len)
        }
    }

    #[inline(always)]
    pub fn as_slice_ind(&self) -> &[usize] {
        unsafe {
            std::slice::from_raw_parts(self.ind_ptr(), self.raw_len)
        }
    }

    #[inline(always)]
    pub fn as_mut_slice_val(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(self.val_ptr(), self.raw_len)
        }
    }

    #[inline(always)]
    pub fn as_mut_slice_ind(&mut self) -> &mut [usize] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ind_ptr(), self.raw_len)
        }
    }
}

unsafe impl<T: Send + Default + PartialEq + Clone> Send for DefaultSparseVec<T> {}
unsafe impl<T: Send + Default + PartialEq + Clone> Sync for DefaultSparseVec<T> {}

impl<T: Default + PartialEq + Clone> Drop for DefaultSparseVec<T> {
    #[inline(always)]
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
    }
}

impl<T: Default + PartialEq + Clone + Debug> Debug for DefaultSparseVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.sign_plus() {
            f.debug_struct("DefaultSparseVec")
                .field("buf", &self.buf)
                .field("raw_len", &self.raw_len)
                .field("len", &self.len)
                .field("default", &self.default)
                .finish()
        } else if f.alternate() {
            write!(f, "DefaultSparseVec({:?})", self.iter().collect::<Vec<_>>())
        } else {
            f.debug_list().entries((0..self.len).map(|i| self.get(i).unwrap())).finish()
        }
    }
}

impl<T: Default + PartialEq + Clone> Index<usize> for DefaultSparseVec<T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

impl<T: Default + PartialEq + Clone> IndexMut<usize> for DefaultSparseVec<T> {
    /// #warning
    /// このメソッドは、非推奨のget_mutメソッドを使用しています
    #[inline(always)]
    #[warn(deprecated)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("index out of bounds")
    }
}

impl <T: Default + PartialEq + Clone> Default for DefaultSparseVec<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default + PartialEq + Clone> From<Vec<T>> for DefaultSparseVec<T> {
    #[inline(always)]
    fn from(vec: Vec<T>) -> Self {
        let mut svec = DefaultSparseVec::new();
        vec.into_iter().for_each(|elem| svec.push(elem));
        svec.shrink_to_fit();
        svec
    }
}

impl<T: Default + PartialEq + Clone> From<HashMap<usize, T>> for DefaultSparseVec<T> {
    #[inline(always)]
    fn from(map: HashMap<usize, T>) -> Self {
        let mut svec = DefaultSparseVec::new();
        map.into_iter().for_each(|(index, elem)| svec.insert(index, elem));
        svec.shrink_to_fit();
        svec
    }
}

impl<T: Default + PartialEq + Clone> Into<Vec<T>> for DefaultSparseVec<T> {
    #[inline(always)]
    fn into(self) -> Vec<T> {
        let mut vec = Vec::new();
        (0..self.len()).for_each(|i| vec.push(self.get(i).unwrap().clone()));
        vec
    }
}

impl<T: Default + PartialEq + Clone> Into<HashMap<usize, T>> for DefaultSparseVec<T> {
    #[inline(always)]
    fn into(self) -> HashMap<usize, T> {
        let mut map = HashMap::new();
        self.iter().for_each(|(index, elem)| {
            map.insert(*index, elem.clone());
        });
        map
    }
}

impl<T: Default + PartialEq + Clone> NormalVecMethods<T> for DefaultSparseVec<T> {
    #[inline(always)]
    fn n_push(&mut self, elem: T) {
        if self.raw_len == self.cap() {
            self.buf.grow();
        }
        if self.default == elem {
            unsafe {
                ptr::write(self.val_ptr().offset(self.raw_len as isize), elem);
                ptr::write(self.ind_ptr().offset(self.raw_len as isize), self.len);
            }
            self.raw_len += 1;
        }
        self.len += 1;
    }

    #[inline(always)]
    fn n_pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        // 空らずraw_len =< len であることが保証されている
        let pop_elem = 
            if self.raw_len == self.len {
                self.raw_len -= 1;
                unsafe {
                    Some(ptr::read(self.val_ptr().offset(self.raw_len as isize)))
                }
            } else {
                Some(self.default.clone())
            };
        self.len -= 1;
        pop_elem
    }

    #[inline(always)]
    fn n_insert(&mut self, index: usize, elem: T) {
        self.insert(index, elem);
    }
}

impl<T> Math<T> for DefaultSparseVec<T>
    where
    T: Num + Default + PartialEq + Clone + std::ops::AddAssign + std::ops::Mul<Output = T> + Into<u64>,
{
    #[inline(always)]
    fn u64_dot(&self, other: &Self) -> u64 {
        let mut sum: u64 = 0;
        let mut self_iter = self.iter();
        let mut other_iter = other.iter();
        let mut self_current = self_iter.next();
        let mut other_current = other_iter.next();

        while self_current.is_some() && other_current.is_some() {
            if self_current.unwrap().0 < other_current.unwrap().0 {
                self_current = self_iter.next();
            } else if self_current.unwrap().0 > other_current.unwrap().0 {
                other_current = other_iter.next();
            } else {
                sum += (self_current.unwrap().1.clone() * other_current.unwrap().1.clone()).into();
                self_current = self_iter.next();
                other_current = other_iter.next();
            }
        }
        sum
    }
}


/// RawDefaultSparseVec構造体の定義
/// T: スパースするデータの型
/// val_ptr: スパースするデータの値のポインタ
/// ind_ptr: スパースするデータのインデックスのポインタ
/// cap: スパースするデータの容量
/// _marker: 所有権管理用のPhantomData
#[derive(Debug, Clone, )]
struct RawDefaultSparseVec<T> {
    val_ptr: NonNull<T>,
    ind_ptr: NonNull<usize>,
    /// cap 定義
    /// 0 => メモリ未確保 (flag)
    /// usize::MAX =>  zero size struct (ZST) として定義 処理の簡略化を実施 (flag)
    /// _ => 実際のcap
    cap: usize,
    _marker: PhantomData<T>, // 所有権管理用にPhantomDataを追加
}

impl<T> RawDefaultSparseVec<T> {
    #[inline(always)]
    fn new() -> Self {
        // 効率化: zero size struct (ZST)をusize::MAXと定義 ある種のフラグとして使用
        let cap = if mem::size_of::<T>() == 0 { std::usize::MAX } else { 0 }; 

        RawDefaultSparseVec {
            // 効率化: 空のポインタを代入しておく メモリ確保を遅延させる
            val_ptr: NonNull::dangling(),
            // 効率化: 空のポインタを代入しておく メモリ確保を遅延させる
            ind_ptr: NonNull::dangling(),
            cap: cap,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    fn grow(&mut self) {
        unsafe {
            let val_elem_size = mem::size_of::<T>();
            let ind_elem_size = mem::size_of::<usize>();

            // 安全性: ZSTの場合growはcapを超えた場合にしか呼ばれない
            // これは必然的にオーバーフローしていることをしめしている
            assert!(val_elem_size != 0, "capacity overflow");

            // アライメントの取得 適切なメモリ確保を行うため
            let t_align = mem::align_of::<T>();
            let usize_align = mem::align_of::<usize>();

            // アロケーション
            let (new_cap, val_ptr, ind_ptr): (usize, *mut T, *mut usize) = 
                if self.cap == 0 {
                    let new_val_layout = Layout::from_size_align(val_elem_size, t_align).expect("Failed to create memory layout");
                    let new_ind_layout = Layout::from_size_align(ind_elem_size, usize_align).expect("Failed to create memory layout");
                    (
                        1,
                        alloc(new_val_layout) as *mut T,
                        alloc(new_ind_layout) as *mut usize,
                    )
                } else {
                    // 効率化: cap * 2 でメモリを確保する 見た目上はO(log n)の増加を実現
                    let new_cap = self.cap * 2;
                    let new_val_layout = Layout::from_size_align(val_elem_size * self.cap, t_align).expect("Failed to create memory layout for reallocation");
                    let new_ind_layout = Layout::from_size_align(ind_elem_size * self.cap, usize_align).expect("Failed to create memory layout for reallocation");
                    (
                        new_cap,
                        realloc(self.val_ptr.as_ptr() as *mut u8, new_val_layout, val_elem_size * new_cap) as *mut T,
                        realloc(self.ind_ptr.as_ptr() as *mut u8, new_ind_layout, ind_elem_size * new_cap) as *mut usize,
                    )
                };

            // アロケーション失敗時の処理
            if val_ptr.is_null() || ind_ptr.is_null() {
                oom();
            }

            // selfに返却
            self.val_ptr = NonNull::new_unchecked(val_ptr);
            self.ind_ptr = NonNull::new_unchecked(ind_ptr);
            self.cap = new_cap;
        }
    }

    #[inline(always)]
    fn cap_set(&mut self) {
        unsafe {
            let val_elem_size = mem::size_of::<T>();
            let ind_elem_size = mem::size_of::<usize>();

            let t_align = mem::align_of::<T>();
            let usize_align = mem::align_of::<usize>();

            let new_val_layout = Layout::from_size_align(val_elem_size * self.cap, t_align).expect("Failed to create memory layout");
            let new_ind_layout = Layout::from_size_align(ind_elem_size * self.cap, usize_align).expect("Failed to create memory layout");
            let new_val_ptr = alloc(new_val_layout) as *mut T;
            let new_ind_ptr = alloc(new_ind_layout) as *mut usize;
            if new_val_ptr.is_null() || new_ind_ptr.is_null() {
                oom();
            }
            self.val_ptr = NonNull::new_unchecked(new_val_ptr);
            self.ind_ptr = NonNull::new_unchecked(new_ind_ptr);
        }
    }

    #[inline(always)]
    fn re_cap_set(&mut self) {
        unsafe {
            let val_elem_size = mem::size_of::<T>();
            let ind_elem_size = mem::size_of::<usize>();

            let t_align = mem::align_of::<T>();
            let usize_align = mem::align_of::<usize>();

            let new_val_layout = Layout::from_size_align(val_elem_size * self.cap, t_align).expect("Failed to create memory layout");
            let new_ind_layout = Layout::from_size_align(ind_elem_size * self.cap, usize_align).expect("Failed to create memory layout");
            let new_val_ptr = realloc(self.val_ptr.as_ptr() as *mut u8, new_val_layout, val_elem_size * self.cap) as *mut T;
            let new_ind_ptr = realloc(self.ind_ptr.as_ptr() as *mut u8, new_ind_layout, ind_elem_size * self.cap) as *mut usize;
            if new_val_ptr.is_null() || new_ind_ptr.is_null() {
                oom();
            }
            self.val_ptr = NonNull::new_unchecked(new_val_ptr);
            self.ind_ptr = NonNull::new_unchecked(new_ind_ptr);
        }
    }
}

unsafe impl<T: Send> Send for RawDefaultSparseVec<T> {}
unsafe impl<T: Sync> Sync for RawDefaultSparseVec<T> {}

impl<T> Drop for RawDefaultSparseVec<T> {
    #[inline(always)]
    fn drop(&mut self) {
        let val_elem_size = mem::size_of::<T>();
        let ind_elem_size = mem::size_of::<usize>();
        if self.cap != 0 && val_elem_size != 0 {
            let t_align = mem::align_of::<T>();
            let usize_align = mem::align_of::<usize>();
            unsafe {
                let val_layout = Layout::from_size_align(val_elem_size * self.cap, t_align).expect("Failed to create memory layout");
                let ind_layout = Layout::from_size_align(ind_elem_size * self.cap, usize_align).expect("Failed to create memory layout");
                dealloc(self.val_ptr.as_ptr() as *mut u8, val_layout);
                dealloc(self.ind_ptr.as_ptr() as *mut u8, ind_layout);
            }
        }
    }
}

/// OutOfMemoryへの対処用
/// プロセスを終了させる
/// 本来はpanic!を使用するべきだが、
/// OOMの場合panic!を発生させるとTraceBackによるメモリ仕様が起きてしまうため
/// 仕方なく強制終了させる
/// 本来OOMはOSにより管理され発生前にKillされるはずなのであんまり意味はない。
fn oom() {
    ::std::process::exit(-9999);
}