//! The `.ksav` files you actually have, and the places they cite.
//!
//! # What *your own documents* used to mean
//!
//! spec.md §10.4 promises *"standing on a passage, see which of **your own
//! documents** cite it"*. [`crate::citing::who_cites`] delivered it by
//! iterating `Buffer::list(personal)` — `personal/ksav/*.ksav`, and nothing
//! else. Which is the documents written in **Girsa's own toy editor**: a text
//! box, W17, four hundred lines, built so the loop could be demonstrated
//! without Ksav installed.
//!
//! A `.ksav` written in the real Ksav — *the application this entire pairing
//! exists for* — was never found. The reader's actual work, in the actual
//! editor, answered *nothing cites this*.
//!
//! And after `buffer_to_ksav`, which saves into the personal layer **and**
//! POSTs to the desk, there are two copies of one document with no owner
//! between them, and the answer came from the stale one.
//!
//! # A registry, not a walk
//!
//! There is nowhere to walk. A reader's documents live wherever they keep
//! documents — a Dropbox folder, a shiur directory, a USB stick — and Girsa has
//! no business enumerating a disk. So the desk tells it: Ksav posts
//! `/document` when it saves, and the path lands here.
//!
//! `personal/documents.jsonl`, a [`girsa_personal::Log`] like everything else in
//! the layer. This region already argued for exactly this shape at length, for
//! the link graph — `links.rs`'s module note — and then wrote a directory walk.
//!
//! # The refs are cached and the modification time is the cache key
//!
//! Reading a `.ksav` to pull its refs out is cheap; doing it for two hundred
//! documents on every keystroke of *where did I use this* is not. So each row
//! carries the refs it held and when it was read, and a row is re-read only
//! when the file has moved on. Same rule as `girsa_note::since`, and the same
//! reason: a modification time answers the question actually being asked.
//!
//! A file that has gone is **not forgotten**. It is reported as missing and its
//! cached refs are still answered from, because a document on a USB stick that
//! is not plugged in is not a document that was never written — and quietly
//! dropping a row would be this project's own definition of a silent gap.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_personal::{Log, LogError};
use serde::{Deserialize, Serialize};

/// Where the registry lives.
#[must_use]
pub fn path_in(personal: &Path) -> PathBuf {
    personal.join("documents.jsonl")
}

/// One document the reader has, as the registry holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// Where it is. The key: one path is one document, however many times it
    /// is saved.
    pub path: String,
    /// What to call it on a row. The file stem unless Ksav said otherwise.
    pub name: String,
    /// The refs it held when it was last read.
    #[serde(default)]
    pub refs: Vec<String>,
    /// The file's modification time when those refs were read, in seconds since
    /// the epoch. `0` for a row that has never been read.
    #[serde(default)]
    pub read_at: u64,
    /// When the row was made or last touched. Every record in this layer
    /// carries one — see `girsa_personal::since`.
    #[serde(default)]
    pub when: u64,
}

impl Document {
    /// A row for a path, unread.
    #[must_use]
    pub fn at(path: &Path, name: Option<&str>) -> Self {
        Self {
            name: name.map_or_else(
                || {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string()
                },
                ToString::to_string,
            ),
            path: path.display().to_string(),
            refs: Vec::new(),
            read_at: 0,
            when: now_seconds(),
        }
    }

    /// Whether the file has moved on since its refs were read.
    ///
    /// A file that has gone is **not** stale — there is nothing to re-read, and
    /// what is cached is the last true thing anybody knew about it.
    #[must_use]
    pub fn is_stale(&self) -> bool {
        modified(Path::new(&self.path)).is_some_and(|now| now > self.read_at)
    }

    /// Whether the file is where the registry says it is.
    #[must_use]
    pub fn is_here(&self) -> bool {
        Path::new(&self.path).is_file()
    }
}

/// The documents the reader has told Girsa about.
#[derive(Debug)]
pub struct Documents {
    log: Log,
    by_path: BTreeMap<String, Document>,
}

