//! Storage, ingest, schemas, and permanent segment IDs.
//!
//! Text files on disk are the truth; the caches beside them are rebuildable
//! (spec.md §4.1). Corrupt a cache and you rebuild it; you never lose text.
//!
//! **The caches are the tantivy index and `girsa-corpus`'s own segment and
//! line-index stores.** This used to say *"SQLite and the tantivy index"*, and
//! there is no SQLite here — there never was. It was named in an early plan, the
//! plan changed, and the sentence outlived it in the header of the crate whose
//! subject it is.
//!
//! Filled in by W5–W8.

// Sefaria's commentary anchors, mined to spans and out of the text (W34, W33-A).
// They were indexed as words, which broke phrase search on the most-searched shelf
// in the corpus.
/// Spell a fieldless enum **once**, for the file and for the code.
///
/// # The shape this exists to stop
///
/// `girsa_fix::Kind` had `as_str`, `named`, *and* `#[derive(Serialize,
/// Deserialize)] #[serde(rename_all = "lowercase")]` — with `as_str`'s own doc
/// comment saying *"one implementation, so the word in the file, the word on
/// the button and the word the tests use cannot drift."* Two spellings of one
/// wire format, on one type, under a sentence about there being one.
///
/// They agreed. That is not the point: `rename_all` is a rule about *how to
/// derive* a spelling and `as_str` is the spelling, so the day a variant is
/// renamed — `FixedWithVariants` → `WithVariants` — the derive follows the
/// identifier and `as_str` does not, and a corrections file written by one
/// build stops reading in the next with nothing anywhere saying so.
///
/// ```ignore
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// pub enum Kind { Ocr, Girsa }
///
/// girsa_corpus::spelled!(Kind { Ocr => "ocr", Girsa => "girsa" });
/// ```
///
/// `as_str`, `named`, `SPELLINGS`, `Serialize` and `Deserialize` all come off
/// that one list. A word this project never writes deserialises as an **error
/// naming the words it does** — not as a fallback variant, because a value
/// invented on read is a claim nobody made.
#[macro_export]
macro_rules! spelled {
    ($t:ident { $($variant:ident => $word:literal),+ $(,)? }) => {
        impl $t {
            /// Every variant and the word it is written as, in declared order.
            pub const SPELLINGS: &'static [(Self, &'static str)] =
                &[$((Self::$variant, $word)),+];

            /// What this is called — in a file, on a button, and in a test.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $word),+
                }
            }

            /// Read back what [`Self::as_str`] wrote.
            ///
            /// `None` for a word this project does not write. Never a fallback
            /// variant: a value invented on read is a claim nobody made.
            #[must_use]
            pub fn named(word: &str) -> Option<Self> {
                match word {
                    $($word => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }

        impl ::serde::Serialize for $t {
            fn serialize<S: ::serde::Serializer>(&self, into: S) -> Result<S::Ok, S::Error> {
                into.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $t {
            fn deserialize<D: ::serde::Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
                let word = <::std::string::String as ::serde::Deserialize>::deserialize(from)?;
                Self::named(&word)
                    .ok_or_else(|| <D::Error as ::serde::de::Error>::unknown_variant(
                        &word,
                        &[$($word),+],
                    ))
            }
        }
    };
}

pub mod anchors;
// Sixteen binaries, five conventions for reading a command line, and three
// parsers that could not tell a switch from a value flag.
pub mod argv;
pub mod csv;
pub mod era;
pub mod fetch;
pub mod import;
pub mod index;
// A permanent id that names 1.2 MB of text names a volume, not a place (B12).
// Counting them, and cutting them into places that anchors on the parent still
// cover.
pub mod oversized;
// Three modules compose "what this answer could not see", each documented as
// the only one, and what drifted was the separator and the number format
// between them.
pub mod roots;
pub mod said;
pub mod segment;
pub mod span;
// A dotted name means two opposite things — a piece a cut carved out, or a
// se'if upstream inserted — and a prefix test says yes to both. What separates
// them is that a cut deletes its parent.
pub mod standing;
pub mod store;
pub mod taxonomy;
pub mod work;

/// Identifies the rules a derived cache was built under.
///
/// The database and the tantivy index are derived from the corpus by way of
/// [`girsa_hebrew`] and [`girsa_ref`]. If either changes how it normalizes or
/// how it writes a ref, the derived artifacts are stale in a way that is
/// invisible — a query normalizes under the new rules and the index holds terms
/// written under the old ones, so text that is right there stops being found.
///
/// Recording this at build time turns that into a detectable mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheProvenance {
    /// [`girsa_hebrew::NORMALIZER_VERSION`] the index was written under.
    pub normalizer_version: u32,
    /// The ref scheme, from [`girsa_ref::SCHEME`].
    pub ref_scheme: &'static str,
}

impl CacheProvenance {
    /// What a cache built by *this* binary would be stamped with.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            normalizer_version: girsa_hebrew::NORMALIZER_VERSION,
            ref_scheme: girsa_ref::SCHEME,
        }
    }

    /// Whether a cache stamped `self` can still be trusted by this binary.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        *self == Self::current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cache_built_now_is_usable_now() {
        assert!(CacheProvenance::current().is_usable());
    }

    #[test]
    fn a_cache_built_under_older_normalizer_rules_is_rejected() {
        let stale = CacheProvenance {
            normalizer_version: girsa_hebrew::NORMALIZER_VERSION.wrapping_sub(1),
            ..CacheProvenance::current()
        };
        assert!(!stale.is_usable());
    }
}
