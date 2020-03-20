//! bloom2 implements a 2-level bloom filter to provide sparse, lazily
//! initialised, high performance bloom filters with reduced memory footprints.
//!
//! The memory usage of a sparse bloom filter grows proportionally with the load
//! factor of the filter, resulting in substantially smaller memory footprints
//! for filters with average, or low load factors. As bloom filters are
//! typically sized to avoid high load factors in order to minimise false
//! positives, this is highly effective for the typical use case.
//!
//! The [`CompressedBitmap`] filter provides amortised `O(1)` insert, and `O(1)`
//! lookup with similar average case latency compared to a normal bloom filter
//! (~10ns on a Core i7 @ 2.60GHz).
//!
//! ## Features
//!
//! * `serde` - enable serialisation with [serde], disabled by default
//!
//! [serde]: (https://github.com/serde-rs/serde)

mod compressed_bitmap;
pub use compressed_bitmap::*;
