//! The table of contents of one sefer — what it is made of, in order.
//!
//! # The finding
//!
//! > *"there should be a table of contents on the side for each sefer, so you
//! > can jump around."* And: *"their toc and siman and daf heading and search
//! > within sefer is easy to steal and much better than ours."*
//!
//! Otzaria has one (`lib/text_book/view/toc_navigator_screen.dart`) and builds
//! it by scanning the text for `<h1>`…`<h6>` lines, hanging each under the last
//! heading a level above it. That works because its corpus is HTML with heading
//! tags in it.
//!
//! Girsa does not need to scan for anything. **Every segment already carries its
//! address**, and an address is the tree: `1:1` is se'if 1 of siman 1,
//! `yoreh_deah:1:5` is se'if 5 of siman 1 of Yoreh De'ah in a Tur that holds all
//! four chalakim. So the contents are the addresses with the last level taken
//! off, and that is exact rather than a guess about markup — which matters,
//! because a scan for headings finds nothing at all in a Sefaria work: not one
//! of them in this corpus carries a single `heading` segment.
//!
//! # Where the titles come from
//!
//! The numbers alone are a list of numbers. What makes a table of contents worth
//! opening is *מי הם הכשרים לשחוט* beside סימן א, and the Shulchan Arukh does
//! say it — inside se'if 1, in bold, with a line break after it. That is
//! [`crate::display::opens_a_siman`], and it is the same reading that stops the
//! title running into the se'if on the page. One rule, two readers, which is
//! the point of it being a function.
//!
//! A sefer that names nothing gets numbers, and says so by having no titles
//! rather than by inventing them.

use girsa_cite::CiteStyle;
use girsa_corpus::segment::SegmentId;
use serde::Serialize;

/// One line of the contents: a place in the sefer, and what it is called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// The segment to open — the first one under this heading.
    pub at: String,
    /// The address, printed the way a citation prints it: `סימן א'`, `דף ב.`.
    /// The same formatter the margin of the page uses, so the contents and the
    /// line agree about what the place is called.
    pub address: String,
    /// What the sefer calls it, where it says. Absent is *the sefer does not
    /// say*, and is left off the wire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// How deep — `0` for the outermost level, `1` for what stands inside it.
    pub depth: usize,
    /// Where this place begins, counted in segments from the start of the
    /// sefer.
    ///
    /// What lets the window say **which entry the reader is inside** without
    /// asking anything: the pane knows the index of the line it is on, and the
    /// answer is the last entry at or before it. Otzaria works this out by
    /// searching its heading list for the closest index, which is the same
    /// answer arrived at by scanning; here it is one comparison because the
    /// number was free to record.
    pub from: usize,
}

/// The contents of one sefer, in reading order, outermost level first.
///
/// Empty for a sefer with no structure to show — a flat sefer whose segments
/// are addressed `1`, `2`, `3` is a list of lines and a table of contents of it
/// would be the sefer again.
#[must_use]
pub fn of(sefer: &crate::Open, style: CiteStyle) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    // The container path currently open, and for each of its levels, which
    // entry in `out` describes it — so a title found on the first se'if can be
    // hung on the siman it opened.
    let mut open: Vec<(String, usize)> = Vec::new();

    for (index, segment) in sefer.segments.iter().enumerate() {
        let path = segment.id.path();
        // The last level is the leaf — the se'if, the line, the pasuk. A sefer
        // with only leaves has no contents.
        let Some(container) = path.len().checked_sub(1).filter(|n| *n > 0) else {
            continue;
        };
        let container = &path[..container];

        let same = open
            .iter()
            .zip(container)
            .take_while(|((mine, _), theirs)| mine == *theirs)
            .count();
        open.truncate(same);
        for level in same..container.len() {
            let here = &container[..=level];
            out.push(Entry {
                at: segment.id.to_string(),
                address: crate::sending::printed_address_in(
                    &sefer.work,
                    Some(sefer.sections()),
                    // The container's own address, which is a place in this
                    // sefer even though no segment sits exactly on it. The
                    // ordinal is the first segment under it, because that is
                    // where the reader lands and an ordinal is never printed.
                    &SegmentId::new(
                        sefer.work.slug.clone(),
                        here.to_vec(),
                        segment.id.ordinal().clone(),
                    ),
                    style,
                ),
                title: None,
                depth: level,
                from: index,
            });
            open.push((container[level].clone(), out.len() - 1));
        }

        // …and what this place is called, where the sefer says so. Read off the
        // same runs the page draws, through the same rule.
        if let Some((_, entry)) = open.last() {
            if let Some(title) = titled(segment) {
                if let Some(row) = out.get_mut(*entry) {
                    if row.title.is_none() {
                        row.title = Some(title);
                    }
                }
            }
        }
    }
    out
}

