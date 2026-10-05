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
//!
//! # `Anchored`, and why it is here rather than next to the mark
//!
//! [`Anchored`] is that rule and [`crate::standing::Standing`] together, and it
//! is what **every** stored span in this repository is written down as: which
//! segment, which characters of it, and which words those characters were.
//!
//! It is the pair issue #43 asks a commentary to carry — a `data-target` and a
//! `data-range` — and it lives here for the same reason [`locate`] does. A
//! correction asks *which words*; a highlight asks *which words*; a link's span
//! (§8.4) asks *which words*. Three places that each rolled their own answer
//! were three places a span could land on the wrong letters, and one of them —
//! the link's — was also answering *which segment* by ordinal prefix, which is
//! the one question [`crate::standing`] was written to stop anybody asking.

use std::ops::Range;

use crate::segment::SegmentId;
use crate::standing::Standing;

/// A range of one segment, and the words that range named.
///
/// **The words are not a nicety.** `span` alone is a position in a text that
/// does not hold still, and applying it to a text that moved puts a
/// highlight, a correction or a commentary on letters nobody chose. [`place`]
/// is the one place that decides, and it is [`locate`]'s rule under a name that
/// says which segment is being asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchored {
    /// The permanent name of the segment the words are in.
    ///
    /// **Not an address, and not checked against one.** The path is descriptive
    /// (`SegmentId::path`); the ordinal is the durable name, and a re-import
    /// that re-sections a work changes the path and keeps the ordinal.
    pub at: SegmentId,
    /// Where the words were, in characters of the text as it stood.
    ///
    /// Believed first, because that is where the mark was made. Believed
    /// *only* while they still hold the words — see [`place`].
    pub span: Range<usize>,
    /// The words themselves, so they can be found again.
    ///
    /// Empty means **nobody recorded them**, which is a record written before
    /// this field existed. Such a mark is placed by its offsets and nothing
    /// else, because its offsets are all it ever had — see [`place`].
    pub was: String,
}

impl Anchored {
    /// A span and the words it names.
    #[must_use]
    pub fn new(at: SegmentId, span: Range<usize>, was: impl Into<String>) -> Self {
        Self {
            at,
            span,
            was: was.into(),
        }
    }

