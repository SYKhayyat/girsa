//! Correcting a typo from where you are reading.
//!
//! spec.md §7.5, BUILDER.md W20: *if correcting a typo is not a three-second
//! interaction from where you are reading, nobody does it — including you.*
//! That is a requirement, and this module is what it costs on this side: one
//! call, taking what the pane already has.
//!
//! ```text
//! highlight  →  correction(sefer, at, 16..20, "הדבר", …)  →  a patch
//!               ↑ what the pane already has     ↑ what they typed
//! ```
//!
//! There is no dialog in that, no navigation, and nothing for the reader to
//! look up. `tests/three_seconds.rs` measures what is left.
//!
//! # The two coordinate systems, and why neither of them is the file
//!
//! The window counts a highlight in characters of **what it drew** — markup
//! off, nikud applied, and corrections already in place. A patch names
//! characters of **the segment on disk**. Between them sit two translations:
//!
//! 1. [`crate::display::Shown`] — what the markup and the nikud toggle took out;
//! 2. [`girsa_fix::Corrected::base_span`] — what the corrections already on this
//!    line put in.
//!
//! Neither is arithmetic anyone should do by hand, and both are recorded by the
//! code that did the taking out and the putting in.

use girsa_corpus::segment::SegmentId;
use girsa_fix::{Kind, Patch};

use crate::display::Shown;
use crate::shelf::Open;

/// Why a highlight could not be turned into a correction.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FixHere {
    #[error("{0} is not in this sefer")]
    NotHere(String),
    /// A highlight of nothing. There is no correction to make, and inventing a
    /// span from the caret would correct a letter nobody pointed at.
    #[error("nothing is selected")]
    Empty,
    /// The reader highlighted words a correction already put there. Taking the
    /// first one back is the answer; guessing what the file underneath says is
    /// not.
    #[error("there is already a correction here — it reads {now} for {was}")]
    AlreadyCorrected { was: String, now: String },
    #[error("a correction has to change something")]
    Changes,
}

