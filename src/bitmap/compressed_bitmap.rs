use crate::Bitmap;
use std::mem;

/// A sparse, 2-level bitmap with a low memory footprint.
///
/// A `CompressedBitmap` splits the bitmap up into blocks of `usize` bits, and
/// uses a second bitmap to mark populated blocks, lazily allocating them as
/// required. This strategy represents a sparsely populated bitmap such as:
///
/// ```text
///    ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///    │ 0 │ 0 │ 0 │ 0 │ 1 │ 0 │ 0 │ 1 │ 0 │ 0 │ 0 │ 0 │
///    └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
/// ```
///
/// As two bitmaps, here initialising only a single blocks of `usize` bits in
/// the second bitmap:
///
/// ```text
///                  ┌───┬───┬───┬───┐                       
///       Block map: │ 0 │ 1 │ 0 │ 0 │                       
///                  └───┴─┬─┴───┴───┘                       
///                        └──────┐                          
///     ┌ ─ ┬ ─ ┬ ─ ┬ ─ ┐ ┌───┬───▼───┬───┐ ┌ ─ ┬ ─ ┬ ─ ┬ ─ ┐
///       0   0   0   0   │ 1 │ 0 │ 0 │ 1 │   0   0   0   0  
///     └ ─ ┴ ─ ┴ ─ ┴ ─ ┘ └───┴───┴───┴───┘ └ ─ ┴ ─ ┴ ─ ┴ ─ ┘
/// ```
///
/// This amortised `O(1)` insert operation takes ~4ns, while reading a value
/// takes a constant time ~1ns on a Core i7 @ 2.60GHz.
///
///
/// ## Features
///
/// If the `serde` feature is enabled, a `CompressedBitmap` supports
/// (de)serialisation with [serde].
///
/// [serde]: https://github.com/serde-rs/serde
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompressedBitmap {
    block_map: Vec<usize>,
    bitmap: Vec<usize>,

    #[cfg(debug_assertions)]
    max_key: usize,
}

impl CompressedBitmap {
    /// Construct a `CompressedBitmap` for space to hold up to `max_key` number
    /// of bits.
    pub fn new(max_key: usize) -> Self {
        // Calculate how many instances of usize (blocks) are needed to hold
        // max_key number of bits.
        let blocks = index_for_key(max_key);

        // Figure out how many usize elements are needed to represent blocks
        // number of bitmaps.
        let num_blocks = match blocks % (mem::size_of::<usize>() * 8) {
            0 => index_for_key(blocks),
            _ => index_for_key(blocks) + 1, // +1 to cover the remainder
        };

        // Allocate a block map.
        //
        // The block map contains bitmaps with 1 bits indicating the bitmap for
        // that key has been allocated.
        let mut block_map = Vec::new();
        block_map.resize(num_blocks, 0);

        CompressedBitmap {
            bitmap: Vec::new(),
            block_map,

            #[cfg(debug_assertions)]
            max_key,
        }
    }

    pub fn size(&self) -> usize {
        (self.block_map.capacity() * std::mem::size_of::<usize>())
            + (self.bitmap.capacity() * std::mem::size_of::<usize>())
            + std::mem::size_of_val(self)
    }

    /// Reduces the allocated memory usage of the filter to the minimum required
    /// for the current filter contents.
    ///
    /// This is useful to minimise the memory footprint of a populated,
    /// read-only CompressedBitmap.
    ///
    /// See [`Vec::shrink_to_fit`](std::vec::Vec::shrink_to_fit).
    pub fn shrink_to_fit(&mut self) {
        self.bitmap.shrink_to_fit();
        self.block_map.shrink_to_fit();
        // TODO: remove 0 blocks
    }

    /// Resets the state of the filter.
    ///
    /// An efficient way to remove all elements in the filter to allow it to be
    /// reused. Does not shrink the allocated backing memory, instead retaining
    /// the capacity to avoid reallocations.
    pub fn clear(&mut self) {
        for block in self.block_map.iter_mut() {
            *block = 0;
        }
        self.bitmap.truncate(0);
    }

