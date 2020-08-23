//! bloom2 implements a 2-level bitmap to provide a sparse, lazily initialised,
//! high performance bloom filter with a reduced memory footprint.
//!
//! The memory usage of a sparse bloom filter grows proportionally with the load
//! factor of the filter, resulting in substantially smaller memory footprints
//! for filters with average, or low load factors. As bloom filters are
//! typically sized to avoid high load factors in order to minimise false
//! positives, this is highly effective for the typical use case.
//!
//! The [`Bloom2`] filter provides amortised `O(1)` insert, and constant time
//! `O(1)` lookup with a similar average case latency compared to a standard
//! bloom filter (~30ns on a Core i7 @ 2.60GHz, with a majority of this taken up
//! by the hashing of values).
//!
//! The sparse memory behaviour is implemented in the underlying
//! [`CompressedBitmap`], which is used as the bit storage for the filter. The
//! `CompressedBitmap` is a fast (~4ns set, 1ns get) space efficient
//! general-purpose bitmap suitable for use in applications in addition to the
//! bloom filter.
//!
//! ## Features
//!
//! * `serde` - enable serialisation with [serde], disabled by default
//!
//! [serde]: https://github.com/serde-rs/serde
//! [`Bloom2`]: crate::Bloom2
//! [`CompressedBitmap`]: crate::bitmap::CompressedBitmap

mod bitmap;
pub use bitmap::*;

mod bloom;
pub use bloom::*;

mod filter_size;
pub use filter_size::*;
