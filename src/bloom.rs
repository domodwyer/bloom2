use crate::{bitmap::CompressedBitmap, FilterSize};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::marker::PhantomData;

// TODO(dom): AND, OR, XOR, NOT + examples

// [`Bloom2`]: crate::bloom2::Bloom2
// [`BloomFilterBuilder`]: crate::BloomFilterBuilder
// [`hash`]: std::hash::Hash
// [`FilterSize`]: crate::FilterSize

/// A trait to abstract bit storage for use in a [`Bloom2`](crate::Bloom2)
/// filter.
pub trait Bitmap {
    /// Set bit indexed by `key` to `value`.
    fn set(&mut self, key: usize, value: bool);

    /// Return `true` if the given bit index was previously set to `true`.
    fn get(&self, key: usize) -> bool;

    /// Return the size of the bitmap in bytes.
    fn byte_size(&self) -> usize;
}

/// Construct [`Bloom2`] instances with varying parameters.
///
/// ```rust
/// use std::collections::hash_map::RandomState;
/// use bloom2::{BloomFilterBuilder, FilterSize};
///
/// let mut filter = BloomFilterBuilder::default()
///                     .hasher(RandomState::default())
///                     .size(FilterSize::KeyBytes2)
///                     .build();
///
/// filter.insert(&"success!");
/// ```
pub struct BloomFilterBuilder<H, B>
where
    H: BuildHasher,
    B: Bitmap,
{
    hasher: H,
    bitmap: B,
    key_size: FilterSize,
}

/// Initialise a `BloomFilterBuilder` that unless changed, will construct a
/// `Bloom2` instance using a [2 byte key] and use Rust's [`DefaultHasher`]
/// ([SipHash] at the time of writing).
///
/// [2 byte key]: crate::FilterSize::KeyBytes2
/// [`DefaultHasher`]: std::collections::hash_map::RandomState
/// [SipHash]: https://131002.net/siphash/
impl std::default::Default for BloomFilterBuilder<RandomState, CompressedBitmap> {
    fn default() -> BloomFilterBuilder<RandomState, CompressedBitmap> {
        let size = FilterSize::KeyBytes2;
        BloomFilterBuilder {
            hasher: RandomState::default(),
            bitmap: CompressedBitmap::new(key_size_to_bits(size)),
            key_size: size,
        }
    }
}

impl<H, B> BloomFilterBuilder<H, B>
where
    H: BuildHasher,
    B: Bitmap,
{
    /// Set the hash algorithm.
    pub fn hasher(self, hasher: H) -> Self {
        Self { hasher, ..self }
    }

    /// Set the bit storage (bitmap) for the bloom filter.
    ///
    /// # Safety
    ///
    /// This method is `unsafe` as it is assumed `bitmap` is of a sufficient
    /// size to hold any value in the range produced by the [key
    /// size](FilterSize).
    ///
    /// Providing a `bitmap` instance that is non-empty can be used to restore
    /// the state of a [`Bloom2`] instance (although using `serde` can achieve
    /// this safely too).
    pub unsafe fn bitmap(self, bitmap: B, key_size: FilterSize) -> Self {
        Self {
            bitmap,
            key_size,
            ..self
        }
    }

    /// Initialise the [`Bloom2`] instance with the provided parameters.
    pub fn build<T: Hash>(self) -> Bloom2<H, B, T> {
        Bloom2 {
            hasher: self.hasher,
            bitmap: self.bitmap,
            key_size: self.key_size,
            _key_type: PhantomData,
        }
    }
}

impl<H> BloomFilterBuilder<H, CompressedBitmap>
where
    H: BuildHasher,
{
    /// Control the in-memory size and false-positive probability of the filter.
    ///
    /// Setting the bitmap size replaces the current `Bitmap` instance with a
    /// new `CompressedBitmap` of the appropriate size.
    ///
    /// See [`FilterSize`].
    pub fn size(self, size: FilterSize) -> Self {
        Self {
            key_size: size,
            bitmap: CompressedBitmap::new(key_size_to_bits(size)),
            ..self
        }
    }
}

fn key_size_to_bits(k: FilterSize) -> usize {
    (2 as usize).pow(8 * k as u32)
}

