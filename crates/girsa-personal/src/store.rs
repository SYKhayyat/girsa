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

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::log::{Log, LogError};

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
        if let Err(e) = store.compact() {
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
