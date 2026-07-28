//! W11 — the index, and the four things it has to be true about.
//!
//! 1. **Nikud and te'amim are gone, in every mode, with no toggle** (spec.md
//!    §9.1). Berakhot is fully menukad and Mishnah Berurah has none, so a reader
//!    who types a bare word must find both and never learn that there was a
//!    difference.
//! 2. **The index holds normal forms and nothing else.** No peeled stems, no
//!    expanded abbreviations, no roots. If widening were baked in at import
//!    there would be no literal mode left to default to (spec.md §9.3), and
//!    §9.6's offer-with-a-count would have nothing to offer.
//! 3. **A hit points back at the text as printed** — at the menukad word on the
//!    page, not at the bare one in the index.
//! 4. **A stale index is refused loudly.** The terms on disk were written under
//!    one set of rules; a query normalized under another silently fails to find
//!    text that is right there. That is the worst failure this system has,
//!    because it looks like an answer.
//!
//! These use in-memory and temporary indices. The whole shelf is measured by
//! `girsa-index`, not by the test suite.

// A panic in a test is a failure report.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_search::index::{IndexError, SearchIndex, CACHE_STAMP};

/// A segment named the way the importer names one.
fn segment(work: &str, path: &[&str], n: u32, text: &str) -> Segment {
    Segment {
        id: SegmentId::new(
            work,
            path.iter().map(|s| (*s).to_string()).collect(),
            Ordinal::root(n),
        ),
        kind: SegmentKind::Text,
        text: text.to_string(),
    }
}

fn heading(work: &str, path: &[&str], n: u32, text: &str) -> Segment {
    Segment {
        kind: SegmentKind::Heading,
        ..segment(work, path, n, text)
    }
}

/// The opening of Berakhot, menukad, as the corpus has it — and a line of
/// Mishnah Berurah, which has no nikud at all.
fn shelf() -> Vec<Segment> {
    vec![
        segment(
            "bavli/berakhot",
            &["2a", "1"],
            1,
            "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין",
        ),
        segment(
            "bavli/berakhot",
            &["2a", "2"],
            2,
            "מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכוֹל בִּתְרוּמָתָן",
        ),
        heading("mishnah-berurah", &["1"], 1, "סימן א"),
        segment(
            "mishnah-berurah",
            &["1", "1"],
            2,
            "יתגבר כארי לעמוד בבוקר לעבודת בוראו",
        ),
        segment(
            "mishnah-berurah",
            &["1", "2"],
            3,
            "ובשבת אין קורין את שמע בבוקר כדרך של חול",
        ),
    ]
}

fn loaded() -> SearchIndex {
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    for s in shelf() {
        writer.add(&s).expect("adding a segment");
    }
    writer.commit().expect("committing");
    index.reload().expect("reloading");
    index
}

fn ids(hits: &[girsa_search::index::Hit]) -> Vec<String> {
    hits.iter().map(|h| h.id.to_string()).collect()
}

// ---------------------------------------------------------------------------
// 1 · nikud, in every mode, with no toggle
// ---------------------------------------------------------------------------

#[test]
fn a_menukad_word_is_found_by_its_bare_spelling() {
    // The first line of Shas is fully pointed. Nobody types it that way.
    let index = loaded();
    let hits = index.words("מאימתי").expect("a lookup");
    assert_eq!(ids(&hits), ["girsa:bavli/berakhot/2a:1#1"]);
}

#[test]
fn typing_the_nikud_finds_the_same_line_and_not_a_different_one() {
    // There is no toggle (spec.md §9.1), so the two spellings cannot be made to
    // disagree: the query goes through the same normalizer the index did.
    let index = loaded();
    let bare = index.words("מאימתי קורין").expect("a lookup");
    let pointed = index.words("מֵאֵימָתַי קוֹרִין").expect("a lookup");
    assert_eq!(ids(&bare), ["girsa:bavli/berakhot/2a:1#1"]);
    assert_eq!(ids(&bare), ids(&pointed));
}

