use crate::bitmap::{bitmask_for_key, index_for_key};
use crate::{Bitmap};
#[cfg(feature = "bytes")]
use bytes::{BufMut, Bytes, BytesMut};
use std::convert::TryInto;

/// A plain, heap-allocated, `O(1)` indexed bitmap using `bytes::BytesMut` for storage.
///
/// This type provide fast O(1) read and write operations, but trades O(n) space complexity for the
/// additional performance.
///
/// The [BytesBitmap] representation is suitable for persistence without the need for serialisation;
/// the output of [BytesBitmap::freeze()] can be used to construct a new instance. [Serde]
/// serialisation is also implemented as a conveinence to enable serialisation to various formats.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(feature = "bytes")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BytesBitmap {
    max_key: usize,
    bitmap: BytesMut,
}

#[cfg(feature = "bytes")]
impl BytesBitmap {
    pub fn freeze(self) -> Bytes {
        self.bitmap.freeze()
    }

    pub fn max_key(&self) -> usize {
        self.max_key
    }

    pub fn from_bytes(bitmap: impl Into<Bytes>) -> Self {
        let bitmap = bitmap.into();
        Self {
            max_key: bitmap.len() * 8,
            bitmap: BytesMut::from(bitmap),
        }
    }
}

#[cfg(feature = "bytes")]
impl Bitmap for BytesBitmap {
    fn new_with_capacity(max_key: usize) -> Self {
        let size = (index_for_key(max_key) + 1) * size_of::<usize>();
        let bytes = BytesMut::zeroed(size);

        Self {
            bitmap: bytes,
            max_key,
        }
    }

    fn set(&mut self, key: usize, value: bool) {
        let offset = index_for_key(key);
        let byte_offset = offset * size_of::<usize>();

        let slice = &mut self.bitmap[byte_offset..byte_offset + size_of::<usize>()];
        let mut num = usize::from_ne_bytes(slice.try_into().unwrap());

        if value {
            num |= bitmask_for_key(key);
        } else {
            num &= !bitmask_for_key(key);
        }

        slice.copy_from_slice(&num.to_ne_bytes());
    }

    fn get(&self, key: usize) -> bool {
        let offset = index_for_key(key);
        let byte_offset = offset * size_of::<usize>();
        let slice = &self.bitmap[byte_offset..byte_offset + size_of::<usize>()];
        let num = usize::from_ne_bytes(slice.try_into().unwrap());
        num & bitmask_for_key(key) != 0
    }

    fn byte_size(&self) -> usize {
        self.bitmap.len()
    }
    
    fn or(&self, other: &Self) -> Self {
        assert_eq!(self.bitmap.len(), other.bitmap.len());

        let mut result = BytesMut::with_capacity(self.bitmap.len());
        let chunks = self
            .bitmap
            .chunks_exact(size_of::<usize>())
            .zip(other.bitmap.chunks_exact(size_of::<usize>()));

        for (a_chunk, b_chunk) in chunks {
            let a = usize::from_ne_bytes(a_chunk.try_into().unwrap());
            let b = usize::from_ne_bytes(b_chunk.try_into().unwrap());
            result.put_slice(&(a | b).to_ne_bytes());
        }

        Self {
            bitmap: result,
            max_key: self.max_key,
        }
    }
}

#[cfg(feature = "bytes")]
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
            let mut b = BytesBitmap::new_with_capacity(MAX_KEY);

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
            let mut a_bitmap = BytesBitmap::new_with_capacity(MAX_KEY);
            let mut b_bitmap = BytesBitmap::new_with_capacity(MAX_KEY);
            let mut combined_bitmap = BytesBitmap::new_with_capacity(MAX_KEY);

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
