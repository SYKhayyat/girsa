//! What your own layer has that the index has not seen yet.
//!
//! # Why this exists, and why it is here
//!
//! `girsa-app`'s `reading` module already owned the right mechanism and applied it
//! to one case in three:
//!
//! | the index cannot see                        | told? |
//! |---------------------------------------------|-------|
//! | an un-OCR'd scan                            | yes   |
//! | a note written since the last build         | **no** |
//! | a correction made since the last build      | **no** |
//!
//! Its own module note argues the case: *"a reader who searches a shelf holding
//! four unread scans and gets forty hits has been told these are the forty places
//! this appears, and the forty-first is on a page nobody has read."* Replace
//! *scans* with *your chaburos* and it is the same sentence — and for a bochur,
//! finding his own writing is most of why he would move.
//!
//! It lives in `girsa-note` rather than in `girsa-app` because there are **two**
//! callers that must not disagree and they are on opposite sides of a deliberate
//! dependency boundary: the window and `girsa-read` reach it through `girsa-app`,
//! and `girsa-index find` reaches it through `girsa-search`. `girsa-app` does not
//! depend on `girsa-search` — `gap_over` takes a slice of slugs rather than a
//! `Scope` specifically so it need not, and that call is written down in the README
//! — so the shared thing has to sit under both. Notes are this crate's, and the
//! corrections file is one path.
//!
//! # "Since" is a comparison, not bookkeeping
//!
//! `girsa-search` already writes a stamp beside the index. Its modification time is
//! when the index was built. Nothing new is recorded anywhere; a note is newer than
//! the index or it is not.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The stamp `girsa-search` writes beside an index.
///
/// Spelled here as well as in `girsa_search::index` because this crate sits below
/// it. A shared constant would mean this crate depending on the index, which is
/// the dependency this module exists to avoid.
pub const CACHE_STAMP_NAME: &str = "girsa-cache.json";

/// Whether some part of your own layer has reached the index.
///
/// The distinction between *counted* and *no index* is the point of the type. A
/// fresh install has no index at all, so **nothing** you have written is findable
/// — and answering that state with "0 notes are missing" would be precisely the
/// silence this module closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Written {
    /// The stamp was read, and this many are newer than it.
    Since(usize),
    /// There is no index to compare against, so everything is unsearchable.
    NoIndex,
}

impl Written {
    #[must_use]
    pub fn is_a_gap(self) -> bool {
        match self {
            Self::Since(n) => n > 0,
            Self::NoIndex => true,
        }
    }

    /// How many, or `None` when there is no index to have counted against.
    #[must_use]
    pub fn count(self) -> Option<usize> {
        match self {
            Self::Since(n) => Some(n),
            Self::NoIndex => None,
        }
    }
}

/// What your own layer holds that a search will not find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unindexed {
    pub notes: Written,
    pub fixes: Written,
}

impl Unindexed {
    /// Nothing outstanding.
    #[must_use]
    pub fn none() -> Self {
        Self {
            notes: Written::Since(0),
            fixes: Written::Since(0),
        }
    }

    /// Compare a personal layer against an index, or against nothing.
    #[must_use]
    pub fn of(index: Option<&Path>, personal: &Path) -> Self {
        let built = index.and_then(built_at);
        Self {
            notes: notes_since(personal, built),
            fixes: fixes_since(personal, built),
        }
    }

    #[must_use]
    pub fn is_a_gap(&self) -> bool {
        self.notes.is_a_gap() || self.fixes.is_a_gap()
    }

    /// The clause a reader sees, or `None` when there is nothing to say.
    ///
    /// One implementation, because the window's header, `girsa-read`'s line,
    /// `girsa-index find`'s footer and the MCP server's field drifting apart is how
    /// a header comes to promise a count the button does not do.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        if !self.is_a_gap() {
            return None;
        }
        // "There is no search index" is one fact about the machine, not two facts
        // about notes and corrections, so it is said once and instead of both.
        if self.notes == Written::NoIndex || self.fixes == Written::NoIndex {
            return Some(
                "there is no search index yet, so nothing you have written is findable — \
                 run girsa-index build"
                    .to_string(),
            );
        }
        let mut parts = Vec::new();
        if let Written::Since(n) = self.notes {
            if n > 0 {
                parts.push(format!(
                    "{n} {} written since the index was built {} not searchable yet",
                    if n == 1 { "note" } else { "notes" },
                    if n == 1 { "is" } else { "are" },
                ));
            }
        }
        if let Written::Since(n) = self.fixes {
            if n > 0 {
                parts.push(format!(
                    "{n} {} made since then {} findable by the typo and not by the fix",
                    if n == 1 { "correction" } else { "corrections" },
                    if n == 1 { "is still" } else { "are still" },
                ));
            }
        }
        Some(parts.join(" · "))
    }
}

