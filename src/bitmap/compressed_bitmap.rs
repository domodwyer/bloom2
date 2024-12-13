use crate::Bitmap;

use super::{bitmask_for_key, index_for_key, vec::VecBitmap};

/// A sparse, 2-level bitmap with a low memory footprint, optimised for reads.
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
/// In practice inserting large numbers of values into a [`CompressedBitmap`]
/// can be slow - for higher write performance, use a [`VecBitmap`] and later
/// convert to a [`CompressedBitmap`] when possible.
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
    /// LSB is 0.
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
        let num_blocks = match blocks % (u64::BITS as usize) {
            0 => index_for_key(blocks),
            _ => index_for_key(blocks) + 1, // +1 to cover the remainder
        };

        // Allocate a block map.
        //
        // The block map contains bitmaps with 1 bits indicating the bitmap for
        // that key has been allocated.
        let block_map = vec![0; num_blocks];

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

    /// Reduces the allocated memory usage of the bitmap to the minimum required
    /// for the current bitmap contents.
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

    /// Resets the state of the bitmap.
    ///
    /// An efficient way to remove all elements in the bitmap to allow it to be
    /// reused. Does not shrink the allocated backing memory, instead retaining
    /// the capacity to avoid reallocations.
    pub fn clear(&mut self) {
        for block in self.block_map.iter_mut() {
            *block = 0;
        }
        self.bitmap.truncate(0);
    }

    /// Inserts `key` into the bitmap.
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
        // bitmap behaviour, there may be no block yet allocated for this bitmap
        // index. The block_map data structure is itself bitmap with a 1 bit
        // indicating the block has been allocated.
        //
        // Check which usize in the block_map contains the bit representing the
        // block.
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
            self.bitmap[offset] |= bitmask_for_key(key);
        } else {
            self.bitmap[offset] &= !bitmask_for_key(key);
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

    /// Perform a bitwise OR against `self` and `other`, returning the
    /// resulting merged [`CompressedBitmap`].
    ///
    /// # Panics
    ///
    /// This method panics if `other` was not configured with the same
    /// `max_key`.
    pub fn or(&self, other: &Self) -> Self {
        #[cfg(debug_assertions)]
        debug_assert_eq!(self.max_key, other.max_key);

        // Invariant: the block maps are of equal length, meaning the zipped
        // iters yield both sides to completion.
        assert_eq!(self.block_map.len(), other.block_map.len());

        let left = BlockMapIter::new(self);
        let right = BlockMapIter::new(other);

        // Construct the physical set of compressed bitmap blocks.
        //
        // By iterating over the non-empty logical blocks and OR-ing them
        // together (or picking one if only one is non-empty) the merged output
        // of both compressed bitmaps is computed (itself compressed).
        let bitmap = left
            .zip(right)
            .filter_map(|(l, r)| {
                Some(match (l, r) {
                    (None, None) => return None,
                    (None, Some(r)) => other.bitmap[r],
                    (Some(l), None) => self.bitmap[l],
                    (Some(l), Some(r)) => self.bitmap[l] | other.bitmap[r],
                })
            })
            .collect::<Vec<_>>();

        // Then merge the two bitmap blocks, the OR of which is guaranteed to
        // contain exactly N set bits for the N blocks in "physical".
        let block_map = self
            .block_map
            .iter()
            .zip(&other.block_map)
            .map(|(l, r)| l | r)
            .collect::<Vec<_>>();

        // Invariant: The number of set bits in the block map must match the
        // number of blocks in the bitmap.
        debug_assert_eq!(
            block_map.iter().map(|v| v.count_ones()).sum::<u32>() as usize,
            bitmap.len()
        );

        Self {
            block_map,
            bitmap,

            #[cfg(debug_assertions)]
            max_key: self.max_key,
        }
    }
}

/// Yields the 0-indexed physical indexes into the sparse bitmap for non-empty
/// blocks.
///
/// If for the Nth call to `next()` the Nth sparse bitmap block is elided,
/// [`None`] is returned. If the Nth bitmap block is non-empty, the physical
/// index into the compressed vec is yielded.
#[derive(Debug)]
struct BlockMapIter<'a> {
    bitmap: &'a CompressedBitmap,

    /// The index into bitmap.block_map to be processed next (0 -> N).
    block_idx: usize,
    /// The bit in the block to be evaluated next (LSB -> MSB).
    block_bit: u8,
    /// The physical index to be yielded next.
    physical_idx: usize,
}

impl<'a> BlockMapIter<'a> {
    /// Construct a new [`BlockMapIter`] that yields indexes into the physical
    /// bitmap blocks in `bitmap`.
    fn new(bitmap: &'a CompressedBitmap) -> Self {
        Self {
            bitmap,
            block_idx: 0,
            block_bit: 0,
            physical_idx: 0,
        }
    }
}

impl<'a> Iterator for BlockMapIter<'a> {
    type Item = Option<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.bitmap.block_map.get(self.block_idx)?;

        let v = if (block & (1 << self.block_bit)) > 0 {
            // This logical block is non-empty.

            // Read the physical index for the nth logical block.
            let idx = self.physical_idx;

            // Increment for the next physical block.
            self.physical_idx += 1;

            Some(idx)
        } else {
            // This logical block is empty.
            None
        };

