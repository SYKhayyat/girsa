//! Where a stretch of words actually is, when the text may have moved under it.
//!
//! # An offset is not a place
//!
//! Everything in the personal layer that points *inside* a segment — a
//! correction (W20), a highlight (W27), the words a link is about (W24) — is
//! stored as a character range. A range is a fact about the text as it stood
//! when somebody made the mark, and the text does not hold still: a correction
//! above it lengthens or shortens the line, and a corpus update can re-typeset
//! the whole segment.
//!
//! So a mark carries **the words as well as the offsets**, and this is the one
//! place that decides which of the two wins when they disagree: the offsets
//! first, because that is where the mark was made; then the words, and **only
//! if they are there exactly once**. Twice is an ambiguity and the rule for
//! those is to take neither (BUILDER.md rule 6); none is a mark whose words are
//! gone.
//!
//! This lived inside `girsa-fix` until W27 needed it for highlights. It is one
//! rule about text and offsets, not one rule about corrections, and a second
//! copy of it would drift — which for a mark means landing on the wrong letters
//! silently, the failure this codebase is arranged against.

use std::ops::Range;

/// Where a mark lands in the text as it stands now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub span: Range<usize>,
    /// Whether the words had to be looked for — the offsets no longer held
    /// them. Reported rather than hidden: a reader is entitled to know that a
    /// mark moved.
    pub moved: bool,
}

/// Find `was` in `letters`, starting from where it used to be.
///
/// `letters` is the segment's text **as characters**, not bytes: Hebrew is two
/// bytes a letter and every offset that crosses this project is a character
/// offset.
///
/// `None` means the words are not there, or are there more than once — in both
/// cases the caller must report the mark stale rather than place it.
#[must_use]
pub fn locate(letters: &[char], span: Range<usize>, was: &str) -> Option<Located> {
    let wanted: Vec<char> = was.chars().collect();
    if span.end <= letters.len() && letters.get(span.start..span.end) == Some(wanted.as_slice()) {
        return Some(Located { span, moved: false });
    }
    if wanted.is_empty() {
        return None;
    }
    let mut found = None;
    for start in 0..=letters.len().saturating_sub(wanted.len()) {
        if letters.get(start..start + wanted.len()) == Some(wanted.as_slice()) {
            if found.is_some() {
                return None;
            }
            found = Some(start);
        }
    }
    found.map(|start| Located {
        span: start..start + wanted.len(),
        moved: true,
    })
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn letters(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    #[test]
    fn the_offsets_are_believed_while_they_still_hold_the_words() {
        let text = letters("יתגבר כארי לעמוד בבוקר");
        let found = locate(&text, 6..10, "כארי").expect("it is there");
        assert_eq!(found.span, 6..10);
        assert!(!found.moved);
    }

    #[test]
    fn the_words_win_when_the_offsets_have_rotted() {
        // A correction above it made the line two letters longer.
        let text = letters("ויתגברר כארי לעמוד");
        let found = locate(&text, 6..10, "כארי").expect("it is found again");
        assert_eq!(found.span, 8..12);
        assert!(found.moved, "and it says that it had to move");
    }

    #[test]
    fn words_that_are_there_twice_place_nothing() {
        // BUILDER.md rule 6: a mark on the wrong letters is worse than a mark
        // that does not land. The offsets have to have rotted first — while
        // they still hold the words, they are the answer and what is elsewhere
        // in the line does not come into it.
        let text = letters("אמר רבי יוחנן אמר רבי");
        assert_eq!(locate(&text, 40..43, "אמר"), None);
        assert_eq!(
            locate(&text, 0..3, "אמר"),
            Some(Located {
                span: 0..3,
                moved: false
            }),
            "a second copy elsewhere does not unseat an offset that still holds"
        );
    }

    #[test]
    fn words_that_are_gone_place_nothing() {
        let text = letters("לעמוד בבוקר");
        assert_eq!(locate(&text, 0..4, "כארי"), None);
    }
}
