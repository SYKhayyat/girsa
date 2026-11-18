//! The shape six stores were writing out one at a time.
//!
//! `Log` is the append-only file and `Log::live` replays it. On top of that,
//! **six** stores across three crates had grown the identical arrangement:
//!
//! ```text
//! girsa-fix/src/lib.rs        corrections   by_segment
//! girsa-fix/src/suspect.rs    suspects      by_segment
//! girsa-note/src/mark.rs      marks         by_segment
//! girsa-note/src/collection.rs folders      by_name
//! girsa-note/src/query.rs     saved queries by_name
//! girsa-desk/src/documents.rs documents     by_path
//! ```
//!
//! Each with an `open(personal) -> (Self, Vec<String>)` that opens the log,
//! replays it, holds each record in an index, asks `Log::bloated` and compacts;
//! a `nowhere()`; a `compact()` that is `self.log.rewrite(self.all())`; a
//! `count()`; and an identical `From<LogError>` for its own error type.
//!
//! **Five of them. The sixth forgot to compact**, and it is the one written
//! last, in the crate added most recently: `personal/documents.jsonl` grew
//! without bound on every save, in the store whose whole job is to be re-saved.
//! That is not a bug anybody would find by reading it — it is a bug you find by
//! noticing that five files say the same thing and one says nine-tenths of it.
//!
//! # Why a trait and not a struct
//!
//! `Store<T>` as a *container* was the obvious shape and is the wrong one: the
//! six indexes are genuinely different — `BTreeMap<SegmentId, Vec<Mark>>` keeps
//! several marks per place and sorts them by where they start,
//! `BTreeMap<String, Collection>` keeps one folder per name — and flattening
//! them into one generic map would move that difference into six `hold`
//! closures without removing it.
//!
//! What *is* the same is the **procedure**: open, replay, hold, count, compact
//! when bloated, report rather than fail. So the trait says what a store must be
//! able to answer, and [`open`] is the procedure, once.

use std::collections::BTreeMap;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::log::{Log, LogError};

/// What taking somebody else's layer took, and what it would not take.
///
/// Three numbers rather than one, because *nothing happened* has three
/// different meanings and a reader deciding whether the merge worked needs to
/// know which one they are looking at: everything was already here, everything
/// clashed, or the file was not what it claimed to be.
///
/// It lived in `girsa-fix`, which was the first store to be mergeable and is no
/// longer the only one. `girsa_fix::Merged` re-exports this.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Merged {
    pub taken: usize,
    /// Records that were already here, to the letter. Counted rather than
    /// ignored: taking the same file twice has to be visibly a no-op.
    pub already_had: usize,
    /// Records that would have overwritten something of yours — and lines that
    /// would not parse, which are refused for the same reason.
    pub refused: usize,
}

/// A personal-layer store: an index over a log, which knows how to rebuild
/// itself from one.
///
/// The implementor owns its index and its `Log`; this only asks the questions
/// [`open`] needs answered.
pub trait Store: Sized {
    /// One line of the log.
    type Record: Serialize + DeserializeOwned;

    /// What a record is called, in a message a reader will see: *"a mark"*,
    /// *"a saved query"*. `Log::live` puts it in front of the line number when
    /// a line will not parse.
    const WHAT: &'static str;

    /// The key that makes a later line replace an earlier one.
    ///
    /// The whole of what "append-only, last one wins" means, and it is per
    /// store: a mark is keyed by its id, a folder by its name, a document by
    /// its path.
    fn key_of(record: &Self::Record) -> String;

    /// The log this store was replayed from, and writes to.
    fn log(&self) -> &Log;

    /// Put one replayed record into the index.
    ///
    /// Called in log order, so a later record for the same key arrives after
    /// the one it replaces — which is what makes "last one wins" true of the
    /// index and not only of the file.
    fn hold(&mut self, record: Self::Record);

    /// How many records the index holds, which is what `bloated` compares the
    /// line count against.
    fn count(&self) -> usize;

    /// Every record, for compaction. The order is the store's own; it becomes
    /// the order of the rewritten file.
    fn records(&self) -> Vec<&Self::Record>;