/// When the index was last built, from the stamp's modification time.
///
/// `None` when there is no index, which is a different answer from "nothing has
/// changed" and is treated as one.
#[must_use]
pub fn built_at(index: &Path) -> Option<SystemTime> {
    std::fs::metadata(index.join(CACHE_STAMP_NAME))
        .or_else(|_| std::fs::metadata(index.join("meta.json")))
        .and_then(|m| m.modified())
        .ok()
}

/// Whether a directory holds an index.
///
/// Girsa's own carries [`CACHE_STAMP_NAME`]; a tantivy directory built by something
/// else carries `meta.json`. Either is enough to compare against.
#[must_use]
pub fn is_an_index(dir: &Path) -> bool {
    dir.join(CACHE_STAMP_NAME).is_file() || dir.join("meta.json").is_file()
}

/// Every place the index might be, in the order they are tried.
///
/// `GIRSA_INDEX`, then beside the corpus, then inside it. Shared, so a search panel
/// that finds an index and a `girsa-read` that does not cannot be two answers to
/// one question.
#[must_use]
pub fn index_candidates(corpus: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(from_env) = std::env::var("GIRSA_INDEX") {
        candidates.push(PathBuf::from(from_env));
    }
    if let Some(beside) = corpus.parent() {
        candidates.push(beside.join("index"));
    }
    candidates.push(corpus.join("index"));
    candidates
}

/// The first candidate that is actually an index, if any.
#[must_use]
pub fn find_index(corpus: &Path) -> Option<PathBuf> {
    index_candidates(corpus)
        .into_iter()
        .find(|c| is_an_index(c))
}

/// Notes whose file is newer than the index.
fn notes_since(personal: &Path, built: Option<SystemTime>) -> Written {
    let Some(built) = built else {
        return Written::NoIndex;
    };
    let Ok(entries) = std::fs::read_dir(personal.join("notes")) else {
        return Written::Since(0);
    };
    let mut n = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        if newer_than(&path, built) {
            n += 1;
        }
    }
    Written::Since(n)
}

/// Corrections added since the index was built.
///
/// One file, one line each, so the file's own modification time says only
/// *whether* any are new and not *how many*. Where a patch carries its own `when`
/// the count is exact; where none do, every line is counted. Over-reporting sends
/// a reader to rebuild an index they might not have needed to; under-reporting is
/// the silence this module exists to close, and of the two only one is a lie.
///
/// A tombstone is not a correction. The file is an append-only log, so taking a
/// correction back writes a line too — and a line that says a correction is gone
/// is not something for the index to go and find.
fn fixes_since(personal: &Path, built: Option<SystemTime>) -> Written {
    let Some(built) = built else {
        return Written::NoIndex;
    };
    let path = personal.join("corrections.jsonl");
    if !newer_than(&path, built) {
        return Written::Since(0);
    }
    let Ok(body) = std::fs::read_to_string(&path) else {
        return Written::Since(0);
    };
    let corrections = || {
        body.lines()
            .filter(|l| !l.trim().is_empty())
            .filter(|l| !girsa_personal::is_tombstone(l))
    };
    if !body.contains("\"when\"") {
        return Written::Since(corrections().count());
    }
    Written::Since(corrections().filter(|l| when_after(l, built)).count())
}

/// Whether a correction's own `when` is after the build.
///
/// Anything that will not parse counts as new. A timestamp this cannot read is not
/// a reason to say nothing about the correction it belongs to.
fn when_after(line: &str, built: SystemTime) -> bool {
    let Some(after) = line.split("\"when\"").nth(1) else {
        return true;
    };
    let Some(seconds) = after
        .trim_start_matches([':', ' ', '"'])
        .split(['"', ',', '}'])
        .next()
        .and_then(|s| s.parse::<u64>().ok())
    else {
        return true;
    };
    let Ok(built_secs) = built
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
    else {
        return true;
    };
    seconds > built_secs
}

