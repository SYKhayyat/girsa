//! A shelf small enough to build in a test, real enough to be worth asserting on.
//!
//! # The failure this exists to end
//!
//! Forty-three test functions across ten files gated on a corpus being present
//! and `return`ed when it was not:
//!
//! ```text
//! macro_rules! corpus_or_skip {
//!     () => { match corpus() { Some(root) => root,
//!         None => { eprintln!("skipped: no imported corpus"); return; } } }
//! }
//! ```
//!
//! `cargo test` captures stderr on a passing test, so in CI those printed:
//!
//! ```text
//! test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s
//! ```
//!
//! Eight green tests, no corpus, nothing asserted. `tools/check-ksav-fixture.sh`
//! refuses this by name twelve files away — *"a check that passes because it
//! could not find what it checks is the exact failure this script exists to
//! end"* — and forty-three test functions in the same repository did it anyway.
//! Among them was `spec_counts.rs`, the test that would have caught §3's
//! permanent ids being renumbered by every re-import.
//!
//! The response that worked once already is in `girsa-app/examples/fixture-packet.rs`:
//! the Ksav fixture rotted because regenerating it needed a corpus no gate has,
//! *"so the corpus is the thing that had to go."* That argument generalises. This
//! is it applied to the other forty-three.
//!
//! # Why it writes `merged.json` and not `segments.jsonl`
//!
//! **A fixture that writes what the importer outputs asserts itself back at
//! itself.** Hand `girsa-fixture` a `segments.jsonl` and a test that checks the
//! walker put daf 2a first is checking that this crate typed `2a` — which it did,
//! and which proves nothing about `sefaria::walk`.
//!
//! So everything here is written at the layer the *download* is written at, and
//! the real code reads it:
//!
//! | this crate writes | the code under test runs |
//! |---|---|
//! | `sefaria/<slug>/merged.json` + `schemas/<slug>.json` | [`girsa_corpus::import::sefaria::read`] |
//! | `otzaria/<name>.txt` with `<h1>/<h2>` headings | [`girsa_corpus::import::otzaria::read`] |
//! | `sefaria/links/links0.csv`, columns and misspelling intact | `girsa_link::sefaria::read_file` |
//! | `lexicon.tsv` | `girsa_ref::Lexicon::from_tsv` |
//!
//! Nothing is stubbed and nothing is short-circuited. The segments, the ordinals,
//! the addresses, the redirects, the edges and their orientation are all produced
//! by the same functions that produce them from Sefaria's actual export.
//!
//! That distinction is load-bearing for one test in particular.
//! `the_meforshim_are_on_the_daf` exists because Sefaria's `links*.csv` does not
//! say which of its two citation columns is the commentary, and the importer
//! recorded them in the order it read them — so half the commentary in the corpus
//! was stored as *base → commentary*, and a panel asking "what lands on this daf"
//! answered with two aggadic works out of forty. [`links`] writes its
//! `comments-on` rows **in both column orders, deliberately**, exactly as the
//! export does. If [`girsa_link::orient`] regresses, the rows come out backwards
//! and the test fails — on synthetic data, with no download.
//!
//! # What it deliberately does not claim
//!
//! It is not the corpus and it does not pretend to be. *Shulchan Arukh, Orach
//! Chayim has 697 simanim of 4,171 se'ifim* is a fact about a download, not about
//! this code, and no fixture can check it. Those assertions stay, next to the
//! ones here, marked `#[ignore]` so they read as `1 ignored` rather than as
//! `1 passed` — see `spec_counts.rs`. What moved onto the fixture is every
//! assertion that was ever about the code: that the walker produces exactly what
//! the schema's `lengths` promised, whatever those lengths are.
//!
//! # Where it goes on disk
//!
//! One directory per process, under the system temp dir, built once per test
//! binary by [`std::sync::OnceLock`]. Per process rather than shared because
//! `cargo test` runs the test binaries in parallel and two of them building the
//! same path is a race whose failure mode is a half-written shelf — which is the
//! shape of bug this whole crate exists to make impossible. The cost of not
//! sharing is a few hundred kilobytes written five times.

// A fixture that will not build must take the test down and say why. Every test
// file that uses it makes the same argument for itself: a panic in a test is a
// failure report.
#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub mod seforim;

#[cfg(feature = "links")]
pub mod links;

#[cfg(feature = "index")]
pub mod search;

/// A built fixture: the three roots the application takes.
#[derive(Debug, Clone)]
pub struct Shelf {
    root: PathBuf,
    personal: PathBuf,
    index: PathBuf,
}

impl Shelf {
    /// The corpus root — `works/`, and with the `links` feature, `links/`.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// A personal layer, empty, and its own directory per process.
    ///
    /// Empty rather than absent: a test asserting about the corpus should not
    /// have the reader's own seforim, arrangement or corrections in the way of
    /// it, and a personal layer that does not exist is a different code path
    /// from one that is empty.
    #[must_use]
    pub fn personal(&self) -> &Path {
        &self.personal
    }

    /// Where [`search`] puts the tantivy index.
    #[must_use]
    pub fn index(&self) -> &Path {
        &self.index
    }
}

