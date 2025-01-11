use vec_plus::vec::default_sparse_vec::DefaultSparseVec;

fn main() {
    let mut svec = DefaultSparseVec::<i32>::new();
    svec.push(10);
    svec.push(0);
    svec.push(30);
    svec.remove(1);
    svec.push(0);
    svec.push(0);
    svec.push(0);
    svec.insert(4, 100);
    println!("{}", svec.get(0).unwrap());
    let a = svec.get_mut(2).unwrap();
    *a = 100;
    let vec = vec![1, 2, 3, 4, 5];
    svec.extend(vec);
    println!("{:?}", svec);
}