/// The patch a reader makes by highlighting words and typing what they should
/// read.
///
/// The offsets are the ones the pane reports for a selection — the same ones
/// Ctrl+C uses (`girsa_app::Selection`), so a reader who can copy a phrase can
/// correct one.
///
/// # Errors
///
/// If the segment is not in this sefer, nothing is highlighted, the highlight
/// runs across a correction that is already there, or it changes nothing.
pub fn correction(
    sefer: &Open,
    at: &SegmentId,
    highlighted: std::ops::Range<usize>,
    now: &str,
    kind: Kind,
    who: &str,
    nikud: bool,
) -> Result<Patch, FixHere> {
    let (from_char, to_char) = (highlighted.start, highlighted.end);
    let position = sefer
        .position_of(at)
        .ok_or_else(|| FixHere::NotHere(at.to_string()))?;
    let segment = sefer
        .segments
        .get(position)
        .ok_or_else(|| FixHere::NotHere(at.to_string()))?;

    // What the pane drew, which is the corrected text with the markup off.
    let drawn = Shown::of(&segment.text, nikud);
    let on_screen = drawn.base_span(from_char, to_char).ok_or(FixHere::Empty)?;

    // …and where that is in the file, which is a different question the moment
    // this line has a correction on it already.
    let printed = sefer.as_printed(at);
    let span = match sefer.correction(at) {
        Some(corrected) => corrected
            .base_span(on_screen.start, on_screen.end)
            .ok_or_else(
                || match corrected.covering(on_screen.start, on_screen.end) {
                    Some(applied) => FixHere::AlreadyCorrected {
                        was: applied.was.clone(),
                        now: applied.now.clone(),
                    },
                    None => FixHere::Empty,
                },
            )?,
        None => on_screen,
    };

    let letters: Vec<char> = printed.chars().collect();
    let was: String = letters
        .get(span.clone())
        .ok_or(FixHere::Empty)?
        .iter()
        .collect();
    if was.is_empty() {
        return Err(FixHere::Empty);
    }
    if was == now {
        return Err(FixHere::Changes);
    }
    Ok(Patch::new(at.clone(), span, was, now, kind, who))
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::import::{Segment, SegmentKind};
    use girsa_corpus::segment::Ordinal;
    use girsa_fix::{Layer, Showing};

    const AS_PRINTED: &str = "<b>כל הרבר</b> וּבַשַּׁבָּת הזה";

    fn id() -> SegmentId {
        SegmentId::new(
            "mishnah-berurah",
            vec!["1".into(), "1".into()],
            Ordinal::root(1),
        )
    }

    fn sefer(layer: &Layer) -> Open {
        let segments = vec![Segment {
            id: id(),
            kind: SegmentKind::Text,
            text: AS_PRINTED.to_string(),
        }];
        Open::corrected(
            crate::shelf::tests::work("mishnah-berurah"),
            segments,
            layer,
            Showing::Fixed,
        )
    }

    #[test]
    fn a_highlight_of_a_word_with_nikud_corrects_the_letters_under_it() {
        // Nikud off, so the reader sees `ובשבת` — five characters where the
        // file has thirteen. The patch has to name the thirteen.
        let sefer = sefer(&Layer::nowhere());
        let drawn = Shown::of(AS_PRINTED, false);
        assert_eq!(drawn.text(), "כל הרבר ובשבת הזה");

        let patch = correction(&sefer, &id(), 8..13, "ובשבתות", Kind::Ocr, "me", false)
            .expect("a correction");
        assert_eq!(patch.was, "וּבַשַּׁבָּת");
        assert_eq!(patch.now, "ובשבתות");

        let mut layer = Layer::nowhere();
        layer.add(patch).expect("takes it");
        assert_eq!(
            layer.apply(&id(), AS_PRINTED, Showing::Fixed).text,
            "<b>כל הרבר</b> ובשבתות הזה"
        );
    }

    #[test]
    fn a_second_correction_on_the_same_line_still_names_the_file() {
        let mut layer = Layer::nowhere();
        layer
            .add(
                correction(
                    &sefer(&Layer::nowhere()),
                    &id(),
                    3..7,
                    "הדבר",
                    Kind::Ocr,
                    "me",
                    false,
                )
                .expect("a correction"),
            )
            .expect("takes it");

        // The pane is now drawing `כל הדבר ובשבת הזה`, and the reader corrects
        // the last word. Counted against the screen it is 14..17; in the file
        // it is somewhere else entirely, and both differences — the nikud and
        // the correction — are in play at once.
        let sefer = sefer(&layer);
        assert_eq!(
            Shown::of(&sefer.segments[0].text, false).text(),
            "כל הדבר ובשבת הזה"
        );
        let patch = correction(&sefer, &id(), 14..17, "ההוא", Kind::Ocr, "me", false)
            .expect("a correction");
        assert_eq!(patch.was, "הזה");
        layer.add(patch).expect("takes it");
        assert_eq!(
            Shown::of(&layer.apply(&id(), AS_PRINTED, Showing::Fixed).text, false).text(),
            "כל הדבר ובשבת ההוא"
        );
    }

    #[test]
    fn highlighting_a_word_a_correction_put_there_says_so_rather_than_guessing() {
        let mut layer = Layer::nowhere();
        layer
            .add(
                correction(
                    &sefer(&Layer::nowhere()),
                    &id(),
                    3..7,
                    "הדבר",
                    Kind::Ocr,
                    "me",
                    false,
                )
                .expect("a correction"),
            )
            .expect("takes it");

        let refused = correction(
            &sefer(&layer),
            &id(),
            3..7,
            "הדברים",
            Kind::Ocr,
            "me",
            false,
        );
        assert!(
            matches!(refused, Err(FixHere::AlreadyCorrected { .. })),
            "{refused:?}"
        );
    }

    #[test]
    fn a_highlight_of_nothing_is_refused() {
        let sefer = sefer(&Layer::nowhere());
        assert_eq!(
            correction(&sefer, &id(), 4..4, "x", Kind::Ocr, "me", true),
            Err(FixHere::Empty)
        );
        let elsewhere = SegmentId::new("bavli/berakhot", vec!["2a".into()], Ordinal::root(1));
        assert!(matches!(
            correction(&sefer, &elsewhere, 0..2, "x", Kind::Ocr, "me", true),
            Err(FixHere::NotHere(_))
        ));
    }
}
