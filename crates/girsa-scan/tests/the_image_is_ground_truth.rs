//! The property W26 exists for: a correction to a scan is anchored to the ink,
//! so a better engine can replace every word on the page without moving it.
//!
//! spec.md §6.3 — *the image stays ground truth, which makes fixing OCR errors
//! safe by construction*. This is that sentence as a test, and it is the W26
//! equivalent of W6's 501-link test: the design is chosen because this passes
//! and the obvious design fails it.
//!
//! # The obvious design, and where it goes
//!
//! Store a correction the way W20 stores one for a text sefer: `segment id +
//! character span`. That is right for a text sefer, where the base text is a
//! file on disk that does not change underneath. It is wrong here, because a
//! page's words are **an engine's current opinion** and W26's whole premise is
//! that a better engine replaces them. Re-read the page and there are more
//! words, or fewer, spelled differently — every offset now points somewhere
//! else, with nothing anywhere saying so.
//!
//! `an_offset_into_the_old_reading_lands_on_the_wrong_word` is that failure,
//! kept as a test so the reason survives longer than the commit message.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_scan::reading::{corrected, group, Area, Fix, Glyph, Read, Reader, Word};

/// A page of a sefer, as the file's own text layer gives it: one glyph per
/// letter, positioned individually.
///
/// `מאימתי קורין את שמע בערבית` — the first line of the first mishnah of
/// Berakhot, which is the line every one of these tests is about.
fn as_the_file_draws_it() -> Read {
    let mut glyphs = Vec::new();
    // From the right margin leftwards, because that is the way the line runs.
    let mut right = 0.88;
    for word in ["מאימתי", "קורין", "את", "שמע", "בערבית"] {
        for letter in word.chars() {
            glyphs.push(Glyph {
                text: letter.to_string(),
                at: Area::new(right - 0.008, 0.20, right, 0.222),
            });
            right -= 0.0090;
        }
        right -= 0.0125; // and a real gap between words
    }
    Read::new(4, Reader::Embedded, group(&glyphs).words)
}

/// The same page, read again by something else — and it disagrees about
/// everything except where the ink is.
///
/// Every way a re-read can move an offset, in one page: a word **split** in
/// two, two words **merged** into one, a word **misread**, a speck of dust read
/// as a word that is not there, and every box nudged, because two engines do
/// not draw a box round a letter the same way.
///
/// None of these is invented for the test. The measurement in
/// `crates/girsa-scan/src/engine.rs` found tesseract producing four words that
/// are not on the page for every one it got right, on a page of Rashi script.
fn read_again_by_something_else(first: &Read) -> Read {
    let mut words: Vec<Word> = Vec::new();
    for word in &first.words {
        let nudged = Area::new(
            word.at.left - 0.0012,
            word.at.top - 0.0009,
            word.at.right + 0.0012,
            word.at.bottom + 0.0009,
        );
        match word.text.as_str() {
            // Split: the engine saw a space that is not there.
            "מאימתי" => {
                let middle = (nudged.left + nudged.right) / 2.0;
                words.push(Word {
                    text: "מאי".into(),
                    at: Area::new(middle, nudged.top, nudged.right, nudged.bottom),
                    confidence: 0.71,
                });
                words.push(Word {
                    text: "מתי".into(),
                    at: Area::new(nudged.left, nudged.top, middle, nudged.bottom),
                    confidence: 0.64,
                });
            }
            // Misread: a ר for a ד, which is one serif.
            "קורין" => words.push(Word {
                text: "קודין".into(),
                at: nudged,
                confidence: 0.55,
            }),
            other => words.push(Word {
                text: other.to_string(),
                at: nudged,
                confidence: 0.8,
            }),
        }
        // Invented: a speck in the gap after the word, read as a letter.
        if word.text == "קורין" {
            words.push(Word {
                text: "ו".into(),
                at: Area::new(nudged.left - 0.0105, 0.203, nudged.left - 0.0075, 0.219),
                confidence: 0.31,
            });
        }
    }
    // Merge: the two short words in the middle came back as one.
    let at = words[4].at.with(words[5].at);
    words.splice(
        4..6,
        [Word {
            text: "אתשמע".into(),
            at,
            confidence: 0.6,
        }],
    );
    Read::new(
        first.page,
        Reader::Ocr {
            engine: "tesseract 5.4.0".into(),
        },
        words,
    )
}

/// Where a word's ink is, asked of a reading — which is what a reader marking a
/// word on the screen is doing with a mouse.
fn ink_of(read: &Read, word: &str) -> Area {
    read.words
        .iter()
        .find(|w| w.text == word)
        .map(|w| w.at)
        .unwrap_or_else(|| panic!("{word} is not on this page: {}", read.text()))
}