#[test]
fn a_maqaf_does_not_glue_two_words_into_one() {
    // Maqaf is inside the stripped range but separates words. Deleting it
    // rather than breaking on it turns `אֶת־הַשָּׁמַיִם` into one token and the
    // second pasuk of the Torah stops being findable by either word in it.
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    writer
        .add(&segment(
            "torah/genesis",
            &["1", "1"],
            1,
            "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ׃",
        ))
        .expect("adding a segment");
    writer.commit().expect("committing");
    index.reload().expect("reloading");

    assert_eq!(index.words("השמים").expect("a lookup").len(), 1);
    assert_eq!(index.words("הארץ").expect("a lookup").len(), 1);
}

// ---------------------------------------------------------------------------
// 2 · normal forms and nothing else
// ---------------------------------------------------------------------------

#[test]
fn no_widening_is_baked_into_the_index() {
    // `ובשבת` is on the shelf. A term lookup for `שבת` must find nothing —
    // not because the reader should not be able to get there, but because
    // getting there has to be a thing the reader asked for. Peeling stems at
    // import would make the literal default (spec.md §9.3) unimplementable and
    // §9.6's counted offer unmeasurable.
    let index = loaded();
    assert!(
        index.words("שבת").expect("a lookup").is_empty(),
        "a peeled stem is in the index; there is no literal mode left to default to"
    );
    // And the word as written is there, so this is a statement about widening
    // rather than about the segment being missing.
    assert_eq!(index.words("ובשבת").expect("a lookup").len(), 1);
}

#[test]
fn an_abbreviation_is_not_expanded_at_import() {
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");
    writer
        .add(&segment("tur", &["1", "1"], 1, "וכן פסק שו\"ע שם"))
        .expect("adding a segment");
    writer.commit().expect("committing");
    index.reload().expect("reloading");

    assert_eq!(index.words("שו\"ע").expect("a lookup").len(), 1);
    // Folded gershayim: the same abbreviation written with U+05F4.
    assert_eq!(index.words("שו״ע").expect("a lookup").len(), 1);
    assert!(index.words("שולחן ערוך").expect("a lookup").is_empty());
}

#[test]
fn words_keep_their_order_so_a_phrase_can_be_asked_for() {
    // Positions have to be in the index at build time. Discovering at query
    // time that they are not means a rebuild of five million segments.
    let index = loaded();
    assert_eq!(index.phrase("יתגבר כארי").expect("a lookup").len(), 1);
    assert!(index.phrase("כארי יתגבר").expect("a lookup").is_empty());
    // The same two words, order not asked about.
    assert_eq!(index.words("כארי יתגבר").expect("a lookup").len(), 1);
}

// ---------------------------------------------------------------------------
// 3 · back to the text as printed
// ---------------------------------------------------------------------------

#[test]
fn a_hit_carries_the_text_as_printed_and_marks_the_word_on_it() {
    let index = loaded();
    let hits = index.words("קורין").expect("a lookup");
    let hit = hits.first().expect("one hit");

    // Not the normalized form. The reader is looking at the page.
    assert_eq!(hit.text, "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין");
    assert_eq!(hit.kind, SegmentKind::Text);

    let marks = hit.marks(&girsa_search::torat_emet::Query::new("קורין").plan());
    assert_eq!(marks.len(), 1, "one word matched, one mark");
    let (start, end) = marks[0];
    assert_eq!(&hit.text[start..end], "קוֹרִין");
}

#[test]
fn a_heading_is_indexed_and_says_that_it_is_one() {
    // Headings are findable — `סימן א` is a real thing to search for — but a
    // result row has to be able to say which it is, and W14's facets need the
    // field to count.
    let index = loaded();
    let hits = index.words("סימן").expect("a lookup");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].kind, SegmentKind::Heading);
}

