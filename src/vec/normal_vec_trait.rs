pub trait NormalVecMethods<T> {
    fn n_push(&mut self, elem: T);
    fn n_pop(&mut self) -> Option<T>;
    fn n_insert(&mut self, index: usize, elem: T);
    // fn n_remove(&mut self, index: usize) -> T;
    // fn n_iter(&self) -> impl Iterator<Item = &T>;
}