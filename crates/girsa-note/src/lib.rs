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
// What your own layer holds that the index has not seen yet (B7). Here rather
// than in `girsa-app` because two callers on opposite sides of a deliberate
// dependency boundary must not disagree about the count; see the module note.
pub mod since;

use std::collections::BTreeMap;
use std::path::Path;

// `to_text` is a default method on the store trait now — one jsonl writer for
// the three stores that had it character for character.
use girsa_personal::Store;

pub use collection::{Collection, Collections, Member};
pub use mark::{Kind, Mark, MarkId, Marks};
pub use note::{Note, NoteError, Notes, Para};
pub use query::{Queries, SavedQuery};

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

/// A kind of thing in your layer that can carry a tag.
///
/// # Why this is a list and not four function arguments
///
/// It was four. `Tally` had four named fields, `Tags::of` took four positional
/// references, [`export`] took five, `girsa_app::Shelf` exposed four accessors
/// and the window passed four of them twice — **six signatures for one noun**,
/// so adding a fifth taggable thing (a scan, a link repair; both are already
/// nouns under `personal/`) meant six edits before a line of it counted
/// anything.
///
/// One list, and the compiler holds the rest: a new variant is a `match` arm
/// [`Layer::tags_on`] and [`Layer::count`] will not compile without, and every
/// tally, total and export row follows from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Taggable {
    Note,
    Mark,
    Query,
    Collection,
}

girsa_corpus::spelled!(Taggable {
    Note => "note",
    Mark => "mark",
    Query => "query",
    Collection => "collection",
});

impl Taggable {
    /// Every kind, in declared order.
    pub const ALL: &'static [Self] = &[Self::Note, Self::Mark, Self::Query, Self::Collection];

    /// How many kinds there are — the width of a [`Tally`].
    pub const HOW_MANY: usize = Self::ALL.len();

    /// What this kind is called, in Hebrew, in the plural.
    ///
    /// Here rather than in the window, which held four of them in a
    /// `.filter(Boolean).join(" · ")` — so a fifth taggable noun was a
    /// TypeScript edit to a file that has never been told what a mark is.
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Note => "הערות",
            Self::Mark => "סימונים",
            Self::Query => "שאילתות",
            Self::Collection => "תיקיות",
        }
    }

    /// Where this kind sits in a [`Tally`].
    #[must_use]
    const fn at(self) -> usize {
        match self {
            Self::Note => 0,
            Self::Mark => 1,
            Self::Query => 2,
            Self::Collection => 3,
        }
    }
}

/// The four stores of your own layer, borrowed together.
///
/// Every function that is about *your layer* rather than about one store takes
/// this. It is what made [`Taggable`] worth having: a fifth store is a field
/// here and a variant there, and nothing downstream grows an argument.
#[derive(Debug, Clone, Copy)]
pub struct Layer<'a> {
    pub notes: &'a Notes,
    pub marks: &'a Marks,
    pub queries: &'a Queries,
    pub collections: &'a Collections,
}

impl Layer<'_> {
    /// Every tag carried by everything of one kind, one thing at a time.
    ///
    /// One of the two `match`es a new taggable noun has to be added to, and the
    /// reason a new one cannot be forgotten.
    pub fn tags_on(&self, kind: Taggable) -> Box<dyn Iterator<Item = &String> + '_> {
        match kind {
            Taggable::Note => Box::new(self.notes.all().flat_map(|n| n.tags.iter())),
            Taggable::Mark => Box::new(self.marks.all().flat_map(|m| m.tags.iter())),
            Taggable::Query => Box::new(self.queries.all().flat_map(|q| q.tags.iter())),
            Taggable::Collection => Box::new(self.collections.all().flat_map(|c| c.tags.iter())),
        }
    }

    /// How many things of one kind there are.
    #[must_use]
    pub fn count(&self, kind: Taggable) -> usize {
        match kind {
            Taggable::Note => self.notes.all().count(),
            Taggable::Mark => self.marks.count(),
            Taggable::Query => self.queries.count(),
            Taggable::Collection => self.collections.count(),
        }
    }

    /// How many things your layer holds altogether.
    #[must_use]
    pub fn how_much(&self) -> usize {
        Taggable::ALL.iter().map(|kind| self.count(*kind)).sum()
    }
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
///
/// Keyed by [`Taggable`] rather than four named fields, for the reason in that
/// type's note.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    counts: [usize; Taggable::HOW_MANY],
}

impl Tally {
    /// How many things of one kind carry this tag.
    #[must_use]
    pub const fn of(&self, kind: Taggable) -> usize {
        self.counts[kind.at()]
    }

    /// Every kind and its count, in declared order.
    pub fn iter(&self) -> impl Iterator<Item = (Taggable, usize)> + '_ {
        Taggable::ALL.iter().map(|kind| (*kind, self.of(*kind)))
    }

    #[must_use]
    pub const fn total(&self) -> usize {
        let mut sum = 0;
        let mut at = 0;
        while at < Taggable::HOW_MANY {
            sum += self.counts[at];
            at += 1;
        }
        sum
    }
}

