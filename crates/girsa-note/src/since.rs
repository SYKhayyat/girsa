//! What your own layer has that the index has not seen yet.
//!
//! # Why this exists, and why it is here
//!
//! `girsa-app`'s `reading` module already owned the right mechanism and applied it
//! to one case in three:
//!
//! | the index cannot see                        | told?  |
//! |---------------------------------------------|--------|
//! | an un-OCR'd scan                            | yes    |
//! | a note written since the last build         | **no** |
//! | a correction made since the last build      | **no** |
//!
//! It was four rows, and the fourth was found on 6 August 2026: **a word
//! corrected on a scan since the last build.** The index build *does* apply
//! scan corrections — `girsa-index` reads pages through `Words::page`, which
//! re-finds each fix by its ink — but the index is a snapshot, and a fix made
//! after it holds the misreading. So the reader corrects a word, searches for
//! the word they corrected, finds nothing, and can still find the misreading
//! they fixed. Exactly the corrections row, one layer over, and it was not
//! being counted.
//!
//! Its own module note argues the case: *"a reader who searches a shelf holding
//! four unread scans and gets forty hits has been told these are the forty places
//! this appears, and the forty-first is on a page nobody has read."* Replace
//! *scans* with *your chaburos* and it is the same sentence — and for a bochur,
//! finding his own writing is most of why he would move.
//!
//! It lives in `girsa-note` rather than in `girsa-app` because there are **three**
//! callers that must not disagree and they are on opposite sides of a deliberate
//! dependency boundary: the window and `girsa-read` reach it through `girsa-app`,
//! `girsa-index find` reaches it through `girsa-search`, and the MCP server
//! reaches it through both. (Two, when this was written. The third arrived and
//! the sentence did not change, which is the small version of the whole
//! finding.) `girsa-app` does not
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

use girsa_corpus::said::{counted, plural, Clauses};

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
    /// Scans carrying word corrections the index has not seen.
    ///
    /// Counted in **scans**, not in words, and that is not laziness: counting
    /// the fixes would mean this crate parsing `girsa-scan`'s file, which is
    /// the cross-crate string surgery the `fixes` field above already commits
    /// and which is a finding of its own. A modification time answers the
    /// question the reader is actually asking.
    pub scans: Written,
}

impl Unindexed {
    /// Nothing outstanding.
    #[must_use]
    pub fn none() -> Self {
        Self {
            notes: Written::Since(0),
            fixes: Written::Since(0),
            scans: Written::Since(0),
        }
    }

    /// Compare a personal layer against an index, or against nothing.
    #[must_use]
    pub fn of(index: Option<&Path>, personal: &Path) -> Self {
        let built = index.and_then(built_at);
        Self {
            notes: notes_since(personal, built),
            fixes: fixes_since(personal, built),
            scans: scan_fixes_since(personal, built),
        }
    }

    #[must_use]
    pub fn is_a_gap(&self) -> bool {
        self.notes.is_a_gap() || self.fixes.is_a_gap() || self.scans.is_a_gap()
    }

    /// The clauses a reader sees, worded here and joined nowhere.
    ///
    /// This module knows how to say *what your own layer holds that the index
    /// has not seen* and nothing else. How that sits beside *4 PDFs aren't
    /// searchable yet* and *this lane covers Hilchos Tefillah* is
    /// [`girsa_corpus::said::Clauses`]'s question, because it was three
    /// questions when it was three composers' — and the answers differed in
    /// their separator, their thousands separator, and whether one of them
    /// nested a joined string inside another join.
    #[must_use]
    pub fn clauses(&self) -> Clauses {
        let mut clauses = Clauses::new();
        if !self.is_a_gap() {
            return clauses;
        }
        // "There is no search index" is one fact about the machine, not two facts
        // about notes and corrections, so it is said once and instead of both.
        if self.notes == Written::NoIndex
            || self.fixes == Written::NoIndex
            || self.scans == Written::NoIndex
        {
            clauses.say(
                "there is no search index yet, so nothing you have written is findable — \
                 run girsa-index build",
            );
            return clauses;
        }
        clauses
            .count(self.notes.count().unwrap_or(0), |n| {
                format!(
                    "{} written since the index was built {} not searchable yet",
                    counted(n, "note", "notes"),
                    plural(n, "is", "are"),
                )
            })
            .count(self.fixes.count().unwrap_or(0), |n| {
                format!(
                    "{} made since then {} findable by the typo and not by the fix",
                    counted(n, "correction", "corrections"),
                    plural(n, "is still", "are still"),
                )
            })
            .count(self.scans.count().unwrap_or(0), |n| {
                format!(
                    "words you corrected on {} are still findable by the misreading \
                     and not by the correction",
                    counted(n, "scan", "scans"),
                )
            });
        clauses
    }