/// A fast, memory efficient, sparse bloom filter.
///
/// Most users can quickly initialise a `Bloom2` instance by calling
/// `Bloom2::default()` and start inserting anything that implements the
/// [`Hash`] trait:
///
/// ```rust
/// use bloom2::Bloom2;
///
/// let mut b = Bloom2::default();
/// b.insert(&"hello 🐐");
/// assert!(b.contains(&"hello 🐐"));
/// ```
///
/// Initialising a `Bloom2` this way uses some [sensible
/// default](crate::BloomFilterBuilder) values for a easy-to-use, memory
/// efficient filter. If you want to tune the probability of a false-positive
/// lookup, change the hashing algorithm, memory size of the filter, etc, a
/// [`BloomFilterBuilder`] can be used to initialise a `Bloom2` instance with
/// the desired properties.
///
/// The sparse nature of this filter trades a small amount of insert performance
/// for decreased memory usage. For filters initialised infrequently and held
/// for a meaningful duration of time, this is almost always worth the
/// marginally increased insert latency. When testing performance, be sure to
/// use a release build - there's a significant performance difference!
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bloom2<H, B, T>
where
    H: BuildHasher,
    B: Bitmap,
{
    hasher: H,
    bitmap: B,
    key_size: FilterSize,
    _key_type: PhantomData<T>,
}

/// Initialise a `Bloom2` instance using the default implementation of
/// [`BloomFilterBuilder`].
///
/// It is the equivalent of:
///
/// ```rust
/// use bloom2::BloomFilterBuilder;
///
/// let mut b = BloomFilterBuilder::default().build();
/// # b.insert(&42);
/// ```
impl<T> std::default::Default for Bloom2<RandomState, CompressedBitmap, T>
where
    T: Hash,
{
    fn default() -> Self {
        crate::BloomFilterBuilder::default().build()
    }
}

impl<H, B, T> Bloom2<H, B, T>
where
    H: BuildHasher,
    B: Bitmap,
    T: Hash,
{
    /// Insert places `data` into the bloom filter.
    ///
    /// Any subsequent calls to [`contains`](Bloom2::contains) for the same
    /// `data` will always return true.
    ///
    /// Insertion is significantly faster in release builds.
    ///
    /// The `data` provided can be anything that implements the [`Hash`] trait,
    /// for example:
    ///
    /// ```rust
    /// use bloom2::Bloom2;
    ///
    /// let mut b = Bloom2::default();
    /// b.insert(&"hello 🐐");
    /// assert!(b.contains(&"hello 🐐"));
    ///
    /// let mut b = Bloom2::default();
    /// b.insert(&vec!["fox", "cat", "banana"]);
    /// assert!(b.contains(&vec!["fox", "cat", "banana"]));
    ///
    /// let mut b = Bloom2::default();
    /// let data: [u8; 4] = [1, 2, 3, 42];
    /// b.insert(&data);
    /// assert!(b.contains(&data));
    /// ```
    ///
    /// As well as structs if they implement the [`Hash`] trait, which be
    /// helpfully derived:
    ///
    /// ```rust
    /// # use bloom2::Bloom2;
    /// # let mut b = Bloom2::default();
    /// #[derive(Hash)]
    /// struct User {
    ///     id: u64,
    ///     email: String,
    /// }
    ///
    /// let user = User{
    ///     id: 42,
    ///     email: "dom@itsallbroken.com".to_string(),
    /// };
    ///
    /// b.insert(&&user);
    /// assert!(b.contains(&&user));
    /// ```
    pub fn insert(&mut self, data: &'_ T) {
        // Generate a hash (u64) value for data
        let mut hasher = self.hasher.build_hasher();
        data.hash(&mut hasher);

        // Split the u64 hash into several smaller values to use as unique
        // indexes in the bitmap.
        hasher
            .finish()
            .to_be_bytes()
            .chunks(self.key_size as usize)
            .for_each(|chunk| self.bitmap.set(bytes_to_usize_key(chunk), true));
    }

    /// Checks if `data` exists in the filter.
    ///
    /// If `contains` returns true, `hash` has **probably** been inserted
    /// previously. If `contains` returns false, `hash` has **definitely not**
    /// been inserted into the filter.
    pub fn contains(&mut self, data: &'_ T) -> bool {
        // Generate a hash (u64) value for data
        let mut hasher = self.hasher.build_hasher();
        data.hash(&mut hasher);

        hasher
            .finish()
            .to_be_bytes()
            .chunks(self.key_size as usize)
            .any(|chunk| self.bitmap.get(bytes_to_usize_key(chunk)))
    }

    /// Return the byte size of this filter.
    pub fn byte_size(&mut self) -> usize {
        self.bitmap.byte_size()
    }
}