fn newer_than(path: &Path, built: SystemTime) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .is_ok_and(|m| m > built)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-since-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("personal")).unwrap();
        std::fs::create_dir_all(dir.join("index")).unwrap();
        std::fs::write(dir.join("index").join(CACHE_STAMP_NAME), "{}").unwrap();
        dir
    }

    fn note(personal: &Path, name: &str) {
        let dir = personal.join("notes");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.md")), "# בדיקה\n").unwrap();
    }

    #[test]
    fn nothing_written_since_the_build_says_nothing() {
        let dir = scratch("quiet");
        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(at, Unindexed::none());
        assert_eq!(at.said(), None);
        assert!(!at.is_a_gap());
    }

    #[test]
    fn a_note_newer_than_the_index_is_counted_and_said() {
        let dir = scratch("notes");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        note(&dir.join("personal"), "בדיקה");
        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(at.notes, Written::Since(1));
        let said = at.said().expect("a note since the build is a gap");
        assert!(said.contains("1 note"), "{said}");
        assert!(said.contains("since"), "{said}");

        note(&dir.join("personal"), "שנייה");
        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(at.notes, Written::Since(2));
        assert!(at.said().unwrap().contains("2 notes"));
    }

    #[test]
    fn a_note_older_than_the_index_is_not_a_gap() {
        let dir = scratch("older");
        note(&dir.join("personal"), "ותיקה");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        // Restamp: the index is now newer than the note.
        std::fs::write(dir.join("index").join(CACHE_STAMP_NAME), "{}").unwrap();
        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(at.notes, Written::Since(0));
        assert_eq!(at.said(), None);
    }

    #[test]
    fn no_index_at_all_is_the_largest_gap_and_is_said_once() {
        let dir = scratch("none");
        note(&dir.join("personal"), "בדיקה");
        let at = Unindexed::of(None, &dir.join("personal"));
        assert_eq!(at.notes, Written::NoIndex);
        assert_eq!(at.fixes, Written::NoIndex);
        assert_eq!(at.notes.count(), None);
        let said = at.said().expect("no index is a gap");
        assert_eq!(said.matches("no search index").count(), 1, "{said}");
        assert!(said.contains("girsa-index build"), "{said}");
    }

    #[test]
    fn corrections_with_no_timestamp_are_all_counted() {
        let dir = scratch("fixes-plain");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(
            dir.join("personal").join("corrections.jsonl"),
            "{\"id\":\"a\"}\n{\"id\":\"b\"}\n",
        )
        .unwrap();
        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(at.fixes, Written::Since(2));
        assert!(at.said().unwrap().contains("2 corrections"));
    }

    #[test]
    fn corrections_that_carry_a_when_are_counted_exactly() {
        let dir = scratch("fixes-stamped");
        let built = built_at(&dir.join("index")).unwrap();
        let secs = built
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(
            dir.join("personal").join("corrections.jsonl"),
            format!(
                "{{\"id\":\"old\",\"when\":{}}}\n{{\"id\":\"new\",\"when\":{}}}\n",
                secs - 100,
                secs + 100
            ),
        )
        .unwrap();
        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(
            at.fixes,
            Written::Since(1),
            "only the one made after the build"
        );
    }

    #[test]
    fn an_index_is_recognised_by_either_stamp() {
        let dir = scratch("stamps");
        assert!(is_an_index(&dir.join("index")));
        let other = dir.join("tantivy");
        std::fs::create_dir_all(&other).unwrap();
        assert!(!is_an_index(&other));
        std::fs::write(other.join("meta.json"), "{}").unwrap();
        assert!(is_an_index(&other), "a tantivy directory counts");
    }

    #[test]
    fn the_index_is_looked_for_beside_the_corpus_then_inside_it() {
        let dir = scratch("candidates");
        let corpus = dir.join("corpus");
        std::fs::create_dir_all(&corpus).unwrap();
        let looked = index_candidates(&corpus);
        assert!(looked.contains(&dir.join("index")), "{looked:?}");
        assert!(looked.contains(&corpus.join("index")), "{looked:?}");
        assert_eq!(
            find_index(&corpus).as_deref(),
            Some(dir.join("index").as_path())
        );
    }
}
