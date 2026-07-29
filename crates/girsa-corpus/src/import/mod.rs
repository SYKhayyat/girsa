//! Reading a work into segments that carry their own permanent names.
//!
//! spec.md §4.1: **text files on disk are the truth, the database is a
//! rebuildable cache.** So an import writes files, and the files have to be
//! greppable, diffable and readable without the app.
//!
//! # The one thing the on-disk form may not do
//!
//! It may not imply a segment's id from its **position in the file**. That is
//! T1 — the defect the whole corpus is being rebuilt to escape — reintroduced
//! at the last possible moment: insert a line into a segments file whose ids
//! are line numbers and every anchor below it silently names different words.
//!
//! So each record carries its id as a field:
//!
//! ```jsonl
//! {"id":"girsa:mishnah-berurah/1:1#30","kind":"text","text":"…"}
//! {"id":"girsa:mishnah-berurah/1:2#31","kind":"text","text":"…"}
//! ```
//!
//! One line per segment, so `grep` still works and a diff of a corrected sefer
//! is readable; but the name is *in* the line, so sorting, re-ordering,
//! inserting and deleting are all safe, and a split writes `#30.1` and `#30.2`
//! wherever it likes.

pub mod mine;
pub mod otzaria;
pub mod sefaria;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::segment::SegmentId;
use crate::store::SegmentStore;
use crate::work::{Source, Work};

/// What a segment is, as far as the reader is concerned.
///
/// The distinction is load-bearing twice over: spec.md §2.1 counts Mishnah
/// Berurah as *"18,120 lines with 701 headings"*, which is only checkable if
/// the two are told apart; and a search hit inside a heading is a different
/// kind of result from one inside the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SegmentKind {
    Text,
    /// A structural heading — Otzaria's `<h1>/<h2>/<h3>`, or a Sefaria schema
    /// node that names a section rather than holding one.
    Heading,
    /// A page of a scan you brought (spec.md §6.3). It is addressable and
    /// citable now and it has **no words** until it is OCR'd — which is a
    /// different thing from a segment that is empty, and search has to be able
    /// to tell them apart so that a PDF is *"not searchable yet"* rather than
    /// silently absent (§9.7).
    Page,
    /// A footnote, lifted out of the sentence that carried it (W29).
    ///
    /// Its own segment on purpose. A note spliced into the words around it
    /// corrupts them — the sentence reads as though the author wrote the note
    /// into it — and a note that is its own segment is searchable, citable and
    /// correctable like any other line.
    Note,
    /// One item of a list.
    Item,
    /// One row of a table, its cells kept apart.
    Row,
    /// A block quote — text this sefer is quoting rather than saying.
    Quote,
}

impl SegmentKind {
    /// What the window calls it. One implementation, because two — the shell's
    /// and the fixture writer's — is how a page ends up rendering a heading as
    /// body text in a browser and not in the app.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Heading => "heading",
            Self::Page => "page",
            Self::Note => "note",
            Self::Item => "item",
            Self::Row => "row",
            Self::Quote => "quote",
        }
    }

    /// Every kind, in the order they are written above.
    #[must_use]
    pub const fn all() -> [Self; 7] {
        [
            Self::Text,
            Self::Heading,
            Self::Page,
            Self::Note,
            Self::Item,
            Self::Row,
            Self::Quote,
        ]
    }

    /// Whether this segment is the sefer's own running text.
    ///
    /// False for a heading, which names a section rather than holding one, and
    /// for a page with no words in it yet. **True for a note, an item, a row
    /// and a quote**: those are words somebody wrote and are read, searched and
    /// corrected like any other line — they are only drawn differently.
    #[must_use]
    pub const fn has_words(self) -> bool {
        !matches!(self, Self::Page)
    }

    /// Read back what [`SegmentKind::as_str`] wrote.
    ///
    /// The search index stores the kind as this word — a result row says which
    /// it is, and W14's facets count by it. Kept beside `as_str` so the pair
    /// cannot drift; anything else parsing these words by hand would be a
    /// second implementation of the same fact.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::all().into_iter().find(|kind| kind.as_str() == word)
    }
}

/// A segment as it goes to disk: its permanent name, and its words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub id: SegmentId,
    pub kind: SegmentKind,
    pub text: String,
}

