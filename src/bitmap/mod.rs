//! Bitmap implementations for the backing storage of a [`Bloom2`](crate::Bloom2).

mod bytes;
mod compressed_bitmap;
mod vec;

pub use bytes::*;
pub use compressed_bitmap::*;
pub use vec::*;

#[inline(always)]
pub(crate) fn bitmask_for_key(key: usize) -> usize {
    1 << (key % (u64::BITS as usize))
}

#[inline(always)]
pub(crate) fn index_for_key(key: usize) -> usize {
    key / (u64::BITS as usize)
}
