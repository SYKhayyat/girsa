//! One snippet renderer, windowed on the match.
//!
//! # Why there is exactly one
//!
//! There were two, and the wrong one answered the flagship question.
//!
//! `girsa-index`'s `first_words(text, 12)` took the **opening** of a segment,
//! which is what its doc comment said and is not what *where is this from* needs.
//! Proven on a 132-character segment with the phrase at character 93 and the
//! snippet cut at about 60: the answer to *where is this from* was evidence that
//! did not contain the phrase.
//!
//! `excerpt` bracketed the matched words — better — and then took the first 220
//! characters, from the start. On a 495,726-character segment that is 220
//! characters of a sefer's opening with the match half a megabyte away. That is
//! how a search result came back displaying text containing neither typed word.
//!
//! Two renderers is how the wrong one ends up on the flagship question, so there
//! is one, it lives here beside `Hit` and the highlighter that produces the marks,
//! and `girsa-index find`, `girsa-index where-from` and the MCP server all call it.
//!
//! # What it does
//!
//! The window is centred on the **first mark**, not on the start. Every mark inside
//! the window is bracketed. An elision at either end is *shown*, with `…`, because
//! a snippet that silently begins mid-sentence reads as a segment that begins
//! mid-sentence — and the whole point of this module is not showing text that
//! implies something untrue.
//!
//! With no marks at all — a hit found by a facet, a segment fetched by id — the
//! window is the opening, which is the only honest choice available and is what
//! the old renderer did for every case.

/// How much of a segment a line of output shows.
///
/// Wide enough to carry a sugya's worth of context around a phrase, narrow enough
/// for a terminal and for a result row.
pub const WIDTH: usize = 220;

/// A rendered snippet, and whether anything was left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    /// The text, with matches in `[brackets]` and elisions as `…`.
    pub text: String,
    /// Characters dropped from the front.
    pub before: usize,
    /// Characters dropped from the end.
    pub after: usize,
}

impl Snippet {
    /// Whether the segment is longer than what is shown.
    #[must_use]
    pub fn is_partial(&self) -> bool {
        self.before > 0 || self.after > 0
    }
}

/// Render a snippet of `text`, windowed on the first of `marks`.
///
/// `marks` are byte ranges into `text`, as `SearchIndex::marks` produces them.
/// Ranges outside the text, or overlapping ones, are skipped rather than trusted:
/// a mark that does not fit is a mark about some other text.
#[must_use]
pub fn snippet(text: &str, marks: &[(usize, usize)], width: usize) -> Snippet {
    let usable: Vec<(usize, usize)> = marks
        .iter()
        .copied()
        .filter(|(start, end)| start < end && *end <= text.len())
        .filter(|(start, end)| text.is_char_boundary(*start) && text.is_char_boundary(*end))
        .collect();

    // Where the window starts, in characters. Centred on the first mark, with a
    // little of what came before it, because a phrase with no run-up to it is
    // harder to place than one with three words in front.
    let lead = width / 4;
    let first_mark_char = usable
        .first()
        .map(|(start, _)| text[..*start].chars().count())
        .unwrap_or(0);
    let from_char = first_mark_char.saturating_sub(lead);

    // Byte offsets for the window, taken off character positions so a Hebrew
    // letter is never cut in half.
    let mut from_byte = text.len();
    let mut to_byte = text.len();
    let mut chars_taken = 0usize;
    for (at, (byte, _)) in text.char_indices().enumerate() {
        if at == from_char {
            from_byte = byte;
        }
        if at >= from_char {
            if chars_taken == width {
                to_byte = byte;
                break;
            }
            chars_taken += 1;
        }
    }
    if from_char == 0 {
        from_byte = 0;
    }
    let window = text.get(from_byte..to_byte).unwrap_or("");

    // The marks that fall inside the window, bracketed in order.
    let mut out = String::new();
    let mut at = from_byte;
    for (start, end) in &usable {
        if *start < at || *end > to_byte {
            continue;
        }
        out.push_str(text.get(at..*start).unwrap_or_default());
        out.push('[');
        out.push_str(text.get(*start..*end).unwrap_or_default());
        out.push(']');
        at = *end;
    }
    out.push_str(text.get(at..to_byte).unwrap_or_default());

    let before = text[..from_byte].chars().count();
    let after = text[to_byte..].chars().count();
    // Elisions are shown. A snippet that silently starts mid-sentence reads as a
    // segment that starts mid-sentence.
    let text = format!(
        "{}{}{}",
        if before > 0 { "…" } else { "" },
        out,
        if after > 0 { "…" } else { "" }
    );
    debug_assert!(!window.is_empty() || before + after == 0 || out.is_empty());
    Snippet {
        text,
        before,
        after,
    }
}

