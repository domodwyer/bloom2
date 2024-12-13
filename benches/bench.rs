use bloom2::*;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

pub fn bitmap_bench(c: &mut Criterion) {
    let mut bloom = CompressedBitmap::new(1024);

    c.bench_function("bitmap_insert_true", |b| b.iter(|| bloom.set(42, true)));
    c.bench_function("bitmap_insert_false", |b| b.iter(|| bloom.set(42, false)));
    c.bench_function("bitmap_lookup_hit", |b| {
        bloom.set(42, true);
        b.iter(|| black_box(bloom.get(42)))
    });
    c.bench_function("bitmap_lookup_miss, different block", |b| {
        b.iter(|| black_box(bloom.get(1)))
    });
    c.bench_function("bitmap_lookup miss, same block", |b| {
        b.iter(|| black_box(bloom.get(43)))
    });
}

pub fn basic_bench(c: &mut Criterion) {
    let mut bloom = Bloom2::default();

    c.bench_function("bloom_insert", |b| b.iter(|| bloom.insert(&[1, 2])));

    c.bench_function("bloom_lookup_hit", |b| {
        b.iter(|| black_box(bloom.contains(&[1, 2])))
    });

    c.bench_function("bloom_lookup_miss_partial match", |b| {
        b.iter(|| black_box(bloom.contains(&[1, 42])))
    });

    c.bench_function("bloom_lookup_miss_different block", |b| {
        b.iter(|| black_box(bloom.contains(&[13, 42])))
    });

    c.bench_function("bloom_lookup_miss_same_block", |b| {
        b.iter(|| black_box(bloom.contains(&[1, 3])))
    });
}

pub fn insert_bench(c: &mut Criterion) {
    let mut bloom = Bloom2::default();

    // Insert an initial value to allocate at least one block
    bloom.insert(&[0, 1]);

    c.bench_function("bloom_clone_only", |b| {
        // Insert 10 hashes into the same block
        b.iter(|| {
            let bloom = bloom.clone();
            black_box(bloom);
        })
    });

    c.bench_function("bloom_clone_insert_10_existing_block", |b| {
        // Insert 10 hashes into the same block
        b.iter(|| {
            let mut bloom = bloom.clone();
            bloom.insert(&[0, 2]);
            bloom.insert(&[0, 3]);
            bloom.insert(&[0, 4]);
            bloom.insert(&[0, 5]);
            bloom.insert(&[0, 6]);
            bloom.insert(&[0, 7]);
            bloom.insert(&[0, 8]);
            bloom.insert(&[0, 9]);
            bloom.insert(&[0, 10]);
            bloom.insert(&[0, 11]);
        })
    });

    c.bench_function("bloom_clone_insert_10_new_block_forwards", |b| {
        // Insert into different blocks, potentially requiring an allocation.
        //
        // Each hash produces a key > 64 away from the last, requiring a new to
        // hold it.
        b.iter(|| {
            let mut bloom = bloom.clone();
            bloom.insert(&[1, 2]);
            bloom.insert(&[2, 2]);
            bloom.insert(&[3, 2]);
            bloom.insert(&[4, 2]);
            bloom.insert(&[5, 2]);
            bloom.insert(&[6, 2]);
            bloom.insert(&[7, 2]);
            bloom.insert(&[8, 2]);
            bloom.insert(&[9, 2]);
            bloom.insert(&[10, 2]);
        });
    });

    c.bench_function("bloom_clone_insert_10_new_block_backwards", |b| {
        // Insert into different blocks, potentially requiring an allocation.
        //
        // Each hash produces a key > 64 away from the last, requiring a new to
        // hold it.
        b.iter(|| {
            let mut bloom = bloom.clone();
            bloom.insert(&[10, 2]);
            bloom.insert(&[9, 2]);
            bloom.insert(&[8, 2]);
            bloom.insert(&[7, 2]);
            bloom.insert(&[6, 2]);
            bloom.insert(&[5, 2]);
            bloom.insert(&[4, 2]);
            bloom.insert(&[3, 2]);
            bloom.insert(&[2, 2]);
            bloom.insert(&[1, 2]);
        });
    });

    c.bench_function("bloom_vec_insert_4_000_000", |b| {
        b.iter_batched(
            || {
                BloomFilterBuilder::default()
                    .with_bitmap::<VecBitmap>()
                    .size(bloom2::FilterSize::KeyBytes4)
                    .build()
            },
            |mut bloom| {
                for i in 0..4_000_000 {
                    bloom.insert(black_box(&i));
                }

                black_box(bloom)
            },
            BatchSize::NumBatches(1),
        )
    });

    c.bench_function("bloom_vec_convert_4_000_000", |b| {
        let mut bloom = BloomFilterBuilder::default()
            .with_bitmap::<VecBitmap>()
            .size(bloom2::FilterSize::KeyBytes4)
            .build();

        for i in 0..4_000_000 {
            bloom.insert(black_box(&i));
        }

        b.iter_batched(
            || bloom.clone(),
            |bloom| black_box(bloom.compress()),
            BatchSize::NumBatches(1),
        )
    });
}

criterion_group!(benches, basic_bench, insert_bench, bitmap_bench);
criterion_main!(benches);