    /// Inserts `key` into the filter.
    ///
    /// # Panics
    ///
    /// This method MAY panic if `key` is more than the `max_key` value provided
    /// when initialising the bitmap.
    ///
    /// If `debug_assertions` are enabled (such as in debug builds) inserting
    /// `key > max` will always panic. In release builds, this may not panic for
    /// values of `key` that are only slightly larger than `max_key` for
    /// performance reasons.
    pub fn set(&mut self, key: usize, value: bool) {
        #[cfg(debug_assertions)]
        debug_assert!(key <= self.max_key, "key {} > {} max", key, self.max_key);

        // First compute the index of the bit in the bitmap if it was fully
        // populated.
        //
        //
        //     Bitmap:                │
        //                            ▼
        //       ┌───┬───┬───┬───┐  ┌───┬───┬───┬───┐  ┌───┬───┬───┬───┐
        //       │ 0 │ 0 │ 0 │ 0 │  │ 0 │ 0 │ 0 │ 0 │  │ 0 │ 0 │ 0 │ 0 │
        //       └───┴───┴───┴───┘  └───┴───┴───┴───┘  └───┴───┴───┴───┘
        //            Block 0            Block 1            Block 2
        //
        //
        // Next figure out which block (usize) this bitmap_index is part of.
        //
        //	  Bitmap:                      │
        //	                      ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
        //	    ┌───┬───┬───┬───┐  ┌───┬───┬───┬───┐  ┌───┬───┬───┬───┐
        //	    │ 0 │ 0 │ 0 │ 0 │  │ 0 │ 0 │ 0 │ 0 │  │ 0 │ 0 │ 0 │ 0 │
        //	    └───┴───┴───┴───┘  └───┴───┴───┴───┘  └───┴───┴───┴───┘
        //	         Block 0            Block 1            Block 2
        //
        let block_index = index_for_key(key);

        // Because the blocks are initialised lazily to provide the sparse
        // filter behaviour, there may be no block yet allocated for this
        // bitmap index. The block_map data structure is itself bitmap with
        // a 1 bit indicating the block has been allocated.
        //
        // Check which usize in the block_map contains the bit representing
        // the block.
        //
        //            Block Map:
        //
        //                      ┌───┬───┬───┬───┐
        //                   0: │ 0 │ 1 │ 1 │ 0 │
        //                      └───┴───┴───┴───┘
        //
        //                      ┌───┬───┬───┬───┐
        //                   1: │ 1 │ 0 │ 1 │ 0 │
        //                      └─▲─┴───┴───┴───┘
        //     block_index ━━━━━━━┛
        //                      ┌───┬───┬───┬───┐
        //                   2: │ 0 │ 0 │ 1 │ 1 │
        //                      └───┴───┴───┴───┘
        //
        let block_map_index = index_for_key(block_index);
        let block_map_bitmask = bitmask_for_key(block_index);

        // The block has been allocated if the block usize contains a 1 bit.
        //
        // Because blocks are lazily initialised, block n may not be at
        // block_map[n] if prior blocks have not been initialised. To
        // calculate the offset of block n, the number of 1's in the
        // block_map before bit n. This operation is very fast on modern
        // hardware thanks to the POPCNT instruction.
        //
        //            Block Map:
        //
        //                          ┌───┬───┐
        //                        0 │ 1 │ 1 │ 0
        //                          └─△─┴─△─┘
        //                            └───┼────────── popcount()
        //                      ┏━━━┓   ┌─▽─┐
        //                      ┃ 1 ┃ 0 │ 1 │ 0
        //                      ┗━▲━┛   └───┘
        //     block_index ━━━━━━━┛
        //
        //
        // In the above example, the popcount() is 3, and the block is the
        // 3+1=4th block in bitmap. However as the arrays are zero-indexed,
        // the +1 is omitted to adjust from the position 4, to index 3.

        // Count the ones in the full blocks.
        //
        // This could chain() the final masked count_ones() call below using
        // once_with, and while more readable, it is unfortunately measurably
        // slower in practice.
        let offset: usize = (0..block_map_index)
            .map(|i| self.block_map[i].count_ones() as usize)
            .sum();

        // Mask out the higher bits in the block map to count the populated
        // blocks before block_index
        let mask = block_map_bitmask - 1;
        let offset = offset + (self.block_map[block_map_index] & mask).count_ones() as usize;

        // Offset now contains the index in bitmap at which block_index can
        // be found.
        //
        // Because the blocks are lazily initialised, there may not yet be a
        // block for block_map_index.
        //
        // Read the usize at block_map_index, and check the bit for
        // block_index.
        if self.block_map[block_map_index] & block_map_bitmask == 0 {
            // If the value to be set is false, there's nothing to do.
            if !value {
                return;
            }

            // The block does not exist, insert it into the bitmap at
            // block_index.
            if offset >= self.bitmap.len() {
                self.bitmap.push(bitmask_for_key(key));
            } else {
                // If offset is < bitmap.len() this will require moving all
                // the elements at offset+1 one slot to the right to make
                // room for the new element.
                //
                // For bitmaps with large numbers of elements to the right
                // of offset, this can become expensive.
                self.bitmap.insert(offset, bitmask_for_key(key));
            }
            self.block_map[block_map_index] |= block_map_bitmask;
            return;
        }

        // Otherwise the block map indicates the block is already allocated
        if value {
            self.bitmap[offset as usize] |= bitmask_for_key(key);
        } else {
            self.bitmap[offset as usize] &= !bitmask_for_key(key);
        }
    }

