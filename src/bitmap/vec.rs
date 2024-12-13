use crate::Bitmap;

use super::{bitmask_for_key, index_for_key};

/// A plain, heap-allocated, `O(1)` indexed bitmap.
///
/// This bitmap requires `O(n)` space and can be read and wrote to in `O(1)`
/// time.
///
/// This type is fast for both read and writes, but trades additional space for
/// the additional performance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VecBitmap {
    bitmap: Vec<usize>,
    max_key: usize,
}

impl VecBitmap {
    pub(crate) fn into_parts(self) -> (Vec<usize>, usize) {
        (self.bitmap, self.max_key)
    }
}

impl Bitmap for VecBitmap {
    fn set(&mut self, key: usize, value: bool) {
        let offset = index_for_key(key);

        if value {
            self.bitmap[offset] |= bitmask_for_key(key);
        } else {
            self.bitmap[offset] &= !bitmask_for_key(key);
        }
    }

    fn get(&self, key: usize) -> bool {
        let offset = index_for_key(key);

        self.bitmap[offset] & bitmask_for_key(key) != 0
    }

    fn byte_size(&self) -> usize {
        self.bitmap.len() * std::mem::size_of::<usize>()
    }

    fn or(&self, other: &Self) -> Self {
        // Invariant: the block maps are of equal length, meaning the zipped
        // iters yield both sides to completion.
        assert_eq!(self.bitmap.len(), other.bitmap.len());

        let bitmap = self
            .bitmap
            .iter()
            .zip(&other.bitmap)
            .map(|(a, b)| a | b)
            .collect();

        Self {
            bitmap,
            max_key: self.max_key,
        }
    }

    fn new_with_capacity(max_key: usize) -> Self {
        let bitmap = vec![0; index_for_key(max_key) + 1];
        Self { bitmap, max_key }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const MAX_KEY: usize = 1028;

    proptest! {
        #[test]
        fn prop_insert_contains(
            values in prop::collection::hash_set(0..MAX_KEY, 0..20),
        ) {
            let mut b = VecBitmap::new_with_capacity(MAX_KEY);

            for v in &values {
                b.set(*v, true);
            }

            // Ensure all values are equal in the test range.
            for i in 0..MAX_KEY {
                assert_eq!(b.get(i), values.contains(&i));
            }
        }

        #[test]
        fn prop_or(
            a in prop::collection::vec(0..MAX_KEY, 0..20),
            b in prop::collection::vec(0..MAX_KEY, 0..20),
        ) {
            let mut a_bitmap = VecBitmap::new_with_capacity(MAX_KEY);
            let mut b_bitmap = VecBitmap::new_with_capacity(MAX_KEY);
            let mut combined_bitmap = VecBitmap::new_with_capacity(MAX_KEY);

            for v in a.iter() {
                a_bitmap.set(*v, true);
                combined_bitmap.set(*v, true);
            }

            for v in b.iter() {
                b_bitmap.set(*v, true);
                combined_bitmap.set(*v, true);
            }

            let union = a_bitmap.or(&b_bitmap);

            // Invariant: the union and the combined construction must be equal.
            assert_eq!(union, combined_bitmap);

            // Invariant: the key space contains true entries only when the
            // value appears in a or b.
            for i in 0..MAX_KEY {
                assert_eq!(union.get(i), a_bitmap.get(i) || b_bitmap.get(i));

                // Invariant: the key presence matches the combined bitmap.
                assert_eq!(union.get(i), combined_bitmap.get(i));
            }
        }
    }
}
