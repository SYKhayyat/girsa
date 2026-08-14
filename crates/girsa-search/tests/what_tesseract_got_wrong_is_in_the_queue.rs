//! The half of the W21/W26 join that already worked, written down so it stops
//! being guessed at.
//!
//! `docs/record/scans.md` recorded that *OCR text does not reach the OCR-error
//! queue*, and the sentence was believed for long enough to become a row in
//! `docs/not-yet.md`. It is not what the code does. `girsa-suspects` builds its
//! vocabulary from the index's term dictionary rather than from the corpus
//! files, and `Writer::add_page` has written a page's words
//! into that same dictionary since W26 — so what tesseract read has been ranked
//! beside what Otzaria's scanner read from the day the two existed together.
//!
//! What was actually missing was on the other end, in the window, and is tested
//! in `girsa-app`: a candidate placed on a page could not be **opened**, because
//! opening one looked for the word in the segment's own text and a page segment
//! has none.
//!
//! Two claims here, and they are the ones that would go quietly wrong if
//! somebody changed how a page is indexed:
//!
//! 1. an engine's misreading is a word in the term dictionary, counted like any
//!    other, so `hunt` can find it;
//! 2. the search can place it, so the queue row says where to go and look.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_fix::suspect::{hunt, Settings, Vocabulary};
use girsa_scan::reading::{Area, Read, Reader, Word};
use girsa_search::index::SearchIndex;

/// The thresholds, shrunk to fixture size.
///
/// The defaults are *seen once* against *seen ten thousand times*, which is a
/// statement about Shas and not about four segments. The rule being tested is
/// the shape of the comparison, not the size of the numbers.
const SETTINGS: Settings = Settings {
    rare_at: 1,
    common_at: 3,
    shortest: 4,
};

fn segment(work: &str, n: u32, kind: SegmentKind, text: &str) -> Segment {
    Segment {
        id: SegmentId::new(work, vec![n.to_string()], Ordinal::root(n)),
        kind,
        text: text.to_string(),
        anchors: Vec::new(),
    }
}

fn read(page: usize, words: &[&str]) -> Read {
    Read::new(
        page,
        Reader::Ocr {
            engine: "tesseract v5.4.0".into(),
        },
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
                confidence: 0.71,
            })
            .collect(),
    )
}

/// Three lines of a text sefer that all say `קורין`, and one page of a scan
/// where the engine read the final nun as a vav.
fn loaded() -> SearchIndex {
    let index = SearchIndex::in_memory().expect("an index in memory");
    let mut writer = index.writer().expect("a writer");

    for n in 1..=3 {
        writer
            .add(
                &segment(
                    "bavli/berakhot",
                    n,
                    SegmentKind::Text,
                    "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין",
                ),
                &[],
            )
            .expect("a line of the corpus");
    }

    writer
        .add_page(
            &segment("user/vilna", 12, SegmentKind::Page, ""),
            &[],
            &read(12, &["מאימתי", "קוריו"]),
        )
        .expect("a page a machine read");

    writer.commit().expect("committing");
    index.reload().expect("reloading");
    index
}

fn vocabulary(index: &SearchIndex) -> Vocabulary {
    let mut out = Vocabulary::default();
    for (word, count) in index.vocabulary().expect("the term dictionary") {
        out.add(&word, count);
    }
    out
}

#[test]
fn a_word_an_engine_read_off_a_photograph_is_a_word_in_the_dictionary() {
    let index = loaded();
    let words = vocabulary(&index);
    assert_eq!(
        words.count("קוריו"),
        1,
        "the misreading is counted, once, like any other word"
    );
    assert_eq!(
        words.count("קורינ"),
        3,
        "and the word it is one letter from is counted from the text sefer — \
         final letters folded, which is what makes the comparison mean anything"
    );
}

#[test]
fn the_misreading_is_ranked_against_the_word_the_corpus_has() {
    let found = hunt(&vocabulary(&loaded()), SETTINGS);
    let suspect = found
        .iter()
        .find(|s| s.rare == "קוריו")
        .expect("tesseract's misreading is in the queue");
    assert_eq!(suspect.common, "קורינ");
    assert_eq!(suspect.rare_count, 1);
    assert_eq!(suspect.common_count, 3);
    assert_eq!(
        suspect.how,
        girsa_fix::suspect::Edit::Letter,
        "one letter read as another, which is the thing scanners do"
    );
}

#[test]
fn the_queue_can_say_which_page_to_go_and_look_at() {
    // The row is only a queue row if it points somewhere. `girsa-suspects`
    // fills `places` by asking the index for each candidate, and a page has to
    // answer that question the same way a line does or the reader is handed a
    // word with no way to see it in context.
    let index = loaded();
    let places = index.words("קוריו").expect("a search");
    let ids: Vec<String> = places.iter().map(|hit| hit.id.to_string()).collect();
    assert_eq!(
        ids,
        vec!["girsa:user/vilna/12#12".to_string()],
        "the misreading is placed on the page it was read off"
    );
}
