//! Your own layer, and the file it is written to.
//!
//! Everything you make in Girsa — corrections, marks, saved questions, folders,
//! judgments about links, decisions about spelling candidates — lands in a jsonl
//! file under `personal/`. Six crates grew six copies of the same store, and all
//! six had the same defect, so this crate is the one copy.
//!
//! # What was wrong with the six
//!
//! Every one of them held its records in a map and wrote the **whole map** on
//! every mutation. That reads well and it is quadratic: correcting *n* typos
//! costs *n* full serializations of a file that is *n* lines long. The reading
//! pane's own guardrail measured it and printed the slope —
//!
//! ```text
//! 18120 segments, no corrections yet:        75 ms
//! 18120 segments, 1000 corrections already: 217 ms
//! ```
//!
//! — and then stopped at a thousand, which is the last size at which it passes.
//! spec.md §7.5 says three seconds is the whole interaction. The `girsa-suspects`
//! queue reaches **28,124 entries on the real corpus**, and its whole pitch is
//! being handed thousands of ranked candidates and going through them.
//!
//! # What replaces it
//!
//! [`Log`]: the same jsonl file, read as an append-only log. A record is a line,
//! a later line for the same key wins, and a line that says a key is gone is a
//! tombstone. Writing one record appends one line. Opening replays the file and
//! rewrites it only when it has grown past twice what it holds.
//!
//! The format did not change: a file written by the old stores replays to
//! exactly what it used to mean, because a file with no repeated keys and no
//! tombstones is its own compaction. Nothing has to be migrated, which matters
//! more here than anywhere else in the tree — this is the one directory a reader
//! cannot re-download.
//!
//! And the file stays greppable and diffable (spec.md §4.1, §11), which was the
//! reason it was jsonl in the first place.

pub mod log;
pub mod shared;
pub mod store;
pub mod who;

pub use log::{is_tombstone, replay, since, Live, Log, LogError, Since};
pub use shared::{fingerprint, now_seconds, CORRECTIONS};
pub use store::{open, Store};
pub use who::who;
