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
        let body = std::fs::read_to_string(file).map_err(|source| LogError {
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
        let mine: BTreeMap<String, String> = self
            .records()
            .into_iter()
            .map(|record| {
                (
                    Self::key_of(record),
                    serde_json::to_string(record).unwrap_or_default(),
                )
            })
            .collect();

        let mut taking: Vec<Self::Record> = Vec::new();
        for record in theirs.records {
            let line = serde_json::to_string(&record).unwrap_or_default();
            match mine.get(&Self::key_of(&record)) {
                Some(held) if *held == line => merged.already_had += 1,
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

    #[must_use]
    fn to_text(&self) -> String {
        let mut body = String::new();
        for record in self.records() {
            if let Ok(line) = serde_json::to_string(record) {
                body.push_str(&line);
                body.push('\n');
            }
        }
        body
    }
}

/// Replay a store's log into it, compacting the file if it has grown bloated.
///
/// The caller builds the empty store — with its `Log` already in place, because
/// where the file lives is the store's own business — and this fills it.
///
/// ```ignore
/// pub fn open(personal: &Path) -> (Self, Vec<String>) {
///     girsa_personal::open(Self { log: Log::at(path_in(personal)), by_segment: BTreeMap::new() })
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
