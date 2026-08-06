//! Your own layer, on the line you are standing on.
//!
//! spec.md §11, BUILDER.md W27. The link panel already answers *what does the
//! library say about this line* — and after `girsa-note` it answers *what have
//! **I** said about it* in the same list, because a note's edge is an edge
//! (see [`crate::links::touching`]). What is left for this module is the two
//! things that are **not** edges:
//!
//! - **marks** — a highlight is a range of characters in the line as it is
//!   drawn now, so it has to be placed against the text the pane is holding,
//!   corrections and nikud and all;
//! - **folders** — a chaburah is a list you put this line in, which is a fact
//!   about the list rather than a claim about the text.
//!
//! And the one thing W20 measured and this order inherits: writing a note has
//! to be a **three-second interaction from where you are reading**, or it does
//! not happen. [`note_here`] is that: a place, some words, done — no dialog
//! asking which notebook, no anchor to choose, because you are standing on it.

use girsa_corpus::segment::SegmentId;
use girsa_corpus::standing::Standing;
use girsa_note::mark::Placed;
use girsa_note::{Collection, Mark, Note, NoteError};

use crate::shelf::{Shelf, ShelfError};

/// One of your notes, as a row beside the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wrote {
    pub slug: String,
    pub title: String,
    /// The first words of it, so the row says something.
    pub opening: String,
    pub tags: Vec<String>,
    pub when: u64,
    pub edited: u64,
    /// How many paragraphs — which is how many places in it can be cited.
    pub paragraphs: usize,
}

impl Wrote {
    fn of(note: &Note) -> Self {
        let opening = note
            .paras()
            .iter()
            .map(|p| p.text.as_str())
            .find(|text| !text.trim().is_empty())
            .unwrap_or_default();
        Self {
            slug: note.slug.clone(),
            title: note.title.clone(),
            opening: first_words(opening, 12),
            tags: note.tags.clone(),
            when: note.when,
            edited: note.edited,
            paragraphs: note.paras().len(),
        }
    }
}

/// One mark, and where it lands in the text the pane drew.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marked {
    pub mark: Mark,
    /// Where it is now — or that its words have gone. Never quietly dropped:
    /// a highlight that vanished is a reader wondering what they marked.
    pub placed: Placed,
}

/// What you have on one line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Yours {
    pub notes: Vec<Wrote>,
    pub marks: Vec<Marked>,
    /// The chaburah folders this line is in, by name.
    pub folders: Vec<String>,
}

impl Yours {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.marks.is_empty() && self.folders.is_empty()
    }
}

/// Everything of yours that touches a segment.
///
/// `text` is the line **as the pane is drawing it** — corrected, and with or
/// without nikud — because that is the string a highlight's offsets are
/// against. Handing this the text off the disk instead would place every
/// highlight in a corrected sefer a few characters out.
#[must_use]
pub fn yours(shelf: &Shelf, at: &Standing, text: &str) -> Yours {
    let mut notes: Vec<Wrote> = shelf
        .notes()
        .touching(at)
        .iter()
        .map(|n| Wrote::of(n))
        .collect();
    notes.sort_by(|a, b| b.edited.cmp(&a.edited).then_with(|| a.title.cmp(&b.title)));

    let marks = shelf
        .marks()
        .on(at)
        .into_iter()
        .map(|mark| Marked {
            placed: mark.place(text),
            mark: mark.clone(),
        })
        .collect();

    let folders = shelf
        .collections()
        .holding(at)
        .into_iter()
        .map(|folder| folder.name.clone())
        .collect();

    Yours {
        notes,
        marks,
        folders,
    }
}

/// Write a note about where you are standing. The three-second one.
///
/// The title is taken from the first words if none is given, because being made
/// to name a thought before writing it down is exactly the friction W20
/// measured and this inherits.
///
/// # Errors
///
/// If the note says nothing, or your layer will not take it.
pub fn note_here(
    shelf: &mut Shelf,
    at: &SegmentId,
    title: Option<&str>,
    text: &str,
    who: &str,
) -> Result<Note, ShelfError> {
    let body = text.trim();
    if body.is_empty() {
        return Err(ShelfError::Refused(NoteError::Empty.to_string()));
    }
    let named = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map_or_else(|| first_words(body, 6), ToString::to_string);

    let mut note = shelf.notes().start(&named, who);
    for para in body.split("\n\n") {
        let para = para.trim();
        if !para.is_empty() {
            note.append(para);
        }
    }
    note.anchor(at.clone());
    shelf.write_note(note)
}