impl Documents {
    /// Read the registry. A row that will not parse costs that row and is
    /// reported — never the rest of them.
    ///
    /// **And it compacts now**, which is the finding rather than a tidy-up.
    /// Five sibling stores across three crates ask `Log::bloated` here and
    /// rewrite the file when it has grown past twice what it holds; this one,
    /// written last in the crate added most recently, did not. So
    /// `personal/documents.jsonl` grew **without bound on every save** — in the
    /// store whose entire job is to be re-saved, since `remember` is called
    /// each time a document is written and each call appends a line that
    /// supersedes the last one for that path.
    ///
    /// It is one call now, and the call is shared: `girsa_personal::open`.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        girsa_personal::open(Self {
            log: Log::at(path_in(personal)),
            by_path: BTreeMap::new(),
        })
    }

    /// A registry that is never written, for a caller that only reads.
    #[must_use]
    pub fn nowhere() -> Self {
        Self {
            log: Log::nowhere(),
            by_path: BTreeMap::new(),
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &Document> {
        self.by_path.values()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.by_path.len()
    }

    #[must_use]
    pub fn get(&self, path: &Path) -> Option<&Document> {
        self.by_path.get(&path.display().to_string())
    }

    /// Remember a document — the desk's `/document`, and the shell's command.
    ///
    /// Idempotent: saving the same file twice is one row. Re-registering a path
    /// keeps the refs already cached and takes the new name, because the name
    /// is what Ksav calls it and the refs are what the file says.
    ///
    /// # Errors
    ///
    /// If the registry will not take the write.
    pub fn remember(&mut self, path: &Path, name: Option<&str>) -> Result<&Document, LogError> {
        let key = path.display().to_string();
        let row = match self.by_path.remove(&key) {
            Some(known) => Document {
                name: name.map_or(known.name, ToString::to_string),
                when: now_seconds(),
                ..known
            },
            None => Document::at(path, name),
        };
        self.log.append(&row)?;
        Ok(self.by_path.entry(key).or_insert(row))
    }

    /// Forget one. The file is not touched — this is the registry's row.
    ///
    /// # Errors
    ///
    /// If the registry will not take the write.
    pub fn forget(&mut self, path: &Path) -> Result<bool, LogError> {
        let key = path.display().to_string();
        if self.by_path.remove(&key).is_none() {
            return Ok(false);
        }
        self.log.took(&[key])?;
        Ok(true)
    }

    /// Re-read the documents whose files have moved on, and cache their refs.
    ///
    /// Returns how many were re-read. A file that has gone is left alone and
    /// reported by [`Document::is_here`] — see the module note.
    ///
    /// # Errors
    ///
    /// If the registry will not take the write. The in-memory answer is correct
    /// either way; what is lost is the cache surviving a restart.
    pub fn refreshed(&mut self) -> Result<usize, LogError> {
        let stale: Vec<String> = self
            .by_path
            .values()
            .filter(|d| d.is_stale())
            .map(|d| d.path.clone())
            .collect();
        let mut read = Vec::new();
        for key in &stale {
            let Some(row) = self.by_path.get_mut(key) else {
                continue;
            };
            let path = PathBuf::from(&row.path);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            row.refs = girsa_ksav::refs_in(&text);
            row.read_at = modified(&path).unwrap_or(0);
            row.when = now_seconds();
            read.push(row.clone());
        }
        let did = read.len();
        self.log.append_all(read.iter())?;
        Ok(did)
    }
}

/// Now, in seconds since the epoch. Zero if the clock is before it, which is a
/// machine nobody can reason about and not a reason to refuse a write.
fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A file's modification time, in seconds since the epoch.
fn modified(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-documents-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    /// A `.ksav` document citing each of `refs`, written by the writer both
    /// applications compile.
    ///
    /// It used to build the markup here, as `#מקור:("{r}")[]` — which is **not
    /// a Ksav command**. `מקור:` is a named *argument* of `#מראה_מקום`, and
    /// what this wrote is something Ksav cannot emit and Typst cannot compile.
    /// All six tests below were green over it, because `cited_in` scans for the
    /// literal substring `מקור:` and found one.
    ///
    /// In the one crate whose thesis is *no second markup writer*. So the
    /// fixture goes through `girsa_ksav::mekor`, which is that writer, and a
    /// test over markup no reader could ever have is no longer possible here.
    fn ksav(dir: &Path, name: &str, refs: &[&str]) -> PathBuf {
        let path = dir.join(format!("{name}.ksav"));
        let body: String = refs
            .iter()
            .map(|r| girsa_ksav::mekor("", Some(r), None) + "\n")
            .collect();
        std::fs::write(&path, body).expect("a document");
        path
    }

    /// The registry does not grow without bound, which is the whole finding.
    ///
    /// Five sibling stores compact on open and this one did not: `remember` is
    /// called every time Ksav saves, each call appends a line superseding the
    /// last for that path, and nothing ever rewrote the file. A reader who saves
    /// a shiur two hundred times has a two-hundred-line registry holding one
    /// document.
    ///
    /// Written as *save, reopen, save, reopen* rather than as a line count on
    /// one handle, because compaction happens **on open** — which is also the
    /// only moment it can, since that is when the whole set is in memory.
    #[test]
    fn the_registry_compacts_instead_of_growing_forever() {
        let dir = scratch("compacts");
        let doc = dir.join("shiur.ksav");
        std::fs::write(&doc, "").unwrap();

        for _ in 0..200 {
            let (mut docs, trouble) = Documents::open(&dir);
            assert!(trouble.is_empty(), "{trouble:?}");
            docs.remember(&doc, None).unwrap();
        }

        let (docs, _) = Documents::open(&dir);
        assert_eq!(docs.count(), 1, "one path is one document");

        let lines = std::fs::read_to_string(path_in(&dir))
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        // `Log::bloated` allows twice what is live plus a floor of 64 — a layer
        // with four rows in it does not need rewriting because one was saved
        // twice — so the exact number is the log's business. What matters is
        // that it is **bounded**: two hundred saves did not make two hundred
        // lines, and would have.
        assert!(
            lines < 70,
            "{lines} lines for one document — the registry is not compacting"
        );
    }

    #[test]
    fn a_document_anywhere_on_the_disk_is_remembered_and_read() {
        // The whole finding. `who_cites` walked `personal/ksav/` — the toy
        // editor's directory — so a `.ksav` written in the real Ksav, wherever
        // the reader keeps their work, was never found.
        let dir = scratch("anywhere");
        let personal = dir.join("personal");
        std::fs::create_dir_all(&personal).unwrap();
        let elsewhere = dir.join("shiurim");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let doc = ksav(&elsewhere, "חבורה", &["girsa:bavli/berakhot:2a:1"]);

        let (mut documents, trouble) = Documents::open(&personal);
        assert!(trouble.is_empty(), "{trouble:?}");
        documents.remember(&doc, None).expect("it is remembered");
        assert_eq!(documents.refreshed().expect("it reads"), 1);
        let row = documents.get(&doc).expect("it is there");
        assert_eq!(row.name, "חבורה");
        assert_eq!(row.refs, ["girsa:bavli/berakhot:2a:1"]);
    }

    #[test]
    fn the_registry_survives_a_restart_with_its_refs() {
        let dir = scratch("restart");
        let personal = dir.join("personal");
        std::fs::create_dir_all(&personal).unwrap();
        let doc = ksav(&dir, "שיעור", &["girsa:tur:1"]);

        let (mut documents, _) = Documents::open(&personal);
        documents.remember(&doc, Some("שיעור שלי")).unwrap();
        documents.refreshed().unwrap();

        let (reopened, trouble) = Documents::open(&personal);
        assert!(trouble.is_empty(), "{trouble:?}");
        let row = reopened.get(&doc).expect("it survived");
        assert_eq!(row.name, "שיעור שלי");
        assert_eq!(row.refs, ["girsa:tur:1"], "the cache did not survive");
    }

    #[test]
    fn a_document_is_re_read_only_when_the_file_has_moved_on() {
        // Reading a `.ksav` is cheap; reading two hundred of them on every
        // keystroke of *where did I use this* is not.
        let dir = scratch("stale");
        let personal = dir.join("personal");
        std::fs::create_dir_all(&personal).unwrap();
        let doc = ksav(&dir, "א", &["girsa:tur:1"]);

        let (mut documents, _) = Documents::open(&personal);
        documents.remember(&doc, None).unwrap();
        assert_eq!(documents.refreshed().unwrap(), 1, "the first read");
        assert_eq!(documents.refreshed().unwrap(), 0, "and not a second");

        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&doc, girsa_ksav::mekor("", Some("girsa:tur:2"), None) + "\n").unwrap();
        assert_eq!(documents.refreshed().unwrap(), 1, "the file moved on");
        assert_eq!(documents.get(&doc).unwrap().refs, ["girsa:tur:2"]);
    }

    #[test]
    fn a_document_on_a_stick_that_is_not_plugged_in_is_not_forgotten() {
        // Quietly dropping the row would be this project's own definition of a
        // silent gap. It is reported as missing and still answered from.
        let dir = scratch("gone");
        let personal = dir.join("personal");
        std::fs::create_dir_all(&personal).unwrap();
        let doc = ksav(&dir, "ב", &["girsa:tur:1"]);

        let (mut documents, _) = Documents::open(&personal);
        documents.remember(&doc, None).unwrap();
        documents.refreshed().unwrap();
        std::fs::remove_file(&doc).unwrap();

        let row = documents.get(&doc).expect("still registered");
        assert!(!row.is_here(), "it says it is not here");
        assert!(!row.is_stale(), "and nothing to re-read is not stale");
        assert_eq!(row.refs, ["girsa:tur:1"], "and it still answers");
        assert_eq!(documents.refreshed().unwrap(), 0);
    }

    #[test]
    fn remembering_the_same_path_twice_is_one_document() {
        // `buffer_to_ksav` saves locally *and* posts, so a document arrives
        // twice. Two rows for one file would answer *where did I use this*
        // twice over.
        let dir = scratch("twice");
        let personal = dir.join("personal");
        std::fs::create_dir_all(&personal).unwrap();
        let doc = ksav(&dir, "ג", &["girsa:tur:1"]);

        let (mut documents, _) = Documents::open(&personal);
        documents.remember(&doc, Some("ראשון")).unwrap();
        documents.refreshed().unwrap();
        documents.remember(&doc, Some("שני")).unwrap();
        assert_eq!(documents.count(), 1);
        let row = documents.get(&doc).unwrap();
        assert_eq!(row.name, "שני", "the new name");
        assert_eq!(row.refs, ["girsa:tur:1"], "and the refs it already had");
    }

    #[test]
    fn forgetting_a_document_leaves_the_file_alone() {
        let dir = scratch("forget");
        let personal = dir.join("personal");
        std::fs::create_dir_all(&personal).unwrap();
        let doc = ksav(&dir, "ד", &["girsa:tur:1"]);

        let (mut documents, _) = Documents::open(&personal);
        documents.remember(&doc, None).unwrap();
        assert!(documents.forget(&doc).unwrap());
        assert_eq!(documents.count(), 0);
        assert!(doc.is_file(), "the file was deleted");
        assert!(
            !documents.forget(&doc).unwrap(),
            "and forgetting twice is not an error"
        );

        let (reopened, _) = Documents::open(&personal);
        assert_eq!(reopened.count(), 0, "the tombstone did not survive");
    }
}

/// The replay, the index and the compaction — `girsa_personal::Store`.
///
/// This crate's error type for these operations is `LogError` itself rather
/// than a wrapper, so there is no `io_from_log_error!` here: the store is thin
/// enough that the log's own failure is the only one it can have.
impl girsa_personal::Store for Documents {
    type Record = Document;
    const WHAT: &'static str = "a document";

    fn key_of(d: &Document) -> String {
        d.path.clone()
    }
    fn log(&self) -> &Log {
        &self.log
    }
    fn hold(&mut self, d: Document) {
        self.by_path.insert(d.path.clone(), d);
    }
    fn count(&self) -> usize {
        self.by_path.len()
    }
    fn records(&self) -> Vec<&Document> {
        self.by_path.values().collect()
    }
}