    /// Replace the file with exactly what the index holds.
    ///
    /// Rarely overridden. It is on the trait rather than in [`open`] so a store
    /// can offer it as a public operation — `girsa-fix` compacts on demand
    /// after a merge — without six copies of one line.
    ///
    /// # Warning: this discards anything another process appended
    ///
    /// *Exactly what the index holds* is the whole hazard. The index was built
    /// when the store was opened; a second Girsa on the same layer that has
    /// appended since is not in it, and this rewrites the file without those
    /// lines. They are not recoverable — the file was the only copy.
    ///
    /// The automatic compaction in [`open`] does not have this problem: it
    /// re-reads from the offset it last read to and carries forward whatever it
    /// finds, with a test that fails without it. Closing the same gap **here**
    /// means a read offset threaded through this trait and all six stores that
    /// implement it, which is a wider change than the finding that fixed
    /// [`open`] called for and is deliberately not smuggled into it.
    ///
    /// Until then: call this only from a tool that owns the layer for the
    /// duration. `girsa-fix` is such a tool by convention and not by
    /// enforcement, which is the honest statement of where this stands.
    ///
    /// # Errors
    ///
    /// If a record will not serialize, or the file will not write or rename.
    fn compact(&self) -> Result<(), LogError> {
        self.log().rewrite(self.records())
    }

    /// The file, as it goes to disk: one record a line, so it is greppable and
    /// a diff of it is readable.
    ///
    /// Three of the stores had this character for character —
    /// `girsa_note::{collection, mark, query}` — differing only in the word
    /// each used for its loop variable. It is not a fact about a mark or a
    /// folder or a saved question; it is *what a jsonl file is*, which is this
    /// crate's whole subject.
    ///
    /// A record that will not serialize is skipped rather than fatal, which is
    /// the same call [`Log::rewrite`] makes and for the same reason: the
    /// alternative is a reader whose entire file will not write because of one
    /// line in it.
    /// Take somebody else's file of these (spec.md §11).
    ///
    /// The same shape `girsa-fix` has had for corrections since W20, on the
    /// trait rather than in each store, because *what to do with a record
    /// somebody else made* has one answer for a mark, a saved question and a
    /// chaburah folder and it is this one:
    ///
    /// | | |
    /// |---|---|
    /// | a key I do not hold | **taken** |
    /// | a key I hold, and their record is mine to the letter | already had |
    /// | a key I hold, and their record differs | **refused** |
    ///
    /// The third row is the whole of it. Two people's saved questions are both
    /// called `שאלה` and are two different questions; two chaburah folders are
    /// both called `ברכות` and hold different lines. Last-one-wins is right
    /// *within* a layer, where the later line is the same person changing their
    /// mind, and it is exactly wrong across two — it would silently replace
    /// your folder with theirs, and nothing on the screen afterwards would say
    /// so. This is the same call `girsa-fix` makes when two corrections claim
    /// the same letters: the system does not choose between two people.
    ///
    /// *Mine to the letter* is compared as the serialized line rather than with
    /// `PartialEq`, so no record type has to derive anything. The file is what
    /// is being merged, and two records that write the same line are the same
    /// record by the only definition this layer has.
    ///
    /// Idempotent, and their tombstones stop at their own file: their log is
    /// replayed, so a mark they made and took back is not one they are
    /// offering, and what is taken is what they hold — never a deletion of what
    /// you hold.
    ///
    /// # Errors
    ///
    /// If their file cannot be read, or yours cannot be appended to.
    fn merge(&mut self, file: &Path) -> Result<Merged, LogError> {
        let named = file.display().to_string();
        // Bytes, and not `read_to_string`: their file is a log like ours, with
        // the same torn-tail exposure, and one bad line is theirs to lose —
        // not a reason to refuse the merge outright.
        let body = std::fs::read(file).map_err(|source| LogError {
            path: named.clone(),
            source,
        })?;
        let theirs = crate::replay::<Self::Record>(&body, &named, Self::WHAT, Self::key_of);
        let mut merged = Merged {
            refused: theirs.trouble.len(),
            ..Merged::default()
        };
        // Mine, once, as lines. Built up front rather than scanned per record:
        // a layer with a thousand marks taking a file of a thousand more is a
        // million comparisons the other way, for no reason.
        //
        // `None` is *I hold this key and cannot write it down*, which is a
        // different fact from *I do not hold it* and has to compare differently
        // — see the doc on [`Log::line_of`] and issue #59, where two records
        // that would not serialize both became `""`, compared equal, and the
        // merge reported somebody's record as one it already had.
        let mine: BTreeMap<String, Option<String>> = self
            .records()
            .into_iter()
            .map(|record| (Self::key_of(record), self.log().line_of(record).ok()))
            .collect();

        let mut taking: Vec<Self::Record> = Vec::new();
        for record in theirs.records {
            // A record of theirs that will not write is refused here rather
            // than failing the whole merge at the append — `append_all` would
            // take the whole file down with it, and one bad record in their
            // layer is theirs to lose.
            let Ok(line) = self.log().line_of(&record) else {
                merged.refused += 1;
                continue;
            };
            match mine.get(&Self::key_of(&record)) {
                Some(Some(held)) if *held == line => merged.already_had += 1,
                Some(_) => merged.refused += 1,
                None => taking.push(record),
            }
        }
        // One append for the whole file, and held only once it is down — so a
        // machine that stops mid-merge has a layer whose index and whose file
        // say the same thing.
        self.log().append_all(taking.iter())?;
        for record in taking {
            self.hold(record);
            merged.taken += 1;
        }
        Ok(merged)
    }