/// Put a line in a chaburah folder, making the folder if it is not there yet.
///
/// # Errors
///
/// If your layer will not write.
pub fn collect(
    shelf: &mut Shelf,
    name: &str,
    title: &str,
    at: &SegmentId,
) -> Result<(), ShelfError> {
    let mut folder = shelf
        .collections()
        .get(name)
        .cloned()
        .unwrap_or_else(|| Collection::new(name, title));
    folder.put(girsa_note::Member::Place(at.clone()));
    shelf
        .collections_mut()
        .save(folder)
        .map(|_| ())
        .map_err(|e| ShelfError::Refused(e.to_string()))
}

/// The first `count` words of a string, with an ellipsis if there were more.
fn first_words(text: &str, count: usize) -> String {
    let mut words = text.split_whitespace();
    let taken: Vec<&str> = words.by_ref().take(count).collect();
    let opening = taken.join(" ");
    if words.next().is_some() {
        format!("{opening}…")
    } else {
        opening
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::Ordinal;

    const LINE: &str = "יתגבר כארי לעמוד בבוקר לעבודת בוראו";

    fn at() -> SegmentId {
        SegmentId::new(
            "shulchan-arukh/orach-chayim",
            vec!["1".to_string(), "1".to_string()],
            Ordinal::root(1),
        )
    }

    fn shelf(name: &str) -> Shelf {
        crate::shelf::tests::shelf_of(
            vec![crate::shelf::tests::work("shulchan-arukh/orach-chayim")],
            &crate::shelf::tests::scratch(name),
        )
    }

    #[test]
    fn a_note_written_where_you_are_standing_is_anchored_there_and_named_for_you() {
        let mut shelf = shelf("girsa-notes-here");
        let note = note_here(
            &mut shelf,
            &at(),
            None,
            "וצריך עיון מה שכתב הרמ\"א כאן",
            "me",
        )
        .expect("writes");
        assert_eq!(note.on, vec![at()]);
        assert_eq!(note.title, "וצריך עיון מה שכתב הרמ\"א כאן");
        assert_eq!(note.paras().len(), 1);

        let yours = yours(&shelf, &Standing::just(at()), LINE);
        assert_eq!(yours.notes.len(), 1);
        assert_eq!(yours.notes[0].slug, note.slug);
        assert!(yours.notes[0].opening.starts_with("וצריך עיון"));
    }

    #[test]
    fn a_note_with_nothing_in_it_is_refused() {
        let mut shelf = shelf("girsa-notes-empty");
        assert!(note_here(&mut shelf, &at(), None, "   \n\n  ", "me").is_err());
    }

    #[test]
    fn a_highlight_is_placed_against_the_line_the_pane_is_drawing() {
        let mut shelf = shelf("girsa-notes-marks");
        shelf
            .marks_mut()
            .add(Mark::highlight(at(), 6..10, "כארי", "me"))
            .expect("takes");

        let before = yours(&shelf, &Standing::just(at()), LINE);
        assert_eq!(before.marks.len(), 1);
        assert_eq!(
            before.marks[0].placed,
            Placed::At {
                span: 6..10,
                moved: false
            }
        );

        // The same mark against a corrected line, which is what the pane will
        // actually be holding.
        let corrected = format!("וכן {LINE}");
        let after = yours(&shelf, &Standing::just(at()), &corrected);
        assert_eq!(
            after.marks[0].placed,
            Placed::At {
                span: 10..14,
                moved: true
            }
        );
    }

    #[test]
    fn a_line_says_which_of_your_chaburos_it_is_in() {
        let mut shelf = shelf("girsa-notes-folders");
        collect(&mut shelf, "thursday", "חבורה יום ה", &at()).expect("collects");
        collect(&mut shelf, "thursday", "חבורה יום ה", &at()).expect("twice is once");
        let yours = yours(&shelf, &Standing::just(at()), LINE);
        assert_eq!(yours.folders, vec!["thursday".to_string()]);
        assert_eq!(
            shelf.collections().get("thursday").map(|f| f.members.len()),
            Some(1)
        );
    }
}
