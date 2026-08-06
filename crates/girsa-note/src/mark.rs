//! Highlights and bookmarks — the marks you leave on somebody else's words.
//!
//! spec.md §11. Two things, one record, and the difference between them is
//! whether there is a span: a **highlight** is on some words, a **bookmark** is
//! on the place. Making them two tables would mean two files, two panels and
//! two answers to *what have I marked in this sefer*, for a distinction that is
//! one `Option`.
//!
//! # An offset is not a place
//!
//! A highlight is stored as a character range, and a range is a fact about the
//! text as it stood when you dragged over it. Correct a typo above it and the
//! range now names different letters. So a mark carries **the words as well as
//! the offsets**, and lands through [`girsa_corpus::span::locate`] — the same
//! rule, and the same code, as a correction (W20). When the two disagree the
//! words win, and only if they are there exactly once; otherwise the mark is
//! reported [`Placed::Stale`] rather than drawn over whatever is at those
//! offsets now.
//!
//! That is not defensive coding. A highlight silently sliding onto the wrong
//! half of a sentence is the sort of wrongness a reader never catches, because
//! a highlight looks the same wherever it is.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use girsa_corpus::segment::SegmentId;
use girsa_corpus::standing::Standing;
use serde::{Deserialize, Serialize};

/// What a mark is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// On some words.
    Highlight,
    /// On the place. *Come back here.*
    Bookmark,
}

impl Kind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Highlight => "highlight",
            Self::Bookmark => "bookmark",
        }
    }

    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "highlight" => Some(Self::Highlight),
            "bookmark" => Some(Self::Bookmark),
            _ => None,
        }
    }
}

/// What names a mark.
///
/// Content-addressed over **what it marks** — the place, the words and the kind
/// — and not over who or when, so that a marks file taken twice does not mark
/// everything twice. The same rule `girsa-fix` names a patch by, for the same
/// reason.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MarkId(String);

impl std::fmt::Display for MarkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl MarkId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for MarkId {
    fn from(name: String) -> Self {
        Self(name)
    }
}

/// FNV-1a, 64-bit — small, dependency-free and the same everywhere, which is
/// the only property an id like this needs.
fn fingerprint(parts: &[&str]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{hash:016x}")
}

/// One highlight, or one bookmark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    pub id: MarkId,
    /// The permanent id of the segment (spec.md §3). **Never a line number.**
    pub at: SegmentId,
    pub kind: Kind,
    /// Which characters, for a highlight. `None` is the whole segment, which is
    /// what a bookmark is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_char: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_char: Option<usize>,
    /// The words the mark was made on. What lets it be found again when the
    /// offsets rot — see the module note.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub was: String,
    /// What you called it. A bookmark's name, a highlight's reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Which colour, where the reader chose one. Free text: a palette is a
    /// window's business, not a file format's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub who: String,
    pub when: u64,
}

impl Mark {
    /// Highlight some words.
    #[must_use]
    pub fn highlight(
        at: SegmentId,
        span: Range<usize>,
        was: impl Into<String>,
        who: impl Into<String>,
    ) -> Self {
        let mut mark = Self {
            id: MarkId(String::new()),
            at,
            kind: Kind::Highlight,
            from_char: Some(span.start),
            to_char: Some(span.end),
            was: was.into(),
            label: None,
            colour: None,
            tags: Vec::new(),
            who: who.into(),
            when: crate::now_seconds(),
        };
        mark.id = mark.name();
        mark
    }

    /// Mark the place.
    #[must_use]
    pub fn bookmark(at: SegmentId, who: impl Into<String>) -> Self {
        let mut mark = Self {
            id: MarkId(String::new()),
            at,
            kind: Kind::Bookmark,
            from_char: None,
            to_char: None,
            was: String::new(),
            label: None,
            colour: None,
            tags: Vec::new(),
            who: who.into(),
            when: crate::now_seconds(),
        };
        mark.id = mark.name();
        mark
    }