    /// Every record as one text, for a caller writing the whole store out.
    ///
    /// # Errors
    ///
    /// If a record will not serialize. It used to drop the record and say
    /// nothing, which in an export means writing a file of the reader's own
    /// layer with fewer records in it than the layer holds and reporting the
    /// full count anyway (issue #59).
    fn to_text(&self) -> Result<String, LogError> {
        let mut body = String::new();
        for record in self.records() {
            body.push_str(&self.log().line_of(record)?);
            body.push('\n');
        }
        Ok(body)
    }
}

/// Replay a store's log into it, compacting the file if it has grown bloated.
///
/// The caller builds the empty store — with its `Log` already in place, because
/// where the file lives is the store's own business — and this fills it.
///
/// ```text
/// pub fn open(personal: &Path) -> (Self, Vec<String>) {
///     crate::open(Self { log: Log::at(path_in(personal)), by_segment: BTreeMap::new() })
/// }
/// ```
///
/// # The trouble is returned, never raised
///
/// Two things can go wrong and neither is a reason to refuse a reader their
/// layer. A line that will not parse costs that one record and is reported; a
/// compaction that fails leaves a longer file than necessary and is reported.
/// Both come back in the `Vec<String>`, which the window shows. This is
/// spec.md §9.7's rule — never a silent gap — and it is also why `open` cannot
/// return a `Result`: the useful outcome is *the layer, and what was wrong with
/// it*.
#[must_use]
pub fn open<S: Store>(mut store: S) -> (S, Vec<String>) {
    let live = store.log().live::<S::Record>(S::WHAT, S::key_of);
    let mut trouble = live.trouble;
    for record in live.records {
        store.hold(record);
    }
    // The one moment the whole file is written, and only when it has grown past
    // twice what it holds. Failing to tidy up is not a reason to refuse a reader
    // their layer, so it is reported and not returned.
    if Log::bloated(live.lines, store.count()) {
        // `rewrite_after` and not `compact`: this rewrite replaces a file the
        // replay above read a moment ago, and a second process holding the same
        // layer open — the MCP server, `girsa-suspects` — may have appended to
        // it in between. `live.bytes` is where this reading stopped, so
        // everything past it is carried through rather than renamed away.
        if let Err(e) = store.log().rewrite_after(store.records(), Some(live.bytes)) {
            trouble.push(e.to_string());
        }
    }
    (store, trouble)
}

