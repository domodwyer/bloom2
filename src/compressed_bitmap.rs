use std::mem;

/// FilterSize bounds the allocated size of a CompressedBitmap.
///
/// The false positive probability for a bloom filter increases as the number of
/// entries increases. This relationship is demonstrated using sha256 hashes as
/// keys for each possible filter configuration below - you should choose a
/// filter size for your expected load level and hash size.
///
/// The value of FilterSize controls the `k` property of the filter: `k =
/// input_length_bytes / FilterSize`.
#[derive(Clone, Copy, Debug)]
pub enum FilterSize {
    /// 1 byte / 8 bits per key results in a bloom filter with a minimum memory
    /// usage of ~4 bytes and a maximum memory usage of 36 bytes.
    ///
    /// The false positive probability using `k=1` (a single byte key per entry)
    /// grows proportionally to the number of entries in the filter:
    ///
    /// ```text
    ///           +--+------------+------------+------------+-----------+------------+-----+
    ///         1 +                                                *                   *   +
    ///           |                                                                        |
    ///           |                                   *                                    |
    ///     P 0.8 +                         *                                              +
    ///     r     |                                                                        |
    ///     o     |                   *                                                    |
    ///     b 0.6 +                                                                        +
    ///     a     |              *                                                         |
    ///     b     |                                                                        |
    ///     i 0.4 +          *                                                             +
    ///     l     |                                                                        |
    ///     i     |        *                                                               |
    ///     t 0.2 +      *                                                                 +
    ///     y     |    **                                                                  |
    ///           |   **                                                                   |
    ///         0 +  **                                                                    +
    ///           +--+------------+------------+------------+-----------+------------+-----+
    ///              0           200          400          600         800         1000     
    ///                                       Number of Entries    
    ///
    ///             The probability of false positives reaches 1-in-2 after 178 entries.
    /// ```
    ///
    KeyBytes1 = 1,

    /// 2 bytes / 16 bits per key results in a bloom filter with a minimum memory
    /// usage of ~1024 bytes and a maximum memory usage of ~8KB when fully
    /// populated.
    ///
    /// When using a sha256 hash (256 bits, or 16x2 byte keys, `k=16`) the
    /// probability of a false positive is:
    ///
    /// ```text
    ///           +--+-------------------+-------------------+------------------+----------+
    ///         1 +                                                                   *    +
    ///           |                                                *                       |
    ///           |                                                                        |
    ///     P 0.8 +                                                                        +
    ///     r     |                                  *                                     |
    ///     o     |                                                                        |
    ///     b 0.6 +                                                                        +
    ///     a     |                                                                        |
    ///     b     |                                                                        |
    ///     i 0.4 +                         *                                              +
    ///     l     |                                                                        |
    ///     i     |                                                                        |
    ///     t 0.2 +                                                                        +
    ///     y     |                                                                        |
    ///           |                  *                                                     |
    ///         0 +  ***** * *   *                                                         +
    ///           +--+-------------------+-------------------+------------------+----------+
    ///              0                 10000               20000              30000         
    ///                                       Number of Entries                             
    ///
    ///            The probability of false positives reaches 1-in-2 after 12,947 entries.
    /// ```
    ///
    KeyBytes2 = 2,

    /// 3 bytes / 24 bits per key results in a bloom filter with a minimum memory
    /// usage of ~262KB bytes and a maximum memory usage of ~2MB when fully
    /// populated.
    ///
    /// When using a sha256 hash (256 bits, or ~11x3 byte keys, `k=~11`) the
    /// probability of a false positive is:
    ///
    /// ```text
    ///         1 +--+---------------+--------------+--------------+---------------+-------+
    ///           |                                                                   *    |
    ///           |                                                                        |
    ///       0.8 +                                                *                       +
    ///     P     |                                                                        |
    ///     r     |                                                                        |
    ///     o     |                                                                        |
    ///     b 0.6 +                                                                        +
    ///     a     |                                  *                                     |
    ///     b     |                                                                        |
    ///     i 0.4 +                                                                        +
    ///     l     |                                                                        |
    ///     i     |                                                                        |
    ///     t 0.2 +                         *                                              +
    ///     y     |                                                                        |
    ///           |                                                                        |
    ///         0 +  ***** * *   *   *                                                     +
    ///           +--+---------------+--------------+--------------+---------------+-------+
    ///              0             2e+06          4e+06          6e+06           8e+06      
    ///                                       Number of Entries                             
    ///
    ///           The probability of false positives reaches 1-in-2 after 4,264,082 entries.
    /// ```
    ///
    KeyBytes3 = 3,