/// A segment before it has been given a name.
///
/// What the two source parsers produce. Ordinals are assigned in exactly one
/// place — [`ImportedWork::assemble`] — so that "assigned once, in reading
/// order, never recomputed" is a property of the code and not of a convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSegment {
    pub path: Vec<String>,
    pub kind: SegmentKind,
    pub text: String,
}

/// One work, read.
#[derive(Debug, Clone)]
pub struct ImportedWork {
    pub work: Work,
    pub segments: Vec<Segment>,
}

/// What an import found, for asserting against spec.md §2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub works: usize,
    pub segments: usize,
    pub headings: usize,
    /// Works that produced no segments at all. Loud rather than silent: an
    /// empty sefer on the shelf is a defect, not a small book.
    pub empty_works: usize,
    /// Segments whose id would not survive being written down and read back.
    /// Must be zero — see [`SegmentId::is_well_formed`].
    pub malformed_ids: usize,
}

impl ImportedWork {
    /// Give every segment its permanent name, in reading order, once.
    #[must_use]
    pub fn assemble(work: Work, raw: Vec<RawSegment>) -> Self {
        let kinds: Vec<SegmentKind> = raw.iter().map(|r| r.kind).collect();
        let store = SegmentStore::import(
            work.slug.clone(),
            raw.into_iter().map(|r| (r.path, r.text)).collect(),
        );

        // `SegmentStore` is ordered by ordinal, and the ordinals were handed
        // out in the order the segments were given — so this zip is reading
        // order against reading order. Nothing else here may assume that.
        let segments = store
            .iter()
            .zip(kinds)
            .map(|((id, text), kind)| Segment {
                id: id.clone(),
                kind,
                text: text.to_string(),
            })
            .collect();

        Self { work, segments }
    }

    #[must_use]
    pub fn counts(&self) -> Counts {
        Counts {
            works: 1,
            segments: self.segments.len(),
            headings: self
                .segments
                .iter()
                .filter(|s| s.kind == SegmentKind::Heading)
                .count(),
            empty_works: usize::from(self.segments.is_empty()),
            malformed_ids: self
                .segments
                .iter()
                .filter(|s| !s.id.is_well_formed())
                .count(),
        }
    }
}

impl Counts {
    pub fn absorb(&mut self, other: Counts) {
        self.works += other.works;
        self.segments += other.segments;
        self.headings += other.headings;
        self.empty_works += other.empty_works;
        self.malformed_ids += other.malformed_ids;
    }
}

/// Read one work from whichever corpus supplies it.
///
/// # Errors
///
/// If the source file cannot be read or does not hold what its schema says.
pub fn read(work: &Work) -> Result<ImportedWork, ImportError> {
    let mut work = work.clone();
    let raw = match work.source {
        Source::Sefaria => {
            let (raw, version) = sefaria::read(&work)?;
            // Which printed edition this is, read out of the text file rather
            // than guessed — spec.md §13. Otzaria's is known without opening
            // anything, so the catalogue already set it.
            work.version = version;
            raw
        }
        Source::Otzaria => otzaria::read(&work)?,
        // Yours, re-read from the file itself — the same rule as everything
        // else here (spec.md §4.1: the file on disk is the truth).
        Source::Mine => mine::read(&work)?,
    };
    Ok(ImportedWork::assemble(work, raw))
}

/// Why a work could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Malformed { path: String, message: String },
}

impl ImportError {
    /// Public because the link importer reads the same files this one wrote,
    /// and reports its failures in the same shape.
    pub fn io(path: &Path) -> impl Fn(std::io::Error) -> Self + '_ {
        move |source| Self::Io {
            path: path.display().to_string(),
            source,
        }
    }

    pub fn malformed(path: &Path, message: impl Into<String>) -> Self {
        Self::Malformed {
            path: path.display().to_string(),
            message: message.into(),
        }
    }
}

/// Where a work's files live under the corpus root.
///
/// A slug carries `/` for a volume — `shulchan-arukh/orach-chayim` — and that
/// becomes a directory, so the shelf on disk reads the way the shelf in the app
/// does. Hebrew slugs are directory names too; every filesystem the three
/// target platforms use has been UTF-8-safe for a decade.
#[must_use]
pub fn work_dir(root: &Path, slug: &str) -> PathBuf {
    slug_dir(&root.join("works"), slug)
}

