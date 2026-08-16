//! An append-only jsonl file, keyed by whatever names a record.
//!
//! # The file
//!
//! One JSON object a line, the way every store under `personal/` already wrote
//! it. Two things are new, and both are additive:
//!
//! * **A key may appear more than once**, and the last line wins. That is how a
//!   record is changed: you append the new one.
//! * **`{"gone":"<key>"}` is a tombstone.** That is how a record is taken back.
//!
//! A file that has neither — which is every file any earlier version of Girsa
//! wrote — replays to exactly the records it always meant. There is no version
//! field and nothing to migrate.
//!
//! A tombstone is told from a record by [`Gone`]'s `deny_unknown_fields`: only a
//! line that is exactly `{"gone": "…"}` and nothing else can be one, so a record
//! that happens to carry a `gone` field is still read as a record.
//!
//! # What a crash costs
//!
//! The old stores wrote the whole file to `…jsonl.writing` and renamed it over,
//! so a machine that stopped halfway had the corrections it started with. This
//! appends, so a machine that stops halfway has a torn last line — which
//! [`Log::live`] reports and skips, costing that record and possibly the one
//! written after it.
//!
//! That is the same order of loss for a smaller window: the old scheme had the
//! **entire layer** in flight on every single mutation, and this has one line.
//! Compaction is the one whole-file write left, and it still goes through
//! write-beside-and-rename.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// A tombstone: the key it names is no longer in the layer.
///
/// `deny_unknown_fields` is load-bearing. It is what makes this unambiguous
/// against every record type in the tree — a `Patch`, a `Mark`, a `Suspect`
/// carries fields this does not accept, so it can never be read as a deletion.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gone {
    gone: String,
}

/// Whether a line of a personal-layer file is a tombstone rather than a record.
///
/// For the two callers that count lines of somebody's layer without parsing
/// them — a deletion is not a thing to be indexed.
#[must_use]
pub fn is_tombstone(line: &str) -> bool {
    serde_json::from_str::<Gone>(line).is_ok()
}

/// The one field a caller counting *what has changed since* needs.
///
/// Every record type in this layer carries a `when` — seconds since the epoch,
/// written when the record was made. This reads that and nothing else, so it
/// deserialises a `Patch`, a `Mark`, a saved question and a folder alike
/// without any of them being nameable from here.
#[derive(Debug, Clone, Deserialize)]
struct Stamp {
    /// `None` on a record written before the field existed.
    #[serde(default)]
    when: Option<u64>,
    /// What a tombstone would name, where the record's key **is** its `id`.
    ///
    /// Every keyed store in this layer whose key is a field spells it `id`, and
    /// a tombstone carries that string in `gone`. Where a store's key is
    /// composed of other things instead — `girsa_link::repair` keys on the pair
    /// of anchors and the kind of statement — there is nothing here to match on
    /// and the record is counted as it always was. That is the honest limit,
    /// and it is why this is `Option` rather than a required field.
    #[serde(default)]
    id: Option<String>,
}

/// How many records a log holds, and how many were written after a moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Since {
    /// Live records — blank lines and tombstones excluded. A deletion is not a
    /// thing for an index to go and find.
    pub records: usize,
    /// Of those, the ones written after the moment.
    ///
    /// **A record that cannot be dated is counted as after it.** A timestamp
    /// this cannot read is not a reason to say nothing about the record it
    /// belongs to: over-reporting sends a reader to rebuild an index they might
    /// not have needed to, under-reporting is a silent gap, and of the two only
    /// one is a lie.
    pub after: usize,
}

