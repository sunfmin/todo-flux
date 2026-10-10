//! A vector whose length is in its type, after the `RVec` of Flux's guide.
//!
//! std's `Vec` has Flux specs too, but Flux does not take its indexing as
//! panic-free. Here the struct is opaque and every function is trusted, so
//! this file is where the claims about the vector are made by hand: what each
//! operation does to the length, and that reading an index below the length
//! cannot panic. Only what the todo list uses is here.

use flux_rs::attrs::*;

#[opaque]
#[refined_by(len: int)]
#[invariant(0 <= len)]
pub struct RVec<T> {
    inner: Vec<T>,
}

impl<T> RVec<T> {
    #[trusted]
    #[no_panic]
    #[spec(fn() -> RVec<T>[0])]
    pub fn new() -> RVec<T> {
        RVec { inner: Vec::new() }
    }

    #[trusted]
    #[no_panic]
    #[spec(fn(&RVec<T>[@n]) -> usize[n])]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[trusted]
    #[no_panic]
    #[spec(fn(&RVec<T>[@n], index: usize{index < n}) -> &T)]
    pub fn get(&self, index: usize) -> &T {
        &self.inner[index]
    }

    #[trusted]
    #[no_panic]
    #[spec(fn(&mut RVec<T>[@n], index: usize{index < n}) -> &mut T)]
    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.inner[index]
    }

    #[trusted]
    #[no_panic]
    #[spec(fn(&RVec<T>[@n]) -> &[T][n])]
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    #[trusted]
    #[spec(fn(self: &mut RVec<T>[@n], T) ensures self: RVec<T>[n + 1])]
    pub fn push(&mut self, item: T) {
        self.inner.push(item);
    }

    #[trusted]
    #[spec(fn(self: &mut RVec<T>[@n], index: usize{index < n}) -> T ensures self: RVec<T>[n - 1])]
    pub fn remove(&mut self, index: usize) -> T {
        self.inner.remove(index)
    }

    #[trusted]
    #[spec(fn(self: &mut RVec<T>[@n], index: usize{index <= n}, T) ensures self: RVec<T>[n + 1])]
    pub fn insert(&mut self, index: usize, item: T) {
        self.inner.insert(index, item);
    }
}