impl<H, T> Bloom2<H, CompressedBitmap, T>
where
    H: BuildHasher,
{
    /// Minimise the memory usage of this instance by by shrinking the
    /// underlying vectors, discarding their excess capacity.
    pub fn shrink_to_fit(&mut self) {
        self.bitmap.shrink_to_fit();
    }
}
fn bytes_to_usize_key<'a, I: IntoIterator<Item = &'a u8>>(bytes: I) -> usize {
    bytes
        .into_iter()
        .fold(0, |key, &byte| (key << 8) | byte as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;
    use std::cell::RefCell;

    #[derive(Debug, Clone, Default)]
    struct MockHasher {
        return_hash: u64,
    }

    impl Hasher for MockHasher {
        fn write(&mut self, _bytes: &[u8]) {}
        fn finish(&self) -> u64 {
            self.return_hash
        }
    }

    impl BuildHasher for MockHasher {
        type Hasher = Self;
        fn build_hasher(&self) -> MockHasher {
            self.clone()
        }
    }

    #[derive(Debug, Default)]
    struct MockBitmap {
        set_calls: Vec<(usize, bool)>,
        get_calls: RefCell<Vec<usize>>,
    }
    impl Bitmap for MockBitmap {
        fn set(&mut self, key: usize, value: bool) {
            self.set_calls.push((key, value))
        }
        fn get(&self, key: usize) -> bool {
            self.get_calls.borrow_mut().push(key);
            false
        }
        fn byte_size(&self) -> usize {
            42
        }
    }

    fn new_test_bloom<T: Hash>() -> Bloom2<MockHasher, MockBitmap, T> {
        Bloom2 {
            hasher: MockHasher::default(),
            bitmap: MockBitmap::default(),
            key_size: FilterSize::KeyBytes1,
            _key_type: PhantomData,
        }
    }

    #[test]
    fn test_default() {
        let mut b = Bloom2::default();
        assert_eq!(b.key_size, FilterSize::KeyBytes2);

        b.insert(&42);
        assert!(b.contains(&42));
    }

    #[quickcheck]
    fn test_default_prop(vals: Vec<u16>) {
        let mut b = Bloom2::default();
        for v in &vals {
            b.insert(&*v);
        }

        for v in &vals {
            assert!(b.contains(&*v));
        }
    }

    #[test]
    fn test_insert_contains_kb1() {
        let mut b = new_test_bloom();
        b.hasher.return_hash = 12345678901234567890;

        b.insert(&[1, 2, 3, 4]);
        assert_eq!(
            b.bitmap.set_calls,
            vec![
                (171, true),
                (84, true),
                (169, true),
                (140, true),
                (235, true),
                (31, true),
                (10, true),
                (210, true),
            ]
        );

        b.contains(&[1, 2, 3, 4]);
        assert_eq!(
            b.bitmap.get_calls.into_inner(),
            vec![171, 84, 169, 140, 235, 31, 10, 210]
        );
    }

    #[test]
    fn test_insert_contains_kb2() {
        let mut b = new_test_bloom();
        b.key_size = FilterSize::KeyBytes2;
        b.hasher.return_hash = 12345678901234567890;

        b.insert(&[1, 2, 3, 4]);

        assert_eq!(
            b.bitmap.set_calls,
            vec![(43860, true), (43404, true), (60191, true), (2770, true),]
        );
        assert!(b.bitmap.get_calls.into_inner().is_empty());
    }

    #[test]
    fn test_issue_3() {
        let mut bloom_filter: Bloom2<RandomState, CompressedBitmap, &str> =
            BloomFilterBuilder::default()
                .size(FilterSize::KeyBytes4)
                .build();

        bloom_filter.insert(&"a");
        bloom_filter.insert(&"b");
        bloom_filter.insert(&"c");
        bloom_filter.insert(&"d");
    }

    #[test]
    fn test_size_shrink() {
        let mut bloom_filter: Bloom2<RandomState, CompressedBitmap, _> =
            BloomFilterBuilder::default()
                .size(FilterSize::KeyBytes4)
                .build();

        for i in 0..10 {
            bloom_filter.insert(&i);
        }

        assert_eq!(bloom_filter.byte_size(), 8388920);
        bloom_filter.shrink_to_fit();
        assert_eq!(bloom_filter.byte_size(), 8388824);
    }
}