/// `impl From<LogError>` for a store's own error type.
///
/// Five of the six wrote this out identically, and the sixth is about to. It
/// cannot be generic — `From` for a type in another crate is the orphan rule —
/// so it is a macro, which is the honest form of "this is the same eleven lines
/// again" when the language will not let it be a function.
///
/// The error type must have an `Io { path, source }` variant. All six do,
/// because all six are wrapping the same failure: a file in the personal layer
/// would not read or write.
#[macro_export]
macro_rules! io_from_log_error {
    ($err:ty) => {
        impl From<$crate::LogError> for $err {
            fn from(e: $crate::LogError) -> Self {
                Self::Io {
                    path: e.path,
                    source: e.source,
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use serde::{Deserialize, Serializer};
    use std::path::PathBuf;

    /// A record whose `Serialize` **can** fail, which is the whole point.
    ///
    /// Every record type in this repository happens to be plain data, which is
    /// why two `unwrap_or_default()`s sat here unnoticed: nothing in the tree
    /// could reach them, so nothing in the tree could say what they did. A
    /// `Serialize` that refuses is allowed by the trait and is what a store
    /// written against a foreign type inherits (issue #59).
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    struct Unwritable {
        id: String,
    }

    impl Serialize for Unwritable {
        fn serialize<S: Serializer>(&self, _into: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom(
                "a record with no way to be written",
            ))
        }
    }

    /// The store: a log of `Unwritable`, keyed by `id`.
    #[derive(Debug, Clone)]
    struct Things {
        log: Log,
        held: BTreeMap<String, Unwritable>,
    }

    impl Things {
        fn at(path: PathBuf) -> (Self, Vec<String>) {
            crate::open(Self {
                log: Log::at(path),
                held: BTreeMap::new(),
            })
        }
    }

    impl Store for Things {
        type Record = Unwritable;

        const WHAT: &'static str = "a thing";

        fn key_of(record: &Self::Record) -> String {
            record.id.clone()
        }

        fn log(&self) -> &Log {
            &self.log
        }

        fn hold(&mut self, record: Self::Record) {
            self.held.insert(record.id.clone(), record);
        }

        fn count(&self) -> usize {
            self.held.len()
        }

        fn records(&self) -> Vec<&Self::Record> {
            self.held.values().collect()
        }
    }

    fn unwritable(id: &str) -> Unwritable {
        Unwritable { id: id.to_string() }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-store-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory");
        dir.join("things.jsonl")
    }

    #[test]
    fn a_record_that_will_not_write_is_an_error_and_not_an_empty_line() {
        // `to_text` is what an export writes. Dropping the record and returning
        // the count of what the layer *holds* is the export claiming a record
        // it did not write.
        let (mut things, _) = Things::at(scratch("unwritable"));
        things.held.insert("a".into(), unwritable("a"));
        assert!(
            things.to_text().is_err(),
            "an export claimed a record it could not serialize"
        );
        let refused = things.log().line_of(&unwritable("a"));
        assert!(
            refused.is_err(),
            "a `Serialize` that refuses produced a line: {refused:?}"
        );
        assert!(
            refused.unwrap_err().to_string().contains("things.jsonl"),
            "the error names the file it was about"
        );
    }

    #[test]
    fn two_records_that_will_not_write_are_not_the_same_record() {
        // The merge compared `""` with `""`, so a file of two unserializable
        // records on both sides was reported as one already had — the count
        // saying the merge worked when nothing had been compared at all.
        let (mut mine, _) = Things::at(scratch("both-unwritable"));
        mine.held.insert("a".into(), unwritable("a"));

        let theirs = scratch("their-unwritable");
        let mut body = String::new();
        body.push_str("{\"id\":\"a\"}\n");
        std::fs::write(&theirs, body).expect("writes");

        let merged = mine.merge(&theirs).expect("the merge happens");
        // Their line will not *parse* as this record type either, so it is
        // trouble and refused — and the point is that it is never `already_had`
        // against a record nobody could read.
        assert_eq!(merged.already_had, 0, "{merged:?}");
        assert_eq!(merged.taken, 0, "{merged:?}");
        assert_eq!(merged.refused, 1, "{merged:?}");
    }
}
