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

use girsa_corpus::said::{counted, plural, Clauses};
use girsa_note::since::Unindexed;
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
///
/// **Three kinds, so a struct rather than the two-variant enum this was.** The
/// enum could say one thing — *some scans on this shelf are unread* — and the
/// other two things a reader's own index cannot see had no variant to be reported
/// in, so they were reported nowhere. Any combination of the three can be true at
/// once, which an enum cannot express and which is the state a real reader is in:
/// two scans half-read, a chaburah written this morning, a typo fixed last night.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    /// The scans with pages nobody has read, worst first.
    pub scans: Vec<Scanned>,
    /// How many scan pages altogether, which is the number that says whether this
    /// is a minute of work or an afternoon.
    pub pages: usize,
    /// What your own layer holds that the index has not seen.
    ///
    /// Computed by `girsa-note`, because `girsa-index find` needs the same count
    /// and reaches it from the other side of a dependency boundary this crate
    /// deliberately does not cross. See `girsa_note::since`.
    pub layer: Unindexed,
}

impl Gap {
    /// Nothing is missing. **Not the same as there being nothing to miss** — both
    /// are silence in the header, and only one of them would be a bug if it were
    /// wrong.
    #[must_use]
    pub fn none() -> Self {
        Self {
            scans: Vec::new(),
            pages: 0,
            layer: Unindexed::none(),
        }
    }

    #[must_use]
    pub fn is_none(&self) -> bool {
        self.scans.is_empty() && !self.layer.is_a_gap()
    }

    /// The clauses, worded here and joined nowhere.
    ///
    /// This module knows the scan clause. The two layer clauses are worded by
    /// `girsa-note` so this header and `girsa-index find`'s footer cannot say
    /// different numbers about the same layer — and they arrive **flat**, which
    /// they did not: `said` used to join its own list with `" · "` and push
    /// `Unindexed::said`'s already-joined string into it as a single clause, so
    /// a four-clause sentence had a nesting in it that read correctly only
    /// because both levels happened to pick the same separator.
    #[must_use]
    pub fn clauses(&self) -> Clauses {
        let mut clauses = Clauses::new();
        clauses.count(self.scans.len(), |n| {
            format!(
                "{} on this shelf {} searchable yet — {}",
                counted(n, "PDF", "PDFs"),
                plural(n, "isn't", "aren't"),
                counted(self.pages, "page", "pages"),
            )
        });
        clauses.and(self.layer.clauses());
        clauses
    }

    /// The header line, in words. `None` when there is nothing to say.
    ///
    /// The surfaces that draw the literal gap on its own — `girsa-read`'s line,
    /// and `never_a_silent_gap.rs`. The window's header goes through
    /// [`crate::Unseen`], which is where the lane's coverage clause is.
    ///
    /// One line, always, however many of the clauses are true: a header that
    /// grows into three lines is a header nobody reads.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        self.clauses().said()
    }
}

/// What is not searchable on the whole shelf.
///
/// `index` is where the search index lives, or `None` when there is not one. The
/// gap now depends on it, because two of the three kinds are *since the index was
/// built* — so a caller that cannot say where the index is has to say so, rather
/// than being handed a gap computed against nothing.
#[must_use]
pub fn gap(shelf: &Shelf, personal: &Path, index: Option<&Path>) -> Gap {
    let slugs: Vec<String> = shelf
        .works()
        .iter()
        .filter(|work| is_scan(work))
        .map(|work| work.slug.clone())
        .collect();
    gap_over(shelf, personal, index, &slugs)
}

/// What is not searchable among these seforim — the ones a search is scoped to.
///
/// The caller passes the scope's slugs rather than a `Scope`, because a facet's
/// idea of what is in scope belongs to `girsa-search` and this crate does not
/// depend on it. Anything in the list that is not a scan is skipped, so a
/// caller can hand over the whole scope without filtering first.
#[must_use]
pub fn gap_over(shelf: &Shelf, personal: &Path, index: Option<&Path>, slugs: &[String]) -> Gap {
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
    // Worst first: the sefer with the most unread pages is the one whose
    // absence is costing the reader the most.
    scans.sort_by(|a, b| {
        (b.pages - b.read)
            .cmp(&(a.pages - a.read))
            .then(a.slug.cmp(&b.slug))
    });
    Gap {
        scans,
        pages,
        layer: Unindexed::of(index, personal),
    }
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
    use girsa_note::since::Written;

    #[test]
    fn nothing_to_say_is_said_by_saying_nothing() {
        assert_eq!(Gap::none().said(), None);
    }

    #[test]
    fn the_header_counts_the_seforim_and_the_pages() {
        let gap = Gap {
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
            ..Gap::none()
        };
        assert_eq!(
            gap.said().as_deref(),
            Some("2 PDFs on this shelf aren't searchable yet — 308 pages")
        );
    }

    #[test]
    fn one_of_each_reads_like_english() {
        let gap = Gap {
            scans: vec![Scanned {
                slug: "user/a".into(),
                title: "א".into(),
                pages: 4,
                read: 3,
            }],
            pages: 1,
            ..Gap::none()
        };
        assert_eq!(
            gap.said().as_deref(),
            Some("1 PDF on this shelf isn't searchable yet — 1 page")
        );
    }

    #[test]
    fn the_three_clauses_join_into_one_line() {
        let gap = Gap {
            scans: vec![Scanned {
                slug: "user/a".into(),
                title: "א".into(),
                pages: 6,
                read: 0,
            }],
            pages: 6,
            layer: Unindexed {
                notes: Written::Since(2),
                fixes: Written::Since(1),
                scans: Written::Since(1),
            },
        };
        let said = gap.said().expect("three gaps is a gap");
        assert!(said.contains("1 PDF"), "{said}");
        assert!(said.contains("2 notes"), "{said}");
        assert!(said.contains("1 correction"), "{said}");
        assert_eq!(said.lines().count(), 1, "one line, not three: {said}");
    }

    #[test]
    fn no_index_is_said_once_and_not_twice() {
        // "There is no search index" is one fact about the machine, not two facts
        // about notes and corrections.
        let gap = Gap {
            layer: Unindexed {
                notes: Written::NoIndex,
                fixes: Written::NoIndex,
                scans: Written::NoIndex,
            },
            ..Gap::none()
        };
        let said = gap.said().expect("no index is the largest gap there is");
        assert_eq!(said.matches("no search index").count(), 1, "{said}");
        assert!(said.contains("girsa-index build"), "{said}");
    }

    #[test]
    fn nothing_written_since_the_build_says_nothing() {
        let gap = Gap {
            layer: Unindexed::none(),
            ..Gap::none()
        };
        assert_eq!(gap.said(), None);
        assert!(gap.is_none());
    }
}