    #[must_use]
    pub fn called(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn coloured(mut self, colour: impl Into<String>) -> Self {
        self.colour = Some(colour.into());
        self
    }

    #[must_use]
    pub fn tagged(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        for tag in tags {
            let tag = tag.trim().to_string();
            if !tag.is_empty() && !self.tags.iter().any(|kept| crate::same_tag(kept, &tag)) {
                self.tags.push(tag);
            }
        }
        self
    }

    /// Marked at this moment rather than now — for a test that wants two it can
    /// tell apart, and for reading a marks file somebody else wrote.
    #[must_use]
    pub const fn made_at(mut self, when: u64) -> Self {
        self.when = when;
        self
    }

    fn name(&self) -> MarkId {
        MarkId(fingerprint(&[
            &self.at.to_string(),
            &self.from_char.unwrap_or_default().to_string(),
            &self.to_char.unwrap_or_default().to_string(),
            &self.was,
            self.kind.as_str(),
        ]))
    }

    #[must_use]
    pub fn span(&self) -> Option<Range<usize>> {
        Some(self.from_char?..self.to_char?)
    }

    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|kept| crate::same_tag(kept, tag))
    }

    /// Where this mark lands in the segment as it stands now.
    ///
    /// A bookmark is on the place and has nowhere to land, so it is always
    /// [`Placed::Whole`]. A highlight is put through the same re-anchoring a
    /// correction is, and says whether it had to move.
    #[must_use]
    pub fn place(&self, text: &str) -> Placed {
        let Some(span) = self.span() else {
            return Placed::Whole;
        };
        let letters: Vec<char> = text.chars().collect();
        match girsa_corpus::span::locate(&letters, span, &self.was) {
            Some(found) => Placed::At {
                span: found.span,
                moved: found.moved,
            },
            None => Placed::Stale,
        }
    }
}

/// Where a mark is, now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placed {
    /// On the whole segment — a bookmark.
    Whole,
    At {
        span: Range<usize>,
        /// The words had to be looked for. Reported, because a reader is
        /// entitled to know that a highlight moved.
        moved: bool,
    },
    /// The words are gone, or are now there more than once. Not drawn, and not
    /// deleted either: it is a thing you did, and it is reported so you can put
    /// it right.
    Stale,
}

/// Why a mark was not taken.
#[derive(Debug, thiserror::Error)]
pub enum MarkError {
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0} is not a span of anything")]
    NotASpan(String),
    #[error("a highlight has to be on some words")]
    NoWords,
}

/// Where they live under a personal layer.
#[must_use]
pub fn path_in(personal: &Path) -> PathBuf {
    personal.join("marks.jsonl")
}

/// Everything you have marked.
#[derive(Debug, Clone)]
pub struct Marks {
    path: PathBuf,
    by_segment: BTreeMap<SegmentId, Vec<Mark>>,
}