/// A slug as a directory under any base.
///
/// `girsa-link` puts a work's edges at `corpus/links/<slug>/`, mirroring
/// `corpus/works/<slug>/`, and the sanitizing a slug needs to survive being a
/// Windows path has to be identical in both — a second copy of it would drift
/// and the two halves of a sefer would stop lining up.
#[must_use]
pub fn slug_dir(base: &Path, slug: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for part in slug.split('/') {
        path.push(sanitize_component(part));
    }
    path
}

/// Make a path component Windows will accept, reversibly enough to debug.
///
/// The same problem `fetch` has, from the other end: a Hebrew title is fine,
/// but a work whose title carries `?` or `"` — Sefaria has three — cannot be a
/// directory on Windows at any path.
fn sanitize_component(part: &str) -> String {
    const FORBIDDEN: [char; 9] = ['<', '>', ':', '"', '\\', '|', '?', '*', '/'];
    let mut out = String::with_capacity(part.len());
    for c in part.chars() {
        if FORBIDDEN.contains(&c) || (c as u32) < 0x20 {
            out.push_str(&format!("%{:02X}", c as u32));
        } else {
            out.push(c);
        }
    }
    while out.ends_with(' ') || out.ends_with('.') {
        let last = out.pop().unwrap_or('.');
        out.push_str(&format!("%{:02X}", last as u32));
    }
    if out.is_empty() {
        out.push_str("%00");
    }
    out
}

/// Write a work: its metadata, and its segments one per line.
///
/// # Errors
///
/// If the directory cannot be created or either file cannot be written.
pub fn write(root: &Path, imported: &ImportedWork) -> Result<(), ImportError> {
    let dir = work_dir(root, &imported.work.slug);
    fs::create_dir_all(&dir).map_err(ImportError::io(&dir))?;

    let meta_path = dir.join("work.json");
    let meta = serde_json::to_vec_pretty(&imported.work)
        .map_err(|e| ImportError::malformed(&meta_path, e.to_string()))?;
    fs::write(&meta_path, meta).map_err(ImportError::io(&meta_path))?;

    let segments_path = dir.join("segments.jsonl");
    let mut body = String::new();
    for segment in &imported.segments {
        let line = serde_json::to_string(segment)
            .map_err(|e| ImportError::malformed(&segments_path, e.to_string()))?;
        body.push_str(&line);
        body.push('\n');
    }
    fs::write(&segments_path, body).map_err(ImportError::io(&segments_path))?;
    Ok(())
}

/// Put one work in your own catalogue — `personal/works/index.jsonl`.
///
/// The whole file is rewritten rather than appended to. W8 shipped an importer
/// that opened its shards in append mode and doubled the graph on a second run;
/// the same mistake here would put a sefer on the shelf twice.
///
/// Two callers: a file you dropped on the window ([`mine::add`]) and a note you
/// wrote (`girsa-note`, W27). One implementation, because the second copy of
/// this is the one that appends.
///
/// # Errors
///
/// If the personal layer cannot be written.
pub fn catalogue(personal: &Path, work: &Work) -> Result<(), ImportError> {
    let path = personal.join("works/index.jsonl");
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(ImportError::io(dir))?;
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut body = String::new();
    for line in existing.lines().filter(|l| !l.trim().is_empty()) {
        let same = serde_json::from_str::<Work>(line).is_ok_and(|w| w.slug == work.slug);
        if !same {
            body.push_str(line);
            body.push('\n');
        }
    }
    let line =
        serde_json::to_string(work).map_err(|e| ImportError::malformed(&path, e.to_string()))?;
    body.push_str(&line);
    body.push('\n');
    fs::write(&path, body).map_err(ImportError::io(&path))
}