/// The title this segment carries for the place it opens, if it carries one.
///
/// Two sources and they are the same question asked of two importers. A work
/// from Otzaria has a real `heading` segment and its words *are* the title. A
/// work from Sefaria has the title inside the first se'if — see
/// [`crate::display::opens_a_siman`] — and it is the leading runs of it.
fn titled(segment: &girsa_corpus::import::Segment) -> Option<String> {
    if segment.kind == girsa_corpus::import::SegmentKind::Heading {
        let said = crate::display::plain(&segment.text).trim().to_string();
        return (!said.is_empty()).then_some(said);
    }
    if segment.id.path().last().map(String::as_str) != Some("1") {
        return None;
    }
    let runs = crate::display::runs(&segment.text);
    let opens = crate::display::opens_a_siman(&runs);
    if opens == 0 {
        return None;
    }
    let said: String = runs
        .iter()
        .take(opens)
        .map(|run| run.text.as_str())
        .collect();
    let said = said.trim().to_string();
    (!said.is_empty()).then_some(said)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::import::{Segment, SegmentKind};
    use girsa_corpus::work::{Source, Work};

    fn work(slug: &str, sections: &[&str]) -> Work {
        Work {
            slug: slug.to_string(),
            he_title: slug.to_string(),
            en_title: slug.to_string(),
            categories: vec!["Halakhah".to_string()],
            order: Vec::new(),
            source: Source::Sefaria,
            origin: std::path::PathBuf::new(),
            schema: None,
            he_sections: sections.iter().map(|s| (*s).to_string()).collect(),
            author: None,
            era: None,
            comp_date: None,
            version: None,
            commentary_on: Vec::new(),
        }
    }

    fn segment(slug: &str, path: &[&str], n: u32, text: &str) -> Segment {
        Segment {
            id: format!("girsa:{slug}/{}#{n}", path.to_vec().join(":"))
                .parse()
                .expect("a segment id"),
            kind: SegmentKind::Text,
            text: text.to_string(),
            anchors: Vec::new(),
        }
    }

    #[test]
    fn the_contents_are_the_addresses_with_the_seif_taken_off() {
        // A Shulchan Arukh: simanim of se'ifim. The contents are the simanim,
        // once each, in the order they are printed — not one row per se'if,
        // which would be the sefer again with a scrollbar.
        let sefer = crate::Open::new(
            work("shulchan-arukh/yoreh-deah", &["סימן", "סעיף"]),
            vec![
                segment(
                    "shulchan-arukh/yoreh-deah",
                    &["1", "1"],
                    1,
                    r#"<b>מי הם הכשרים לשחוט. ובו י"ד סעיפים:</b><br>הכל שוחטין"#,
                ),
                segment("shulchan-arukh/yoreh-deah", &["1", "2"], 2, "אין צריך"),
                segment(
                    "shulchan-arukh/yoreh-deah",
                    &["2", "1"],
                    3,
                    "<b>אם שחיטת עובד כוכבים כשרה</b><br>שחיטת",
                ),
            ],
        );
        let toc = of(&sefer, CiteStyle::HebrewFull);
        assert_eq!(toc.len(), 2, "two simanim: {toc:?}");
        assert_eq!(
            toc[0].title.as_deref(),
            Some("מי הם הכשרים לשחוט. ובו י\"ד סעיפים:")
        );
        assert_eq!(toc[0].depth, 0);
        assert_eq!(toc[0].from, 0);
        assert_eq!(
            toc[1].from, 2,
            "the second siman begins at the third segment"
        );
        assert!(
            toc[0].address.contains("סימן"),
            "the address is printed as a citation prints it: {}",
            toc[0].address
        );
    }

    #[test]
    fn a_sefer_that_holds_its_chalakim_gets_a_tree() {
        // The Tur is one work with four chalakim inside it, so its addresses
        // are three levels deep and its contents are two. A flat list of 1,708
        // simanim with nothing saying which chelek they are in is the shape
        // this depth exists to avoid.
        let sefer = crate::Open::new(
            work("tur", &[]),
            vec![
                segment(
                    "tur",
                    &["yoreh_deah", "1", "1"],
                    1,
                    "<b>הלכות שחיטה</b><br>ישראל",
                ),
                segment("tur", &["yoreh_deah", "1", "2"], 2, "ועוד"),
                segment("tur", &["yoreh_deah", "2", "1"], 3, "כותי"),
                segment("tur", &["even_haezer", "1", "1"], 4, "אשה"),
            ],
        );
        let toc = of(&sefer, CiteStyle::HebrewFull);
        let shape: Vec<(usize, Option<&str>)> =
            toc.iter().map(|e| (e.depth, e.title.as_deref())).collect();
        assert_eq!(
            shape,
            vec![
                (0, None),
                (1, Some("הלכות שחיטה")),
                (1, None),
                (0, None),
                (1, None),
            ],
            "chelek, siman, siman, chelek, siman: {toc:?}"
        );
    }

    #[test]
    fn a_flat_sefer_has_no_contents_rather_than_one_row_per_line() {
        // A note, a sefer somebody dropped on the window, anything addressed
        // `1`, `2`, `3`. Its contents would be the sefer, which is not a table
        // of contents — and answering with 1,533 rows of nothing is worse than
        // answering with none.
        let sefer = crate::Open::new(
            work("mine/notes", &[]),
            vec![
                segment("mine/notes", &["1"], 1, "first"),
                segment("mine/notes", &["2"], 2, "second"),
            ],
        );
        assert_eq!(of(&sefer, CiteStyle::HebrewFull), Vec::new());
    }

    #[test]
    fn a_heading_segment_names_the_place_it_opens() {
        // The other importer. Otzaria's works carry real headings, and their
        // words are the title without any rule about bold and breaks.
        let mut heading = segment("otzaria/x", &["1", "1"], 1, "פרק א — דיני שחיטה");
        heading.kind = SegmentKind::Heading;
        let sefer = crate::Open::new(
            work("otzaria/x", &[]),
            vec![heading, segment("otzaria/x", &["1", "2"], 2, "הכל שוחטין")],
        );
        let toc = of(&sefer, CiteStyle::HebrewFull);
        assert_eq!(toc.len(), 1);
        assert_eq!(toc[0].title.as_deref(), Some("פרק א — דיני שחיטה"));
    }
}