/// Count a personal-layer log against a moment, in seconds since the epoch.
///
/// # Why this is here and not where the record type is
///
/// `girsa_note::since` needs to know how many corrections are newer than the
/// search index. Corrections are `girsa_fix`'s, and `girsa-note` may not depend
/// on `girsa-fix` — they are siblings, and neither may name the other. So it
/// did this:
///
/// ```ignore
/// if !body.contains("\"when\"") { … }
/// line.split("\"when\"").nth(1)?.trim_start_matches([':', ' ', '"'])
/// ```
///
/// One crate parsing another crate's file by string surgery, with `serde_json`
/// sitting unused in its own manifest, purely because it is forbidden to name
/// the `Patch` type. It happened to be correct — a `"when"` inside a string
/// value is escaped, so the split cannot land in one — and it was correct by
/// luck rather than by construction, and it would go on being silently correct
/// right up until somebody added a field called `whenever`.
///
/// The answer is not to name `Patch`. It is that **counting records in a log is
/// a fact about the log format**, and the log format is this crate's — the same
/// argument that already puts [`is_tombstone`] here.
///
/// # A tombstone takes its record with it
///
/// This skipped the tombstone **line** and went on counting the record the
/// tombstone had killed. So a correction made and then taken back was counted
/// twice over as a live correction the index had not seen, and the results
/// header said *"2 corrections made since then are still findable by the typo
/// and not by the fix"* about two corrections that no longer existed anywhere.
///
/// [`Since::records`] has said *"blank lines and tombstones excluded — a
/// deletion is not a thing for an index to go and find"* since it was written.
/// That was the intent; excluding the tombstone line is not the same act as
/// excluding what it deletes, and nothing here noticed the difference. It
/// became reachable from a second direction when the MCP end grew `uncorrect`,
/// which is how it was found.
///
/// So the walk is now the same replay [`Log::live`] does, **in order**, and the
/// order is what makes it right rather than a set difference: a `PatchId` is
/// content-addressed, so correcting the same words the same way after undoing
/// produces the *same* key — record, tombstone, record — and that third line is
/// a live correction. Collecting the tombstoned keys in one pass and subtracting
/// them in another would drop it.
#[must_use]
pub fn since(body: &str, seconds: u64) -> Since {
    // Keyed records, last line for a key winning, exactly as `replay` resolves
    // them. `None` where the record carries no `id` to key on — those are held
    // in order and never removed, which is what happened to every record before
    // this.
    let mut keyed: BTreeMap<String, bool> = BTreeMap::new();
    let mut unkeyed: Vec<bool> = Vec::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(stone) = serde_json::from_str::<Gone>(line) {
            keyed.remove(&stone.gone);
            continue;
        }
        let stamp = serde_json::from_str::<Stamp>(line).ok();
        // No `when`, or a line that will not parse at all. Both are records
        // this cannot date, and an undated record counts as new.
        let after = stamp
            .as_ref()
            .and_then(|s| s.when)
            .is_none_or(|when| when > seconds);
        match stamp.and_then(|s| s.id) {
            Some(id) => {
                keyed.insert(id, after);
            }
            None => unkeyed.push(after),
        }
    }
    let live = keyed.values().copied().chain(unkeyed);
    let mut counted = Since::default();
    for after in live {
        counted.records += 1;
        if after {
            counted.after += 1;
        }
    }
    counted
}

/// Anything that stopped a record reaching the disk.
///
/// One variant, because there is one thing a caller can do about any of them
/// and every store already had exactly this shape. A record that will not
/// serialize arrives as an [`std::io::ErrorKind::InvalidData`], which is what
/// the six stores did by hand.
#[derive(Debug, thiserror::Error)]
#[error("writing {path}: {source}")]
pub struct LogError {
    pub path: String,
    #[source]
    pub source: std::io::Error,
}

/// What replaying a log left standing.
#[derive(Debug)]
pub struct Live<T> {
    /// The surviving records, in key order.
    pub records: Vec<T>,
    /// How many lines the file held — records, repeats and tombstones alike.
    /// [`Log::bloated`] compares this against what survived.
    pub lines: usize,
    /// A line that would not parse costs that record and is reported here.
    /// Never the whole file: one bad line silently un-correcting a library is
    /// the failure this shape exists to avoid.
    pub trouble: Vec<String>,
}

/// Replay a log that is already in hand.
///
/// Split out from [`Log::live`] for the one caller that has the bytes and not
/// the file: taking somebody else's corrections (spec.md §7.1) means reading
/// *their* layer, and their layer has the same repeats and the same tombstones
/// as yours. Resolving them here is what makes a merge idempotent — the same
/// file taken twice is the same corrections — rather than a count of how many
/// times they happened to change their mind.
///
/// `named` is how the file is spoken of in the trouble report.
pub fn replay<T: DeserializeOwned>(
    body: &str,
    named: &str,
    what: &str,
    key: impl Fn(&T) -> String,
) -> Live<T> {
    let mut live = Live {
        records: Vec::new(),
        lines: 0,
        trouble: Vec::new(),
    };
    let mut held: BTreeMap<String, T> = BTreeMap::new();
    for (n, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        live.lines += 1;
        if let Ok(stone) = serde_json::from_str::<Gone>(line) {
            held.remove(&stone.gone);
            continue;
        }
        match serde_json::from_str::<T>(line) {
            Ok(record) => {
                held.insert(key(&record), record);
            }
            Err(e) => live
                .trouble
                .push(format!("{named}: line {} is not {what}: {e}", n + 1)),
        }
    }
    live.records = held.into_values().collect();
    live
}

