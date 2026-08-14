//! A note: a sefer of yours, whose paragraphs are nodes.
//!
//! spec.md §11, BUILDER.md W27.
//!
//! # Three ids, and what each of them is for
//!
//! ```text
//! girsa:note/מאימתי-קורין/1#1     the note itself — its title, and what an edge hangs off
//! girsa:note/מאימתי-קורין/3#3     the third paragraph, as it was written
//! girsa:note/מאימתי-קורין/3.1#3.1 a paragraph put in between #3 and #4, later
//! ```
//!
//! The address and the ordinal look alike here and are not the same thing: the
//! **address** is what a citation prints — *הערה, פסקה ג* — and the
//! **ordinal** is the permanent name (spec.md §3). They are equal at the
//! moment a paragraph is written because a note's address is its paragraph
//! number, and they stay equal because neither is ever recomputed.
//!
//! # Writing between two paragraphs
//!
//! The interesting case, and it is W6's trick reused rather than a second
//! mechanism: a paragraph written after `#3` is minted `#3.1`, which sorts
//! after `#3` and before `#4` — so **nothing renumbers** and every note,
//! highlight, link and citation pointing at `#4` still names the words it named
//! before. Write after `#3` twice and you get `#3.1` then `#3.2`, in the order
//! you wrote them, which is what typing two paragraphs in a row means.
//!
//! A paragraph you delete does not give its ordinal back. `next:` is in the
//! file for exactly that reason: an ordinal that could be handed out twice
//! would point two different things at one name, permanently.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_corpus::era;
use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::{Ordinal, SegmentId};
use girsa_corpus::standing::Standing;
use girsa_corpus::work::{self, Source, Version, Work};
use girsa_link::{Anchor, Edge, EdgeType, Method};

/// The slug every note's work slug begins with — `note/מאימתי-קורין`.
///
/// A namespace of its own rather than `user/`, so that *which of my seforim are
/// notes* is answerable without opening any of them, and so a note can never
/// collide with a file you dropped on the window.
pub const SHELF: &str = "note";

/// The first line of a note file, and the whole of what makes it one.
const BANNER: &str = "girsa note";

/// What a note's paragraphs are called when one is cited.
const LEVEL: &str = "פסקה";

/// One paragraph of a note, and its permanent name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Para {
    pub id: SegmentId,
    pub text: String,
}

/// Why a note would not be written, or a paragraph placed.
#[derive(Debug, thiserror::Error)]
pub enum NoteError {
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0} is not a paragraph of this note")]
    NoSuchParagraph(String),
    #[error("a note has to say something")]
    Empty,
    #[error("there is no note called {0}")]
    NoSuchNote(String),
    #[error("{0}")]
    Refused(String),
}

/// Something you wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// `note/מאימתי-קורין`. The work slug, so every id in it begins with this.
    pub slug: String,
    pub title: String,
    pub tags: Vec<String>,
    /// What in the library this note is about (spec.md §11: *anchored to
    /// segment ids*). These are the note's edges — see [`Note::edges`].
    pub on: Vec<SegmentId>,
    /// Free text. This is a personal layer, not a registry.
    pub who: String,
    pub when: u64,
    pub edited: u64,
    /// The next ordinal that may be minted at the end. Never goes down, so an
    /// ordinal is never handed out twice.
    next: u32,
    paras: Vec<Para>,
}

impl Note {
    /// Start one. `slug` is the whole work slug, `note/…`.
    #[must_use]
    pub fn new(slug: impl Into<String>, title: impl Into<String>, who: impl Into<String>) -> Self {
        let when = girsa_personal::now_seconds();
        Self {
            slug: slug.into(),
            title: title.into(),
            tags: Vec::new(),
            on: Vec::new(),
            who: who.into(),
            when,
            edited: when,
            next: 2,
            paras: Vec::new(),
        }
    }

    /// The note itself, as a place: its title segment.
    ///
    /// This is what an edge to a sugya hangs off, so that *the note* is what
    /// links there rather than whichever paragraph happened to be first.
    #[must_use]
    pub fn id(&self) -> SegmentId {
        SegmentId::new(&self.slug, vec!["1".to_string()], Ordinal::root(1))
    }

    /// The name a note is asked for by — the slug without the `note/`.
    #[must_use]
    pub fn name(&self) -> &str {
        self.slug.strip_prefix("note/").unwrap_or(&self.slug)
    }

    #[must_use]
    pub fn paras(&self) -> &[Para] {
        &self.paras
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paras.iter().all(|p| p.text.trim().is_empty())
    }

