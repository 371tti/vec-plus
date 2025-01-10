use std::{alloc::{alloc, dealloc, realloc, Layout}, default, marker::PhantomData, mem, num::NonZero, ops::{Deref, DerefMut}, ptr::{self, NonNull}};
use super::normal_vec_trait::NormalVecMethods;

/// <T> のdefault値をスパースするSparseVectorの実装
/// Vecの実装を参考にします
/// src : https://doc.rust-jp.rs/rust-nomicon-ja/vec.html
///     : https://doc.rust-lang.org/std/vec/struct.Vec.html

#[derive(Debug, Clone, Hash)]
pub struct DefaultSparseVec<T: Default + PartialEq + Clone> {
    buf: RawDefaultSparseVec<T>,
    raw_len: usize,
    len: usize,
    default: T,
}

impl<T: Default + PartialEq + Clone> DefaultSparseVec<T> {
    fn val_ptr(&self) -> *mut T { self.buf.val_ptr.as_ptr() }

    fn ind_ptr(&self) -> *mut usize { self.buf.ind_ptr.as_ptr() }

    fn cap(&self) -> usize { self.buf.cap }

    // ind_binary_searchメソッドの実装
    // 返り値はポインタの位置
    fn ind_binary_search(&self, index: &usize) -> Result<usize, usize> {
        let mut left = 0;
        let mut right = self.raw_len - 1;
        while left < right {
            let mid = left + (right - left) / 2;
            let mid_index = unsafe {ptr::read(self.ind_ptr().offset(mid as isize))};
            if mid_index == *index {
                return Ok(mid);
            } else if mid_index < *index {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        Err(left)
    }

    /// newメソッドの実装
    pub fn new() -> Self {
        DefaultSparseVec {
            buf: RawDefaultSparseVec::new(),
            raw_len: 0,
            len: 0,
            default: Default::default(),
        }
    }

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
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// is_someメソッドの実装
    pub fn is_some(&self) -> bool {
        self.len != 0
    }

    /// nnzメソッドの実装
    /// スパースベクトル長の取得
    pub fn nnz(&self) -> usize {
        self.raw_len
    }

    /// lenメソッドの実装
    pub fn len(&self) -> usize {
        self.len
    }

    /// clearメソッドの実装
    pub fn clear(&mut self) {
        while let Some(_) = self.pop() {}
    }

    /// pushメソッドの実装
    pub fn push(&mut self, elem: T) {
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

    /// popメソッドの実装
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

    /// insertメソッドの実装
    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "index out of bounds");
        if self.raw_len == self.cap() {
            self.buf.grow();
        }
        match self.ind_binary_search(&index) {
            Ok(i) => {
                unsafe {
                    let src = i as isize;
                    let dst = i as isize + 1;
                    let count = self.raw_len - i;
                    ptr::copy(  self.val_ptr().offset(src),
                                self.val_ptr().offset(dst),
                                count);
                    ptr::copy(  self.ind_ptr().offset(src),
                                self.ind_ptr().offset(dst),
                                count);
                    if self.default == elem {
                        ptr::write(self.val_ptr().offset(i as isize), elem);
                        ptr::write(self.ind_ptr().offset(i as isize), index);
                    }
                }
                self.raw_len += 1;
            },
            Err(i) => {
                if i < self.raw_len {
                    unsafe {
                        let src = i as isize;
                        let dst = i as isize + 1;
                        let count = self.raw_len - i;
                        ptr::copy(  self.val_ptr().offset(src),
                                    self.val_ptr().offset(dst),
                                    count);
                        ptr::copy(  self.ind_ptr().offset(src),
                                    self.ind_ptr().offset(dst),
                                    count);
                        ptr::write(self.val_ptr().offset(i as isize), elem);
                        ptr::write(self.ind_ptr().offset(i as isize), index);
                    }
                    self.raw_len += 1;
                } else {
                    if self.default == elem {
                        unsafe {
                            ptr::write(self.val_ptr().offset(self.raw_len as isize), elem);
                            ptr::write(self.ind_ptr().offset(self.raw_len as isize), index);
                        }
                        self.raw_len += 1;
                    }
                }
            }
        }
    }

    // iterメソッドの実装
    pub fn iter(&self) -> impl Iterator<Item = (&usize, &T)> {
        (0..self.raw_len).map(move |i| {
            let val: &T = unsafe { &*self.val_ptr().offset(i as isize) };
            let ind: &usize = unsafe { &*self.ind_ptr().offset(i as isize) };
            (ind, val)
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut usize, &mut T)> {
        (0..self.raw_len).map(move |i| {
            let val: &mut T = unsafe { &mut *self.val_ptr().offset(i as isize) };
            let ind: &mut usize = unsafe { &mut *self.ind_ptr().offset(i as isize) };
            (ind, val)
        })
    }
}

impl<T: Default + PartialEq + Clone> Drop for DefaultSparseVec<T> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
    }
}

impl<T: Default + PartialEq + Clone> Deref for DefaultSparseVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.val_ptr(), self.len)
        }
    }
}

impl <T: Default + PartialEq + Clone> DerefMut for DefaultSparseVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(self.val_ptr(), self.len)
        }
    }
}

impl <T: Default + PartialEq + Clone> Default for DefaultSparseVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default + PartialEq + Clone> NormalVecMethods<T> for DefaultSparseVec<T> {
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

    fn n_insert(&mut self, index: usize, elem: T) {
        self.insert(index, elem);
    }
}




/// RawDefaultSparseVec構造体の定義
/// T: スパースするデータの型
/// val_ptr: スパースするデータの値のポインタ
/// ind_ptr: スパースするデータのインデックスのポインタ
/// cap: スパースするデータの容量
/// _marker: 所有権管理用のPhantomData
#[derive(Debug, Clone, Hash)]
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
}

impl<T> Drop for RawDefaultSparseVec<T> {
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