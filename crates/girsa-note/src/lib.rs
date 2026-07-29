//! Your layer: what you wrote, what you marked, what you asked, and what you
//! keep together.
//!
//! spec.md §11, BUILDER.md W27. Five things live here — notes, highlights and
//! bookmarks, tags, saved queries, and chaburah folders — and one claim holds
//! them together:
//!
//! > **Your notes are nodes.** A note has the same typed edges as anything
//! > else, so *"what have I already written that touches this sugya?"* is the
//! > same query as *"who quotes this Rishon?"*
//!
//! # What that costs, and why it is worth it
//!
//! The cheap way to build notes is a table of `(segment id, text)` and a panel
//! that reads it. It works, it is a day's work, and it produces a system where
//! your own writing is the one kind of material in the library that cannot be
//! linked to, cited, searched beside a Rishon, or asked about from the other
//! end.
//!
//! So a note here is not a row beside the graph. It is:
//!
//! - **a sefer on your shelf** — a [`Work`](girsa_corpus::work::Work) with
//!   [`Source::Mine`](girsa_corpus::work::Source::Mine), whose paragraphs are
//!   segments with permanent ids (spec.md §3), catalogued in
//!   `personal/works/index.jsonl` like a file you dropped on the window. So it
//!   opens in a pane, it is indexed by W11, and it is citable by W15;
//! - **joined to the corpus by [`girsa_link::Edge`]** — the same directed,
//!   typed edge as everything in `corpus/links/`. So W23 can retype or reject
//!   it, W24's lenses filter it, and [`girsa_app::touching`] returns it in the
//!   same list as Rashi without knowing that a note is a different kind of
//!   thing. It is not, and that is the point.
//!
//! # The file is the truth; the graph is derived from it
//!
//! spec.md §4.1's rule, applied to your own material: a note is one plain text
//! file, and *"exportable as plain files"* is not a feature bolted on the side
//! — it is where the note lives. Delete `personal/links.jsonl` and every note
//! is still anchored where you put it, because the anchors are in the note.
//!
//! Which means each paragraph must **carry its own id in the file**, the way a
//! segments file does (`girsa_corpus::import`), and for the same reason: a
//! paragraph whose id were its position would move every anchor below it the
//! first time you inserted a line. That is T1, in your own writing, where it
//! would cost the thing the system exists to accumulate.
//!
//! ```text
//! girsa note
//! title: מאימתי קורין את שמע
//! on: girsa:bavli/berakhot/2a:1#1
//!
//! girsa:note/מאימתי-קורין-את-שמע/2#2
//! הא דתנן מאימתי קורין…
//! ```
//!
//! # What is deliberately not here
//!
//! **Sync.** spec.md §11 offers *optional, off by default, encrypted sync of
//! the personal layer only*. Every part of that is a runtime network
//! dependency, which BUILDER.md §0.1 says is not a decision a work order takes
//! on its own. What is built instead is the half that needs no ruling and that
//! §11 names first: everything is a plain file, and [`export`] writes them
//! somewhere you can copy.

pub mod collection;
pub mod mark;
pub mod note;
pub mod query;

use std::collections::BTreeMap;
use std::path::Path;

pub use collection::{Collection, Collections, Member};
pub use mark::{Kind, Mark, MarkId, Marks};
pub use note::{Note, NoteError, Notes, Para};
pub use query::{Queries, SavedQuery};

/// Seconds since the epoch. The one clock this crate reads.
#[must_use]
pub(crate) fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// A tag, as it is stored and compared.
///
/// Tags are Hebrew and Hebrew has more than one spelling of the same word —
/// `שו"ע` and `שו״ע` are one tag and would be two buckets under a `==`. So a
/// tag is **kept as you typed it** and **compared normalized**, which is W2's
/// rule everywhere else in this codebase and is not re-litigated here.
#[must_use]
pub fn same_tag(a: &str, b: &str) -> bool {
    girsa_hebrew::normalize(a.trim()) == girsa_hebrew::normalize(b.trim())
}

