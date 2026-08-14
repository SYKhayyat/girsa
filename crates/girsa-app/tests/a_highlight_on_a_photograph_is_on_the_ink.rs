//! Highlighting a scan finer than the whole page — W24 meeting W26.
//!
//! W24 attaches a link or a highlight to **specific words** by storing a
//! character span into the segment's text. A page of a scan has no text: the
//! importer gives a dropped PDF one segment per page with an empty string in
//! it, because it will not invent Hebrew it cannot read. So a span had nothing
//! to count into and a scan could be marked whole and no finer.
//!
//! What a page does have is words with rectangles under them, and the answer is
//! the one `girsa-scan` already settled for a correction: **anchor to the ink.**
//! A page's words are an engine's current opinion and the whole premise of the
//! OCR work order is that a better engine replaces them — re-read the page and
//! there are more words, or fewer, spelled differently, and every offset then
//! points somewhere else in silence. The photograph does not move.
//!
//! `girsa-scan/tests/the_image_is_ground_truth.rs` is that property for a
//! correction. This is it for a highlight, plus the one thing a highlight has
//! that a correction does not: it covers a **run**, and a run has a shape.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use girsa_app::scanning::{ink_of, words_under};
use girsa_scan::reading::{Area, Read, Reader, Word};

/// A page laid out in lines: `rows` of `each` words, top to bottom.
///
/// Right to left, because the sefer is — so word 0 is the rightmost of the top
/// line, which is the order a reading arrives in and the order a reader picks.
fn page(rows: usize, each: usize, words: &[&str]) -> Read {
    let mut out = Vec::new();
    for row in 0..rows {
        for column in 0..each {
            let at = row * each + column;
            #[allow(clippy::cast_precision_loss)]
            let (row, column) = (row as f32, column as f32);
            out.push(Word {
                text: words.get(at).copied().unwrap_or("מילה").to_string(),
                at: Area::new(
                    0.90 - column * 0.11,
                    0.10 + row * 0.06,
                    0.98 - column * 0.11,
                    0.14 + row * 0.06,
                ),
                confidence: 0.9,
            });
        }
    }
    Read::new(
        1,
        Reader::Ocr {
            engine: "tesseract v5.4.0".into(),
        },
        out,
    )
}

#[test]
fn a_run_on_one_line_is_one_rectangle_and_covers_only_those_words() {
    let read = page(3, 5, &["א", "ב", "ג", "ד", "ה"]);
    let (ink, was) = ink_of(&read, 1..4).expect("three words of the first line");

    assert_eq!(ink.len(), 1, "one line, one rectangle");
    assert_eq!(was, "ב ג ד");
    assert_eq!(
        words_under(&read, &ink),
        vec![1, 2, 3],
        "and it covers those three and not the words beside them"
    );
}

#[test]
fn a_run_over_three_lines_is_three_rectangles_and_not_one_box_round_them() {
    // The reason this is per line. A highlight from the middle of the top line
    // to the middle of the third has a bounding box that also covers the far
    // ends of all three — including words the reader never touched. Redrawing
    // from that box would grow the mark, and a highlight two words wider than
    // the one somebody made looks exactly like one that landed right, which is
    // the refusal W24 made about a dibur hamatchil and W26 made again about a
    // rectangle.
    let read = page(3, 5, &[]);
    let (ink, _) = ink_of(&read, 2..13).expect("a run across three lines");

    assert_eq!(ink.len(), 3, "one rectangle per line");
    let covered = words_under(&read, &ink);
    assert_eq!(
        covered,
        (2..13).collect::<Vec<_>>(),
        "exactly the words picked — no more, and none skipped"
    );

    // And the bounding box would have been wrong, which is what makes the
    // three rectangles worth having rather than a tidier one.
    let whole = ink
        .iter()
        .copied()
        .reduce(Area::with)
        .expect("a box round all three");
    assert!(
        words_under(&read, &[whole]).len() > covered.len(),
        "one box round the run swallows words nobody marked"
    );
}

#[test]
fn one_word_is_a_run_of_one() {
    // The common case, and it needs no special path: clicking the same word
    // twice picks it and nothing else.
    let read = page(2, 4, &["מאימתי", "קורין", "את", "שמע"]);
    let (ink, was) = ink_of(&read, 1..2).expect("one word");
    assert_eq!(was, "קורין");
    assert_eq!(words_under(&read, &ink), vec![1]);
}

#[test]
fn the_mark_is_still_on_the_same_words_after_the_page_is_read_again() {
    // The property, and the whole argument for the ink. The page is read again
    // by a better engine: it finds a word the first pass missed, splits one in
    // two and spells another differently — so **every offset after the first
    // change points somewhere else**. An offset-anchored highlight would have
    // moved silently. This one is where it was put.
    let first = page(2, 4, &["מאימתי", "קורין", "את", "שמע"]);
    let (ink, was) = ink_of(&first, 1..3).expect("two words of the first line");
    assert_eq!(was, "קורין את");

    // The same page, read again. A word appears before the run — the engine
    // finally saw a letter it had skipped — so the run's words are now at 2..4
    // rather than 1..3, and one of them is spelled differently.
    let mut again = page(2, 4, &["מאימתי", "קורין", "את", "שמע"]);
    again.words.insert(
        0,
        Word {
            text: "ו".to_string(),
            // Ink of its own, to the right of everything: a speck the first
            // pass took for dirt.
            at: Area::new(0.99, 0.10, 1.0, 0.14),
            confidence: 0.4,
        },
    );
    again.words[2].text = "קוריו".to_string();

    let now = words_under(&again, &ink);
    let said: Vec<&str> = now
        .iter()
        .filter_map(|at| again.words.get(*at))
        .map(|word| word.text.as_str())
        .collect();
    assert_eq!(
        said,
        vec!["קוריו", "את"],
        "the same two places on the photograph, and the engine's new opinion of them"
    );
    assert_ne!(
        now,
        vec![1, 2],
        "which is a different pair of indices than the one it was made on"
    );
}

#[test]
fn a_range_that_is_not_on_the_page_is_nothing_rather_than_an_empty_mark() {
    let read = page(1, 3, &[]);
    assert!(
        ink_of(&read, 2..9).is_none(),
        "a run that runs off the end of the page names no words"
    );
    assert!(
        ink_of(&read, 1..1).is_none(),
        "and an empty range names none"
    );
}

#[test]
fn ink_from_a_page_nobody_has_read_covers_nothing() {
    // A reading thrown away and not replaced. The rectangle is still on the
    // photograph and there are no words to name — which is the honest answer,
    // and is what lets the window say *this is what it was made on* beside
    // nothing rather than beside a guess.
    let empty = Read::new(1, Reader::Embedded, Vec::new());
    let ink = vec![Area::new(0.1, 0.1, 0.9, 0.2)];
    assert!(words_under(&empty, &ink).is_empty());
}
