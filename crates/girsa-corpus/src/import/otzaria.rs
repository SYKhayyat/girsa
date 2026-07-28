//! Reading an Otzaria work: one segment per line, structure from the headings.
//!
//! This runs for the **978 works Sefaria does not have** (spec.md §2.3) —
//! גליוני הש"ס, אבני נזר, קרן אורה, חידושי הראב"ד, מהר"ם שיק,
//! שו"ת ישועות מלכו — the acharonim layer, which is disproportionately the
//! learning material. It does not run for anything Sefaria has: §2.3b and
//! decision 1 give those to Sefaria, text and structure both.
//!
//! # The format
//!
//! UTF-8, **one segment per line**, structure inline as HTML: `<h1>` the book,
//! `<h2>` a chapter or a daf, `<h3>` a siman. `<big><strong>` inside a line
//! marks the dibbur hamaschil, and is kept — it is information, and spec.md
//! §4.1 makes the text the truth rather than a rendering of it.
//!
//! # How a line gets an address
//!
//! A heading opens a section and **is** that section's address:
//! `<h3>סימן א</h3>` is `1`. The lines under it are numbered from one, so
//! `girsa:mishnah-berurah/1:2` is the second thing said in siman א — which is
//! how it would be cited.
//!
//! Two cases that look like edge cases and are not, because they are in the
//! very first sefer you open:
//!
//! - **Front matter.** The `<h1>` and the author's name sit above every
//!   heading. They are section `0`, which is not a siman anybody cites and
//!   cannot collide with one.
//! - **Empty headings.** T8, and real: Mishnah Berurah has a literal
//!   `<h2></h2>` on line 28, immediately before its 697 simanim. It names no
//!   section, so it opens none — it closes the one above it and is recorded as
//!   an ordinary line, which keeps the file's 18,120 lines and 701 headings
//!   both true.

use std::fs;

use super::{ImportError, RawSegment, SegmentKind};
use crate::work::Work;

/// Read one Otzaria `.txt` into segments, in reading order.
///
/// # Errors
///
/// If the file cannot be read. Nothing about its *contents* is an error: this
/// is a conversion of a conversion and every shape in it has to be tolerated.
pub fn read(work: &Work) -> Result<Vec<RawSegment>, ImportError> {
    let body = fs::read_to_string(&work.origin).map_err(ImportError::io(&work.origin))?;
    Ok(parse(&body))
}

/// The label of the section that holds everything above the first heading.
///
/// `0` rather than `1`, so it cannot be confused with siman א — which it would
/// be, constantly, since the first heading of most seforim opens siman א.
const FRONT_MATTER: &str = "0";

/// A section a heading opened.
#[derive(Debug)]
struct Open {
    level: u8,
    label: String,
    /// Labels its own sub-sections have taken, so two `<h3>סימן א</h3>` under
    /// one parent do not end up sharing an address.
    children: Vec<String>,
    /// How many lines have been numbered inside it. Kept per section, so a
    /// section that closes and hands back to its parent does not restart the
    /// parent's numbering and give two lines the same address.
    lines: usize,
}

#[must_use]
pub fn parse(body: &str) -> Vec<RawSegment> {
    parse_with_lines(body)
        .into_iter()
        .map(|(_, segment)| segment)
        .collect()
}

