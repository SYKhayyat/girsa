//! Scans in the search box: what is searchable, and what is honestly not yet.
//!
//! spec.md §9.7, BUILDER.md W26. One index, two location types — a text hit is
//! a sefer and a segment id, a scanned hit is a sefer, a page and a rectangle —
//! and since OCR is off at onboarding, the scans are absent from the index
//! until somebody runs it.
//!
//! > **Never a silent gap:** the results header says *"4 PDFs on this shelf
//! > aren't searchable yet — [OCR now]"*.
//!
//! That sentence is this module. It is not a nicety and it is not a progress
//! indicator: a reader who searches a shelf holding four unread scans and gets
//! forty hits has been told *these are the forty places this appears*, and the
//! forty-first is on a page nobody has read. Search that quietly omits a shelf
//! is worse than search that has not been run, because it looks like an answer
//! — BUILDER.md rule 6, one layer up from a citation.
//!
//! # A scan is counted by pages, not by whether it has been touched
//!
//! A job stopped at page 40 of 302 is not *read* and is not *unread*. Both
//! numbers are reported, because *"3 PDFs aren't searchable yet"* over a sefer
//! that is two-thirds done would send a reader to run a job that is nearly
//! finished, and *"searchable"* over the same sefer would be a lie about a
//! hundred pages.

use std::path::Path;

use girsa_scan::reading::Read;
use girsa_scan::words::{Job, Words};

use crate::scanning::{is_scan, pages_of};
use crate::shelf::Shelf;

/// One scan, and how much of it has been read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scanned {
    pub slug: String,
    pub title: String,
    pub pages: usize,
    pub read: usize,
}

impl Scanned {
    #[must_use]
    pub fn is_read(&self) -> bool {
        self.read >= self.pages
    }
}

/// What a search over these seforim cannot see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gap {
    /// Every scan in scope has been read. **Not the same as there being no
    /// scans** — both are silence in the header, and only one of them would be
    /// a bug if it were wrong.
    None,
    Some {
        /// The scans with pages nobody has read, worst first.
        scans: Vec<Scanned>,
        /// How many pages altogether, which is the number that says whether
        /// this is a minute of work or an afternoon.
        pages: usize,
    },
}

impl Gap {
    /// The header line, in words. `None` when there is nothing to say.
    ///
    /// One implementation, because the window's line, the CLI's line and the
    /// test's expectation drifting apart is how a header comes to promise a
    /// count the button does not do.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        match self {
            Self::None => None,
            Self::Some { scans, pages } => Some(format!(
                "{} on this shelf {} searchable yet — {pages} {}",
                if scans.len() == 1 {
                    "1 PDF".to_string()
                } else {
                    format!("{} PDFs", scans.len())
                },
                if scans.len() == 1 { "isn't" } else { "aren't" },
                if *pages == 1 { "page" } else { "pages" },
            )),
        }
    }
}

/// What is not searchable on the whole shelf.
#[must_use]
pub fn gap(shelf: &Shelf, personal: &Path) -> Gap {
    let slugs: Vec<String> = shelf
        .works()
        .iter()
        .filter(|work| is_scan(work))
        .map(|work| work.slug.clone())
        .collect();
    gap_over(shelf, personal, &slugs)
}

/// What is not searchable among these seforim — the ones a search is scoped to.
///
/// The caller passes the scope's slugs rather than a `Scope`, because a facet's
/// idea of what is in scope belongs to `girsa-search` and this crate does not
/// depend on it. Anything in the list that is not a scan is skipped, so a
/// caller can hand over the whole scope without filtering first.
#[must_use]
pub fn gap_over(shelf: &Shelf, personal: &Path, slugs: &[String]) -> Gap {
    let mut scans = Vec::new();
    let mut pages = 0;
    for slug in slugs {
        let Some(work) = shelf.work(slug) else {
            continue;
        };
        if !is_scan(work) {
            continue;
        }
        let Ok(sefer) = shelf.read(slug) else {
            continue;
        };
        let total = pages_of(&sefer);
        let (words, _) = Words::open(personal, slug);
        let job = Job::of(slug, total, &words);
        if job.is_finished() {
            continue;
        }
        pages += job.remaining();
        scans.push(Scanned {
            slug: slug.clone(),
            title: work.he_title.clone(),
            pages: total,
            read: job.done(),
        });
    }
    if scans.is_empty() {
        return Gap::None;
    }
    // Worst first: the sefer with the most unread pages is the one whose
    // absence is costing the reader the most.
    scans.sort_by(|a, b| {
        (b.pages - b.read)
            .cmp(&(a.pages - a.read))
            .then(a.slug.cmp(&b.slug))
    });
    Gap::Some { scans, pages }
}

/// Every page of a scan that has words, ready for the index.
///
/// Corrections applied, because the index has to find what the reader can see:
/// a reader who fixed a misread word and then cannot find it has been given a
/// correction that only corrects the display.
#[must_use]
pub fn readings(personal: &Path, slug: &str, pages: usize) -> Vec<Read> {
    let (words, _) = Words::open(personal, slug);
    (1..=pages).filter_map(|page| words.page(page)).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn nothing_to_say_is_said_by_saying_nothing() {
        assert_eq!(Gap::None.said(), None);
    }

    #[test]
    fn the_header_counts_the_seforim_and_the_pages() {
        let gap = Gap::Some {
            scans: vec![
                Scanned {
                    slug: "user/a".into(),
                    title: "א".into(),
                    pages: 300,
                    read: 0,
                },
                Scanned {
                    slug: "user/b".into(),
                    title: "ב".into(),
                    pages: 12,
                    read: 4,
                },
            ],
            pages: 308,
        };
        assert_eq!(
            gap.said().as_deref(),
            Some("2 PDFs on this shelf aren't searchable yet — 308 pages")
        );
    }

    #[test]
    fn one_of_each_reads_like_english() {
        let gap = Gap::Some {
            scans: vec![Scanned {
                slug: "user/a".into(),
                title: "א".into(),
                pages: 4,
                read: 3,
            }],
            pages: 1,
        };
        assert_eq!(
            gap.said().as_deref(),
            Some("1 PDF on this shelf isn't searchable yet — 1 page")
        );
    }
}