/// Take one back out of your catalogue. `false` if it was not in it.
///
/// # Errors
///
/// If the personal layer cannot be written.
pub fn uncatalogue(personal: &Path, slug: &str) -> Result<bool, ImportError> {
    let path = personal.join("works/index.jsonl");
    let Ok(existing) = fs::read_to_string(&path) else {
        return Ok(false);
    };
    let mut body = String::new();
    let mut gone = false;
    for line in existing.lines().filter(|l| !l.trim().is_empty()) {
        if serde_json::from_str::<Work>(line).is_ok_and(|w| w.slug == slug) {
            gone = true;
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    if gone {
        fs::write(&path, body).map_err(ImportError::io(&path))?;
    }
    Ok(gone)
}

/// Read back what [`write`] wrote.
///
/// # Errors
///
/// If the files are missing or a record does not parse. A record that does not
/// parse fails the read rather than being skipped: a segments file silently
/// one segment short is the failure this whole design is arranged against.
pub fn read_back(root: &Path, slug: &str) -> Result<ImportedWork, ImportError> {
    let dir = work_dir(root, slug);
    let meta_path = dir.join("work.json");
    let meta = fs::read_to_string(&meta_path).map_err(ImportError::io(&meta_path))?;
    let work: Work = serde_json::from_str(&meta)
        .map_err(|e| ImportError::malformed(&meta_path, e.to_string()))?;

    let segments_path = dir.join("segments.jsonl");
    let body = fs::read_to_string(&segments_path).map_err(ImportError::io(&segments_path))?;
    let mut segments = Vec::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let segment: Segment = serde_json::from_str(line)
            .map_err(|e| ImportError::malformed(&segments_path, e.to_string()))?;
        segments.push(segment);
    }
    Ok(ImportedWork { work, segments })
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::work::Source;

    fn work() -> Work {
        Work {
            slug: "shulchan-arukh/orach-chayim".into(),
            he_title: "שולחן ערוך, אורח חיים".into(),
            en_title: "Shulchan Arukh, Orach Chayim".into(),
            categories: vec!["Halakhah".into()],
            source: Source::Sefaria,
            origin: PathBuf::from("merged.json"),
            schema: None,
            author: None,
            era: None,
            comp_date: None,
            version: None,
            he_sections: Vec::new(),
            commentary_on: Vec::new(),
        }
    }

    fn raw(path: &[&str], text: &str) -> RawSegment {
        RawSegment {
            path: path.iter().map(|p| (*p).to_string()).collect(),
            kind: SegmentKind::Text,
            text: text.to_string(),
        }
    }

    #[test]
    fn ordinals_are_handed_out_in_reading_order_starting_at_one() {
        let imported = ImportedWork::assemble(
            work(),
            vec![
                raw(&["1", "1"], "יתגבר כארי"),
                raw(&["1", "2"], "ולא יתבייש"),
                raw(&["2", "1"], "המשכים"),
            ],
        );
        let ids: Vec<String> = imported.segments.iter().map(|s| s.id.to_string()).collect();
        assert_eq!(
            ids,
            [
                "girsa:shulchan-arukh/orach-chayim/1:1#1",
                "girsa:shulchan-arukh/orach-chayim/1:2#2",
                "girsa:shulchan-arukh/orach-chayim/2:1#3",
            ]
        );
    }

    #[test]
    fn a_segments_file_names_each_segment_rather_than_counting_lines() {
        // The property that keeps T1 out of the storage format. Shuffle the
        // file and every id still names the same words, because the id is in
        // the line rather than being the line's position.
        let dir = std::env::temp_dir().join("girsa-import-test-idnames");
        let _ = fs::remove_dir_all(&dir);
        let imported = ImportedWork::assemble(
            work(),
            vec![
                raw(&["1", "1"], "first"),
                raw(&["1", "2"], "second"),
                raw(&["1", "3"], "third"),
            ],
        );
        write(&dir, &imported).expect("writes");

        let path = work_dir(&dir, "shulchan-arukh/orach-chayim").join("segments.jsonl");
        let body = fs::read_to_string(&path).expect("reads");
        let mut lines: Vec<&str> = body.lines().collect();
        lines.reverse();
        fs::write(&path, lines.join("\n")).expect("rewrites");

        let back = read_back(&dir, "shulchan-arukh/orach-chayim").expect("reads back");
        let found = back
            .segments
            .iter()
            .find(|s| s.id.to_string().ends_with("/1:2#2"))
            .map(|s| s.text.clone());
        assert_eq!(found.as_deref(), Some("second"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn counts_tell_headings_from_text() {
        let imported = ImportedWork::assemble(
            work(),
            vec![
                RawSegment {
                    path: vec!["1".into()],
                    kind: SegmentKind::Heading,
                    text: "סימן א".into(),
                },
                raw(&["1", "1"], "יתגבר כארי"),
            ],
        );
        let counts = imported.counts();
        assert_eq!(counts.segments, 2);
        assert_eq!(counts.headings, 1);
        assert_eq!(counts.malformed_ids, 0);
    }
}
