use crate::bitmap::{bitmask_for_key, index_for_key};
use crate::Bitmap;
use bytes::{BufMut, Bytes, BytesMut};
use std::convert::TryInto;

/// A plain, heap-allocated, `O(1)` indexed bitmap using `bytes::BytesMut` for storage.
///
/// This bitmap requires `O(n)` space and can be read and wrote to in `O(1)` time.
///
/// This type is fast for both reads and writes, trading internal complexity for speed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BytesBitmap {
    max_key: usize,
    bitmap: BytesMut,
}

impl BytesBitmap {
    pub(crate) fn into_parts(self) -> (Vec<usize>, usize) {
        let mut vec = Vec::with_capacity(self.bitmap.len() / size_of::<usize>());
        let chunks = self.bitmap.chunks_exact(size_of::<usize>());
        for chunk in chunks {
            vec.push(usize::from_ne_bytes(chunk.try_into().unwrap()));
        }
        (vec, self.max_key)
    }

    pub(crate) fn shrink(&mut self) {
        self.bitmap.resize(self.bitmap.len(), 0);
    }

    pub fn bitmap(self) -> Bytes {
        self.bitmap.freeze()
    }

    pub fn max_key(&self) -> usize {
        self.max_key
    }

    pub fn from_bytes(bitmap: Bytes, max_key: usize) -> Self {
        Self {
            max_key,
            bitmap: BytesMut::from(bitmap),
        }
    }
}

impl Bitmap for BytesBitmap {
    fn new_with_capacity(max_key: usize) -> Self {
        let size = (index_for_key(max_key) + 1) * size_of::<usize>();
        let mut bytes = BytesMut::zeroed(size);

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

// impl serde::Serialize for BytesBitmap {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         use serde::ser::SerializeStruct;
//
//         // Convert BytesMut to Vec<usize>
//         let mut vec = Vec::with_capacity(self.bitmap.len() / size_of::<usize>());
//         let chunks = self.bitmap.chunks_exact(size_of::<usize>());
//
//         for chunk in chunks {
//             let value = usize::from_ne_bytes(chunk.try_into().unwrap());
//             if value != 0 {  // Only include non-zero values
//                 vec.push(value);
//             }
//         }
//
//         let mut state = serializer.serialize_struct("BytesBitmap", 2)?;
//         state.serialize_field("max_key", &self.max_key)?;
//         state.serialize_field("bitmap", &vec)?;  // Serialize the Vec instead
//         state.end()
//     }
// }
//
// impl<'de> serde::Deserialize<'de> for BytesBitmap {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         use serde::de::{self, MapAccess, Visitor};
//         use std::fmt;
//
//         struct BytesBitmapVisitor;
//
//         impl<'de> Visitor<'de> for BytesBitmapVisitor {
//             type Value = BytesBitmap;
//
//             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//                 formatter.write_str("struct BytesBitmap")
//             }
//
//             fn visit_map<V>(self, mut map: V) -> Result<BytesBitmap, V::Error>
//             where
//                 V: MapAccess<'de>,
//             {
//                 let mut max_key = None;
//                 let mut bitmap_vec = None;
//
//                 while let Some(key) = map.next_key::<&str>()? {
//                     match key {
//                         "max_key" => {
//                             max_key = Some(map.next_value()?);
//                         }
//                         "bitmap" => {
//                             bitmap_vec = Some(map.next_value::<Vec<usize>>()?);
//                         }
//                         _ => {
//                             return Err(de::Error::unknown_field(key, &["max_key", "bitmap"]));
//                         }
//                     }
//                 }
//
//                 let max_key = max_key.ok_or_else(|| de::Error::missing_field("max_key"))?;
//                 let bitmap_vec = bitmap_vec.ok_or_else(|| de::Error::missing_field("bitmap"))?;
//
//                 // Calculate required size based on max_key
//                 let size = (index_for_key(max_key) + 1) * size_of::<usize>();
//                 let mut bitmap = BytesMut::with_capacity(size);
//                 bitmap.resize(size, 0);
//
//                 // Reconstruct the bitmap
//                 for (idx, &value) in bitmap_vec.iter().enumerate() {
//                     let byte_offset = idx * size_of::<usize>();
//                     bitmap[byte_offset..byte_offset + size_of::<usize>()]
//                         .copy_from_slice(&value.to_ne_bytes());
//                 }
//
//                 Ok(BytesBitmap { max_key, bitmap })
//             }
//         }
//
//         const FIELDS: &[&str] = &["max_key", "bitmap"];
//         deserializer.deserialize_struct("BytesBitmap", FIELDS, BytesBitmapVisitor)
//     }
// }

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