    /// Returns the value at `key`.
    ///
    /// If a value for `key` was not previously set, `false` is returned.
    ///
    /// # Panics
    ///
    /// This method MAY panic if `key` is more than the `max_key` value provided
    /// when initialising the bitmap.
    pub fn get(&self, key: usize) -> bool {
        let block_index = index_for_key(key);
        let block_map_index = index_for_key(block_index);
        let block_map_bitmask = bitmask_for_key(block_index);

        if self.block_map[block_map_index] & block_map_bitmask == 0 {
            return false;
        }

        let offset: usize = (0..block_map_index)
            .map(|i| self.block_map[i].count_ones() as usize)
            .sum();

        let mask = block_map_bitmask - 1;
        let offset: usize = offset + (self.block_map[block_map_index] & mask).count_ones() as usize;

        self.bitmap[offset] & bitmask_for_key(key) != 0
    }
}

impl Bitmap for CompressedBitmap {
    fn get(&self, key: usize) -> bool {
        self.get(key)
    }

    fn set(&mut self, key: usize, value: bool) {
        self.set(key, value)
    }

    fn byte_size(&self) -> usize {
        self.size()
    }
}

#[inline(always)]
fn bitmask_for_key(key: usize) -> usize {
    1 << (key % (mem::size_of::<usize>() * 8))
}

#[inline(always)]
fn index_for_key(key: usize) -> usize {
    key / (mem::size_of::<usize>() * 8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    macro_rules! contains_only_truthy {
		($bitmap:ident, $max:expr; $(
            $element:expr
        ),*) => {
			let truthy = vec![$($element,)*];
			for i in 0..$max {
				assert!($bitmap.get(i) == truthy.contains(&i), "unexpected value {}", i);
			}
		};
	}

    #[test]
    fn test_set_contains() {
        let mut b = CompressedBitmap::new(100);
        b.set(100, true);
        b.set(0, true);
        b.set(42, true);

        contains_only_truthy!(b, 100; 100, 0, 42);

        assert!(b.get(100));
        assert!(b.get(0));
        assert!(b.get(42));
    }

    #[test]
    fn test_clear() {
        let mut b = CompressedBitmap::new(100);
        b.set(100, true);
        b.set(0, true);
        b.set(42, true);

        contains_only_truthy!(b, 100; 100, 0, 42);
        b.clear();
        contains_only_truthy!(b, 100;);
    }

    #[test]
    fn test_set_true_false() {
        let mut b = CompressedBitmap::new(100);
        b.set(42, true);
        assert!(b.get(42));
        b.set(42, false);
        assert!(!b.get(42));
    }

    #[quickcheck]
    #[should_panic]
    fn test_panic_exceeds_max(max: u16) {
        let max = max as usize;
        let mut b = CompressedBitmap::new(max);
        b.set(max + 1, true);
    }

    #[quickcheck]
    fn test_set_contains_prop(mut vals: Vec<u16>) {
        vals.truncate(10);
        let mut b = CompressedBitmap::new(u16::MAX.into());
        for v in &vals {
            b.set(*v as usize, true);
        }

        for i in 0..u16::MAX {
            assert!(
                b.get(i as usize) == vals.contains(&i),
                "unexpected value {}",
                i
            );
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        let mut b = CompressedBitmap::new(100);
        b.set(1, true);
        b.set(2, false);
        b.set(3, true);
        contains_only_truthy!(b, 100; 1, 3);

        let encoded = serde_json::to_string(&b).unwrap();
        let decoded: CompressedBitmap = serde_json::from_str(&encoded).unwrap();
        contains_only_truthy!(decoded, 100; 1, 3);
    }
}
