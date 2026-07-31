//! Storage, ingest, schemas, and permanent segment IDs.
//!
//! Text files on disk are the truth; SQLite and the tantivy index are a
//! rebuildable cache (spec.md §4.1). Corrupt the cache and you rebuild it; you
//! never lose text.
//!
//! Filled in by W5–W8.

pub mod csv;
pub mod era;
pub mod fetch;
pub mod import;
pub mod index;
// A permanent id that names 1.2 MB of text names a volume, not a place (B12).
// Counting them, and cutting them into places that anchors on the parent still
// cover.
pub mod oversized;
pub mod segment;
pub mod span;
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