/// The shelf: works, segments, redirects, catalogue and lexicon.
///
/// Built through [`girsa_corpus::import::read`] and [`girsa_corpus::import::write`]
/// from the `merged.json` files [`seforim`] lays down.
pub fn shelf() -> &'static Shelf {
    static BUILT: OnceLock<Shelf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let shelf = fresh();
        seforim::build(&shelf.root);
        shelf
    })
}

/// The shelf with the link graph over it: `edges.jsonl`, the inbound cache and
/// the touching cache, all built by the real importer from a `links0.csv`.
#[cfg(feature = "links")]
pub fn linked() -> &'static Shelf {
    static BUILT: OnceLock<()> = OnceLock::new();
    let shelf = shelf();
    BUILT.get_or_init(|| links::build(&shelf.root));
    shelf
}

/// The above, indexed, so the search engine and everything standing on it have
/// something to answer from.
#[cfg(feature = "index")]
pub fn indexed() -> &'static Shelf {
    static BUILT: OnceLock<()> = OnceLock::new();
    let shelf = linked();
    BUILT.get_or_init(|| search::build(&shelf.root, &shelf.index));
    shelf
}

/// An empty set of three roots, wiped if a previous run of this process id left
/// anything behind.
fn fresh() -> Shelf {
    let base = std::env::temp_dir().join("girsa-fixture").join(format!(
        "{}-{}",
        std::process::id(),
        binary()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let shelf = Shelf {
        root: base.join("corpus"),
        personal: base.join("personal"),
        index: base.join("index"),
    };
    std::fs::create_dir_all(&shelf.personal).expect("a fixture personal layer");
    shelf
}

/// The test binary's own name, so two binaries in one process — which `cargo`
/// does not do today and might — cannot collide either.
fn binary() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "test".to_string())
}

/// Write a file, creating the directories above it.
///
/// # Panics
///
/// If the fixture cannot be written. A test standing on half a shelf reports a
/// failure that has nothing to do with what it was asserting.
pub fn put(path: &Path, body: &str) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("a fixture directory");
    }
    std::fs::write(path, body).expect("a fixture file");
}

/// The fixture's own gate.
///
/// Something that exists so other things can be checked has to be checked
/// itself, or the forty-three tests standing on it are back where they started —
/// green over a shelf that quietly stopped having a Rashi on it. These assert the
/// shape every consumer relies on, and they are the reason `--features index` is
/// in CI rather than only in whoever remembers.
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn the_shelf_has_the_works_the_tests_name() {
        let root = shelf().root();
        for slug in [
            "bavli/berakhot",
            "bavli/rashi-on-berakhot",
            "bavli/tosafot-on-berakhot",
            "bavli/bava-metzia",
            "mishnah-berakhot",
            "rambam-on-mishnah-berakhot",
            "shulchan-arukh/orach-chayim",
            "mishnah-berurah",
            "genesis",
            "abarbanel-on-ezekiel",
            "otzaria/tzlach-berakhot",
        ] {
            let work = girsa_corpus::import::read_back(root, slug)
                .unwrap_or_else(|e| panic!("{slug}: {e}"));
            assert!(!work.segments.is_empty(), "{slug} has no segments");
        }
    }

    #[test]
    fn the_walker_put_berakhot_at_daf_2a() {
        // The one address rule in the corpus that is not index-plus-one, checked
        // through the real walker over a real flat amudim array.
        let work = girsa_corpus::import::read_back(shelf().root(), "bavli/berakhot").unwrap();
        let first = work.segments.first().unwrap();
        assert_eq!(first.id.path().first().map(String::as_str), Some("2a"));
        assert!(first.text.contains("מֵאֵימָתַי"));
    }

    #[test]
    fn every_id_the_fixture_mints_survives_a_round_trip() {
        let root = shelf().root();
        let body = std::fs::read_to_string(root.join("works/index.jsonl")).unwrap();
        let mut checked = 0usize;
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            let work: girsa_corpus::work::Work = serde_json::from_str(line).unwrap();
            let imported = girsa_corpus::import::read_back(root, &work.slug).unwrap();
            for segment in &imported.segments {
                let text = segment.id.to_string();
                let back: girsa_corpus::segment::SegmentId = text.parse().unwrap();
                assert_eq!(back, segment.id, "{text}");
                assert!(segment.id.is_well_formed(), "{text}");
                checked += 1;
            }
        }
        assert!(checked > 40, "only {checked} ids on the fixture shelf");
    }

    #[cfg(feature = "links")]
    #[test]
    fn every_row_resolved_and_the_backwards_ones_came_out_forwards() {
        // `links::import` already asserts the resolution rate. This is the other
        // half: that the rows written base-first were turned round, which is the
        // property the fixture exists to be able to check at all.
        let root = linked().root();
        assert!(links::backwards_rows() > 0, "the trap is not set");
        let edges = girsa_link::store::read_back(root, "bavli/berakhot").unwrap();
        let backwards = edges
            .iter()
            .filter(|e| e.edge_type == girsa_link::EdgeType::CommentsOn)
            .count();
        assert_eq!(
            backwards, 0,
            "Berakhot's own shard holds comments-on edges, which reads: \
             the gemara is a commentary on its commentaries"
        );
    }

    #[cfg(feature = "index")]
    #[test]
    fn the_index_can_be_asked_a_question() {
        let shelf = indexed();
        let index = girsa_search::index::SearchIndex::open(shelf.index()).unwrap();
        assert!(index.count() > 40, "{} segments indexed", index.count());
    }
}
