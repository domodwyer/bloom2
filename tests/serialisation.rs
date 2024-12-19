#![cfg(feature = "serde")]

use std::{fmt::Debug, fs, hash::BuildHasherDefault, ops::Range, path::PathBuf};

use bloom2::{Bloom2, BloomFilterBuilder, CompressedBitmap, FilterSize};

/// Fixed value range to insert into the bloom filter.
const VALUES: Range<usize> = Range {
    start: 42,
    end: 100,
};

type StableBuildHasher = BuildHasherDefault<twox_hash::XxHash64>;

/// Generate a test for a specific bitmap storage type that asserts the
/// serialised representation matches some known fixture value.
macro_rules! test_serde_fixture {
    (
		$name:ident, // Test name - the fixture filename is derived from it.
		$bitmap:ty   // The concrete bitmap type to test.
	) => {
        paste::paste! {
            #[test]
            fn [<test_serde_fixture_ $name>]() {
                let mut b: Bloom2<StableBuildHasher, $bitmap, usize> =
                    BloomFilterBuilder::hasher(StableBuildHasher::default())
                        .with_bitmap::<$bitmap>()
                        .size(FilterSize::KeyBytes1)
                        .build();

                for i in VALUES {
                    b.insert(&i);
                }

                assert_fixture(b, stringify!($name));
            }
        }
    };
}

test_serde_fixture!(compressed_bitmap, CompressedBitmap);

/// Serialise `t` as JSON and assert it matches a fixture value stored in a
/// file, and that deserialising the fixture results in the same filter state.
///
/// # Panics
///
/// This fn panics if the serialised output of `t` does not match the fixture
/// value read from `tests/fixtures/$name.json`, and writes the actual result to
/// `tests/fixtures/$name.actual.json` for review.
#[track_caller]
fn assert_fixture<T>(t: T, name: &str)
where
    for<'a> T: serde::Serialize + serde::Deserialize<'a> + PartialEq + Debug,
{
    let mut path = PathBuf::default();
    path.push("tests");
    path.push("fixtures");
    path.push(format!("{name}.json"));

    // Serialise the filter.
    let got = serde_json::to_string_pretty(&t).expect("must serialise");

    // Reconstruct an instance from the serialised form.
    let round_trip = serde_json::from_str(&got).expect("must deserialise from serialised form");
    assert_eq!(t, round_trip, "must round-trip through serialisation");

    // Read the existing fixture and ensure they match.
    let want = fs::read_to_string(&path).unwrap_or_else(|_| "<no fixture found>".to_string());
    if got != want {
        // They do not - write the new repr for use with `diff`.
        path.set_file_name(format!("{name}.actual.json"));
        fs::write(&path, &got).expect("failed to create fixture output file");
    }

    // Assert the serialised form matches.
    assert!(
        got == want,
        "fixture output differs, wrote actual fixture output to {}",
        path.display()
    );
}