/// The same, keeping the 1-based line each segment came from.
///
/// **This is the only place a line number is allowed to exist, and it does not
/// leave.** Otzaria's link files address both ends as `file + line_index`, so
/// importing them means translating that addressing into ours exactly once
/// (W8) — and the translation needs the mapping.
///
/// It is deliberately not a field on [`RawSegment`], and nothing writes it to
/// disk. W6's acceptance is that no line number is persisted as a durable
/// reference; a link importer that recomputes the mapping from the source file
/// each run keeps that true, because the number exists for the length of one
/// function and is then gone.
///
/// Blank lines are skipped, so the *n*th segment is generally not line *n* —
/// which is exactly why this has to be returned rather than assumed.
#[must_use]
pub fn parse_with_lines(body: &str) -> Vec<(usize, RawSegment)> {
    let mut out = Vec::new();
    // The section above every heading. It is section `0`, and it exists even
    // when nothing is in it, so the code below never has a special case.
    let mut front = Open {
        level: 0,
        label: FRONT_MATTER.to_string(),
        children: Vec::new(),
        lines: 0,
    };
    // The open sections, outermost first, so a deeper heading nests inside the
    // current one and a shallower one closes it.
    let mut open: Vec<Open> = Vec::new();

    for (n, line) in body.lines().enumerate() {
        // Otzaria's link files count from 1.
        let line_number = n + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // `<h1>` is the book, and the book is the work — it names no section
        // inside itself, so it is read as an ordinary line of front matter.
        let parsed = heading(line);
        let mut opened = false;
        if let Some((level, label)) = &parsed {
            if *level > 1 {
                // A heading closes everything at its level or deeper, whether
                // or not it goes on to open something. An empty one — T8, and
                // Mishnah Berurah has one right before its 697 simanim — only
                // closes.
                while open.last().is_some_and(|s| s.level >= *level) {
                    open.pop();
                }
                if !label.is_empty() {
                    let label = {
                        let parent = open.last_mut().unwrap_or(&mut front);
                        let label = disambiguate(label.clone(), &parent.children);
                        parent.children.push(label.clone());
                        label
                    };
                    open.push(Open {
                        level: *level,
                        label,
                        children: Vec::new(),
                        lines: 0,
                    });
                    opened = true;
                }
            }
        }

        if opened {
            out.push((
                line_number,
                RawSegment {
                    path: section_path(&open),
                    kind: SegmentKind::Heading,
                    text: strip_tags(line),
                },
            ));
            continue;
        }

        let index = {
            let section = open.last_mut().unwrap_or(&mut front);
            section.lines += 1;
            section.lines.to_string()
        };
        let mut path = if open.is_empty() {
            vec![FRONT_MATTER.to_string()]
        } else {
            section_path(&open)
        };
        path.push(index);
        out.push((
            line_number,
            RawSegment {
                path,
                kind: if parsed.is_some() {
                    SegmentKind::Heading
                } else {
                    SegmentKind::Text
                },
                text: line.to_string(),
            },
        ));
    }
    out
}

fn section_path(open: &[Open]) -> Vec<String> {
    open.iter().map(|s| s.label.clone()).collect()
}

