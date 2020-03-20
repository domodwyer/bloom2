## bloom2

A fast 2-level, sparse bloom filter implementation consuming 2% of memory when
empty compared to a standard bloom filter.

* Sparse allocation grows memory usuage proportionally w.r.t filter load
* Low overhead, fast `O(1)` lookups with amortised `O(1)` inserts
* 32bit and 64bit safe
* Maintains same false positive probabilities compared to standard bloom filters
* No 'unsafe' code

The CompressedBitmap maintains the same false-positive properties and similar
performance properties as a normal bloom filter while lazily initialising the
backing memory as it is needed, resulting in smaller memory footprints for all
except completely loaded filters.

As the false positive probability for a bloom filter increases as the number of
entries increases, they are typically sized to maintain a small load factor,
resulting in inefficient use of the underlying bitmap:

```text
		┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
		│ 0 │ 0 │ 0 │ 0 │ 1 │ 0 │ 0 │ 1 │ 0 │ 0 │ 0 │ 0 │
		└───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
```

This 2-level bloom filter splits the bitmap up into blocks of `usize` bits, and
uses a second bitmap to mark populated blocks, lazily allocating them as
required:

```text
	                 ┌───┬───┬───┬───┐                       
	      Block map: │ 0 │ 1 │ 0 │ 0 │                       
	                 └───┴─┬─┴───┴───┘                       
	                       └──────┐                          
	    ┌ ─ ┬ ─ ┬ ─ ┬ ─ ┐ ┌───┬───▼───┬───┐ ┌ ─ ┬ ─ ┬ ─ ┬ ─ ┐
	      0   0   0   0   │ 1 │ 0 │ 0 │ 1 │   0   0   0   0  
	    └ ─ ┴ ─ ┴ ─ ┴ ─ ┘ └───┴───┴───┴───┘ └ ─ ┴ ─ ┴ ─ ┴ ─ ┘
```

Lookups for indexes that land in unpopulated blocks check the single block map
bit and return immediately. 

Lookups for indexes in populated blocks first check the block map bit, before
computing the offset to the bitmap block in the bitmap array by counting the
number of 1 bits preceding it in the block map. This is highly efficient as it
uses the `POPCNT` instruction on modern CPUs.

## Use case

Perfect for long lived, sparsely populated bloom filters held in RAM or on disk.

If the filter is larger than available RAM / stored on disk, mmap can be used to
load in 2-level bloom filters for a significant performance improvement. The OS
lazily loads bitmap blocks from disk as they're accessed, while the frequently
accessed block map remains in memory to provide a fast negative response for
unpopulated blocks.

## Serialisation

Enable optional serialisation with the `serde` feature - disabled by default.