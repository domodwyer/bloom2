//! Bitmap implementations for the backing storage of a [`Bloom2`](crate::Bloom2).

mod compressed_bitmap;
pub use compressed_bitmap::*;
