use std::{cmp::Ordering, marker::PhantomData};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct ZeroSparseVec<T> {
    len: usize,
    indices: Vec<usize>,
    values: Vec<T>,
    _marker: PhantomData<T>,
}

impl<T> Default for ZeroSparseVec<T> 
where
    T: Default,
{
    fn default() -> Self {
        ZeroSparseVec {
            len: 0,
            indices: Vec::new(),
            values: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T> PartialOrd for ZeroSparseVec<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.len.partial_cmp(&other.len)
    }
}

impl<T> Ord for ZeroSparseVec<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.len.cmp(&other.len)
    }
}

impl<T> ZeroSparseVec<T> {
    pub fn new() {}
}