    /// The words, for a preview or a search snippet.
    #[must_use]
    pub fn words(&self) -> String {
        self.paras
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Write a paragraph at the end.
    pub fn append(&mut self, text: impl Into<String>) -> SegmentId {
        let ordinal = Ordinal::root(self.next);
        self.next = self.next.saturating_add(1);
        let id = self.mint(&ordinal);
        self.paras.push(Para {
            id: id.clone(),
            text: text.into(),
        });
        self.touched();
        id
    }

    /// Write a paragraph directly after another one.
    ///
    /// Mints a **child** of `after`, which sorts between it and whatever came
    /// next — so nothing already written moves. See the module note.
    ///
    /// # Errors
    ///
    /// If `after` is not a paragraph of this note.
    pub fn insert_after(
        &mut self,
        after: &SegmentId,
        text: impl Into<String>,
    ) -> Result<SegmentId, NoteError> {
        let parent = self
            .paras
            .iter()
            .find(|p| p.id == *after)
            .map(|p| p.id.ordinal().clone())
            .ok_or_else(|| NoteError::NoSuchParagraph(after.to_string()))?;

        // The first child index nothing is using. Counting up rather than
        // taking `children + 1`: a paragraph deleted from the middle of a run
        // may not have its name handed to the next one.
        let mut k = 1u32;
        let ordinal = loop {
            let candidate = parent.child(k);
            if !self.paras.iter().any(|p| *p.id.ordinal() == candidate) {
                break candidate;
            }
            k = k.saturating_add(1);
        };

        let id = self.mint(&ordinal);
        self.paras.push(Para {
            id: id.clone(),
            text: text.into(),
        });
        self.order();
        self.touched();
        Ok(id)
    }

    /// Change what a paragraph says. Its id does not move — that is the whole
    /// arrangement (spec.md §3).
    pub fn set(&mut self, id: &SegmentId, text: impl Into<String>) -> bool {
        let Some(para) = self.paras.iter_mut().find(|p| p.id == *id) else {
            return false;
        };
        para.text = text.into();
        self.touched();
        true
    }

    /// Take a paragraph out. Its ordinal is retired, not reused.
    pub fn remove(&mut self, id: &SegmentId) -> bool {
        let before = self.paras.len();
        self.paras.retain(|p| p.id != *id);
        let gone = self.paras.len() != before;
        if gone {
            self.touched();
        }
        gone
    }

    /// Say this note is about a place in the library. `false` if it already was.
    pub fn anchor(&mut self, at: SegmentId) -> bool {
        if self.on.contains(&at) {
            return false;
        }
        self.on.push(at);
        self.on.sort();
        self.touched();
        true
    }

    /// Take an anchor back.
    pub fn unanchor(&mut self, at: &SegmentId) -> bool {
        let before = self.on.len();
        self.on.retain(|id| id != at);
        let gone = self.on.len() != before;
        if gone {
            self.touched();
        }
        gone
    }

    /// Tag it. `false` if it already carried that tag, however spelled.
    pub fn tag(&mut self, tag: &str) -> bool {
        let tag = tag.trim();
        if tag.is_empty() || self.tags.iter().any(|kept| crate::same_tag(kept, tag)) {
            return false;
        }
        self.tags.push(tag.to_string());
        self.touched();
        true
    }

    /// Untag it.
    pub fn untag(&mut self, tag: &str) -> bool {
        let before = self.tags.len();
        self.tags.retain(|kept| !crate::same_tag(kept, tag));
        let gone = self.tags.len() != before;
        if gone {
            self.touched();
        }
        gone
    }

    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|kept| crate::same_tag(kept, tag))
    }

    /// The edges this note makes: one from the note to each place it is about.
    ///
    /// [`EdgeType::CommentsOn`] and not the `references` catch-all, because a
    /// note written on a line **is** a comment on it — that is what writing one
    /// means — and calling it *references* would file your own considered claim
    /// under the label spec.md §8.2 keeps for links nobody has looked at. The
    /// method is [`Method::ByHand`], which is the truth and which is also what
    /// makes it certain: you are the authority on your own layer.
    ///
    /// Retyping one is W23's job and needs nothing new — the edge is an edge.
    #[must_use]
    pub fn edges(&self) -> Vec<Edge> {
        let from = Anchor::point(self.id());
        self.on
            .iter()
            .map(|at| Edge {
                from: from.clone(),
                to: Anchor::point(at.clone()),
                edge_type: EdgeType::CommentsOn,
                method: Method::ByHand,
                // You wrote the note and you said what it is on. Nobody had to
                // infer which end is the commentary.
                direction: girsa_link::Direction::Declared,
                source_label: String::new(),
            })
            .collect()
    }

    /// The note as segments, ready to go on the shelf.
    ///
    /// The title is a [`SegmentKind::Heading`], which is what it is, and which
    /// makes a note render in a pane the way a sefer with headings does.
    #[must_use]
    pub fn segments(&self) -> Vec<Segment> {
        let mut out = vec![Segment {
            id: self.id(),
            kind: SegmentKind::Heading,
            text: self.title.clone(),
            // Sefaria's inline commentary anchors are a property of Sefaria's
            // markup (W34). You typed this, so there are none — and an empty
            // vec is the honest answer rather than a reason for the field to
            // be optional.
            anchors: Vec::new(),
        }];
        out.extend(self.paras.iter().map(|para| Segment {
            id: para.id.clone(),
            kind: SegmentKind::Text,
            text: para.text.clone(),
            anchors: Vec::new(),
        }));
        out
    }

    /// The catalogue entry: a note is a sefer of yours.
    #[must_use]
    pub fn work(&self, personal: &Path) -> Work {
        Work {
            slug: self.slug.clone(),
            he_title: self.title.clone(),
            en_title: self.title.clone(),
            // spec.md §5's *yours*, so the shelf files it where your own
            // material goes and nothing has to know a note is special.
            categories: vec!["שלי".to_string()],
            order: Vec::new(),
            source: Source::Mine,
            origin: path_in(personal, self.name()),
            schema: None,
            author: (!self.who.trim().is_empty()).then(|| self.who.clone()),
            // The one work in this library whose date is known rather than
            // estimated. Sefaria's schemas say `c.1065  – c.1115 CE`; a note
            // says the second it was saved, and `when` has carried it since
            // the file format existed — it was simply never copied onto the
            // catalogue entry, so the timeline could not place a note and the
            // chain could not walk into one.
            //
            // Dated from `when` and not `edited`: a chain asks when a thing
            // was written, and rewording a paragraph in 2030 does not move a
            // note behind the sefer it was answering.
            era: Some(era::Era::Contemporary.code().to_string()),
            comp_date: Some(era::written_at(self.when)),
            version: Some(Version {
                edition: "your own note".to_string(),
                provenance: None,
                license: None,
            }),
            // So `girsa-cite` prints *פסקה ג* rather than a bare number.
            he_sections: vec![LEVEL.to_string()],
            commentary_on: Vec::new(),
        }
    }

    /// The note as it goes to disk.
    ///
    /// Plain text, with each paragraph's permanent id on the line above it. The
    /// id is in the file rather than implied by position for the reason the
    /// segments file has the same shape (`girsa_corpus::import`): this file is
    /// meant to be edited, moved and diffed, and none of that may move an
    /// anchor.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(BANNER);
        out.push('\n');
        out.push_str(&format!("title: {}\n", one_line(&self.title)));
        out.push_str(&format!("who: {}\n", one_line(&self.who)));
        out.push_str(&format!("when: {}\n", self.when));
        out.push_str(&format!("edited: {}\n", self.edited));
        out.push_str(&format!("next: {}\n", self.next));
        for tag in &self.tags {
            out.push_str(&format!("tag: {}\n", one_line(tag)));
        }
        for at in &self.on {
            out.push_str(&format!("on: {at}\n"));
        }
        for para in &self.paras {
            out.push_str(&format!("\n{}\n{}\n", para.id, para.text.trim_end()));
        }
        out
    }

    /// Read one back.
    ///
    /// A file that does not begin with the banner is **read as a note anyway**
    /// — a plain `.md` you wrote in something else, whose paragraphs are given
    /// ids in order the first time it is opened. That is the import half of
    /// *exportable as plain files*, and it is stated rather than silent: the
    /// ids are minted at that moment and written down at the next save, and
    /// from then on they are permanent like any others.
    #[must_use]
    pub fn parse(slug: &str, body: &str) -> (Self, Vec<String>) {
        let mut trouble = Vec::new();
        let name = slug.strip_prefix("note/").unwrap_or(slug);
        let mut note = Self::new(slug, name, String::new());
        note.paras.clear();

        let mut lines = body.lines().peekable();
        let banner = lines.peek().is_some_and(|l| l.trim() == BANNER);
        if banner {
            lines.next();
            for line in lines.by_ref() {
                let line = line.trim_end();
                if line.trim().is_empty() {
                    break;
                }
                let Some((key, value)) = line.split_once(':') else {
                    trouble.push(format!("{slug}: `{line}` is not a heading of a note"));
                    continue;
                };
                let value = value.trim();
                match key.trim() {
                    "title" => note.title = value.to_string(),
                    "who" => note.who = value.to_string(),
                    "when" => note.when = value.parse().unwrap_or(note.when),
                    "edited" => note.edited = value.parse().unwrap_or(note.edited),
                    "next" => note.next = value.parse().unwrap_or(note.next),
                    "tag" => {
                        if !value.is_empty() {
                            note.tags.push(value.to_string());
                        }
                    }
                    "on" => match value.parse::<SegmentId>() {
                        Ok(at) => note.on.push(at),
                        // Never guessed at (BUILDER.md rule 6). An anchor that
                        // will not read is one anchor, said out loud, and the
                        // rest of the note is still yours.
                        Err(e) => trouble.push(format!("{slug}: `{value}` is not a place: {e}")),
                    },
                    other => trouble.push(format!("{slug}: `{other}` is not a heading of a note")),
                }
            }
        }

        // The body: a line that is an id of this note names the paragraph
        // under it. Everything before the first one is loose text, which is
        // what a file written somewhere else is made of.
        let mut current: Option<SegmentId> = None;
        let mut held = String::new();
        let mut loose: Vec<String> = Vec::new();
        let rest: Vec<&str> = lines.collect();
        for line in rest {
            let marker = line
                .trim()
                .parse::<SegmentId>()
                .ok()
                .filter(|id| id.work() == slug);
            if let Some(id) = marker {
                match current.take() {
                    Some(previous) => note.hold(previous, &held),
                    None => loose.push(held.clone()),
                }
                held.clear();
                current = Some(id);
                continue;
            }
            held.push_str(line);
            held.push('\n');
        }
        match current.take() {
            Some(previous) => note.hold(previous, &held),
            None => loose.push(held.clone()),
        }

        // `next:` may have been wrong, or absent on a file written elsewhere.
        // It may only ever go up, and it is settled **before** any loose text
        // is given a name: an ordinal already in use is one that may not be
        // minted again, and minting over one is how two paragraphs come to
        // share a permanent name.
        note.settle_next();

        // Loose text gets ids now, in the order it is written — the only
        // moment there is nothing to preserve.
        for block in loose.join("\n").split("\n\n") {
            let block = block.trim();
            if !block.is_empty() {
                note.append(block);
            }
        }
        if !banner {
            note.title = name.replace('-', " ");
        }

        note.order();
        note.settle_next();
        note.edited = note.edited.max(note.when);
        (note, trouble)
    }

    /// Move `next` past every ordinal already minted at the top level.
    fn settle_next(&mut self) {
        let highest = self
            .paras
            .iter()
            .filter(|p| p.id.ordinal().depth() == 1)
            .filter_map(|p| p.id.path().first().and_then(|n| n.parse::<u32>().ok()))
            .max()
            .unwrap_or(1);
        self.next = self.next.max(highest.saturating_add(1)).max(2);
    }

    fn hold(&mut self, id: SegmentId, text: &str) {
        self.paras.push(Para {
            id,
            text: text.trim().to_string(),
        });
    }

    fn mint(&self, ordinal: &Ordinal) -> SegmentId {
        SegmentId::new(&self.slug, vec![ordinal.to_string()], ordinal.clone())
    }

    /// Reading order, which is ordinal order — the same rule as the corpus.
    fn order(&mut self) {
        self.paras.sort_by(|a, b| a.id.cmp(&b.id));
    }

    fn touched(&mut self) {
        self.edited = girsa_personal::now_seconds();
    }
}

