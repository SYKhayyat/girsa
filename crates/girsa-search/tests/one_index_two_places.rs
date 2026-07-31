//! A page of a scan and a line of the corpus are two rows of the same result
//! list, and the row says which it is.
//!
//! spec.md §9.7, BUILDER.md W26:
//!
//! > One index, two location types. A text hit is sefer + segment ID. A PDF hit
//! > is sefer + page + box. Same result row; only the highlight differs —
//! > reflowed text versus a rectangle on the scan. […] Scanned hits carry a
//! > badge, because OCR text is dirtier and you should know which kind of
//! > result you are reading. **Badge them, don't demote them.**
//!
//! Three claims, and each is a test here:
//!
//! 1. a page with words on it is found by the same query as a text sefer, in
//!    the same list, scored by the same rules — **not demoted**;
//! 2. it says who read it, and *the file said so* and *a machine guessed*
//!    are different badges, because the measurement in `girsa-scan`'s
//!    `engine.rs` puts them 40 points of precision apart;
//! 3. a page nobody has read is **in the index and out of the results** — a
//!    row with no words, which is what lets the header count what the reader
//!    cannot see instead of the sefer vanishing.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_scan::reading::{Area, Read, Reader, Word};
use girsa_search::index::{Hit, SearchIndex};

fn segment(work: &str, path: &[&str], n: u32, kind: SegmentKind, text: &str) -> Segment {
    Segment {
        id: SegmentId::new(
            work,
            path.iter().map(|s| (*s).to_string()).collect(),
            Ordinal::root(n),
        ),
        kind,
        text: text.to_string(),
        // W34's mined anchors: a fixture types its own text, so none.
        anchors: Vec::new(),
    }
}

/// A page of a scan, read — words with the ink they sit on.
fn read(page: usize, by: Reader, words: &[&str]) -> Read {
    Read::new(
        page,
        by,
        words
            .iter()
            .enumerate()
            .map(|(at, text)| Word {
                text: (*text).to_string(),
                #[allow(clippy::cast_precision_loss)]
                at: Area::new(
                    0.82 - at as f32 * 0.09,
                    0.31,
                    0.89 - at as f32 * 0.09,
                    0.332,
                ),
                confidence: 0.87,
            })
            .collect(),
    )
}

/// One shelf: a line of the corpus, a page read off a PDF's own text, a page
/// somebody OCR'd, and a page nobody has touched.
fn loaded() -> SearchIndex {
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");

    writer
        .add(
            &segment(
                "bavli/berakhot",
                &["2a", "1"],
                1,
                SegmentKind::Text,
                "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין",
            ),
            &[],
        )
        .expect("a line of the corpus");

    writer
        .add_page(
            &segment("user/berachos", &["7"], 7, SegmentKind::Page, ""),
            &[],
            &read(7, Reader::Embedded, &["מאימתי", "קורין", "את", "שמע"]),
        )
        .expect("a page the file described");

    writer
        .add_page(
            &segment("user/vilna", &["12"], 12, SegmentKind::Page, ""),
            &[],
            &read(
                12,
                Reader::Ocr {
                    engine: "tesseract v5.4.0".into(),
                },
                &["מאימתי", "קוריו"],
            ),
        )
        .expect("a page a machine read");

    // Nobody has read this one. It goes in as itself: a page, addressable,
    // citable, notable — and with nothing to find.
    writer
        .add(
            &segment("user/vilna", &["13"], 13, SegmentKind::Page, ""),
            &[],
        )
        .expect("a page nobody has read");

    writer.commit().expect("committing");
    index.reload().expect("reloading");
    index
}

fn ids(hits: &[Hit]) -> Vec<String> {
    let mut out: Vec<String> = hits.iter().map(|h| h.id.to_string()).collect();
    out.sort();
    out
}

/// One hit, by name — the order is the score's business and not this test's.
fn found<'a>(hits: &'a [Hit], id: &str) -> &'a Hit {
    hits.iter()
        .find(|hit| hit.id.to_string() == id)
        .unwrap_or_else(|| panic!("{id} is not in {:?}", ids(hits)))
}

#[test]
fn a_page_of_a_scan_answers_the_same_query_as_a_line_of_the_corpus() {
    let index = loaded();
    let hits = index.words("מאימתי").expect("a lookup");
    assert_eq!(
        ids(&hits),
        [
            "girsa:bavli/berakhot/2a:1#1",
            "girsa:user/berachos/7#7",
            "girsa:user/vilna/12#12",
        ],
        "one index, two location types"
    );
    // **Badge them, don't demote them.** The pages are ranked by the same rule
    // as everything else — here that puts the OCR'd page first, because it is
    // the shortest document the word appears in — and nothing anywhere subtracts
    // from a row's score for having come off a photograph.
    assert!(hits[0].is_scanned(), "{:?}", hits[0].id.to_string());
}

#[test]
fn a_scanned_hit_says_who_read_it_and_a_line_of_the_corpus_says_nobody_did() {
    let index = loaded();
    let hits = index.words("מאימתי").expect("a lookup");

    let corpus = found(&hits, "girsa:bavli/berakhot/2a:1#1");
    assert_eq!(corpus.by, None, "a text sefer was not read off anything");
    assert!(!corpus.is_scanned());
    assert!(!corpus.is_a_page());

    let told = found(&hits, "girsa:user/berachos/7#7");
    assert_eq!(told.by, Some(Reader::Embedded));
    assert!(told.is_a_page());
    assert!(
        !told.is_scanned(),
        "the file said what its words are; nothing guessed, so nothing to warn about"
    );

    let guessed = found(&hits, "girsa:user/vilna/12#12");
    assert!(guessed.is_scanned(), "spec.md §9.7's badge");
    assert_eq!(
        guessed.by.as_ref().map(Reader::name),
        Some("tesseract v5.4.0")
    );
    // Badged, not demoted: it is above nothing and below nothing on account of
    // how it was read.
    assert!(guessed.score > 0.0);
}

#[test]
fn a_page_nobody_has_read_is_in_the_index_and_out_of_the_results() {
    let index = loaded();
    // Four documents, one of which can never match anything.
    assert_eq!(index.count(), 4);
    let hits = index.words("מאימתי").expect("a lookup");
    assert!(
        !ids(&hits).contains(&"girsa:user/vilna/13#13".to_string()),
        "a page with no words cannot be a hit"
    );
    // And it is a page, so the count of what is missing can be taken from the
    // shelf — which is what `girsa_app::reading::gap` does and what the results
    // header prints. The alternative, leaving it out of the index, would make
    // the sefer disappear rather than be reported.
    assert_eq!(index.words("").map(|h| h.len()).unwrap_or(0), 0);
}

#[test]
fn the_words_of_a_page_are_marked_where_the_ink_is() {
    // The other half of §9.7's *only the highlight differs*: a text hit is
    // marked in its printed string, a page is marked with a rectangle. The
    // rectangle is not in the index — it is in the reading, looked up when the
    // row is opened — because a query cannot be asked about a rectangle and
    // duplicating one into five million documents would buy nothing.
    let page = read(7, Reader::Embedded, &["מאימתי", "קורין", "את", "שמע"]);
    let marks =
        page.marks(|word| girsa_hebrew::normalize(word) == girsa_hebrew::normalize("קורין"));
    assert_eq!(marks.len(), 1);
    assert!(
        marks[0].left >= 0.0 && marks[0].right <= 1.0,
        "{:?}",
        marks[0]
    );
    assert_eq!(page.covering(marks[0]), Some(1));
}