    /// The clause a reader sees, or `None` when there is nothing to say.
    ///
    /// The surfaces that draw it on their own — `girsa-index find`'s footer —
    /// rather than as part of a longer sentence. The window's header and the
    /// MCP server's field go through `girsa_app::Unseen`, which is where the
    /// scan clause and the lane clause are.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        self.clauses().said()
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

/// The first candidate that is actually an index — or where it looked.
///
/// # There was a second one of these, and it accepted a different thing
///
/// `app/src-tauri/src/lib.rs:855` had its own `find_index`, forty lines from a
/// call to this one, in the same file. Same three candidates in the same order,
/// and a **different accept predicate**: it took only `girsa-cache.json`, where
/// [`is_an_index`] also takes a bare tantivy `meta.json`. So a directory
/// `girsa-read` called an index the window called *no search index*, which is
/// two answers to one question and is precisely what the note on
/// [`index_candidates`] says must not happen.
///
/// The permissive one is the one that survived, and that is not a coin toss.
/// Finding a directory and *trusting* it are different questions:
/// `girsa_search::index::SearchIndex::open` already refuses an index built
/// under other rules and says which rules, so a foreign tantivy directory now
/// gets *"the index at … cannot be trusted"* instead of *"no search index"* —
/// which is the true statement of the two.
///
/// # Errors
///
/// With the sentence a reader is shown, naming every place it looked. That
/// wording came from the copy in the shell and is the one thing that copy had
/// which this did not.
pub fn find_index(corpus: &Path) -> Result<PathBuf, String> {
    let mut tried = Vec::new();
    for candidate in index_candidates(corpus) {
        if is_an_index(&candidate) {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }
    Err(format!(
        "no search index. Looked in: {}. Run girsa-index build, or set GIRSA_INDEX.",
        tried.join(", ")
    ))
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
/// *whether* any are new and not *how many*. Where a patch carries its own
/// `when` the count is exact; where it does not, it counts as new.
/// Over-reporting sends a reader to rebuild an index they might not have needed
/// to; under-reporting is the silence this module exists to close, and of the
/// two only one is a lie.
///
/// A tombstone is not a correction. The file is an append-only log, so taking a
/// correction back writes a line too — and a line that says a correction is
/// gone is not something for the index to go and find.
///
/// # The counting is [`girsa_personal::since`] and used to be string surgery
///
/// This crate may not depend on `girsa-fix` — siblings, neither may name the
/// other — so it could not read a `Patch`. What it did instead was
/// `body.contains("\"when\"")` and
/// `line.split("\"when\"").nth(1).trim_start_matches([':', ' ', '"'])`: one
/// crate parsing another's file by hand, with `serde_json` sitting unused in
/// its own manifest, purely because a type name was out of reach.
///
/// It was correct. A `"when"` inside a string value is escaped, so the split
/// could not land in one — correct by luck rather than by construction, and
/// silently so until somebody added a field called `whenever`.
///
/// The answer was never to name `Patch`. **Counting records in a log is a fact
/// about the log format**, and the format is `girsa-personal`'s, which both
/// crates already depend on — the same argument that already put
/// `is_tombstone` there.
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
    let Ok(built) = built
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
    else {
        // A build stamp older than the epoch is a clock nobody can reason
        // about. Everything counts as new, which is the safe direction.
        return Written::Since(girsa_personal::since(&body, 0).records);
    };
    Written::Since(girsa_personal::since(&body, built).after)
}

/// Scans whose word corrections are newer than the index.
///
/// `personal/words/<slug>/fixes.json`, one per scan. A modification time and
/// nothing else — this crate does not open the file, because the shape inside
/// it is `girsa-scan`'s and a fourth crate reading a third crate's format by
/// hand is how the `fixes` counter above ended up doing string surgery.
///
/// **Deliberately not `pages.jsonl`.** A page read since the build is already
/// reported, honestly, by the un-OCR'd count: the index holds it as a page with
/// no words, so *"not searchable yet"* is exactly true of it. Counting it here
/// as well would say the same gap twice.
fn scan_fixes_since(personal: &Path, built: Option<SystemTime>) -> Written {
    let Some(built) = built else {
        return Written::NoIndex;
    };
    let mut found = 0usize;
    let mut stack = vec![personal.join("words")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "fixes.json")
                && newer_than(&path, built)
            {
                found += 1;
            }
        }
    }
    Written::Since(found)
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
            find_index(&corpus).ok().as_deref(),
            Some(dir.join("index").as_path())
        );
    }

    /// A word corrected on a scan since the index was built.
    #[test]
    fn a_scan_correction_made_since_the_build_is_a_gap_and_says_which_way_round() {
        // The row this table was missing. `girsa-index` *does* apply scan
        // corrections when it builds — it reads pages through `Words::page` —
        // so the failure is not that they are never applied. It is that the
        // index is a snapshot: correct a word afterwards and you cannot find
        // what you fixed, and you can still find what you unfixed. Nothing said
        // so.
        let dir = scratch("scan-fixes");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let words = dir.join("personal").join("words").join("user").join("a");
        std::fs::create_dir_all(&words).unwrap();
        std::fs::write(words.join("fixes.json"), "{\"3\":[]}").unwrap();

        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(at.scans, Written::Since(1));
        let said = at
            .said()
            .expect("a scan correction since the build is a gap");
        assert!(said.contains("1 scan"), "{said}");
        assert!(
            said.contains("misreading") && said.contains("correction"),
            "the sentence has to say which way round it is wrong: {said}"
        );
    }

    #[test]
    fn a_scan_correction_older_than_the_index_is_not_a_gap() {
        let dir = scratch("scan-fixes-old");
        let words = dir.join("personal").join("words").join("user").join("a");
        std::fs::create_dir_all(&words).unwrap();
        std::fs::write(words.join("fixes.json"), "{}").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(dir.join("index").join(CACHE_STAMP_NAME), "{}").unwrap();

        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(at.scans, Written::Since(0));
        assert_eq!(at.said(), None);
    }

    #[test]
    fn a_page_read_since_the_build_is_not_counted_here_because_it_is_counted_there() {
        // `pages.jsonl` newer than the index means a page was OCR'd since. That
        // gap is already reported, and honestly: the index holds the page with
        // no words, so *"not searchable yet"* is exactly true of it. Saying it
        // twice would be two sentences about one silence.
        let dir = scratch("scan-pages");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let words = dir.join("personal").join("words").join("user").join("a");
        std::fs::create_dir_all(&words).unwrap();
        std::fs::write(
            words.join("pages.jsonl"),
            "{}
",
        )
        .unwrap();

        let at = Unindexed::of(Some(&dir.join("index")), &dir.join("personal"));
        assert_eq!(at.scans, Written::Since(0));
    }
}