/// Strip the newlines out of a heading value, so a title with a line break in
/// it cannot swallow the rest of the header.
fn one_line(value: &str) -> String {
    value.replace(['\n', '\r'], " ").trim().to_string()
}

/// Where the notes live under a personal layer.
#[must_use]
pub fn dir_in(personal: &Path) -> PathBuf {
    personal.join("notes")
}

/// The file one note lives in.
#[must_use]
pub fn path_in(personal: &Path, name: &str) -> PathBuf {
    dir_in(personal).join(file_name(name))
}

/// The file name of a note, from its name or its whole slug.
#[must_use]
pub fn file_name(name: &str) -> String {
    format!("{}.md", name.strip_prefix("note/").unwrap_or(name))
}

/// Every note you have.
#[derive(Debug, Clone)]
pub struct Notes {
    personal: PathBuf,
    by_slug: BTreeMap<String, Note>,
}

impl Notes {
    /// Read them all.
    ///
    /// A note that will not read costs that note and is reported — never the
    /// rest of them.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        let mut notes = Self {
            personal: personal.to_path_buf(),
            by_slug: BTreeMap::new(),
        };
        let mut trouble = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir_in(personal)) else {
            return (notes, trouble);
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let body = match std::fs::read_to_string(&path) {
                Ok(body) => body,
                Err(e) => {
                    trouble.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            let slug = format!("{SHELF}/{name}");
            let (note, said) = Note::parse(&slug, &body);
            trouble.extend(said);
            if stale(&path, &notes.personal, &slug) {
                if let Err(e) = notes.shelve(&note) {
                    trouble.push(format!("{}: {e}", path.display()));
                }
            }
            notes.by_slug.insert(slug, note);
        }
        (notes, trouble)
    }

    /// A layer that is never written, for a caller that only wants what it
    /// already has.
    #[must_use]
    pub fn nowhere() -> Self {
        Self {
            personal: PathBuf::new(),
            by_slug: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.by_slug.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_slug.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &Note> {
        self.by_slug.values()
    }

    /// One note, by its slug or by its name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Note> {
        self.by_slug
            .get(name)
            .or_else(|| self.by_slug.get(&format!("{SHELF}/{name}")))
    }

    /// Start a note, with a slug nothing else is using.
    ///
    /// Two notes called *חבורה* are two notes, and the second may not land on
    /// the first: a segment id is permanent, and reusing one would point every
    /// link and highlight anchored to it at somebody else's words.
    #[must_use]
    pub fn start(&self, title: &str, who: &str) -> Note {
        let base = work::hebrew_slug_of(title);
        let base = if base.is_empty() {
            "הערה".to_string()
        } else {
            base
        };
        let mut slug = format!("{SHELF}/{base}");
        if self.taken(&slug) {
            for n in 2..u32::MAX {
                slug = format!("{SHELF}/{base}-{n}");
                if !self.taken(&slug) {
                    break;
                }
            }
        }
        Note::new(slug, title, who)
    }

    fn taken(&self, slug: &str) -> bool {
        self.by_slug.contains_key(slug)
            || path_in(&self.personal, slug.strip_prefix("note/").unwrap_or(slug)).is_file()
    }

    /// Write a note down: the file, and the sefer on your shelf.
    ///
    /// Both, always, and in that order — the file is the truth (spec.md §4.1)
    /// and the catalogue entry is derived from it, so a crash between the two
    /// costs a shelf entry that the next open puts back, rather than a note.
    ///
    /// # Errors
    ///
    /// If the note says nothing, or your layer will not take it.
    pub fn write(&mut self, note: Note) -> Result<&Note, NoteError> {
        if note.title.trim().is_empty() && note.is_empty() {
            return Err(NoteError::Empty);
        }
        if self.personal.as_os_str().is_empty() {
            let slug = note.slug.clone();
            self.by_slug.insert(slug.clone(), note);
            return self.by_slug.get(&slug).ok_or(NoteError::Empty);
        }

        let path = path_in(&self.personal, note.name());
        let io = |source: std::io::Error| NoteError::Io {
            path: path.display().to_string(),
            source,
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        // Written beside and renamed over, so a machine that stops halfway
        // through leaves the note it had rather than half of one.
        let temp = path.with_extension("md.writing");
        std::fs::write(&temp, note.to_text()).map_err(io)?;
        std::fs::rename(&temp, &path).map_err(io)?;

        self.shelve(&note)?;
        let slug = note.slug.clone();
        self.by_slug.insert(slug.clone(), note);
        self.by_slug.get(&slug).ok_or(NoteError::NoSuchNote(slug))
    }

    /// Put the note on the shelf as a sefer: `work.json`, `segments.jsonl`, and
    /// a line in your catalogue. The same three files a dropped `.txt` gets, by
    /// the same code, so that nothing downstream knows a note from a sefer.
    fn shelve(&self, note: &Note) -> Result<(), NoteError> {
        let imported = girsa_corpus::import::ImportedWork {
            work: note.work(&self.personal),
            segments: note.segments(),
            // A note is a paragraph at a time and no paragraph is 10,000
            // characters, so there is nothing oversized to count (B12). Default
            // rather than measured, because measuring would be measuring zero.
            oversized: girsa_corpus::oversized::Tally::default(),
            // A note's paragraph ids are minted by the note itself and kept in
            // its own file (W27), so re-shelving one never renames anything and
            // there is never anywhere for a name to have gone.
            redirects: Vec::new(),
            continuity: girsa_corpus::import::continuity::Continuity::default(),
        };
        girsa_corpus::import::write(&self.personal, &imported)
            .map_err(|e| NoteError::Refused(e.to_string()))?;
        girsa_corpus::import::catalogue(&self.personal, &imported.work)
            .map_err(|e| NoteError::Refused(e.to_string()))
    }

    /// Throw a note away: the file, the sefer, and the catalogue line.
    ///
    /// # Errors
    ///
    /// If your layer will not write.
    pub fn remove(&mut self, name: &str) -> Result<bool, NoteError> {
        let Some(note) = self.get(name).cloned() else {
            return Ok(false);
        };
        self.by_slug.remove(&note.slug);
        if self.personal.as_os_str().is_empty() {
            return Ok(true);
        }
        let path = path_in(&self.personal, note.name());
        if let Err(source) = std::fs::remove_file(&path) {
            if source.kind() != std::io::ErrorKind::NotFound {
                return Err(NoteError::Io {
                    path: path.display().to_string(),
                    source,
                });
            }
        }
        let dir = girsa_corpus::import::work_dir(&self.personal, &note.slug);
        let _ = std::fs::remove_dir_all(dir);
        girsa_corpus::import::uncatalogue(&self.personal, &note.slug)
            .map_err(|e| NoteError::Refused(e.to_string()))?;
        Ok(true)
    }

    /// The notes about a place.
    ///
    /// Asked of a [`Standing`] and not of an id, because a note is anchored
    /// under the name the place had **when you wrote it** — which is the promise
    /// spec.md §3 is for, and the one it would be worst to break, since nobody
    /// else has a copy of your notes. A note on `#7` is still a note on the
    /// sugya after a cut carves `#7` into pieces, and is *not* a note on a se'if
    /// upstream inserted after it and named `#7.1`.
    #[must_use]
    pub fn touching(&self, at: &Standing) -> Vec<&Note> {
        self.all()
            .filter(|note| note.on.iter().any(|on| at.named_by(on)))
            .collect()
    }

    /// The same question as an answer the link graph understands.
    ///
    /// This is the whole of W27's claim in one method: what comes back is
    /// [`girsa_link::Edge`], the type `corpus/links/` is full of, so the caller
    /// that answers *who quotes this Rishon* answers *what have I written about
    /// this* with the same code.
    #[must_use]
    pub fn edges_touching(&self, at: &Standing) -> Vec<Edge> {
        self.all()
            .flat_map(Note::edges)
            .filter(|edge| edge.to.names(at) || edge.from.names(at))
            .collect()
    }

    /// Your notes in one sefer — *what have I written on Berakhot*.
    #[must_use]
    pub fn about_work(&self, slug: &str) -> Vec<&Note> {
        self.all()
            .filter(|note| note.on.iter().any(|on| on.work() == slug))
            .collect()
    }

    /// The notes carrying a tag.
    #[must_use]
    pub fn by_tag(&self, tag: &str) -> Vec<&Note> {
        self.all().filter(|note| note.has_tag(tag)).collect()
    }
}

/// Whether a note's file has moved on since the shelf entry derived from it.
///
/// # The loop this closes
///
/// This module says *the file is the truth (spec.md §4.1) and the catalogue
/// entry is derived from it* — and it was derived **once**, when the note was
/// written, and never again. `Notes::open` read only the `.md`; `Shelf::read`
/// and the index build read only `segments.jsonl`. So editing a note in vim,
/// which the design explicitly invites, left two versions of it: the words you
/// wrote, and the words the search box can find.
///
/// And the machinery that exists to make that loud made it silent instead:
///
/// 1. `since.rs` stats the `.md`, sees it is newer than the index, and says *N
///    notes are not searchable yet*.
/// 2. You rebuild the index. It reads the **stale** `segments.jsonl`.
/// 3. The stamp is now newer than the `.md`, so the gap reports zero.
///
/// A closed loop in which *"never a silent gap"* reports success over a gap it
/// created. Re-deriving on open is what makes the sentence in the module note
/// true.
///
/// Missing counts as stale: a note with no shelf entry is a note the search box
/// cannot see, which is the same problem arrived at from the other side.
fn stale(md: &Path, personal: &Path, slug: &str) -> bool {
    let Ok(note) = std::fs::metadata(md).and_then(|m| m.modified()) else {
        return false;
    };
    let shelved = girsa_corpus::import::work_dir(personal, slug).join("segments.jsonl");
    match std::fs::metadata(&shelved).and_then(|m| m.modified()) {
        Ok(shelved) => note > shelved,
        Err(_) => true,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    pub fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-note-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn sugya() -> SegmentId {
        SegmentId::new(
            "bavli/berakhot",
            vec!["2a".to_string(), "1".to_string()],
            Ordinal::root(1),
        )
    }

    fn note() -> Note {
        let mut note = Note::new("note/מאימתי", "מאימתי קורין", "me");
        note.append("הא דתנן מאימתי קורין את שמע בערבין");
        note.append("ומה שכתב הרמב\"ם");
        note.append("ולפי זה יוצא");
        note.anchor(sugya());
        note
    }

    #[test]
    fn a_paragraph_written_in_the_middle_moves_nothing() {
        // W6's test, at the scale a note is. On a design where a paragraph's
        // id is its position, inserting one moves every id below it; here the
        // ordinal extends instead, and both halves of that are asserted.
        let mut note = note();
        let before: Vec<String> = note.paras().iter().map(|p| p.id.to_string()).collect();
        let second = note.paras()[1].id.clone();

        let minted = note.insert_after(&second, "ובאמת יש לדקדק").expect("takes");
        assert_eq!(minted.to_string(), "girsa:note/מאימתי/3.1#3.1");

        let after: Vec<String> = note
            .paras()
            .iter()
            .map(|p| p.id.to_string())
            .filter(|id| id != &minted.to_string())
            .collect();
        assert_eq!(before, after, "not one paragraph was renamed");
        assert_eq!(
            note.paras()[2].id,
            minted,
            "and it went in where it was asked to"
        );

        // And the defect, shown rather than described. A store that named a
        // paragraph by its position would now have *the third paragraph*
        // meaning different words than it did a moment ago; by id, `#4` is the
        // words it always was.
        assert_eq!(note.paras()[2].text, "ובאמת יש לדקדק");
        let by_id = note
            .paras()
            .iter()
            .find(|p| p.id.to_string() == "girsa:note/מאימתי/4#4")
            .expect("#4 is still there");
        assert_eq!(by_id.text, "ולפי זה יוצא");
    }

    #[test]
    fn writing_after_the_same_paragraph_twice_comes_out_in_the_order_written() {
        let mut note = note();
        let first = note.paras()[0].id.clone();
        let a = note.insert_after(&first, "אחת").expect("takes");
        let b = note.insert_after(&first, "שתיים").expect("takes");
        let order: Vec<&str> = note.paras().iter().map(|p| p.text.as_str()).collect();
        assert_eq!(order[1], "אחת");
        assert_eq!(order[2], "שתיים");
        assert!(a < b);
    }

    #[test]
    fn a_deleted_paragraph_does_not_give_its_name_back() {
        let mut note = note();
        let last = note.paras()[2].id.clone();
        assert!(note.remove(&last));
        let again = note.append("משהו אחר");
        assert_ne!(again, last, "an ordinal may not be minted twice");
        assert_eq!(again.to_string(), "girsa:note/מאימתי/5#5");
    }

    #[test]
    fn a_note_survives_being_written_down_and_read_back() {
        let note = note();
        let (back, trouble) = Note::parse(&note.slug, &note.to_text());
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(back, note);
    }

    #[test]
    fn the_ids_are_in_the_file_so_reordering_it_changes_nothing() {
        // The property that keeps T1 out of a note. The same test the segments
        // file has, because it is the same claim.
        let mut note = note();
        let second = note.paras()[1].id.clone();
        note.insert_after(&second, "ובאמת").expect("takes");

        let text = note.to_text();
        let (head, body) = text.split_once("\n\n").expect("a header and a body");
        let mut blocks: Vec<&str> = body.split("\n\n").collect();
        blocks.reverse();
        let shuffled = format!("{head}\n\n{}", blocks.join("\n\n"));

        let (back, trouble) = Note::parse(&note.slug, &shuffled);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(back.paras(), note.paras());
    }

    #[test]
    fn a_plain_file_written_somewhere_else_is_read_as_a_note() {
        let (note, trouble) =
            Note::parse("note/חבורה-על-מאימתי", "הפסקה הראשונה\n\nוהפסקה השנייה\n");
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(note.title, "חבורה על מאימתי");
        assert_eq!(note.paras().len(), 2);
        assert_eq!(
            note.paras()[0].id.to_string(),
            "girsa:note/חבורה-על-מאימתי/2#2"
        );
        // And from the next save on it is a note like any other.
        let (again, _) = Note::parse(&note.slug, &note.to_text());
        assert_eq!(again.paras(), note.paras());
    }

    #[test]
    fn an_anchor_that_will_not_read_costs_one_anchor_and_is_said_out_loud() {
        let (note, trouble) = Note::parse(
            "note/x",
            "girsa note\ntitle: x\non: girsa:bavli/berakhot/2a:1#1\non: not a place\n",
        );
        assert_eq!(note.on.len(), 1);
        assert_eq!(trouble.len(), 1);
        assert!(trouble[0].contains("not a place"), "{trouble:?}");
    }

    #[test]
    fn a_note_is_joined_to_the_library_by_an_edge_like_any_other() {
        let note = note();
        let edges = note.edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, EdgeType::CommentsOn);
        assert_eq!(edges[0].method, Method::ByHand);
        assert_eq!(edges[0].from.from, note.id());
        assert!(edges[0].to.covers(&sugya()));
        // Certain, because you are the authority on your own layer.
        assert!((edges[0].confidence() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_note_written_before_a_split_is_still_a_note_on_the_sugya_after_it() {
        // spec.md §3: an anchor on `#7` covers what `#7` became. A note is
        // anchored the same way, so a correction that splits the line it is
        // about does not orphan it.
        let mut notes = Notes::nowhere();
        notes.write(note()).expect("takes");
        let child = sugya().split(2).remove(1);
        let standing = Standing::of(child.clone(), [sugya()]);
        assert_eq!(notes.touching(&standing).len(), 1);
        assert_eq!(notes.edges_touching(&standing).len(), 1);
        // And not on a se'if that merely sorts under the sugya's name.
        let beside = Standing::just(child);
        assert_eq!(notes.touching(&beside).len(), 0);
        assert_eq!(notes.edges_touching(&beside).len(), 0);
    }

    #[test]
    fn a_note_is_a_sefer_on_your_shelf_and_the_catalogue_says_so() {
        let personal = scratch("shelf");
        let (mut notes, _) = Notes::open(&personal);
        let note = note();
        let slug = note.slug.clone();
        notes.write(note).expect("takes");

        let read = girsa_corpus::import::read_back(&personal, &slug).expect("reads back");
        assert_eq!(read.work.source, Source::Mine);
        assert_eq!(read.work.he_sections, vec![LEVEL.to_string()]);
        assert_eq!(read.segments.len(), 4, "the title and three paragraphs");
        assert_eq!(read.segments[0].kind, SegmentKind::Heading);

        let catalogue =
            std::fs::read_to_string(personal.join("works/index.jsonl")).expect("catalogued");
        assert!(catalogue.contains(&slug));

        // And re-opening the layer finds it as it was.
        let (again, trouble) = Notes::open(&personal);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(again.count(), 1);
        assert_eq!(
            again.get("מאימתי").map(|n| n.title.clone()),
            Some("מאימתי קורין".to_string())
        );
        let _ = std::fs::remove_dir_all(&personal);
    }

    #[test]
    fn a_note_is_placed_in_time_by_the_second_it_was_saved() {
        // W27 shipped with `era: None, comp_date: None` on a note's catalogue
        // entry, and the chain's own record wrote the consequence down: a note
        // is `Unknown` against everything and is never a hop, *which is the
        // truthful answer, and not a useful one.*
        //
        // It was not the truthful answer. `when` has been on every note since
        // the format existed, and it is the one date anywhere in this corpus
        // that is known to the second instead of estimated to the century.
        // Sefaria's dates are `c.1065  – c.1115 CE`; this one is not a `c.`.
        let personal = scratch("dated");
        let (mut notes, _) = Notes::open(&personal);
        let note = note();
        let slug = note.slug.clone();
        let when = note.when;
        notes.write(note).expect("takes");

        let read = girsa_corpus::import::read_back(&personal, &slug).expect("reads back");
        assert_eq!(
            read.work.comp_date.as_deref(),
            Some(girsa_corpus::era::written_at(when).as_str()),
            "the catalogue carries the year the note was saved"
        );
        assert_eq!(
            read.work.era.as_deref(),
            Some(girsa_corpus::era::Era::Contemporary.code()),
            "and the era a reader recognises, so a facet can name it"
        );

        // The claim that matters: the timeline can place it now.
        let mut timeline = girsa_corpus::era::Timeline::default();
        timeline.load(&personal).expect("a catalogue to read");
        assert!(
            timeline.when(&slug).is_placed(),
            "a note the timeline cannot place is a note no chain can walk into"
        );
        assert_eq!(timeline.undated(), 0);
        let _ = std::fs::remove_dir_all(&personal);
    }

    #[test]
    fn two_notes_of_one_name_are_two_notes() {
        let personal = scratch("names");
        let (mut notes, _) = Notes::open(&personal);
        let first = notes.start("חבורה", "me");
        assert_eq!(first.slug, "note/חבורה");
        notes.write(first).expect("takes");
        let second = notes.start("חבורה", "me");
        assert_eq!(second.slug, "note/חבורה-2");
        let _ = std::fs::remove_dir_all(&personal);
    }

    #[test]
    fn a_note_thrown_away_leaves_nothing_on_the_shelf() {
        let personal = scratch("removed");
        let (mut notes, _) = Notes::open(&personal);
        notes.write(note()).expect("takes");
        assert!(notes.remove("מאימתי").expect("removes"));
        assert_eq!(notes.count(), 0);
        assert!(!path_in(&personal, "מאימתי").exists());
        let catalogue =
            std::fs::read_to_string(personal.join("works/index.jsonl")).unwrap_or_default();
        assert!(!catalogue.contains("note/מאימתי"));
        let _ = std::fs::remove_dir_all(&personal);
    }

    #[test]
    fn a_note_edited_in_vim_is_what_the_search_box_finds() {
        // This module says *the file is the truth and the catalogue entry is
        // derived from it*. It was derived once, when the note was written, and
        // never again — `Notes::open` read only the `.md`, `Shelf::read` and the
        // index build read only `segments.jsonl`.
        //
        // And the gap machinery made it worse rather than louder: `since.rs`
        // stats the `.md`, says *N notes are not searchable yet*, you rebuild
        // the index, it reads the **stale** `segments.jsonl`, the stamp is now
        // newer than the `.md`, and the gap reports zero. A closed loop in which
        // *"never a silent gap"* reports success over a gap it created.
        let dir = std::env::temp_dir().join("girsa-note-vim");
        let _ = std::fs::remove_dir_all(&dir);

        let mut notes = Notes::open(&dir).0;
        let mut note = Note::new("note/בדיקה", "בדיקה", "me");
        note.append("ראשון");
        notes.write(note).expect("it is written");

        let md = path_in(&dir, "בדיקה");
        let shelved = girsa_corpus::import::work_dir(&dir, "note/בדיקה").join("segments.jsonl");
        assert!(
            std::fs::read_to_string(&shelved)
                .expect("shelved")
                .contains("ראשון"),
            "the premise: writing a note shelves it"
        );

        // Now edit it the way the design invites, with something that is not
        // this application. `vim` rewrites the file whole; the front matter and
        // the banner survive because they are in the file it opened.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let edited = std::fs::read_to_string(&md)
            .expect("vim reads")
            .replace("ראשון", "שני");
        std::fs::write(&md, &edited).expect("vim writes");

        let (reopened, trouble) = Notes::open(&dir);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert!(reopened.get("בדיקה").is_some(), "the note is still there");
        let on_the_shelf = std::fs::read_to_string(&shelved).expect("shelved");
        assert!(
            on_the_shelf.contains("שני"),
            "the shelf entry was not re-derived — the search box would still find \
             the old words: {on_the_shelf}"
        );
        assert!(!on_the_shelf.contains("ראשון"), "and not the old ones too");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