/// A file's worth of records under the reader's own layer.
///
/// Cheap to clone and holds no file handle, so a store that derives `Clone`
/// still can. Every write opens the file, appends, and closes.
#[derive(Debug, Clone)]
pub struct Log {
    path: PathBuf,
}

/// Below this many lines, rewriting the file is not worth the syscalls — a
/// layer with four marks in it does not need compacting because one was
/// deleted twice.
const FLOOR: usize = 64;

impl Log {
    /// The log at a path.
    #[must_use]
    pub const fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// A log that is never written, for a caller that only wants to apply what
    /// it already has. Every write is a no-op and [`Log::live`] finds nothing.
    #[must_use]
    pub fn nowhere() -> Self {
        Self {
            path: PathBuf::new(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether this log is the one that is never written.
    #[must_use]
    pub fn is_nowhere(&self) -> bool {
        self.path.as_os_str().is_empty()
    }

    /// Replay the file: the last line for a key wins, a tombstone removes it.
    ///
    /// `what` names the record in the trouble report — *"line 12 is not a
    /// correction"* — so a reader is told which of their files is hurt.
    ///
    /// `key` is what names a record. Two records with the same key are the same
    /// record said twice and the later one stands; it is also the string a
    /// tombstone carries, so the two have to agree.
    pub fn live<T: DeserializeOwned>(&self, what: &str, key: impl Fn(&T) -> String) -> Live<T> {
        let Ok(body) = std::fs::read_to_string(&self.path) else {
            return Live {
                records: Vec::new(),
                lines: 0,
                trouble: Vec::new(),
            };
        };
        replay(&body, &self.path.display().to_string(), what, key)
    }

    /// Whether the file has grown far enough past what it holds to be worth
    /// rewriting.
    ///
    /// Twice the live set plus a floor. That is what makes the whole thing
    /// amortized: a store can never spend more than one full rewrite per *n*
    /// mutations, and the file can never be more than about twice the size of
    /// what is in it.
    #[must_use]
    pub const fn bloated(lines: usize, live: usize) -> bool {
        lines > live.saturating_mul(2).saturating_add(FLOOR)
    }

    /// Write one record down.
    ///
    /// # Errors
    ///
    /// If it will not serialize, or the file will not open or append.
    pub fn append<T: Serialize>(&self, record: &T) -> Result<(), LogError> {
        if self.is_nowhere() {
            return Ok(());
        }
        let line = serde_json::to_string(record).map_err(|e| self.invalid(&e))?;
        self.push(std::iter::once(line))
    }

    /// Write several records down, in one append.
    ///
    /// For a merge, which takes a whole file of somebody else's corrections and
    /// should cost one write rather than one per correction.
    ///
    /// # Errors
    ///
    /// If a record will not serialize, or the file will not open or append.
    pub fn append_all<'a, T: Serialize + 'a>(
        &self,
        records: impl IntoIterator<Item = &'a T>,
    ) -> Result<(), LogError> {
        if self.is_nowhere() {
            return Ok(());
        }
        let mut lines = Vec::new();
        for record in records {
            lines.push(serde_json::to_string(record).map_err(|e| self.invalid(&e))?);
        }
        self.push(lines)
    }

    /// Write down that some keys are gone.
    ///
    /// # Errors
    ///
    /// If the file will not open or append.
    pub fn took<K: AsRef<str>>(&self, keys: &[K]) -> Result<(), LogError> {
        if self.is_nowhere() || keys.is_empty() {
            return Ok(());
        }
        let mut lines = Vec::with_capacity(keys.len());
        for key in keys {
            let stone = Gone {
                gone: key.as_ref().to_string(),
            };
            lines.push(serde_json::to_string(&stone).map_err(|e| self.invalid(&e))?);
        }
        self.push(lines)
    }

    /// Replace the file with exactly these records.
    ///
    /// This is compaction, and it is also what a batch job that rebuilds a
    /// whole queue wants. The one write in this module that is not an append,
    /// and so the one that still goes beside-and-renames.
    ///
    /// # Errors
    ///
    /// If a record will not serialize, or the file will not write or rename.
    pub fn rewrite<'a, T: Serialize + 'a>(
        &self,
        records: impl IntoIterator<Item = &'a T>,
    ) -> Result<(), LogError> {
        if self.is_nowhere() {
            return Ok(());
        }
        self.make_room()?;
        let mut body = String::new();
        for record in records {
            body.push_str(&serde_json::to_string(record).map_err(|e| self.invalid(&e))?);
            body.push('\n');
        }
        // Written beside and renamed over, so a machine that stops halfway
        // through leaves the layer it had rather than half of it.
        let temp = self.path.with_extension("jsonl.writing");
        std::fs::write(&temp, body).map_err(|e| self.io(e))?;
        std::fs::rename(&temp, &self.path).map_err(|e| self.io(e))
    }

    fn push(&self, lines: impl IntoIterator<Item = String>) -> Result<(), LogError> {
        let mut body = String::new();
        for line in lines {
            body.push_str(&line);
            body.push('\n');
        }
        if body.is_empty() {
            return Ok(());
        }
        self.make_room()?;
        // One `write_all` on a handle opened for append, so the line lands
        // whole or the tear is at the end of the file, where replay can see it.
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| self.io(e))?;
        file.write_all(body.as_bytes()).map_err(|e| self.io(e))?;
        file.flush().map_err(|e| self.io(e))
    }

    fn make_room(&self) -> Result<(), LogError> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| self.io(e))?;
        }
        Ok(())
    }

    fn io(&self, source: std::io::Error) -> LogError {
        LogError {
            path: self.path.display().to_string(),
            source,
        }
    }

    fn invalid(&self, e: &serde_json::Error) -> LogError {
        self.io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Thing {
        id: String,
        says: String,
    }

    fn thing(id: &str, says: &str) -> Thing {
        Thing {
            id: id.to_string(),
            says: says.to_string(),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-log-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("things.jsonl")
    }

    fn read(log: &Log) -> Vec<Thing> {
        log.live("a thing", |t: &Thing| t.id.clone()).records
    }

    #[test]
    fn a_file_the_old_store_wrote_replays_to_what_it_always_meant() {
        // The whole migration story, and it is that there isn't one: no
        // repeated keys and no tombstones is a file that is its own compaction.
        let path = scratch("old-file");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "{\"id\":\"a\",\"says\":\"one\"}\n{\"id\":\"b\",\"says\":\"two\"}\n",
        )
        .unwrap();
        let log = Log::at(path);
        assert_eq!(read(&log), vec![thing("a", "one"), thing("b", "two")]);
    }

    #[test]
    fn the_last_line_for_a_key_is_the_one_that_stands() {
        let log = Log::at(scratch("last-wins"));
        log.append(&thing("a", "one")).unwrap();
        log.append(&thing("b", "two")).unwrap();
        log.append(&thing("a", "again")).unwrap();
        assert_eq!(read(&log), vec![thing("a", "again"), thing("b", "two")]);
    }

    #[test]
    fn a_tombstone_takes_a_record_back_and_a_later_record_brings_it_home() {
        let log = Log::at(scratch("tombstone"));
        log.append(&thing("a", "one")).unwrap();
        log.took(&["a"]).unwrap();
        assert_eq!(read(&log), Vec::new());
        log.append(&thing("a", "risen")).unwrap();
        assert_eq!(read(&log), vec![thing("a", "risen")]);
    }

    #[test]
    fn a_record_is_never_mistaken_for_a_deletion() {
        // `Thing` has no `gone` field, so this is about the other direction: a
        // record line must not satisfy `Gone`. `deny_unknown_fields` is why.
        #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Awkward {
            gone: String,
            when: u64,
        }
        let path = scratch("awkward");
        let log = Log::at(path);
        log.append(&Awkward {
            gone: "a".into(),
            when: 7,
        })
        .unwrap();
        let live = log.live("an awkward thing", |a: &Awkward| a.gone.clone());
        assert_eq!(live.records.len(), 1, "it is a record, not a tombstone");
        assert!(live.trouble.is_empty());
    }

    #[test]
    fn one_bad_line_costs_that_record_and_is_reported() {
        let path = scratch("bad-line");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "{\"id\":\"a\",\"says\":\"one\"}\n{\"id\":\"b\",\"sa\n{\"id\":\"c\",\"says\":\"three\"}\n",
        )
        .unwrap();
        let log = Log::at(path);
        let live = log.live("a thing", |t: &Thing| t.id.clone());
        assert_eq!(live.records, vec![thing("a", "one"), thing("c", "three")]);
        assert_eq!(live.trouble.len(), 1);
        assert!(live.trouble[0].contains("line 2 is not a thing"));
    }

    #[test]
    fn a_torn_last_line_costs_the_last_record_and_nothing_before_it() {
        // What a crash mid-append actually leaves. The records already on the
        // file are untouched, which is the property that matters.
        let path = scratch("torn");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "{\"id\":\"a\",\"says\":\"one\"}\n{\"id\":\"b\",\"say",
        )
        .unwrap();
        let log = Log::at(path);
        let live = log.live("a thing", |t: &Thing| t.id.clone());
        assert_eq!(live.records, vec![thing("a", "one")]);
        assert_eq!(live.trouble.len(), 1);
    }

    #[test]
    fn compaction_leaves_the_same_records_and_a_shorter_file() {
        let log = Log::at(scratch("compact"));
        for n in 0..200 {
            log.append(&thing("a", &format!("try {n}"))).unwrap();
        }
        let before = log.live("a thing", |t: &Thing| t.id.clone());
        assert_eq!(before.lines, 200);
        assert_eq!(before.records, vec![thing("a", "try 199")]);
        assert!(Log::bloated(before.lines, before.records.len()));

        log.rewrite(before.records.iter()).unwrap();
        let after = log.live("a thing", |t: &Thing| t.id.clone());
        assert_eq!(after.lines, 1);
        assert_eq!(after.records, vec![thing("a", "try 199")]);
        assert!(!Log::bloated(after.lines, after.records.len()));
    }

    #[test]
    fn a_small_layer_is_never_rewritten_for_a_stray_deletion() {
        // Four marks and one of them deleted twice is not a file worth
        // rewriting, and the floor is what says so.
        assert!(!Log::bloated(6, 4));
        assert!(!Log::bloated(64, 0));
        assert!(Log::bloated(65, 0));
        assert!(Log::bloated(201, 60));
        assert!(!Log::bloated(200, 100));
    }

    #[test]
    fn a_log_that_is_nowhere_writes_nothing_and_says_so() {
        let log = Log::nowhere();
        assert!(log.is_nowhere());
        log.append(&thing("a", "one")).unwrap();
        log.took(&["a"]).unwrap();
        log.rewrite([&thing("a", "one")]).unwrap();
        assert_eq!(read(&log), Vec::new());
    }

    #[test]
    fn a_thousand_writes_leave_a_thousand_lines_and_not_half_a_million() {
        // The finding itself. Under the old stores this file would have had
        // 1 + 2 + … + 1000 = 500,500 lines written through it; here it is 1,000
        // lines written once each, and the file is 1,000 lines long.
        let log = Log::at(scratch("flat"));
        for n in 0..1_000 {
            log.append(&thing(&format!("{n:04}"), "held")).unwrap();
        }
        let live = log.live("a thing", |t: &Thing| t.id.clone());
        assert_eq!(live.lines, 1_000);
        assert_eq!(live.records.len(), 1_000);
        assert!(!Log::bloated(live.lines, live.records.len()));
    }

    #[test]
    fn a_record_with_no_stamp_counts_as_new() {
        // The safe direction. Over-reporting sends a reader to rebuild an index
        // they might not have needed to; under-reporting is a silent gap.
        let body = "{\"id\":\"a\"}\n{\"id\":\"b\",\"when\":100}\n";
        assert_eq!(
            since(body, 500),
            Since {
                records: 2,
                after: 1
            }
        );
    }

    #[test]
    fn a_tombstone_is_not_a_record_and_neither_is_what_it_deleted() {
        // This test said the first half and asserted against the second. Its
        // comment was already right — "a line saying a correction is gone is
        // not something for an index to go and find" — and it required
        // `records: 1` for a body holding one correction and its tombstone.
        // So the counter skipped the stone, kept the dead record, and the
        // results header reported corrections that no longer existed.
        let body = "{\"id\":\"a\",\"when\":900}\n{\"gone\":\"a\"}\n";
        assert_eq!(
            since(body, 100),
            Since {
                records: 0,
                after: 0
            }
        );
        // And what the tombstone did not name is untouched.
        let and_another =
            "{\"id\":\"a\",\"when\":900}\n{\"gone\":\"a\"}\n{\"id\":\"b\",\"when\":900}\n";
        assert_eq!(
            since(and_another, 100),
            Since {
                records: 1,
                after: 1
            }
        );
    }

    #[test]
    fn a_stamp_on_the_moment_itself_is_not_after_it() {
        let body = "{\"when\":100}\n";
        assert_eq!(since(body, 100).after, 0);
        assert_eq!(since(body, 99).after, 1);
    }

    #[test]
    fn a_line_that_will_not_parse_costs_nothing_and_counts_as_new() {
        // One bad line silently un-counting a reader's corrections is the
        // failure this whole module's shape exists to avoid.
        let body = "{\"when\":100}\nnot json at all\n";
        assert_eq!(
            since(body, 500),
            Since {
                records: 2,
                after: 1
            }
        );
    }

    #[test]
    fn the_word_when_inside_somebody_s_note_is_not_a_timestamp() {
        // What the string surgery in `girsa_note::since` could not see it was
        // relying on. `line.split("\"when\"")` was safe only because JSON
        // escapes a quote inside a value — so this line's `"when"` is
        // `\"when\"` on disk and the split misses it. Correct by luck. Here it
        // is a field, read as a field.
        let body = "{\"note\":\"he wrote \\\"when\\\" here\",\"when\":100}\n";
        assert_eq!(
            since(body, 500),
            Since {
                records: 1,
                after: 0
            }
        );
        assert_eq!(
            since(body, 50),
            Since {
                records: 1,
                after: 1
            }
        );
    }

    #[test]
    fn a_field_whose_name_merely_ends_in_when_is_not_the_stamp() {
        // `whenever` — the rename that would have made the old parser wrong,
        // written down so it stays wrong-proof.
        let body = "{\"whenever\":100,\"when\":900}\n";
        assert_eq!(since(body, 500).after, 1);
        let only = "{\"whenever\":900}\n";
        assert_eq!(since(only, 500).after, 1, "undated counts as new");
    }

    #[test]
    fn a_record_written_again_after_its_tombstone_is_live() {
        // Why this replays in order instead of subtracting a set of dead keys.
        // A `PatchId` is content-addressed, so correcting the same words the
        // same way after undoing produces the **same** key — and the third line
        // is a correction that exists. Collecting the stones in one pass and
        // removing them in another would delete it.
        let body = concat!(
            "{\"id\":\"a\",\"when\":900}\n",
            "{\"gone\":\"a\"}\n",
            "{\"id\":\"a\",\"when\":950}\n",
        );
        assert_eq!(
            since(body, 500),
            Since {
                records: 1,
                after: 1
            }
        );
    }

    #[test]
    fn saying_the_same_record_twice_is_one_record() {
        // The other half of last-line-wins, which `replay` has always had and
        // this counter did not: a store that appends rather than rewrites can
        // hold a key twice, and two lines about one correction are one
        // correction.
        let body = "{\"id\":\"a\",\"when\":100}\n{\"id\":\"a\",\"when\":900}\n";
        assert_eq!(
            since(body, 500),
            Since {
                records: 1,
                after: 1
            },
            "the later line is the one that stands, and it is after"
        );
    }

    #[test]
    fn a_record_with_no_id_is_counted_as_it_always_was() {
        // The honest limit. A store whose key is composed rather than a field —
        // `girsa_link::repair` keys on the pair of anchors and the kind of
        // statement — has nothing here to match a tombstone against, so nothing
        // is removed and nothing is worse than before.
        let body = "{\"when\":900}\n{\"gone\":\"whatever\"}\n{\"when\":900}\n";
        assert_eq!(
            since(body, 500),
            Since {
                records: 2,
                after: 2
            }
        );
    }

    #[test]
    fn an_empty_line_is_not_a_record() {
        assert_eq!(
            since("\n\n{\"when\":9}\n\n", 0),
            Since {
                records: 1,
                after: 1
            }
        );
    }
}