/// A second `סימן א` in one file becomes `1_2`, so no two sections share an
/// address. Rare, and silent collisions would make one of them unreachable.
///
/// `_` and not `-`, for the reason [`crate::work::section_label_of`] gives: a
/// hyphen in an address level is how `girsa-ref` writes a span.
fn disambiguate(label: String, taken: &[String]) -> String {
    if !taken.contains(&label) {
        return label;
    }
    for n in 2..=u32::MAX {
        let candidate = format!("{label}_{n}");
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    label
}

/// `<h3>סימן ג</h3>` → `(3, "3")`. Anything else → `None`.
fn heading(line: &str) -> Option<(u8, String)> {
    let rest = line.strip_prefix("<h")?;
    let level = rest.chars().next()?.to_digit(10)? as u8;
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = rest.get(1..)?.strip_prefix('>')?;
    let close = format!("</h{level}>");
    let inner = rest.strip_suffix(&close)?;
    Some((level, section_label(inner)))
}

/// Words that name a division rather than being part of its name.
///
/// **Written in their normal form**, final letters folded — `סעיף` normalizes
/// to `סעיפ`, and a list holding the printed spelling never matches.
const SECTION_WORDS: [&str; 14] = [
    "סימנ",
    "סעיפ",
    "פרק",
    "הלכה",
    "משנה",
    "דפ",
    "אות",
    "פסוק",
    "מזמור",
    "שער",
    "חלק",
    "מסכת",
    "פרשה",
    "עמוד",
];

/// The address a heading names.
///
/// `סימן ג` is siman 3, so the address is `3` — which is what a citation into
/// it says. Anything the section-word list does not recognise keeps its whole
/// name: `הקדמה` stays `הקדמה`.
///
/// **The section word is required** before a numeral is read out of a heading.
/// Every Hebrew word is a number if you insist — `הקדמה` sums to 154 — and a
/// heading read as a number it never was is a section nobody can cite and a
/// link that lands on the wrong page.
fn section_label(inner: &str) -> String {
    let inner = strip_tags(inner);
    let words: Vec<&str> = inner.split_whitespace().collect();
    if let Some(first) = words.first() {
        let normalized = girsa_hebrew::normalize(first);
        let bare = normalized.trim_end_matches(['\'', '"']);
        if SECTION_WORDS.contains(&bare) && words.len() == 2 {
            if let Some(n) = girsa_ref::numerals::parse(words[1]) {
                return n.to_string();
            }
        }
    }
    crate::work::section_label_of(&inner)
}

/// Take the markup off, keeping the words.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn addressed(body: &str) -> Vec<(String, String)> {
        parse(body)
            .into_iter()
            .map(|s| (s.path.join(":"), s.text))
            .collect()
    }

    #[test]
    fn a_siman_is_addressed_the_way_it_is_cited() {
        let got = addressed(
            "<h3>סימן א</h3>\n\
             ראשון\n\
             שני\n\
             <h3>סימן ב</h3>\n\
             שלישי\n",
        );
        assert_eq!(
            got.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
            ["1", "1:1", "1:2", "2", "2:1"]
        );
    }

    #[test]
    fn front_matter_cannot_be_mistaken_for_siman_alef() {
        // The `<h1>` and the author's name sit above every heading. Numbered
        // from 1 at the top level they would take siman א's address.
        let got = addressed(
            "<h1>משנה ברורה</h1>\n\
             רבי ישראל מאיר הכהן\n\
             <h3>סימן א</h3>\n\
             ראשון\n",
        );
        assert_eq!(
            got.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
            ["0:1", "0:2", "1", "1:1"]
        );
    }

    #[test]
    fn an_empty_heading_closes_a_section_without_opening_one() {
        // T8, and it is on line 28 of Mishnah Berurah — immediately before the
        // 697 simanim. Opening a nameless section here would bury every one of
        // them a level deeper than it is cited at.
        let got = addressed(
            "<h2>הקדמה</h2>\n\
             פתיחה\n\
             <h2></h2>\n\
             <h3>סימן א</h3>\n\
             ראשון\n",
        );
        assert_eq!(
            got.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
            ["הקדמה", "הקדמה:1", "0:1", "1", "1:1"]
        );
        assert_eq!(parse("<h2></h2>").len(), 1, "and it is still a line");
    }

    #[test]
    fn a_heading_that_is_not_a_numbered_section_keeps_its_name() {
        // `הקדמה` sums to 154 as a numeral. A resolver that read it as one
        // would put the introduction at siman 154.
        let got = addressed("<h2>הקדמה</h2>\nפתיחה\n");
        assert_eq!(got[0].0, "הקדמה");
    }

    #[test]
    fn two_sections_with_the_same_name_do_not_share_an_address() {
        let got = addressed("<h3>סימן א</h3>\nראשון\n<h3>סימן א</h3>\nשני\n");
        assert_eq!(
            got.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
            ["1", "1:1", "1_2", "1_2:1"]
        );
    }

    #[test]
    fn the_dibbur_hamaschil_markup_survives_the_import() {
        // spec.md §4.1 makes the text the truth rather than a rendering of it,
        // and `<big><strong>` is where the dibbur hamaschil is.
        let got = addressed("<h3>סימן א</h3>\n<big><strong>מאימתי</strong></big> קורין\n");
        assert_eq!(got[1].1, "<big><strong>מאימתי</strong></big> קורין");
    }

    #[test]
    fn the_nth_segment_is_not_the_nth_line_and_the_mapping_says_so() {
        // Blank lines are skipped, so a link addressed as "line 4" is not the
        // fourth segment. W8 translates Otzaria's line addressing into ours
        // exactly once, and this is the table it does it with — recomputed from
        // the file each run and never written down (W6).
        let mapped: Vec<(usize, String)> = parse_with_lines(
            "<h3>סימן א</h3>\n\
             \n\
             ראשון\n\
             \n\
             \n\
             שני\n",
        )
        .into_iter()
        .map(|(line, s)| (line, s.path.join(":")))
        .collect();
        assert_eq!(
            mapped,
            [
                (1, "1".to_string()),
                (3, "1:1".to_string()),
                (6, "1:2".to_string()),
            ]
        );
    }

    #[test]
    fn headings_are_counted_as_headings() {
        let segments = parse("<h1>ספר</h1>\nשורה\n<h3>סימן א</h3>\nשורה\n");
        let headings = segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Heading)
            .count();
        assert_eq!(segments.len(), 4);
        assert_eq!(headings, 2);
    }
}
