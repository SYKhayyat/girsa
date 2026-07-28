//! The lookup BUILDER.md W8 says to reproduce, end to end.
//!
//! > **Acceptance.** Reproduce the known-good lookup end-to-end: **Mishnah
//! > Berakhot segment 3 → Rambam segment 5**, correct text, verified
//! > independently in `OtzariaSonim/SPEC.md`.
//!
//! That verification was done in Otzaria's addressing: *Mishnah Berakhot line 3
//! ("מאימתי קורין את שמע בערבית") → Rambam file line 5 ("מאימתי קורין את שמע
//! בערבין וכו': כבר בארנו…")*. Both are works Sefaria also has, so decision 1
//! puts Sefaria's copy of each on the shelf and the link comes from Sefaria's
//! `links*.csv` rather than from Otzaria's line-indexed conversion of it.
//!
//! **It is the same fact, checked through the new addressing**: the first
//! mishnah of Berakhot, the Rambam's commentary on that mishnah, the link
//! between them, and the two texts. What changed is that neither end is a line
//! number any more, so correcting a typo in either sefer cannot move it.
//!
//! # Why it skips when the corpus is absent
//!
//! It needs the fetched corpus, imported, with links imported over it — not
//! committed, and not present on a fresh clone. A test that failed there would
//! be noise everybody learns to ignore.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import;
use girsa_link::{store, Anchor, EdgeType};

const MISHNAH: &str = "mishnah-berakhot";
const RAMBAM: &str = "rambam-on-mishnah-berakhot";

fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    root.join("links").is_dir().then_some(root)
}

macro_rules! corpus_or_skip {
    () => {
        match corpus() {
            Some(root) => root,
            None => {
                eprintln!("skipped: no imported link graph — run girsa-link-import first");
                return;
            }
        }
    };
}

/// The text a segment id names, read off the shelf.
fn text_at(root: &Path, anchor: &Anchor) -> String {
    let work = import::read_back(root, anchor.from.work()).expect("the work is on the shelf");
    work.segments
        .iter()
        .find(|s| s.id == anchor.from)
        .map(|s| s.text.clone())
        .unwrap_or_default()
}

#[test]
fn the_first_mishnah_of_berakhot_reaches_the_rambam_on_it_and_the_text_is_right() {
    let root = corpus_or_skip!();

    // Edges are stored once, in the direction they were written (spec.md §8.2),
    // so the pair is looked for from both sides rather than assuming which way
    // Sefaria wrote it.
    let mut edges = store::read_back(&root, RAMBAM).expect("rambam shard reads");
    edges.extend(store::read_back(&root, MISHNAH).expect("mishnah shard reads"));

    // Perek 1, mishnah 1, in both. The Rambam's address has a third level —
    // his commentary on one mishnah is several segments — so the match is on
    // the first two levels rather than on the whole address.
    let first_mishnah = |id: &girsa_corpus::segment::SegmentId, work: &str| {
        id.work() == work
            && id.path().first().map(String::as_str) == Some("1")
            && id.path().get(1).map(String::as_str) == Some("1")
    };

    let found = edges
        .iter()
        .find(|e| {
            let ends = [&e.from.from, &e.to.from];
            ends.iter().any(|id| first_mishnah(id, MISHNAH))
                && ends.iter().any(|id| first_mishnah(id, RAMBAM))
        })
        .expect("Mishnah Berakhot 1:1 is linked to the Rambam on it");

    let (mishnah_end, rambam_end) = if found.from.from.work() == MISHNAH {
        (&found.from, &found.to)
    } else {
        (&found.to, &found.from)
    };

    let mishnah = text_at(&root, mishnah_end);
    let rambam = text_at(&root, rambam_end);

    // Compared through the normalizer, because Berakhot ships fully menukad
    // and the Rambam on it has no nikud at all — the two sides of this very
    // link are the example spec.md §2.1 gives for inconsistent coverage. W2's
    // sibling rule: nothing in this codebase compares Hebrew with `==`.
    let says = |text: &str, phrase: &str| {
        girsa_hebrew::normalize(text).contains(&girsa_hebrew::normalize(phrase))
    };

    assert!(
        says(&mishnah, "מאימתי קורין את שמע"),
        "the mishnah end reads: {}",
        mishnah.chars().take(120).collect::<String>()
    );
    assert!(
        says(&rambam, "מאימתי קורין את שמע") && says(&rambam, "כבר בארנו"),
        "the rambam end reads: {}",
        rambam.chars().take(120).collect::<String>()
    );

    // And the edge says what kind of link it is, from day one (spec.md §8.2).
    assert!(
        matches!(found.edge_type, EdgeType::CommentsOn | EdgeType::References),
        "{:?} / {:?}",
        found.edge_type,
        found.source_label
    );

    println!(
        "{} -> {}\n  {}\n  {}",
        mishnah_end,
        rambam_end,
        mishnah.chars().take(60).collect::<String>(),
        rambam.chars().take(60).collect::<String>()
    );
}

#[test]
fn no_edge_anywhere_anchors_to_something_that_is_not_on_the_shelf() {
    // A link that resolves to a segment that does not exist is the silent
    // failure the whole design is arranged against — it opens a page, and the
    // page is the wrong one or is not there. Checked on the two shards this
    // test already loads rather than on all seven thousand, which would take
    // longer than a test should.
    let root = corpus_or_skip!();
    // One work read once. Read per edge, this is a two-gigabyte test.
    let mut shelf: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();

    for slug in [MISHNAH, RAMBAM] {
        let edges = store::read_back(&root, slug).expect("shard reads");
        assert!(!edges.is_empty(), "{slug} has no outgoing edges at all");
        for edge in &edges {
            for anchor in [&edge.from, &edge.to] {
                for id in [Some(&anchor.from), anchor.to.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    let ids = shelf.entry(id.work().to_string()).or_insert_with(|| {
                        import::read_back(&root, id.work())
                            .map(|w| w.segments.iter().map(|s| s.id.to_string()).collect())
                            .unwrap_or_default()
                    });
                    assert!(
                        ids.contains(&id.to_string()),
                        "{id} names a segment that is not on the shelf"
                    );
                }
            }
        }
    }
}