impl Marks {
    /// Read them. A line that will not parse costs that mark and is reported.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        let path = path_in(personal);
        let mut marks = Self {
            path,
            by_segment: BTreeMap::new(),
        };
        let mut trouble = Vec::new();
        let Ok(body) = std::fs::read_to_string(&marks.path) else {
            return (marks, trouble);
        };
        for (n, line) in body.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Mark>(line) {
                Ok(mark) => marks.hold(mark),
                Err(e) => trouble.push(format!(
                    "{}: line {} is not a mark: {e}",
                    marks.path.display(),
                    n + 1
                )),
            }
        }
        (marks, trouble)
    }

    /// A layer that is never written.
    #[must_use]
    pub fn nowhere() -> Self {
        Self {
            path: PathBuf::new(),
            by_segment: BTreeMap::new(),
        }
    }

    fn hold(&mut self, mark: Mark) {
        let held = self.by_segment.entry(mark.at.clone()).or_default();
        if held.iter().any(|kept| kept.id == mark.id) {
            return;
        }
        held.push(mark);
        held.sort_by_key(|m| m.from_char.unwrap_or_default());
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.by_segment.values().map(Vec::len).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_segment.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &Mark> {
        self.by_segment.values().flatten()
    }

    /// The marks on a place, in reading order.
    ///
    /// A mark made before a cut carved the line up is still on what the line
    /// became (spec.md §3), and a mark made before upstream folded the line into
    /// its neighbour is still on the words — both because a [`Standing`] answers
    /// to every name those words have carried.
    ///
    /// # One direction, where there were two
    ///
    /// This used to also ask `at.covers(id)` — the reader's id being the
    /// *coarser* one, catching marks made on pieces of where they stand. That
    /// arm could only fire when an id and something below it were both places at
    /// once, and a cut deletes its parent, so the only thing it ever caught was
    /// a se'if upstream had **inserted** below the line — someone else's words.
    /// A caller holding a name that is no longer a place resolves it through
    /// `Open::covered_by` first; asking about somewhere that is not somewhere is
    /// the wrong question one level up.
    #[must_use]
    pub fn on(&self, at: &Standing) -> Vec<&Mark> {
        self.by_segment
            .iter()
            .filter(|(id, _)| at.named_by(id))
            .flat_map(|(_, marks)| marks)
            .collect()
    }

    /// The marks in one sefer — what a *my highlights in Berakhot* list is.
    pub fn in_work<'a>(&'a self, slug: &'a str) -> impl Iterator<Item = &'a Mark> {
        self.all().filter(move |mark| mark.at.work() == slug)
    }

    #[must_use]
    pub fn by_tag(&self, tag: &str) -> Vec<&Mark> {
        self.all().filter(|mark| mark.has_tag(tag)).collect()
    }

    /// Every bookmark, most recent first — the *take me back* list.
    #[must_use]
    pub fn bookmarks(&self) -> Vec<&Mark> {
        let mut found: Vec<&Mark> = self
            .all()
            .filter(|mark| mark.kind == Kind::Bookmark)
            .collect();
        found.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| a.at.cmp(&b.at)));
        found
    }

    /// Take a mark, and write it down.
    ///
    /// # Errors
    ///
    /// If a highlight is not on a span or carries no words, or your layer will
    /// not write. **A mark that will not save is not held in memory either** —
    /// the same rule as a correction, so that what is on the screen and what is
    /// on the disk are the same marks.
    pub fn add(&mut self, mark: Mark) -> Result<&Mark, MarkError> {
        if mark.kind == Kind::Highlight {
            match mark.span() {
                Some(span) if span.start < span.end => {}
                _ => {
                    return Err(MarkError::NotASpan(format!(
                        "{:?}..{:?}",
                        mark.from_char, mark.to_char
                    )))
                }
            }
            if mark.was.trim().is_empty() {
                return Err(MarkError::NoWords);
            }
        }
        let (at, id) = (mark.at.clone(), mark.id.clone());
        self.hold(mark);
        if let Err(e) = self.save() {
            self.forget(&at, &id);
            return Err(e);
        }
        self.by_segment
            .get(&at)
            .and_then(|held| held.iter().find(|m| m.id == id))
            .ok_or(MarkError::NoWords)
    }

    fn forget(&mut self, at: &SegmentId, id: &MarkId) -> bool {
        let Some(held) = self.by_segment.get_mut(at) else {
            return false;
        };
        let before = held.len();
        held.retain(|m| m.id != *id);
        let gone = held.len() != before;
        if held.is_empty() {
            self.by_segment.remove(at);
        }
        gone
    }

    /// Take a mark back. `false` if there was no such mark.
    ///
    /// # Errors
    ///
    /// If your layer will not write.
    pub fn remove(&mut self, id: &MarkId) -> Result<bool, MarkError> {
        let Some(at) = self.all().find(|m| m.id == *id).map(|m| m.at.clone()) else {
            return Ok(false);
        };
        let gone = self.forget(&at, id);
        self.save()?;
        Ok(gone)
    }

    /// The file, as it goes to disk. One mark a line, so it is greppable and a
    /// diff of it is readable.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut body = String::new();
        for mark in self.all() {
            if let Ok(line) = serde_json::to_string(mark) {
                body.push_str(&line);
                body.push('\n');
            }
        }
        body
    }

    fn save(&self) -> Result<(), MarkError> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        let io = |source: std::io::Error| MarkError::Io {
            path: self.path.display().to_string(),
            source,
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let temp = self.path.with_extension("jsonl.writing");
        std::fs::write(&temp, self.to_text()).map_err(io)?;
        std::fs::rename(&temp, &self.path).map_err(io)?;
        Ok(())
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

    #[test]
    fn a_highlight_follows_its_words_when_the_line_moves_under_it() {
        let mark = Mark::highlight(at(), 6..10, "כארי", "me");
        assert_eq!(
            mark.place(LINE),
            Placed::At {
                span: 6..10,
                moved: false
            }
        );
        // A correction above it made the line four characters longer.
        let corrected = format!("וכן {LINE}");
        assert_eq!(
            mark.place(&corrected),
            Placed::At {
                span: 10..14,
                moved: true
            },
            "and it says that it moved"
        );
    }

    #[test]
    fn a_highlight_whose_words_are_gone_is_stale_rather_than_drawn_elsewhere() {
        // BUILDER.md rule 6. A highlight looks the same wherever it is, so a
        // wrong one is a wrongness nobody catches.
        let mark = Mark::highlight(at(), 6..10, "כארי", "me");
        assert_eq!(mark.place("משהו אחר לגמרי"), Placed::Stale);
    }

    #[test]
    fn a_bookmark_is_on_the_place_and_has_nothing_to_land_on() {
        let mark = Mark::bookmark(at(), "me").called("להתחיל כאן");
        assert_eq!(mark.place(LINE), Placed::Whole);
        assert_eq!(mark.span(), None);
    }

    #[test]
    fn the_same_mark_made_twice_is_one_mark() {
        let mut marks = Marks::nowhere();
        marks
            .add(Mark::highlight(at(), 6..10, "כארי", "me").made_at(1))
            .expect("takes");
        marks
            .add(Mark::highlight(at(), 6..10, "כארי", "somebody else").made_at(999))
            .expect("takes");
        assert_eq!(
            marks.count(),
            1,
            "named by what it marks, not by who or when"
        );
    }

    #[test]
    fn a_highlight_on_nothing_is_refused() {
        let mut marks = Marks::nowhere();
        assert!(marks
            .add(Mark::highlight(at(), 6..6, "כארי", "me"))
            .is_err());
        assert!(marks.add(Mark::highlight(at(), 6..10, "", "me")).is_err());
    }

    #[test]
    fn a_mark_made_before_a_split_is_still_on_what_the_line_became() {
        let mut marks = Marks::nowhere();
        marks
            .add(Mark::highlight(at(), 6..10, "כארי", "me"))
            .expect("takes");
        // A cut deletes its parent, so the piece answers to the name the mark
        // was made under. A se'if inserted below a line that is still there
        // does not — see `girsa_corpus::standing`.
        let child = at().split(2).remove(0);
        assert_eq!(marks.on(&Standing::of(child.clone(), [at()])).len(), 1);
        assert_eq!(
            marks.on(&Standing::just(child)).len(),
            0,
            "a name below a line that is still on the shelf is somewhere else"
        );
    }

    #[test]
    fn marks_survive_a_restart() {
        let dir = crate::note::tests::scratch("marks");
        let (mut marks, _) = Marks::open(&dir);
        marks
            .add(
                Mark::highlight(at(), 6..10, "כארי", "me")
                    .called("עיין כאן")
                    .coloured("amber")
                    .tagged(["השכמת הבוקר".to_string()]),
            )
            .expect("takes");
        marks.add(Mark::bookmark(at(), "me")).expect("takes");

        let (back, trouble) = Marks::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(back.count(), 2);
        assert_eq!(back.bookmarks().len(), 1);
        assert_eq!(back.by_tag("השכמת הבוקר").len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
