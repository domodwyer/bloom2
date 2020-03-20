use bloom2::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn basic_bench(c: &mut Criterion) {
    let mut bloom = CompressedBitmap::new(FilterSize::KeyBytes2);

    c.bench_function("insert", |b| {
        b.iter(|| bloom.insert_hash(black_box([1, 2])))
    });

    c.bench_function("lookup hit", |b| {
        b.iter(|| black_box(bloom.contains_hash([1, 2])))
    });

    c.bench_function("lookup miss, partial match", |b| {
        b.iter(|| black_box(bloom.contains_hash([1, 42])))
    });

    c.bench_function("lookup miss, different block", |b| {
        b.iter(|| black_box(bloom.contains_hash([13, 42])))
    });

    c.bench_function("lookup miss, same block", |b| {
        b.iter(|| black_box(bloom.contains_hash([1, 3])))
    });
}

pub fn insert_bench(c: &mut Criterion) {
    let mut bloom = CompressedBitmap::new(FilterSize::KeyBytes2);

    // Insert an initial value to allocate at least one block
    bloom.insert_hash([0, 1]);

    c.bench_function("clone_only", |b| {
        // Insert 10 hashes into the same block
        b.iter(|| {
            let bloom = bloom.clone();
            black_box(bloom);
        })
    });

    c.bench_function("clone_insert_10_existing_block", |b| {
        // Insert 10 hashes into the same block
        b.iter(|| {
            let mut bloom = bloom.clone();
            black_box(bloom.insert_hash([0, 2]));
            black_box(bloom.insert_hash([0, 3]));
            black_box(bloom.insert_hash([0, 4]));
            black_box(bloom.insert_hash([0, 5]));
            black_box(bloom.insert_hash([0, 6]));
            black_box(bloom.insert_hash([0, 7]));
            black_box(bloom.insert_hash([0, 8]));
            black_box(bloom.insert_hash([0, 9]));
            black_box(bloom.insert_hash([0, 10]));
            black_box(bloom.insert_hash([0, 11]));
        })
    });

    c.bench_function("clone_insert_10_new_block_forwards", |b| {
        // Insert into different blocks, potentially requiring an allocation.
        //
        // Each hash produces a key > 64 away from the last, requiring a new to
        // hold it.
        b.iter(|| {
            let mut bloom = bloom.clone();
            black_box(bloom.insert_hash([1, 2]));
            black_box(bloom.insert_hash([2, 2]));
            black_box(bloom.insert_hash([3, 2]));
            black_box(bloom.insert_hash([4, 2]));
            black_box(bloom.insert_hash([5, 2]));
            black_box(bloom.insert_hash([6, 2]));
            black_box(bloom.insert_hash([7, 2]));
            black_box(bloom.insert_hash([8, 2]));
            black_box(bloom.insert_hash([9, 2]));
            black_box(bloom.insert_hash([10, 2]));
        });
    });

    c.bench_function("clone_insert_10_new_block_backwards", |b| {
        // Insert into different blocks, potentially requiring an allocation.
        //
        // Each hash produces a key > 64 away from the last, requiring a new to
        // hold it.
        b.iter(|| {
            let mut bloom = bloom.clone();
            black_box(bloom.insert_hash([10, 2]));
            black_box(bloom.insert_hash([9, 2]));
            black_box(bloom.insert_hash([8, 2]));
            black_box(bloom.insert_hash([7, 2]));
            black_box(bloom.insert_hash([6, 2]));
            black_box(bloom.insert_hash([5, 2]));
            black_box(bloom.insert_hash([4, 2]));
            black_box(bloom.insert_hash([3, 2]));
            black_box(bloom.insert_hash([2, 2]));
            black_box(bloom.insert_hash([1, 2]));
        });
    });
}

criterion_group!(benches, basic_bench, insert_bench);
criterion_main!(benches);