#[test]
fn a_correction_lands_on_the_same_word_after_the_page_is_read_again() {
    let first = as_the_file_draws_it();
    assert_eq!(first.text(), "מאימתי קורין את שמע בערבית");

    // The reader is looking at the photograph. They see that the second word is
    // wrong and fix it, by marking the ink.
    let fixes = vec![
        Fix {
            at: ink_of(&first, "קורין"),
            was: "קורין".into(),
            says: "קוראין".into(),
        },
        Fix {
            at: ink_of(&first, "בערבית"),
            was: "בערבית".into(),
            says: "בערבין".into(),
        },
    ];

    let (fixed, lost) = corrected(&first, &fixes);
    assert!(lost.is_empty(), "{lost:?}");
    assert_eq!(fixed.text(), "מאימתי קוראין את שמע בערבין");

    // Now a better engine reads the page. It splits a word, merges two others,
    // misspells a third and nudges every box. Nothing about the photograph
    // changed, so nothing about the corrections may.
    let again = read_again_by_something_else(&first);
    assert_eq!(again.text(), "מאי מתי קודין ו אתשמע בערבית");
    assert_ne!(again.words.len(), first.words.len());

    let (fixed_again, lost) = corrected(&again, &fixes);
    assert!(lost.is_empty(), "a correction went missing: {lost:?}");
    assert_eq!(fixed_again.text(), "מאי מתי קוראין ו אתשמע בערבין");
}

#[test]
fn an_offset_into_the_old_reading_lands_on_the_wrong_word() {
    // The design this crate does not use, run against the same two readings.
    // Kept because "we thought about it" is not evidence and this is.
    let first = as_the_file_draws_it();
    let again = read_again_by_something_else(&first);

    // Characters and not bytes, the way `girsa_fix::Patch` counts them — so
    // this is the strongest form of the offset design, not a straw one.
    let text: Vec<char> = first.text().chars().collect();
    let at = first
        .text()
        .chars()
        .collect::<String>()
        .find("בערבית")
        .map(|byte| first.text()[..byte].chars().count())
        .expect("the word is in the first reading");
    let span = at..at + "בערבית".chars().count();
    assert_eq!(text[span.clone()].iter().collect::<String>(), "בערבית");

    // The same span, into the reading taken five minutes later.
    let then: Vec<char> = again.text().chars().collect();
    let landed: String = then.get(span).map_or_else(
        || "off the end of the page".to_string(),
        |s| s.iter().collect(),
    );
    assert_ne!(
        landed, "בערבית",
        "an offset anchor happened to survive — the fixture is not testing anything"
    );
    // It is not off by a little, and it is not off the end where somebody would
    // notice. It names the end of one word, a space, and the start of another.
    assert_eq!(landed, "ע בערב");
}

#[test]
fn a_correction_whose_ink_nobody_read_is_handed_back_rather_than_dropped() {
    let first = as_the_file_draws_it();
    let fixes = vec![Fix {
        // Ink in the margin: a word the reader can see and no engine found.
        at: Area::new(0.10, 0.80, 0.16, 0.822),
        was: String::new(),
        says: "הגהה".into(),
    }];

    let (fixed, lost) = corrected(&first, &fixes);
    assert_eq!(fixed.text(), first.text());
    assert_eq!(lost.len(), 1, "the correction was silently dropped");
    assert_eq!(lost[0].says, "הגהה");
}

#[test]
fn a_reading_is_marked_where_the_words_are_and_the_marks_are_on_the_page() {
    let read = as_the_file_draws_it();
    let marks = read.marks(|word| word == "שמע");
    assert_eq!(marks.len(), 1);
    let mark = marks[0];
    assert_eq!(
        read.covering(mark),
        read.words.iter().position(|w| w.text == "שמע")
    );
    // A rectangle on the page, not a pixel of somebody's raster.
    assert!(mark.left >= 0.0 && mark.right <= 1.0, "{mark:?}");
    assert!(mark.top >= 0.0 && mark.bottom <= 1.0, "{mark:?}");
}

#[test]
fn a_page_read_by_an_engine_says_which_engine() {
    let by = read_again_by_something_else(&as_the_file_draws_it()).by;
    assert!(by.is_ocr());
    assert_eq!(by.name(), "tesseract 5.4.0");
    assert_eq!(Reader::named(by.name()), by);
    // And the other one is not an engine at all, which is the difference the
    // badge of spec.md §9.7 is drawn from.
    assert!(!Reader::Embedded.is_ocr());
    assert_eq!(Reader::named("embedded"), Reader::Embedded);
}