    /// 4 bytes / 32 bits per key results in a bloom filter with a minimum memory
    /// usage of ~67MB and a maximum memory usage of ~603MB when fully
    /// populated.
    ///
    /// When using a sha256 hash (256 bits, or 8x3 byte keys, `k=8`) the
    /// probability of a false positive is:
    ///
    /// ```text
    ///         1 +--+----------+---------+----------+----------+---------+----------+-----+
    ///           |                                                                   *    |
    ///           |                                                                        |
    ///           |                                                *                       |
    ///     P 0.8 +                                                                        +
    ///     r     |                                                                        |
    ///     o     |                                  *                                     |
    ///     b 0.6 +                                                                        +
    ///     a     |                                                                        |
    ///     b     |                                                                        |
    ///     i 0.4 +                                                                        +
    ///     l     |                         *                                              |
    ///     i     |                                                                        |
    ///     t 0.2 +                                                                        +
    ///     y     |                                                                        |
    ///           |                  *                                                     |
    ///         0 +  ***** * *   *                                                         +
    ///           +--+----------+---------+----------+----------+---------+----------+-----+
    ///              0        5e+08     1e+09     1.5e+09     2e+09    2.5e+09     3e+09    
    ///                                       Number of Entries                             
    ///
    ///         The probability of false positives reaches 1-in-2 after 1,336,252,043 entries.
    /// ```
    ///
    KeyBytes4 = 4,

    /// 5 bytes / 32 bits per key results in a bloom filter with a minimum memory
    /// usage of ~17GB and a maximum memory usage of ~1117GB when fully
    /// populated.
    ///
    /// If you actually need this get in touch - I have some ideas for reducing
    /// the memory footprint even further.
    ///
    /// When using a sha256 hash (256 bits, or ~7x3 byte keys, `k=~7`) the
    /// probability of a false positive is:
    ///
    /// ```text
    ///         1 +--+----------------+---------------+----------------+---------------+---+
    ///           |                                                                   *    |
    ///           |                                                                        |
    ///       0.8 +                                                *                       +
    ///     P     |                                                                        |
    ///     r     |                                                                        |
    ///     o     |                                                                        |
    ///     b 0.6 +                                  *                                     +
    ///     a     |                                                                        |
    ///     b     |                                                                        |
    ///     i 0.4 +                                                                        +
    ///     l     |                                                                        |
    ///     i     |                         *                                              |
    ///     t 0.2 +                                                                        +
    ///     y     |                                                                        |
    ///           |                  *                                                     |
    ///         0 +  ***** * *   *                                                         +
    ///           +--+----------------+---------------+----------------+---------------+---+
    ///              0              2e+11           4e+11            6e+11           8e+11  
    ///                                       Number of Entries                             
    ///
    ///        The probability of false positives reaches 1-in-2 after 370,932,038,704 entries.
    /// ```
    ///
    KeyBytes5 = 5,
}

/// CompressedBitmap implements a sparse, 2 level bloom filter - a space
/// efficient, probabilistic set.
///
/// Users of a CompressedBitmap call
/// [`insert_hash`](CompressedBitmap::insert_hash) with deterministic, unique
/// hashes (a fingerprint) of their entries and check the existence of the entry
/// by calling [`contains_hash`](CompressedBitmap::contains_hash).
///
/// ```
/// use bloom2::{CompressedBitmap, FilterSize};
///
/// let mut filter = CompressedBitmap::new(FilterSize::KeyBytes2);
///
/// let data_hashes = vec![
///     "bananas",
///     "batman",
///     "bintang",
/// ];
///
/// for v in data_hashes.iter() {
///     filter.insert_hash(v);
/// }
///
/// assert_eq!(filter.contains_hash("bananas"), true);
/// assert_eq!(filter.contains_hash("apples"), false);
/// ```
///
/// The CompressedBitmap maintains the same false-positive properties and
/// similar performance properties as a normal bloom filter while lazily
/// initialising the backing memory as it is needed, resulting in smaller memory
/// footprints for all except completely loaded filters.
///
/// Insertions are amortised `O(1)` and lookups are always `O(1)`. The backing
/// memory is lazily initialised by growing a [`std::vec::Vec`], therefore it
/// uses the same (undefined) allocation strategy to amortise the expansion of
/// the backing memory - call [`shrink_to_fit`](CompressedBitmap::shrink_to_fit)
/// to reduce the underlying memory allocation to the minimum required.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompressedBitmap {
    key_byte_size: usize,
    block_map: Vec<usize>,
    bitmap: Vec<usize>,
}