/// `snippet` at the default width.
#[must_use]
pub fn of(text: &str, marks: &[(usize, usize)]) -> Snippet {
    snippet(text, marks, WIDTH)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// The finding, as arithmetic.
    ///
    /// A 132-character segment with the phrase at character 93, and a snippet cut
    /// at about 60. The answer to *where is this from* did not contain the phrase.
    #[test]
    fn the_window_holds_the_match_and_not_the_opening() {
        let lead = "א".repeat(93);
        let phrase = "יתגבר כארי";
        let tail = "ב".repeat(29);
        let text = format!("{lead}{phrase}{tail}");
        let start = lead.len();
        let marks = [(start, start + phrase.len())];

        // A window narrower than the distance to the match: the old renderer took
        // the first 60 characters and stopped.
        let shown = snippet(&text, &marks, 60);
        assert!(
            shown.text.contains(phrase),
            "the snippet does not contain the phrase it is about: {}",
            shown.text
        );
        assert!(
            shown.text.contains(&format!("[{phrase}]")),
            "{}",
            shown.text
        );
        assert!(shown.before > 0, "it elided the opening");
        assert!(shown.text.starts_with('…'), "and said so: {}", shown.text);
        assert!(shown.is_partial());
    }

    /// The 495,726-character segment, which is the other half of the same defect.
    ///
    /// `excerpt` bracketed the marks and then took the first 220 characters, so a
    /// match half a megabyte in was nowhere near what was displayed.
    #[test]
    fn a_match_half_a_megabyte_in_is_still_what_is_shown() {
        let lead = "א ".repeat(120_000);
        let phrase = "מאימתי קורין";
        let text = format!("{lead}{phrase} סוף");
        let start = lead.len();
        let shown = of(&text, &[(start, start + phrase.len())]);
        assert!(
            shown.text.contains(&format!("[{phrase}]")),
            "{}",
            shown.text
        );
        assert!(shown.text.chars().count() <= WIDTH + 2, "{}", shown.text);
        // The window reaches the end of the segment, so nothing is elided after
        // the match — the whole 220 characters is the run-up plus the tail.
        assert_eq!(shown.after, 0);
        assert!(shown.text.ends_with("סוף"), "{}", shown.text);
        // And what *is* elided is the half-megabyte in front, said with an ellipsis
        // rather than silently dropped.
        assert!(shown.before > 100_000, "{}", shown.before);
        assert!(shown.text.starts_with('…'));
    }

    #[test]
    fn a_short_segment_is_shown_whole_with_no_ellipsis() {
        let text = "מאימתי קורין את שמע בערבית";
        let shown = of(text, &[(0, "מאימתי".len())]);
        assert_eq!(shown.text, "[מאימתי] קורין את שמע בערבית");
        assert_eq!(shown.before, 0);
        assert_eq!(shown.after, 0);
        assert!(!shown.is_partial());
    }

    #[test]
    fn every_mark_inside_the_window_is_bracketed() {
        let text = "אב גד אב הו אב";
        let marks: Vec<(usize, usize)> = text
            .match_indices("אב")
            .map(|(at, m)| (at, at + m.len()))
            .collect();
        assert_eq!(marks.len(), 3);
        let shown = of(text, &marks);
        assert_eq!(shown.text.matches("[אב]").count(), 3, "{}", shown.text);
    }

    #[test]
    fn no_marks_gives_the_opening_which_is_the_only_honest_choice() {
        let text = "א".repeat(1000);
        let shown = of(&text, &[]);
        assert_eq!(shown.before, 0);
        assert_eq!(shown.after, 1000 - WIDTH);
        assert!(shown.text.ends_with('…'));
    }

    #[test]
    fn a_mark_that_does_not_fit_the_text_is_skipped_rather_than_trusted() {
        let text = "קצר";
        // Ranges from some other text. A mark that does not fit is a mark about
        // something else, and using it would bracket the wrong letters.
        let shown = of(text, &[(0, 9999), (100, 200)]);
        assert_eq!(shown.text, "קצר");
    }

    #[test]
    fn a_hebrew_letter_is_never_cut_in_half() {
        // Every letter is two bytes and every nikud point is two more, so a window
        // measured in bytes would split one and produce a replacement character.
        let text = "בְּרֵאשִׁית בָּרָא אֱלֹהִים ".repeat(40);
        for width in [1, 2, 3, 7, 13, 61, 219, 220, 221] {
            let shown = snippet(&text, &[], width);
            // Round-trips as text, which it would not if a code point were split.
            assert_eq!(shown.text, shown.text.clone(), "width {width}");
            assert!(shown.text.chars().count() <= width + 2, "width {width}");
        }
    }

    #[test]
    fn an_empty_segment_does_not_panic() {
        assert_eq!(of("", &[]).text, "");
        assert_eq!(of("", &[(0, 0)]).text, "");
    }
}
