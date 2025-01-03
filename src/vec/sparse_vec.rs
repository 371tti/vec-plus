use std::{cmp::Ordering, marker::PhantomData};
use num::Num;

use serde::{de::value, Deserialize, Serialize};
use super::vec_trait::Math;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ZeroSparseVec<T>
where 
    T: Default + PartialEq + Clone,
{
    len: usize,
    indices: Vec<usize>,
    values: Vec<T>,
    default_value: T,
    _marker: PhantomData<T>,
}
impl<T> Default for ZeroSparseVec<T> 
where
    T: Default + PartialEq + Clone,
{
    fn default() -> Self {
        ZeroSparseVec {
            len: 0,
            indices: Vec::new(),
            values: Vec::new(),
            default_value: T::default(),
            _marker: PhantomData,
        }
    }
}

impl<T> From<Vec<T>> for ZeroSparseVec<T>
where
    T: Default + PartialEq + Clone,
{
    fn from(data: Vec<T>) -> Self {
        let mut indices = Vec::new();
        let mut values = Vec::new();

        for (index, value) in data.iter().enumerate() {
            if value != &T::default() {
                indices.push(index);
                values.push((*value).clone());
            }
        }

        ZeroSparseVec {
            len: data.len(),
            indices,
            values,
            _marker: PhantomData,
            default_value: T::default(),
        }
    }
}

impl<T> Into<Vec<T>> for ZeroSparseVec<T>
where
    T: Default + PartialEq + Clone,
{
    fn into(self) -> Vec<T> {
        let mut full_vec = vec![T::default(); self.len];
        for (index, value) in self.sparse_iter() {
            full_vec[*index] = value.clone();
        }
        full_vec
    }
}

impl<T> PartialOrd for ZeroSparseVec<T>
where
    T: Default + PartialEq + Clone + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.values.partial_cmp(&other.values)
    }
}

impl<T> Ord for ZeroSparseVec<T>
where
    T: Default + PartialEq + Clone + Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.values.cmp(&other.values)
    }
}