impl CompressedBitmap {
    /// Initialises a new, empty bloom filter configured to consume hashes in
    /// chunks of `key_byte_size` number of bytes to use as keys.
    pub fn new(key_byte_size: FilterSize) -> Self {
        // Calculate the capacity of the bitvec.
        //
        // This is the size of a u32 (8) to the power of the size of the keys in
        // bytes used to index it. This results in:
        //
        // 		| Key Bytes | Max Index  | Filter Size        |
        // 		|-----------|------------|--------------------|
        // 		| 1         | 256        | 32                 |
        // 		| 2         | 65536      | 8192 (~8KB)        |
        // 		| 3         | 16777216   | 2097152 (~2MB)     |
        // 		| 4         | 4294967296 | 536870912 (~536MB) |
        // 		| 5         | 1.0995e+12 | ~1100GB            |
        //
        let max_index = (2 as usize).pow(8 * key_byte_size as u32);

        // Calculate how many instances of usize (blocks) are needed to hold
        // max_index number of bits.
        let blocks = index_for_key(max_index);

        // Allocate a block map.
        //
        // The block map contains bitmaps with 1 bits indicating the usize for
        // that key has been allocated.
        let mut block_map = Vec::new();
        block_map.resize(index_for_key(blocks), 0);

        if blocks % (mem::size_of::<usize>() * 8) != 0 {
            block_map.push(0);
        }

        CompressedBitmap {
            key_byte_size: key_byte_size as usize,
            bitmap: Vec::new(),
            block_map,
        }
    }