        // Advance the bit within the block to evaluate next.
        self.block_bit += 1;

        // Advance the block index (and wrap the bit index) if the last
        // inspected bit was the last bit in the block.
        if self.block_bit == usize::BITS as u8 {
            self.block_bit = 0;
            self.block_idx += 1;
        }

        Some(v)
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

    fn or(&self, other: &Self) -> Self {
        self.or(other)
    }

    fn new_with_capacity(max_key: usize) -> Self {
        Self::new(max_key)
    }
}

impl From<VecBitmap> for CompressedBitmap {
    fn from(bitmap: VecBitmap) -> Self {
        let (bitmap, max_key) = bitmap.into_parts();

        // Calculate how many instances of usize (blocks) are needed to hold
        // max_key number of bits.
        let num_blocks = index_for_key(max_key);

        // Figure out how many usize elements are needed to represent blocks
        // number of bitmaps.
        let num_blocks = match num_blocks % (u64::BITS as usize) {
            0 => index_for_key(num_blocks),
            _ => index_for_key(num_blocks) + 1, // +1 to cover the remainder
        };

        // Then shrink the bitmap into a 2-level compressed bitmap, dropping runs of
        // 0 bits in the raw bitmap.
        let mut block_map = vec![0; num_blocks];
        let mut compressed = Vec::default();
        for (idx, block) in bitmap.into_iter().enumerate() {
            // If this block contains no set bits, it is elided from the compressed
            // representation.
            if block == 0 {
                continue;
            }

            // This block contains data.
            //
            // Add the block to the compressed representation and mark it in the
            // block map.
            compressed.push(block);
            block_map[index_for_key(idx)] |= bitmask_for_key(idx);
        }

        CompressedBitmap {
            block_map,
            bitmap: compressed,

            #[cfg(debug_assertions)]
            max_key,
        }
    }
}

// TODO(dom:test): proptest conversion

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use quickcheck_macros::quickcheck;

    use super::*;

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

    #[test]
    fn test_block_map_iter() {
        let mut bitmap = CompressedBitmap::new(i16::MAX as _);
        bitmap.set(1, true); // Block 0
        bitmap.set(usize::BITS as usize * 4, true); // Block 4
        bitmap.set(usize::BITS as usize * 64, true); // Block 64
        bitmap.set(usize::BITS as usize * 65, true); // Block 65
        bitmap.set(usize::BITS as usize * 128, true); // Block 128

        let mut iter = BlockMapIter::new(&bitmap).enumerate();

        assert_eq!(iter.next().unwrap(), (0, Some(0))); // The 0th block is non-empty and at physical index 0.
        assert_eq!(iter.next().unwrap(), (1, None)); // The 1st block is all zero and elided.
        assert_eq!(iter.next().unwrap(), (2, None)); // The 2nd block is all zero and elided.
        assert_eq!(iter.next().unwrap(), (3, None)); // The 3rd block is all zero and elided.
        assert_eq!(iter.next().unwrap(), (4, Some(1))); // The 4rd block is non-empty and at physical index 1.

        // Filter out all the None entries, preserving the enumerated idx.
        //
        // This causes the iterator to yield (logical block, physical block).
        let mut iter = iter.filter_map(|(idx, block)| block.map(|v| (idx, v)));

        // Then the next non-empty blocks and their physical indexes:
        assert_eq!(iter.next().unwrap(), (64, 2)); // The 64th block is non-empty and at physical index 2.
        assert_eq!(iter.next().unwrap(), (65, 3)); // The 65th block is non-empty and at physical index 3.

        // Finally the last bit!
        assert_eq!(iter.next().unwrap(), (128, 4)); // The 128th block is non-empty and at physical index 4.

        // And the iterator should terminate.
        assert!(iter.next().is_none());
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

    #[quickcheck]
    fn test_or(mut a: Vec<u16>, mut b: Vec<u16>) {
        a.truncate(10);
        let mut bitmap_a = CompressedBitmap::new(u16::MAX.into());
        for v in &a {
            bitmap_a.set(*v as usize, true);
        }

        b.truncate(10);
        let mut bitmap_b = CompressedBitmap::new(u16::MAX.into());
        for v in &b {
            bitmap_b.set(*v as usize, true);
        }

        let merged = bitmap_a.or(&bitmap_b);

        for i in 0..u16::MAX {
            let want_hit = a.contains(&i) || b.contains(&i);
            assert!(
                merged.get(i as usize) == want_hit,
                "unexpected value {} want={:?}",
                i,
                want_hit
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

    const MAX_KEY: usize = 1028;

    proptest! {
        #[test]
        fn prop_compress(
            values in prop::collection::hash_set(0..MAX_KEY, 0..20),
        ) {
            let mut b = VecBitmap::new_with_capacity(MAX_KEY);

            for v in &values {
                b.set(*v, true);
            }

            // Compress
            let b = CompressedBitmap::from(b);

            // Ensure all values are equal in the test range.
            for i in 0..MAX_KEY {
                assert_eq!(b.get(i), values.contains(&i));
            }
        }
    }
}