impl Tags {
    /// Count the tags across the whole of your layer.
    #[must_use]
    pub fn of(layer: &Layer<'_>) -> Self {
        let mut tags = Self::default();
        for kind in Taggable::ALL {
            // Collected first: `row` borrows `tags` mutably while the iterator
            // borrows the layer, which is two borrows the compiler is right to
            // refuse and one `Vec` of references to satisfy.
            let carried: Vec<&String> = layer.tags_on(*kind).collect();
            for tag in carried {
                tags.row(tag).counts[kind.at()] += 1;
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

/// What an export wrote, by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Exported {
    written: [usize; Taggable::HOW_MANY],
}

impl Exported {
    /// How many of one kind were written.
    #[must_use]
    pub const fn of(&self, kind: Taggable) -> usize {
        self.written[kind.at()]
    }

    /// Every kind and how many, in declared order.
    pub fn iter(&self) -> impl Iterator<Item = (Taggable, usize)> + '_ {
        Taggable::ALL.iter().map(|kind| (*kind, self.of(*kind)))
    }

    /// How many things were written altogether.
    #[must_use]
    pub const fn total(&self) -> usize {
        let mut sum = 0;
        let mut at = 0;
        while at < Taggable::HOW_MANY {
            sum += self.written[at];
            at += 1;
        }
        sum
    }
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
pub fn export(layer: &Layer<'_>, into: &Path) -> Result<Exported, ExportError> {
    let io = |path: &Path| {
        let path = path.display().to_string();
        move |source| ExportError::Io {
            path: path.clone(),
            source,
        }
    };
    std::fs::create_dir_all(into).map_err(io(into))?;

    // A note is a file each, because a note *is* a file each — which is the
    // claim §11 makes, and this is where it is either true or a conversion.
    let notes_dir = into.join(where_it_goes(Taggable::Note));
    std::fs::create_dir_all(&notes_dir).map_err(io(&notes_dir))?;
    let mut written = Exported::default();
    for note in layer.notes.all() {
        let path = notes_dir.join(note::file_name(&note.slug));
        std::fs::write(&path, note.to_text()).map_err(io(&path))?;
        written.written[Taggable::Note.at()] += 1;
    }

    for (kind, text) in [
        (Taggable::Mark, layer.marks.to_text()),
        (Taggable::Query, layer.queries.to_text()),
        (Taggable::Collection, layer.collections.to_text()),
    ] {
        let path = into.join(where_it_goes(kind));
        std::fs::write(&path, text).map_err(io(&path))?;
        written.written[kind.at()] = layer.count(kind);
    }

    Ok(written)
}

/// What one store is exported as. `Taggable::Note` is a directory of files.
///
/// Spelled out rather than made up from the variant: `marks.jsonl` and
/// `queries.jsonl` are the store's own file names, and `querys.jsonl` is what a
/// generated plural would have written.
#[must_use]
const fn where_it_goes(kind: Taggable) -> &'static str {
    match kind {
        Taggable::Note => "notes",
        Taggable::Mark => "marks.jsonl",
        Taggable::Query => "queries.jsonl",
        Taggable::Collection => "collections.jsonl",
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_kinds_are_a_list_and_every_one_of_them_has_a_slot() {
        // `at()` is hand-written because a `const fn` cannot be derived, and a
        // hand-written index into an array is exactly the thing that goes wrong
        // quietly: two kinds sharing a slot would add their counts together and
        // read as a popular tag.
        let mut seen = std::collections::BTreeSet::new();
        for kind in Taggable::ALL {
            assert!(kind.at() < Taggable::HOW_MANY, "{kind:?} is off the end");
            assert!(seen.insert(kind.at()), "{kind:?} shares a slot");
        }
        assert_eq!(seen.len(), Taggable::HOW_MANY);
        assert_eq!(Taggable::SPELLINGS.len(), Taggable::HOW_MANY);
    }

    #[test]
    fn a_tally_totals_every_kind_and_not_the_four_it_was_written_with() {
        let mut tally = Tally::default();
        for kind in Taggable::ALL {
            tally.counts[kind.at()] = 1;
        }
        assert_eq!(tally.total(), Taggable::HOW_MANY);
        assert_eq!(tally.iter().count(), Taggable::HOW_MANY);
        assert!(tally.iter().all(|(_, n)| n == 1));
    }

    #[test]
    fn two_spellings_of_one_word_are_one_tag() {
        // The gershayim variants are the ones that look done and are not (W2).
        assert!(same_tag("שו\"ע", "שו״ע"));
        assert!(same_tag(" ברכות ", "ברכות"));
        assert!(!same_tag("ברכות", "שבת"));
    }
}
