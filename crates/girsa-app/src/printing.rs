//! What goes on a sheet of paper.
//!
//! # Why an application that exports to `.docx` still needs this
//!
//! Girsa could put a sefer in a file and could not put it on paper. The answer
//! was *export to `.docx`, open Word, print from there* — three applications and
//! a file on a disk to produce the thing a bachur wants at seven in the
//! morning, which is the daf in his hand on the way to the shiur.
//!
//! Otzaria prints. It was the one place a person's ordinary morning ran into a
//! wall here and did not there.
//!
//! # What a sheet is
//!
//! **The section you are standing in**, and not the sefer. A siman, an amud, a
//! perek — whatever the address says the thing above this line is. That is what
//! somebody prints: nobody has ever wanted the Mishnah Berurah on paper, and
//! anybody who does has the export.
//!
//! Found by the address rather than by counting lines, which is the same rule
//! as everywhere else in this repository: `31a:4` and `31a:11` are the same amud
//! because they say so, not because they are near each other in a file. A
//! sefer that is re-segmented tomorrow prints the same amud.
//!
//! # And what goes on it besides the words
//!
//! The edition and the licence. spec.md §13 asks every text to carry them and a
//! file leaving the application is the one place that has to be true outside it
//! — a printed page is a file leaving the application by another road.
//! `girsa-export` puts the same four lines at the top of a `.docx` and this is
//! the same header, said the same way.

use girsa_corpus::segment::SegmentId;

use crate::Open;

/// The half-open run of segments a sheet covers: **the section the reader is
/// standing in**.
///
/// There used to be a second answer here — a `Sheet::Chosen` that printed the
/// one line named — behind a `whole:` flag on the shell's command whose name
/// meant the opposite of what it did, and whose only caller passed `false`.
/// Both lies and the branch they guarded are gone rather than renamed: nothing
/// reached the one-line path, and a real print-a-highlight will want an
/// explicit span, which a flag naming one segment never carried.
///
/// `None` when the sefer does not have that place, which is a caller asking
/// about a line that is not here — never the nearest thing.
#[must_use]
pub fn run_of(sefer: &Open, at: &SegmentId) -> Option<(usize, usize)> {
    let here = sefer.position_of(at)?;
    // The address one level up. A line at the top level of its sefer — a work
    // addressed `1`, `2`, `3` with nothing above it — is its own section, which
    // is the honest answer rather than *the whole sefer*.
    let path = at.path();
    let section = &path[..path.len().saturating_sub(1)];
    if section.is_empty() {
        return Some((here, here + 1));
    }
    let under = |nth: usize| -> bool {
        sefer
            .segments
            .get(nth)
            .is_some_and(|segment| segment.id.path().starts_with(section))
    };
    let mut from = here;
    while from > 0 && under(from - 1) {
        from -= 1;
    }
    let mut to = here + 1;
    while under(to) {
        to += 1;
    }
    Some((from, to))
}

/// The four lines at the head of a sheet: which sefer, from where, and under
/// what terms.
///
/// The same four `girsa_export::header` writes, and for the same reason. Kept
/// separate rather than shared with it because that one builds a `.docx` header
/// and this one is read by a window — the words agree, the shapes do not.
#[must_use]
pub fn header(sefer: &Open) -> Vec<String> {
    let mut out = vec![sefer.work.he_title.clone()];
    if !sefer.work.en_title.is_empty() && sefer.work.en_title != sefer.work.he_title {
        out.push(sefer.work.en_title.clone());
    }
    if let Some(version) = sefer.work.version.as_ref() {
        out.push(version.edition.clone());
        if let Some(license) = version.license.as_deref() {
            out.push(license.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::import::{Segment, SegmentKind};
    use girsa_corpus::segment::Ordinal;

    /// A sefer whose segments are addressed exactly as the paths given.
    ///
    /// Built through `Open::new` and not by replacing the segments of one that
    /// already exists: an `Open` indexes its own ids when it is made, and a
    /// sefer whose segments were swapped underneath it answers `None` to every
    /// question about where a line is. Which is what the first version of this
    /// fixture did, and four tests said so.
    fn sefer(paths: &[&[&str]]) -> Open {
        let work = crate::pretend::sefer("s", "ספר", &["דף", "שורה"], &[]).work;
        let segments = paths
            .iter()
            .enumerate()
            .map(|(i, path)| {
                #[allow(clippy::cast_possible_truncation)]
                let n = i as u32 + 1;
                Segment {
                    id: SegmentId::new(
                        "s",
                        path.iter().map(|p| (*p).to_string()).collect(),
                        Ordinal::root(n),
                    ),
                    kind: SegmentKind::Text,
                    text: path.join(":"),
                    anchors: Vec::new(),
                }
            })
            .collect();
        Open::new(work, segments)
    }

    #[test]
    fn a_sheet_is_the_amud_and_not_the_lines_near_it() {
        let sefer = sefer(&[
            &["30b", "9"],
            &["31a", "1"],
            &["31a", "2"],
            &["31a", "3"],
            &["31b", "1"],
        ]);
        let at = sefer.segments[2].id.clone();
        assert_eq!(run_of(&sefer, &at), Some((1, 4)));
        // Standing on the first line of the amud gives the same amud, not the
        // one before it.
        let first = sefer.segments[1].id.clone();
        assert_eq!(run_of(&sefer, &first), Some((1, 4)));
    }

    #[test]
    fn a_siman_three_levels_deep_is_still_one_section() {
        // A branch work: the chelek is part of the address and a siman is the
        // level above a se'if, so a sheet is the se'ifim of one siman and does
        // not run on into the next.
        let sefer = sefer(&[
            &["orach_chayim", "1", "1"],
            &["orach_chayim", "1", "2"],
            &["orach_chayim", "2", "1"],
        ]);
        let at = sefer.segments[0].id.clone();
        assert_eq!(run_of(&sefer, &at), Some((0, 2)));
    }

    #[test]
    fn a_flat_sefer_has_nothing_above_a_line_and_says_so() {
        // Every level of the address is the line itself. *The section above
        // this* is the whole sefer, and printing the whole sefer because
        // somebody pressed the print key is not an answer — so it is the line.
        let sefer = sefer(&[&["1"], &["2"], &["3"]]);
        let at = sefer.segments[1].id.clone();
        assert_eq!(run_of(&sefer, &at), Some((1, 2)));
    }

    #[test]
    fn a_line_this_sefer_does_not_have_is_refused_and_not_rounded() {
        let sefer = sefer(&[&["31a", "1"]]);
        // A different ordinal as well as a different address. `position_of`
        // falls back to *what this id was cut into*, which is a question about
        // the ordinal — so an id with the same `#1` on a different daf is a
        // fair question and this is not asking it.
        let elsewhere = SegmentId::new("s", vec!["99a".into(), "1".into()], Ordinal::root(99));
        assert_eq!(run_of(&sefer, &elsewhere), None);
    }

    #[test]
    fn the_header_carries_the_edition_and_the_terms() {
        let sefer = crate::pretend::shulchan_arukh();
        let head = header(&sefer);
        assert!(head[0].contains("שולחן ערוך"), "{head:?}");
        assert!(
            head.iter().any(|line| line.contains("Lemberg")),
            "the printed edition: {head:?}"
        );
        assert!(
            head.iter().any(|line| line.contains("Public Domain")),
            "the terms: {head:?}"
        );
    }
}