    /// Where these words are now, or nothing.
    ///
    /// Two questions, and the order they are asked in is the whole of it:
    ///
    /// * **Which segment?** [`Standing::named_by`] — a cut hands its name to its
    ///   pieces, an *insertion* does not, and an upstream merge is a redirect
    ///   row rather than a descent. `SegmentId::covers` cannot tell those three
    ///   apart and answers yes to two different situations at once, which is
    ///   `girsa_corpus::standing`'s entire subject.
    /// * **Which words?** [`locate`] — the offsets while they still hold the
    ///   words, and the words alone if they have rotted, and **nothing** if the
    ///   words are there twice.
    ///
    /// `None` is an answer, not a failure: a mark whose words are gone says so
    /// rather than landing somewhere adjacent, and `moved` says when a mark had
    /// to be looked for again.
    #[must_use]
    pub fn place(&self, at: &Standing, text: &str) -> Option<Located> {
        if !at.named_by(&self.at) {
            return None;
        }
        if self.was.is_empty() {
            // A record written before the words were recorded. Its offsets are
            // all it has, so they are what it is placed by — and only if they
            // are a span of this text at all, which is the check that stops an
            // offset from a longer line running off the end of a shorter one.
            let chars = text.chars().count();
            return (self.span.end <= chars).then(|| Located {
                span: self.span.clone(),
                moved: false,
            });
        }
        locate(
            &text.chars().collect::<Vec<char>>(),
            self.span.clone(),
            &self.was,
        )
    }
}

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
    use crate::segment::Ordinal;

    fn letters(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    fn seif(n: u32) -> SegmentId {
        SegmentId::new(
            "shulchan-arukh/orach-chayim",
            vec!["1".into()],
            Ordinal::root(n),
        )
    }

    /// One place, and no history: the whole corpus as it stands today.
    fn here(at: &SegmentId) -> Standing {
        let live: std::collections::BTreeSet<SegmentId> = [at.clone()].into();
        Standing::derived(at, |id| live.contains(id), |_| Vec::new())
    }

    // ------------------------------------------------------------ Anchored

    #[test]
    fn a_mark_whose_offsets_still_hold_its_words_is_where_it_was_left() {
        let at = seif(7);
        let mark = Anchored::new(at.clone(), 6..10, "כארי");
        let found = mark
            .place(&here(&at), "יתגבר כארי לעמוד")
            .expect("it is there");
        assert_eq!(found.span, 6..10);
        assert!(!found.moved);
    }

    #[test]
    fn a_mark_is_found_again_after_a_correction_above_it_moves_every_character() {
        // The whole of `was`. `יתגבר` became `ויתגבר` — one character longer, so
        // `6..10` is now the space and three letters of `כאר`.
        let at = seif(7);
        let mark = Anchored::new(at.clone(), 6..10, "כארי");
        let found = mark
            .place(&here(&at), "ויתגבר כארי לעמוד")
            .expect("found again");
        assert_eq!(found.span, 7..11, "and it says that it had to move");
        assert!(found.moved);
    }

    #[test]
    fn a_mark_whose_words_are_there_twice_places_nothing() {
        // BUILDER.md rule 6, and the failure is invisible: a highlight on the
        // wrong letters looks exactly like a highlight on the right ones.
        let at = seif(7);
        let mark = Anchored::new(at.clone(), 40..43, "אמר");
        assert!(mark.place(&here(&at), "אמר רבי יוחנן אמר רבי").is_none());
    }

    #[test]
    fn a_mark_belonging_to_another_seif_places_nothing() {
        // The question `SegmentId::covers` answers wrongly, asked the right way.
        // `#1.1` is what a cut of `#1` mints and what upstream inserting a se'if
        // after `#1` is named; only the first is `#1`'s words, and what tells
        // them apart is that an insertion leaves the parent **live**.
        let parent = seif(1);
        let inserted = seif(1).split(2).remove(0);
        assert!(
            parent.covers(&inserted),
            "descent says yes — which is why not"
        );
        let live: std::collections::BTreeSet<SegmentId> = [parent.clone(), inserted.clone()].into();
        let standing = Standing::derived(&inserted, |id| live.contains(id), |_| Vec::new());
        let mark = Anchored::new(parent.clone(), 0..3, "יתגבר");
        assert!(
            mark.place(&standing, "יתגבר כארי").is_none(),
            "a mark on se'if 1 was placed on a se'if inserted beside it"
        );
        // And the same name, where the parent is **gone**, is that parent's words.
        let cut: std::collections::BTreeSet<SegmentId> = [inserted.clone()].into();
        let carved = Standing::derived(&inserted, |id| cut.contains(id), |_| Vec::new());
        assert!(
            mark.place(&carved, "יתגבר כארי").is_some(),
            "and a mark on a parent that was cut up does place on its pieces"
        );
    }

    #[test]
    fn a_mark_written_before_the_words_were_recorded_is_placed_by_its_offsets() {
        // Such a record has offsets and nothing else, and the honest answer is
        // to place it by them. What it must never do is run off the end of a
        // line that has since got shorter.
        let at = seif(7);
        let mark = Anchored::new(at.clone(), 6..10, "");
        let found = mark
            .place(&here(&at), "יתגבר כארי לעמוד")
            .expect("the offsets hold");
        assert_eq!(found.span, 6..10);
        assert!(!found.moved, "and it does not claim to have moved");
        assert!(
            mark.place(&here(&at), "כארי").is_none(),
            "an offset from a longer line is not a span of a shorter one"
        );
    }

    #[test]
    fn a_cut_hands_its_name_to_its_pieces_and_a_pin_on_it_still_lands() {
        let parent = seif(7);
        let pieces = parent.split(2);
        let live: std::collections::BTreeSet<SegmentId> = pieces.iter().cloned().collect();
        let mark = Anchored::new(parent, 6..10, "כארי");
        for piece in &pieces {
            let standing = Standing::derived(piece, |id| live.contains(id), |_| Vec::new());
            let found = mark
                .place(&standing, "יתגבר כארי לעמוד")
                .expect("its words are here");
            assert_eq!(found.span, 6..10);
        }
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