impl<T> ZeroSparseVec<T>
where
    T: Default + PartialEq + Clone,
{
    pub fn new(len: usize, indices: Vec<usize>, values: Vec<T>) -> Self {
        ZeroSparseVec {
            len,
            indices,
            values,
            _marker: PhantomData,
            default_value: T::default(),
        }
    }

    pub fn with_capacity(sparse_capacity: usize) -> Self {
        ZeroSparseVec {
            len: 0,
            indices: Vec::with_capacity(sparse_capacity),
            values: Vec::with_capacity(sparse_capacity),
            _marker: PhantomData,
            default_value: T::default(),
        }
    }

    pub fn subset(&self, range: std::ops::Range<usize>) -> Self {
        let mut indices = Vec::new();
        let mut values = Vec::new();

        for (index, value) in self.sparse_iter() {
            if range.contains(&index) {
                indices.push(index - range.start);
                values.push(value.clone());
            }
        }

        ZeroSparseVec {
            len: range.end - range.start,
            indices,
            values,
            _marker: PhantomData,
            default_value: T::default(),
        }
    }

    pub fn clear(&mut self) {
        self.indices.clear();
        self.values.clear();
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn sparse_len_mut(&mut self) -> &mut usize {
        &mut self.len
    }

    pub fn sparse_indices(&self) -> &[usize] {
        &self.indices
    }

    pub fn sparse_indices_mut(&mut self) -> &mut [usize] {
        &mut self.indices
    }

    pub fn sparse_values(&self) -> &[T] {
        &self.values
    }

    pub fn sparse_values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }

    pub fn sparse_iter (&self) -> impl Iterator<Item = (&usize, &T)> {
        self.indices.iter().zip(self.values.iter())
    }

    pub fn sparse_iter_mut (&mut self) -> impl Iterator<Item = (&mut usize, &mut T)> {
        self.indices.iter_mut().zip(self.values.iter_mut())
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    pub fn get(&self, index: &usize) -> Option<&T> {
        if *index < self.len {
            if let Some(pos) = self.indices.binary_search(&index).ok() {
                Some(&self.values[pos])
            } else {
                Some(&self.default_value)
            }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: &usize) -> Option<&mut T> {
        match self.indices.binary_search(&index) {
            Ok(pos) => {
                return Some(&mut self.values[pos]);
            },
            Err(pos) => {
                if *index < self.len {
                    self.indices.insert(pos, *index);
                    self.values.insert(pos, self.default_value.clone());
                    self.len += 1;
                    return Some(&mut self.values[pos]);
                } else {
                    return None;
                }
            },
        }
    }

    pub fn index(&self, index: &usize) -> Option<&T> {
        if *index < self.len {
            if let Some(pos) = self.indices.binary_search(&index).ok() {
                Some(&self.values[pos])
            } else {
                Some(&self.default_value)
            }
        } else {
            None
        }
    } 

    pub fn index_mut(&mut self, index: &usize) -> Option<&mut T> {
        match self.indices.binary_search(&index) {
            Ok(pos) => {
                return Some(&mut self.values[pos]);
            },
            Err(pos) => {
                if *index < self.len {
                    self.indices.insert(pos, *index);
                    self.values.insert(pos, self.default_value.clone());
                    self.len += 1;
                    return Some(&mut self.values[pos]);
                } else {
                    return None;
                }
            },
        }
    }

    pub fn push(&mut self, value: T) {
        if value != self.default_value {
            self.indices.push(self.len);
            self.values.push(value);
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        match self.indices.last() {
            Some(&index) => {
                if index == self.len - 1 {
                    self.len -= 1;
                    self.indices.pop();
                    return self.values.pop();
                } else {
                    self.len -= 1;
                    return Some(self.default_value.clone());
                }
            }
            _ => {
                return None;
            }    
        }
    }

    pub fn first(&self) -> Option<&T> {
        self.get(&0)
    }

    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.get_mut(&0)
    }

    pub fn last(&self) -> Option<&T> {
        self.get(&(self.len - 1))
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.get_mut(&(self.len - 1))
    }

    pub fn sparse_extend(&mut self, other: &Self) {
        for value in other.iter() {
            self.push(value.clone());
        }
    }

    pub fn extend(&mut self, other: &Vec<T>) {
        for value in other {
            self.push(value.clone());
        }
    }

    pub fn sparse_append(&mut self, other: &mut Self) {
        self.sparse_extend(other);
        other.clear();
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        self.extend(other);
        other.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..(self.len-1)).map(|index| self.get(&index).unwrap())
    }

    // pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
    //     (0..self.len).map(move |index| self.get_mut(&index).unwrap())
    // }

    pub fn compress(&mut self) {
            let to_remove: Vec<usize> = self.sparse_iter().filter_map(|(index, value)| {
                if value == &self.default_value {
                    Some(*index)
                } else {
                    None
                }
            }).collect();
    
            for index in to_remove.iter().rev() {
                self.indices.remove(*index);
                self.values.remove(*index);
                self.len -= 1;
            }
        }

    
}

pub mod marh {
    use std::cmp::Ordering;

    use num::Num;

    use crate::vec::vec_trait::Math;

    use super::ZeroSparseVec;

    impl<T> Math<T> for ZeroSparseVec<T>
    where
        T: Num + Default + PartialEq + Clone + std::ops::AddAssign + std::ops::Mul<Output = T> + Into<u64>,
    {
        fn u64_dot(&self, other: &Self) -> u64 {
            let mut result: u64 = 0;
            let mut self_iter = self.sparse_iter();
            let mut other_iter = other.sparse_iter();

            let mut self_current = self_iter.next();
            let mut other_current = other_iter.next();

            while self_current.is_some() && other_current.is_some() {
                match self_current.unwrap().0.cmp(&other_current.unwrap().0) {
                    Ordering::Less => {
                        self_current = self_iter.next();
                    },
                    Ordering::Greater => {
                        other_current = other_iter.next();
                    },
                    Ordering::Equal => {
                        result += (self_current.unwrap().1.clone() * other_current.unwrap().1.clone()).into();
                        self_current = self_iter.next();
                        other_current = other_iter.next();
                    },
                }
            }
            result
        }
    }

}