    /// Reduces the allocated memory usage of the filter to the minimum required
    /// for the current filter contents.
    ///
    /// This is useful to minimise the memory footprint of a populated,
    /// read-only CompressedBitmap.
    ///
    /// See [`Vec::shrink_to_fit`](std::vec::Vec::shrink_to_fit).
    pub fn shrink_to_fit(&mut self) {
        self.bitmap.shrink_to_fit()
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

    /// Inserts hash into the filter, chunking it into the configured key size.
    ///
    /// Calling `insert_hash` with a hash length greater than the configured key
    /// size effectively increases the "hash" count, or `k` property of the
    /// filter.
    pub fn insert_hash<T: AsRef<[u8]>>(&mut self, hash: T) {
        for chunk in hash.as_ref().chunks(self.key_byte_size) {
            let mut key = 0;
            for b in chunk.iter() {
                key <<= 8;
                key |= *b as usize;
            }

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

            // Count the ones in the full blocks
            let mut offset: usize = 0;
            for i in 0..block_map_index {
                offset += self.block_map[i].count_ones() as usize;
            }

            // Mask out the higher bits in the block map to count the populated
            // blocks before block_index
            let mask = block_map_bitmask - 1;
            offset += (self.block_map[block_map_index] & mask).count_ones() as usize;

            // Offset now contains the index in bitmap at which block_index can
            // be found.
            //
            // Because the blocks are lazily initialised, there may not yet be a
            // block for block_index.
            //
            // Read the usize at block_map_index, and check the bit for
            // block_index.
            if self.block_map[block_map_index] & block_map_bitmask == 0 {
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
                continue;
            }

            // Otherwise the block map indicates the block is already allocated
            self.bitmap[offset as usize] |= bitmask_for_key(key);
        }
    }

    /// Checks if hash exists in the filter.
    ///
    /// If `contains_hash` returns true, `hash` has **probably** been inserted
    /// previously. If `contains_hash` returns false, `hash` has **definitely
    /// not been inserted** into the filter.
    pub fn contains_hash<T: AsRef<[u8]>>(&self, hash: T) -> bool {
        for chunk in hash.as_ref().chunks(self.key_byte_size) {
            let mut key = 0;
            for b in chunk.iter() {
                key <<= 8;
                key |= *b as usize;
            }

            let block_index = index_for_key(key);
            let block_map_index = index_for_key(block_index);
            let block_map_bitmask = bitmask_for_key(block_index);

            if self.block_map[block_map_index] & block_map_bitmask == 0 {
                return false;
            }

            let mut offset = 0;
            for i in 0..block_map_index {
                offset += self.block_map[i].count_ones();
            }

            let mask = block_map_bitmask - 1;
            offset += (self.block_map[block_map_index] & mask).count_ones();

            if self.bitmap[offset as usize] & bitmask_for_key(key) == 0 {
                return false;
            }
        }

        return true;
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

    fn matches_only(bloom: &CompressedBitmap, hash: [u8; 2]) {
        for i in 0..255 as u8 {
            for j in 0..255 as u8 {
                let lookup = [i, j];
                if (i == hash[0] || i == hash[1]) && (j == hash[0] || j == hash[1]) {
                    continue;
                }
                assert!(
                    !bloom.contains_hash(lookup),
                    "expected contains_hash false, got true for key [{}, {}]",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn test_bananas() {
        let mut filter = CompressedBitmap::new(FilterSize::KeyBytes2);
        filter.insert_hash("bananas");
        assert_eq!(filter.contains_hash("bananas"), true);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn construct_key_1_64bit() {
        let b = CompressedBitmap::new(FilterSize::KeyBytes1);

        // 1 byte key -> 256 values -> 4 x 64bits -> 0.0625 blocks rounded to 1
        assert_eq!(b.block_map.len(), 1);
        assert_eq!(b.bitmap.len(), 0);
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn construct_key_1_32bit() {
        let b = CompressedBitmap::new(FilterSize::KeyBytes1);

        // 1 byte key -> 256 values -> 8 x 32bits -> 0.09375 blocks rounded to 1
        assert_eq!(b.block_map.len(), 1);
        assert_eq!(b.bitmap.len(), 0);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn construct_key_2_64bit() {
        let b = CompressedBitmap::new(FilterSize::KeyBytes2);

        // 2 byte key -> 65,536 values -> 1024 x 64bits -> 16 blocks
        assert_eq!(b.block_map.len(), 16);
        assert_eq!(b.bitmap.len(), 0);
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn construct_key_2_32bit() {
        let b = CompressedBitmap::new(FilterSize::KeyBytes2);

        // 2 byte key -> 65,536 values -> 2048 x 32bits -> 64 blocks
        assert_eq!(b.block_map.len(), 64);
        assert_eq!(b.bitmap.len(), 0);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn construct_key_3_64bit() {
        let b = CompressedBitmap::new(FilterSize::KeyBytes3);

        // 3 byte key -> 16,777,216 values -> 262,144 x 64bits -> 4,096 blocks
        assert_eq!(b.block_map.len(), 4096);
        assert_eq!(b.bitmap.len(), 0);
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn construct_key_2_32bit() {
        let b = CompressedBitmap::new(FilterSize::KeyBytes3);

        // 3 byte key -> 16,777,216 values -> 524,288 x 64bits -> 16,384 blocks
        assert_eq!(b.block_map.len(), 16_384);
        assert_eq!(b.bitmap.len(), 0);
    }

    #[test]
    fn contains_inserted_value() {
        let mut b = CompressedBitmap::new(FilterSize::KeyBytes2);
        let hash = [1, 2];

        b.insert_hash(hash);
        assert!(b.contains_hash(hash));

        matches_only(&b, hash);

        // Must not contain any empty blocks
        for block in b.bitmap {
            assert_ne!(block, 0);
        }
    }

    #[test]
    fn contains_inserted_value_short_key() {
        let mut b = CompressedBitmap::new(FilterSize::KeyBytes2);
        let hash = [42]; // Shorter than the specified key size

        b.insert_hash(hash);
        assert!(b.contains_hash(hash));

        // Must not contain any empty blocks
        for block in b.bitmap {
            assert_ne!(block, 0);
        }
    }

    #[test]
    fn min_max_values() {
        for hash in vec![[0, 0], [255, 255]] {
            let mut b = CompressedBitmap::new(FilterSize::KeyBytes2);
            b.insert_hash(hash);
            assert!(b.contains_hash(hash));
            matches_only(&b, hash);
        }
    }

    #[test]
    fn clear() {
        let mut b = CompressedBitmap::new(FilterSize::KeyBytes2);
        let hash = [42]; // Shorter than the specified key size

        b.insert_hash(hash);
        assert!(b.contains_hash(hash));

        b.clear();
        assert_eq!(b.contains_hash(hash), false);

        // Must not contain any blocks
        for block in b.block_map {
            assert_eq!(block, 0);
        }

        assert_eq!(b.bitmap.len(), 0);
        assert_ne!(b.bitmap.capacity(), 0);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        let mut b = CompressedBitmap::new(FilterSize::KeyBytes2);
        let hash = [1, 2];

        b.insert_hash(hash);
        assert!(b.contains_hash(hash));

        let encoded = serde_json::to_string(&b).unwrap();
        let decoded: CompressedBitmap = serde_json::from_str(&encoded).unwrap();

        assert!(decoded.contains_hash(hash));
    }

    #[quickcheck]
    fn prop_inserted_hash_is_found(mut xs: Vec<u8>) -> bool {
        let mut b = CompressedBitmap::new(FilterSize::KeyBytes1);
        let hash = match xs.pop() {
            Some(v) => v,
            None => return true,
        };

        println!("Using hash {}", hash);

        b.insert_hash(&[hash]);
        if b.contains_hash(&[hash]) == false {
            return false;
        };

        // Must not contain any empty blocks
        for block in &b.bitmap {
            assert_ne!(*block, 0);
        }

        return !xs
            .iter()
            .filter(|v| hash != **v)
            .fold(false, |acc, v| acc || b.contains_hash(&[*v]));
    }
}
