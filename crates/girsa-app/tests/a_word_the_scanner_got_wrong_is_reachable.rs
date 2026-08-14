//! The OCR queue meeting the OCR'd page — W21 and W26, which were built to the
//! same shape and never joined.
//!
//! `docs/record/scans.md` says the queue does not see what tesseract read. Half
//! of that turned out to be wrong and the other half worse. **The ranking was
//! never the problem**: `girsa-suspects` reads the index's term dictionary, and
//! `SearchIndex::add_page` has put a page's words into that dictionary since
//! W26, so a word tesseract got wrong has been ranked beside a word Otzaria's
//! scanner got wrong all along. What did not work was reaching one. A candidate
//! placed on a page was a **dead row**: opening it looked for the word in the
//! segment's own text, and a page segment's text is the empty string the
//! importer minted — the words are in `personal/words/<slug>/pages.jsonl`.
//! Every candidate on a photograph answered *that word is not in that line any
//! more*, which is a sentence about a word that was never looked for.
//!
//! So the fix is a second lookup, over the reading rather than over the line,
//! and these are its properties. The second test is the raya that the two
//! cannot be one function.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_app::scanning::where_word_on_page;
use girsa_corpus::import::SegmentKind;
use girsa_scan::reading::{Area, Read, Reader, Word};

/// A page as an engine reports one: words, each with the ink it sits on.
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
                confidence: 0.87,
            })
            .collect(),
    )
}

#[test]
fn the_word_is_found_on_a_page_whose_segment_has_no_text() {
    // `קוריו` is `קורין` with the final nun read as a vav — one of the eight
    // shape confusions `girsa_fix::suspect` ranks by, and exactly the kind of
    // thing that lands in the queue with a high score and used to go nowhere.
    let page = read(12, &["מאימתי", "קוריו", "את", "שמע"]);
    assert_eq!(where_word_on_page(&page, "קוריו"), Some(1));
    assert_eq!(
        where_word_on_page(&page, "בערבין"),
        None,
        "a word that is not on the page is not somewhere on it"
    );
}

#[test]
fn the_line_lookup_finds_nothing_on_a_page_which_is_why_this_exists() {
    // The defect, kept as a test rather than as a paragraph. A dropped PDF gets
    // one segment per page and `text: String::new()` — the importer will not
    // invent Hebrew it cannot read — so the lookup the queue used for every
    // other sefer tokenizes an empty string. It does not fail loudly. It
    // returns "not here" about a place it never read.
    let mut sefer = girsa_app::pretend::sefer("user/vilna", "וילנא", &["עמוד"], &[""]);
    sefer.segments[0].kind = SegmentKind::Page;
    let at = sefer.segments[0].id.clone();

    assert_eq!(
        girsa_app::fixing::where_word(&sefer, &at, "קוריו", girsa_app::session::Pointing::Full),
        None,
        "the text of a page segment is empty, so the line lookup cannot answer"
    );
    // And the same word, asked of the reading, is right there.
    assert_eq!(
        where_word_on_page(&read(12, &["מאימתי", "קוריו"]), "קוריו"),
        Some(1)
    );
}

#[test]
fn the_queue_spelling_finds_a_word_the_page_prints_with_nikud() {
    // The queue works in the index's spelling — nikud off, final letters folded
    // — and a reading is in whatever the engine saw, which on a menukad sefer
    // is pointed. Comparing the printed strings would find nothing at all on
    // most of the pages this is for.
    let page = read(151, &["מֵאֵימָתַי", "קוֹרִיו", "אֶת"]);
    assert_eq!(where_word_on_page(&page, "קוריו"), Some(1));
    assert_eq!(where_word_on_page(&page, "מאימתי"), Some(0));
}

#[test]
fn a_word_inside_a_line_the_file_positioned_as_one_item_is_found() {
    // The honest complication `scans.md` records: the same PDF gives a
    // vocalized page as 707 positioned glyphs and an unvocalized one as 35
    // items, each a whole line with its spaces in it. Then the word wanted is
    // one token inside a `Word`, and matching the whole string would miss it
    // on exactly the pages where OCR is worst.
    let page = read(151, &["פרק ראשון", "מאימתי קוריו את שמע"]);
    assert_eq!(where_word_on_page(&page, "קוריו"), Some(1));
}

#[test]
fn a_word_the_reader_has_already_corrected_is_gone() {
    // The queue is a batch job's opinion from whenever it last ran, and a
    // reader who fixed this word an hour ago must not be handed a box that
    // opens on a word no longer there. The reading is asked for with the
    // reader's own fixes applied, so the candidate reports itself as gone —
    // which is the true answer, and the one that leaves the row dismissible.
    let dir = std::env::temp_dir().join("girsa-suspect-on-a-page");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch personal root");

    let (mut words, trouble) = girsa_scan::Words::open(&dir, "user/vilna");
    assert!(
        trouble.is_empty(),
        "a fresh store has nothing to complain of"
    );
    let page = read(12, &["מאימתי", "קוריו"]);
    let ink = page.words[1].at;
    words.record(page).expect("a page is written down");

    let before = words.page(12).expect("the page reads back");
    assert_eq!(where_word_on_page(&before, "קוריו"), Some(1));

    words
        .fix(
            12,
            girsa_scan::Fix {
                at: ink,
                was: "קוריו".to_string(),
                says: "קורין".to_string(),
            },
        )
        .expect("a correction is written down");

    let after = words.page(12).expect("the page reads back corrected");
    assert_eq!(
        where_word_on_page(&after, "קוריו"),
        None,
        "the word the queue is asking about is not what the page says any more"
    );
    // And what the reader put there is — asked for in the queue's spelling,
    // which folds the final nun. Asking for `קורין` as the reader typed it
    // finds nothing, and that is the rule working rather than failing: a
    // candidate that came out of the index is never spelled that way.
    assert_eq!(where_word_on_page(&after, "קורין"), None);
    assert_eq!(where_word_on_page(&after, "קורינ"), Some(1));
}