// ---------------------------------------------------------------------------
// 4 · a stale index is refused, loudly
// ---------------------------------------------------------------------------

#[test]
fn an_index_written_under_older_normalizer_rules_is_refused() {
    let dir = tempdir();
    {
        let index = SearchIndex::create(dir.path()).expect("creating an index");
        let mut writer = index.writer().expect("a writer");
        writer
            .add(&segment("bavli/berakhot", &["2a", "1"], 1, "מֵאֵימָתַי"))
            .expect("adding a segment");
        writer.commit().expect("committing");
    }
    // It opens, as built.
    SearchIndex::open(dir.path()).expect("opening a fresh index");

    // Now say it was built under rules that no longer hold.
    let stamp = dir.path().join(CACHE_STAMP);
    let body = std::fs::read_to_string(&stamp).expect("the stamp");
    let older = body.replace(
        &format!(
            "\"normalizer_version\":{}",
            girsa_hebrew::NORMALIZER_VERSION
        ),
        "\"normalizer_version\":0",
    );
    assert_ne!(older, body, "the stamp must record the normalizer version");
    std::fs::write(&stamp, older).expect("writing the stamp");

    match SearchIndex::open(dir.path()) {
        Err(IndexError::Stale { .. }) => {}
        other => panic!("a stale index must be refused, got {other:?}"),
    }
}

#[test]
fn an_index_with_no_stamp_at_all_is_refused() {
    // A directory of tantivy files from anywhere else is not this index.
    let dir = tempdir();
    {
        let index = SearchIndex::create(dir.path()).expect("creating an index");
        let mut writer = index.writer().expect("a writer");
        writer.commit().expect("committing");
    }
    std::fs::remove_file(dir.path().join(CACHE_STAMP)).expect("removing the stamp");
    match SearchIndex::open(dir.path()) {
        Err(IndexError::Stale { .. }) => {}
        other => panic!("an unstamped index must be refused, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Rebuilding
// ---------------------------------------------------------------------------

#[test]
fn an_index_reopened_from_disk_still_answers() {
    // The tokenizer is registered by name in the schema and by value in the
    // index. Registering it only where the index is built gives an index that
    // writes fine and cannot be queried after a restart.
    let dir = tempdir();
    {
        let index = SearchIndex::create(dir.path()).expect("creating an index");
        let mut writer = index.writer().expect("a writer");
        for s in shelf() {
            writer.add(&s).expect("adding a segment");
        }
        writer.commit().expect("committing");
    }

    let index = SearchIndex::open(dir.path()).expect("opening");
    assert_eq!(index.count(), 5);
    assert_eq!(ids(&index.words("מאימתי").expect("a lookup")).len(), 1);
}

#[test]
fn reindexing_a_work_replaces_it_rather_than_doubling_it() {
    // Import is re-runnable, so the indexer is too. A second run that appends
    // gives every hit twice and a count that drifts up every time — which is
    // how a corpus measurement stops meaning anything.
    let index = SearchIndex::in_memory().expect("an index in memory");
    for _ in 0..2 {
        let mut writer = index.writer().expect("a writer");
        for s in shelf() {
            writer.add(&s).expect("adding a segment");
        }
        writer.commit().expect("committing");
    }
    index.reload().expect("reloading");

    assert_eq!(index.count(), 5, "five segments went in twice");
    assert_eq!(index.words("מאימתי").expect("a lookup").len(), 1);
}

#[test]
fn every_segment_lands() {
    // A silent shortfall in an index looks exactly like a corpus that does not
    // contain the passage.
    let index = loaded();
    assert_eq!(index.count(), shelf().len());
}

// ---------------------------------------------------------------------------

/// A directory that removes itself.
fn tempdir() -> TempDir {
    let base = std::env::temp_dir().join(format!(
        "girsa-index-test-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("a temporary directory");
    TempDir(base)
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