/// Everything you have tagged, tag by tag, with how many things carry it.
///
/// One tag per row, spelled the way it was first written. The count is what a
/// tag list is for: a tag on one thing is a typo and a tag on forty is a
/// subject.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags {
    rows: BTreeMap<String, Tally>,
}

/// How many of each kind of thing carry one tag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub notes: usize,
    pub marks: usize,
    pub queries: usize,
    pub collections: usize,
}

impl Tally {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.notes + self.marks + self.queries + self.collections
    }
}

impl Tags {
    /// Count the tags across the whole of your layer.
    #[must_use]
    pub fn of(notes: &Notes, marks: &Marks, queries: &Queries, collections: &Collections) -> Self {
        let mut tags = Self::default();
        for note in notes.all() {
            for tag in &note.tags {
                tags.row(tag).notes += 1;
            }
        }
        for mark in marks.all() {
            for tag in &mark.tags {
                tags.row(tag).marks += 1;
            }
        }
        for query in queries.all() {
            for tag in &query.tags {
                tags.row(tag).queries += 1;
            }
        }
        for collection in collections.all() {
            for tag in &collection.tags {
                tags.row(tag).collections += 1;
            }
        }
        tags
    }

    /// The tally for one tag, under whichever spelling of it came first.
    fn row(&mut self, tag: &str) -> &mut Tally {
        let key = self
            .rows
            .keys()
            .find(|kept| same_tag(kept, tag))
            .cloned()
            .unwrap_or_else(|| tag.trim().to_string());
        self.rows.entry(key).or_default()
    }

    /// Every tag, alphabetically, with its tally.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Tally)> {
        self.rows.iter().map(|(tag, tally)| (tag.as_str(), tally))
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.rows.len()
    }

    /// How many things carry one tag, however it is spelled.
    #[must_use]
    pub fn tally(&self, tag: &str) -> Tally {
        self.rows
            .iter()
            .find(|(kept, _)| same_tag(kept, tag))
            .map_or_else(Tally::default, |(_, tally)| *tally)
    }
}

/// What an export wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exported {
    pub notes: usize,
    pub marks: usize,
    pub queries: usize,
    pub collections: usize,
}

/// Why an export stopped.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Write the whole of your layer into a directory, as plain files.
///
/// spec.md §11: *everything local, everything exportable as plain files, no
/// account.* There is nothing to convert — a note is already a text file and
/// the three lists are already JSONL — so this is a copy, and that is the
/// evidence rather than the shortcut: a format that needs an exporter is a
/// format you do not have.
///
/// # Errors
///
/// If the destination cannot be written.
pub fn export(
    notes: &Notes,
    marks: &Marks,
    queries: &Queries,
    collections: &Collections,
    into: &Path,
) -> Result<Exported, ExportError> {
    let io = |path: &Path| {
        let path = path.display().to_string();
        move |source| ExportError::Io {
            path: path.clone(),
            source,
        }
    };
    std::fs::create_dir_all(into).map_err(io(into))?;

    let notes_dir = into.join("notes");
    std::fs::create_dir_all(&notes_dir).map_err(io(&notes_dir))?;
    let mut written = Exported::default();
    for note in notes.all() {
        let path = notes_dir.join(note::file_name(&note.slug));
        std::fs::write(&path, note.to_text()).map_err(io(&path))?;
        written.notes += 1;
    }

    let marks_path = into.join("marks.jsonl");
    std::fs::write(&marks_path, marks.to_text()).map_err(io(&marks_path))?;
    written.marks = marks.count();

    let queries_path = into.join("queries.jsonl");
    std::fs::write(&queries_path, queries.to_text()).map_err(io(&queries_path))?;
    written.queries = queries.count();

    let collections_path = into.join("collections.jsonl");
    std::fs::write(&collections_path, collections.to_text()).map_err(io(&collections_path))?;
    written.collections = collections.count();

    Ok(written)
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn two_spellings_of_one_word_are_one_tag() {
        // The gershayim variants are the ones that look done and are not (W2).
        assert!(same_tag("שו\"ע", "שו״ע"));
        assert!(same_tag(" ברכות ", "ברכות"));
        assert!(!same_tag("ברכות", "שבת"));
    }
}
