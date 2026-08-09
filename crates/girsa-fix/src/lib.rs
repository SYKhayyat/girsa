//! Corrections, as an overlay over text nobody is allowed to edit.
//!
//! spec.md §7, BUILDER.md W20. A correction is a **patch**: a permanent segment
//! id, a span of characters, what was printed there, what it should read, who
//! says so and when. The shipped corpus is never written to.
//!
//! # Why not simply fix the word
//!
//! Because four things stop working, and three of them stop silently:
//!
//! | | overlay | fixing the file |
//! |---|---|---|
//! | show as printed | yes | the printed words are gone |
//! | take it back | yes | you would have to remember them |
//! | survive `girsa-import` running again | yes | overwritten, without a word |
//! | hand your corrections to somebody | a file of lines | a 3 GB corpus |
//!
//! `tests/a_correction_is_not_an_edit.rs` runs both sides of that table.
//!
//! # A typo and a girsa variant are one mechanism
//!
//! spec.md §7.2: an OCR error (ד read as ר) and a hagahah are both *this span
//! should read differently, and here is who says so*. The difference is the
//! [`Kind`], and it is what the reader sees — a scanning error is a repair, and
//! an emendation is a claim. That is why [`Showing::Fixed`], the default,
//! applies the first and only **notes** the second: silently replacing the text
//! you are learning with somebody's emendation is a claim made on your behalf.
//!
//! # What anchors a patch
//!
//! The segment id, which is permanent (spec.md §3), plus the words as they were
//! when the correction was made. The span is an offset and offsets rot; the
//! words are what the correction is *about*, so when the two disagree the words
//! win — and only when they are there exactly once, or nothing is applied and
//! the patch is reported stale. A correction that lands on the wrong letters is
//! worse than one that does not land (BUILDER.md rule 6).

pub mod suspect;

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use girsa_personal::{fingerprint, now_seconds, Log};

use girsa_corpus::segment::SegmentId;
use girsa_corpus::standing::Standing;
use serde::{Deserialize, Serialize};

/// What a patch claims.
///
/// Spelled once — see [`girsa_corpus::spelled`]. This type used to carry
/// `as_str`, `named` **and** `#[serde(rename_all = "lowercase")]`, with
/// `as_str`'s own doc saying *"one implementation, so the word in the file, the
/// word on the button and the word the tests use cannot drift."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The scanner, or the typist, got a letter wrong. A repair: nobody thinks
    /// the sefer says this.
    Ocr,
    /// A textual variant — a hagahah, a Gra emendation, another edition. A
    /// claim about what the sefer should read, and it unifies with the `emends`
    /// edge type (spec.md §8.2): the same statement, made from the other end.
    Girsa,
}

girsa_corpus::spelled!(Kind {
    Ocr => "ocr",
    Girsa => "girsa",
});

/// Which corrections are applied to the text being read.
///
/// Three states rather than two, because "corrected" is two different questions
/// (see [`Kind`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Showing {
    /// The sefer as it is shipped. Corrections are still reported, so a reader
    /// can see there is one without it being applied.
    AsPrinted,
    /// Scanning errors repaired; variants noted, not applied. The default.
    #[default]
    Fixed,
    /// Everything applied, emendations included.
    FixedWithVariants,
}

girsa_corpus::spelled!(Showing {
    AsPrinted => "as_printed",
    Fixed => "fixed",
    FixedWithVariants => "fixed_with_variants",
});

impl Showing {
    /// Whether a patch of this kind is applied under this setting.
    #[must_use]
    pub const fn applies(self, kind: Kind) -> bool {
        match self {
            Self::AsPrinted => false,
            Self::Fixed => matches!(kind, Kind::Ocr),
            Self::FixedWithVariants => true,
        }
    }
}

/// What names a correction.
///
/// Content-addressed, over **what the correction claims** — the place, the
/// words, and which kind of claim it is. Not over who made it and not over
/// when: two people who fix the same typo have made one correction, and a patch
/// file taken twice must not apply itself twice (see [`Layer::merge`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PatchId(String);

impl fmt::Display for PatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PatchId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A name the window handed back. It names a correction or it names nothing —
/// there is no way to build one that means something else, because the only
/// thing done with it is a lookup.
impl From<String> for PatchId {
    fn from(name: String) -> Self {
        Self(name)
    }
}


/// One correction.
///
/// # Both the span and the words
///
/// Storing `was` beside the offsets looks redundant and is the whole
/// verification: an offset says *where* and the words say *what*, and when the
/// text underneath changes they stop agreeing. That disagreement is the signal
/// — see [`Layer::apply`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patch {
    pub id: PatchId,
    /// The permanent id of the segment this is a correction to (spec.md §3).
    /// **Never a line number** — see T1.
    pub segment: SegmentId,
    /// Where in the segment, in characters of the text as it stands on disk.
    /// Characters and not bytes: Hebrew is two bytes a letter and every offset
    /// that has ever crossed this project has been a character offset.
    pub from_char: usize,
    /// Exclusive.
    pub to_char: usize,
    /// The words the correction was made against.
    pub was: String,
    /// What they should read.
    pub now: String,
    pub kind: Kind,
    /// Who says so. A name, free text — this is a personal layer, not a
    /// registry.
    pub who: String,
    /// When, in seconds since the epoch.
    pub when: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The sefer that says it, for a variant — `girsa:hagahot-hagra/1:1`. Kept
    /// as text because it is a ref and refs travel as text (spec.md §4.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl Patch {
    /// A correction to `span` of `segment`, which reads `was` and should read
    /// `now`.
    #[must_use]
    pub fn new(
        segment: SegmentId,
        span: Range<usize>,
        was: impl Into<String>,
        now: impl Into<String>,
        kind: Kind,
        who: impl Into<String>,
    ) -> Self {
        let (was, now) = (was.into(), now.into());
        let mut patch = Self {
            id: PatchId(String::new()),
            segment,
            from_char: span.start,
            to_char: span.end,
            was,
            now,
            kind,
            who: who.into(),
            when: now_seconds(),
            note: None,
            source: None,
        };
        patch.id = patch.name();
        patch
    }

    /// What this correction claims, hashed.
    fn name(&self) -> PatchId {
        PatchId(fingerprint(&[
            &self.segment.to_string(),
            &self.from_char.to_string(),
            &self.to_char.to_string(),
            &self.was,
            &self.now,
            self.kind.as_str(),
            self.source.as_deref().unwrap_or(""),
        ]))
    }

    /// Say which sefer this variant comes from. Re-names the patch, because the
    /// source is part of what it claims.
    #[must_use]
    pub fn from_source(mut self, reference: impl Into<String>) -> Self {
        self.source = Some(reference.into());
        self.id = self.name();
        self
    }

    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Fixed at this moment, rather than now. For a test that wants two patches
    /// it can tell apart, and for reading a patch file somebody else wrote.
    #[must_use]
    pub const fn made_at(mut self, when: u64) -> Self {
        self.when = when;
        self
    }

    #[must_use]
    pub const fn span(&self) -> Range<usize> {
        self.from_char..self.to_char
    }
}


/// Why a correction was not taken.
#[derive(Debug, thiserror::Error)]
pub enum FixError {
    #[error("writing {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("a correction has to change something")]
    Changes,
    #[error("{0} is not a span of anything")]
    NotASpan(String),
    /// Two corrections claiming the same letters. Refused where the reader is,
    /// which is the only moment there is anybody to tell.
    #[error("there is already a correction on those words: {0}")]
    Clash(String),
    #[error("reading {path}: {source}")]
    Unreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Your corrections, all of them.
///
/// One file, one line each, under your own layer — `personal/corrections.jsonl`.
/// Greppable and diffable for the same reason the corpus is (spec.md §4.1), and
/// because handing it to somebody else is a feature rather than an export.
///
/// # One line written per correction
///
/// The file is a [`Log`]: a correction is appended, taking one back appends a
/// tombstone, and the file is rewritten only when it has grown past twice what
/// it holds. It used to be serialized in full on every single mutation, which
/// made the reading pane's three-second budget (spec.md §7.5) a function of how
/// many typos you had ever fixed — see the crate docs on `girsa-personal`.
#[derive(Debug, Clone)]
pub struct Layer {
    log: Log,
    by_segment: BTreeMap<SegmentId, Vec<Patch>>,
}

girsa_personal::io_from_log_error!(FixError);

/// The replay, the index and the compaction — `girsa_personal::Store`.
///
/// Six stores in this repository grew this same arrangement. See the module
/// note on `girsa-personal/src/store.rs`.
impl girsa_personal::Store for Layer {
    type Record = Patch;
    const WHAT: &'static str = "a correction";

    fn key_of(patch: &Patch) -> String {
        key_of(patch)
    }
    fn log(&self) -> &Log {
        &self.log
    }
    fn hold(&mut self, patch: Patch) {
        self.hold_patch(patch);
    }
    fn count(&self) -> usize {
        Layer::count(self)
    }
    fn records(&self) -> Vec<&Patch> {
        self.all().collect()
    }
}

/// What a merge did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Merged {
    pub taken: usize,
    /// Corrections that were already here. Counted rather than ignored: taking
    /// the same patch file twice has to be visibly a no-op.
    pub already_had: usize,
    pub refused: usize,
}

/// Where the corrections live under a personal layer.
///
/// The name comes from `girsa_personal::CORRECTIONS` rather than from a literal
/// here, because it was a literal here **and** in `girsa_note::since`, which
/// counts what is newer than the index. The wall between the two sibling crates
/// is right; a wall does not stop them needing the same string, it only stops
/// them sharing one. A rename would have left `since` counting a file nobody
/// writes, reporting zero, and telling a reader their index was up to date —
/// which is the exact reading that mechanism exists to prevent.
#[must_use]
pub fn path_in(personal: &Path) -> PathBuf {
    personal.join(girsa_personal::CORRECTIONS)
}

/// What names a correction in the file.
///
/// The patch id, which is a fingerprint of what the correction claims — so the
/// same correction made twice is one line, and a tombstone naming it is
/// unambiguous.
fn key_of(patch: &Patch) -> String {
    patch.id.to_string()
}

impl Layer {
    /// Read your corrections.
    ///
    /// A line that will not parse costs that correction and is reported —
    /// never the whole file. The alternative is one bad line silently
    /// un-correcting a library.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        girsa_personal::open(Self {
            log: Log::at(path_in(personal)),
            by_segment: BTreeMap::new(),
        })
    }

    /// A layer that is never written, for a caller that only wants to apply
    /// patches it already has.
    #[must_use]
    pub fn nowhere() -> Self {
        Self {
            log: Log::nowhere(),
            by_segment: BTreeMap::new(),
        }
    }

    fn hold_patch(&mut self, patch: Patch) {
        let held = self.by_segment.entry(patch.segment.clone()).or_default();
        if held.iter().any(|p| p.id == patch.id) {
            return;
        }
        held.push(patch);
        held.sort_by_key(|p| p.from_char);
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.log.path()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.by_segment.values().map(Vec::len).sum()
    }

    /// The corrections stored under **exactly** this id, in reading order.
    ///
    /// For writing, and for a caller that already knows the name it wants.
    /// **A reader wants [`Layer::at`]** — a correction written before the corpus
    /// cut its segment in two is stored under a name that is no longer a
    /// segment, and this will not find it.
    #[must_use]
    pub fn on(&self, segment: &SegmentId) -> &[Patch] {
        self.by_segment.get(segment).map_or(&[], Vec::as_slice)
    }

    /// The corrections on a place, **under every name it has carried**.
    ///
    /// # The one this had and the other three did not
    ///
    /// Four things in this repository ask *does something anchored back then
    /// belong to this line now*. `girsa_note::Marks::on` and
    /// `Collection::holds` ask `Standing::named_by`; `girsa_app::links` builds a
    /// `Landing` out of a `Standing`. This layer asked `by_segment.get(id)` —
    /// **exact equality** — so a correction made before the corpus split its
    /// se'if simply stopped applying. Not reported as stale, either: `apply`
    /// reports a patch whose letters it cannot find, and this one was never
    /// looked up, so there was nothing to report.
    ///
    /// `Standing` is the answer the rest of the codebase already settled on,
    /// and it covers both halves — a name carved into children, and a name
    /// upstream moved (`redirects.jsonl`).
    #[must_use]
    pub fn at(&self, at: &Standing) -> Vec<&Patch> {
        let mut found: Vec<&Patch> = at
            .names()
            .filter_map(|name| self.by_segment.get(name))
            .flatten()
            .collect();
        found.sort_by_key(|p| p.from_char);
        found
    }

    /// The places in one work that have corrections, **by the name they were
    /// stored under** — which is not always a name the work still has.
    ///
    /// The one question a reader of this layer can ask cheaply about whether
    /// re-anchoring is needed at all: if every name here is still a segment,
    /// nothing has been re-segmented under these corrections and a place
    /// answers to exactly its own name.
    pub fn names_in<'a>(&'a self, slug: &'a str) -> impl Iterator<Item = &'a SegmentId> {
        self.by_segment.keys().filter(move |id| id.work() == slug)
    }

    /// Every correction, oldest segment first.
    pub fn all(&self) -> impl Iterator<Item = &Patch> {
        self.by_segment.values().flatten()
    }

    /// The corrections in one sefer.
    pub fn in_work<'a>(&'a self, slug: &'a str) -> impl Iterator<Item = &'a Patch> {
        self.all().filter(move |p| p.segment.work() == slug)
    }

    /// Whether a sefer has any corrections at all — the cheap question the
    /// reading pane asks before it does anything else.
    #[must_use]
    pub fn touches(&self, slug: &str) -> bool {
        self.in_work(slug).next().is_some()
    }

    /// Take a correction, and write it down.
    ///
    /// # Errors
    ///
    /// If it changes nothing, is not a span, claims letters another correction
    /// already claims, or the file will not write. **A correction that will not
    /// save is not applied in memory either** — the same rule the shelf's
    /// arrangement follows, and for the same reason: a fix that quietly
    /// disappears at the next restart is worse than one that says it could not
    /// be taken.
    pub fn add(&mut self, patch: Patch) -> Result<&Patch, FixError> {
        if patch.from_char >= patch.to_char {
            return Err(FixError::NotASpan(format!(
                "{}..{}",
                patch.from_char, patch.to_char
            )));
        }
        if patch.was == patch.now {
            return Err(FixError::Changes);
        }
        let already = self.on(&patch.segment).iter().any(|p| p.id == patch.id);
        if !already {
            if let Some(clash) = self
                .on(&patch.segment)
                .iter()
                .find(|p| overlaps(&p.span(), &patch.span()))
            {
                return Err(FixError::Clash(clash.was.clone()));
            }
        }

        let id = patch.id.clone();
        let segment = patch.segment.clone();
        // Written down before it is held, so what is in memory and what is on
        // disk are the same corrections — and with one line appended rather
        // than the whole layer serialized, there is nothing to put back if it
        // fails.
        if !already {
            self.log.append(&patch)?;
        }
        self.hold_patch(patch);
        self.on(&segment)
            .iter()
            .find(|p| p.id == id)
            .ok_or(FixError::Changes)
    }

    fn forget(&mut self, segment: &SegmentId, id: &PatchId) -> bool {
        let Some(held) = self.by_segment.get_mut(segment) else {
            return false;
        };
        let before = held.len();
        held.retain(|p| p.id != *id);
        if held.is_empty() {
            self.by_segment.remove(segment);
        }
        before != self.by_segment.get(segment).map_or(0, Vec::len)
    }

    /// Take a correction back. `false` if there was no such correction.
    ///
    /// # Errors
    ///
    /// If the file will not write.
    pub fn remove(&mut self, id: &PatchId) -> Result<bool, FixError> {
        let Some(segment) = self.all().find(|p| p.id == *id).map(|p| p.segment.clone()) else {
            return Ok(false);
        };
        self.log.took(&[id.as_str()])?;
        let gone = self.forget(&segment, id);
        Ok(gone)
    }

    /// Take somebody else's corrections (spec.md §7.1).
    ///
    /// Idempotent: a patch is named by what it claims, so the same file taken
    /// twice is the same corrections.
    ///
    /// Their file is a log too, so it is replayed rather than read line by line
    /// — a correction they made and took back is not one they are offering, and
    /// a correction they changed their mind about twice is still one
    /// correction. Their tombstones stop at their own file: what is being taken
    /// is what they hold, never a deletion of what I hold.
    ///
    /// # Errors
    ///
    /// If the file cannot be read, or ours cannot be written afterwards.
    pub fn merge(&mut self, file: &Path) -> Result<Merged, FixError> {
        let body = std::fs::read_to_string(file).map_err(|source| FixError::Unreadable {
            path: file.display().to_string(),
            source,
        })?;
        let theirs = girsa_personal::replay::<Patch>(
            &body,
            &file.display().to_string(),
            "a correction",
            key_of,
        );
        let mut merged = Merged {
            refused: theirs.trouble.len(),
            ..Merged::default()
        };
        let mut taking: Vec<Patch> = Vec::new();
        for patch in theirs.records {
            let held = self.on(&patch.segment);
            if held.iter().any(|p| p.id == patch.id) {
                merged.already_had += 1;
                continue;
            }
            // Against what I hold *and* against what this same file has already
            // offered: two of their corrections claiming the same letters is
            // the same refusal as one of theirs against one of mine, and it
            // used to be caught only because each was held before the next was
            // read.
            let clashes_within = taking
                .iter()
                .any(|p| p.segment == patch.segment && overlaps(&p.span(), &patch.span()));
            if clashes_within || held.iter().any(|p| overlaps(&p.span(), &patch.span())) {
                // Their correction and mine claim the same letters. Refused,
                // and counted — a merge that quietly dropped one would be the
                // system choosing between two people's readings.
                merged.refused += 1;
                continue;
            }
            taking.push(patch);
        }
        // One append for the whole file, and held only once it is down.
        self.log.append_all(taking.iter())?;
        for patch in taking {
            self.hold_patch(patch);
            merged.taken += 1;
        }
        Ok(merged)
    }

    /// One segment's text, corrected.
    ///
    /// Every patch is re-anchored against the words it was made from before it
    /// is applied, so a corpus update that moved the line does not move the
    /// correction onto different letters. See [`Corrected`].
    #[must_use]
    pub fn apply(&self, segment: &SegmentId, base: &str, showing: Showing) -> Corrected {
        self.applying(self.on(segment).iter().collect(), base, showing)
    }

    /// The same, for a place under every name it has carried — see [`Layer::at`].
    ///
    /// This is what the reading pane wants. `apply` above finds corrections
    /// stored under exactly the id it is handed, which is right for the write
    /// path and silently wrong the day upstream re-segments a work.
    #[must_use]
    pub fn apply_at(&self, at: &Standing, base: &str, showing: Showing) -> Corrected {
        self.applying(self.at(at), base, showing)
    }

    fn applying(&self, patches: Vec<&Patch>, base: &str, showing: Showing) -> Corrected {
        if patches.is_empty() {
            return Corrected::unchanged(base);
        }
        let letters: Vec<char> = base.chars().collect();

        // Where each patch actually lands now, and whether it had to move.
        let mut resolved: Vec<(Range<usize>, &Patch, bool)> = Vec::new();
        let mut stale = Vec::new();
        for patch in patches {
            match anchor(&letters, patch) {
                Some((span, moved)) => resolved.push((span, patch, moved)),
                None => stale.push(patch.id.clone()),
            }
        }
        resolved.sort_by_key(|(span, _, _)| span.start);

        let mut out = String::with_capacity(base.len());
        let mut applied = Vec::new();
        let mut noted = Vec::new();
        let mut moved = Vec::new();
        let mut at = 0usize; // in characters of `letters`
        let mut wrote = 0usize; // in characters of `out`
        let mut last_end = 0usize;
        for (span, patch, has_moved) in resolved {
            if span.start < last_end {
                // Re-anchoring put two corrections on top of each other. Take
                // the first and report the second, rather than interleaving
                // two replacements into one stretch of text.
                stale.push(patch.id.clone());
                continue;
            }
            let take = showing.applies(patch.kind);
            let head: String = letters
                .get(at..span.start)
                .unwrap_or_default()
                .iter()
                .collect();
            wrote += head.chars().count();
            out.push_str(&head);
            at = span.start;

            let mark = Applied {
                id: patch.id.clone(),
                kind: patch.kind,
                was: patch.was.clone(),
                now: patch.now.clone(),
                who: patch.who.clone(),
                source: patch.source.clone(),
                note: patch.note.clone(),
                from_char: wrote,
                to_char: wrote
                    + if take {
                        patch.now.chars().count()
                    } else {
                        span.len()
                    },
            };
            if take {
                out.push_str(&patch.now);
                wrote += patch.now.chars().count();
                at = span.end;
                applied.push(mark);
                if has_moved {
                    moved.push(patch.id.clone());
                }
            } else {
                noted.push(mark);
            }
            last_end = span.end;
        }
        out.extend(letters.get(at..).unwrap_or_default().iter());

        Corrected {
            text: out,
            applied,
            noted,
            moved,
            stale,
        }
    }
}

/// Where a patch lands in the text as it stands now, and whether it had to be
/// re-found.
///
/// The rule is [`girsa_corpus::span::locate`]'s, and it is there rather than
/// here because W27's highlights need the same one: an offset says *where* and
/// the words say *what*, and when they disagree the words win — only if they
/// are there exactly once.
fn anchor(letters: &[char], patch: &Patch) -> Option<(Range<usize>, bool)> {
    girsa_corpus::span::locate(letters, patch.span(), &patch.was)
        .map(|found| (found.span, found.moved))
}

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// One segment as the reader sees it, and what was done to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Corrected {
    pub text: String,
    /// The corrections applied, in reading order. Their spans are into
    /// [`Corrected::text`].
    pub applied: Vec<Applied>,
    /// Corrections that are here and were **not** applied — a variant under
    /// [`Showing::Fixed`], or everything under [`Showing::AsPrinted`]. Their
    /// spans are into the text as it stands, so the window can mark the words a
    /// correction is about without changing them.
    pub noted: Vec<Applied>,
    /// Corrections whose words had moved and were re-found.
    pub moved: Vec<PatchId>,
    /// Corrections whose words are no longer there, or are there twice. Never
    /// applied, and never silently dropped either: the reader is the only one
    /// who can say what happened to them.
    pub stale: Vec<PatchId>,
}

impl Corrected {
    #[must_use]
    fn unchanged(base: &str) -> Self {
        Self {
            text: base.to_string(),
            applied: Vec::new(),
            noted: Vec::new(),
            moved: Vec::new(),
            stale: Vec::new(),
        }
    }

    /// Whether anything at all happened here.
    #[must_use]
    pub fn is_untouched(&self) -> bool {
        self.applied.is_empty() && self.noted.is_empty() && self.stale.is_empty()
    }

    /// The correction a stretch of the corrected text runs across, if it runs
    /// across one.
    #[must_use]
    pub fn covering(&self, from_char: usize, to_char: usize) -> Option<&Applied> {
        self.applied
            .iter()
            .find(|a| a.from_char < to_char && from_char < a.to_char)
    }

    /// The span of the segment **as it stands on disk** that a span of the
    /// corrected text names.
    ///
    /// This is what makes a second correction possible on a segment that
    /// already has one: the reader is looking at corrected words and a patch
    /// has to name the file. Every correction before the place shifts it by
    /// however much longer or shorter it made the line.
    ///
    /// `None` when the span runs across a correction that is already there —
    /// those words are not in the file, so there is nothing to name. The reader
    /// is told to take the first correction back, which is the only answer that
    /// is not this system inventing a base text.
    #[must_use]
    pub fn base_span(&self, from_char: usize, to_char: usize) -> Option<Range<usize>> {
        if from_char >= to_char || self.covering(from_char, to_char).is_some() {
            return None;
        }
        let mut shift: isize = 0;
        for mark in &self.applied {
            if mark.to_char > from_char {
                break;
            }
            let grew = isize::try_from(mark.now.chars().count()).ok()?;
            let shrank = isize::try_from(mark.was.chars().count()).ok()?;
            shift += grew - shrank;
        }
        let back = |at: usize| usize::try_from(isize::try_from(at).ok()? - shift).ok();
        Some(back(from_char)?..back(to_char)?)
    }
}

/// One correction, placed in the text that came back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    pub id: PatchId,
    pub kind: Kind,
    pub was: String,
    pub now: String,
    pub who: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub from_char: usize,
    pub to_char: usize,
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn id() -> SegmentId {
        "girsa:mishnah-berurah/1:1#7".parse().expect("an id")
    }

    fn patch(span: Range<usize>, was: &str, now: &str) -> Patch {
        Patch::new(id(), span, was, now, Kind::Ocr, "me")
    }

    #[test]
    fn a_patch_is_named_by_what_it_claims_and_not_by_who_said_it() {
        // So that two people who fix the same typo have made one correction,
        // and a patch file taken twice applies once.
        let mine = Patch::new(id(), 0..3, "אבג", "אבד", Kind::Ocr, "me").made_at(1);
        let yours = Patch::new(id(), 0..3, "אבג", "אבד", Kind::Ocr, "you").made_at(2);
        assert_eq!(mine.id, yours.id);

        // A different claim about the same words is a different correction.
        let variant = Patch::new(id(), 0..3, "אבג", "אבד", Kind::Girsa, "me");
        assert_ne!(mine.id, variant.id);
        let elsewhere = Patch::new(id(), 4..7, "אבג", "אבד", Kind::Ocr, "me");
        assert_ne!(mine.id, elsewhere.id);
    }

    #[test]
    fn a_correction_that_changes_nothing_is_refused() {
        let mut layer = Layer::nowhere();
        assert!(layer.add(patch(0..3, "אבג", "אבג")).is_err());
        assert!(layer.add(patch(3..3, "", "אבג")).is_err());
    }

    #[test]
    fn the_span_is_counted_in_letters_and_not_in_bytes() {
        // Hebrew is two bytes a letter. A byte-counted span on this line lands
        // between the halves of a letter, and the correction comes out as
        // mojibake — or, on `split_at`, as a panic in the reader's window.
        let mut layer = Layer::nowhere();
        layer.add(patch(3..7, "הרבר", "הדבר")).expect("takes it");
        let corrected = layer.apply(&id(), "כל הרבר הזה", Showing::Fixed);
        assert_eq!(corrected.text, "כל הדבר הזה");
        assert_eq!(corrected.applied[0].from_char, 3);
        assert_eq!(corrected.applied[0].to_char, 7);
    }

    #[test]
    fn as_printed_reports_the_corrections_without_applying_them() {
        let mut layer = Layer::nowhere();
        layer.add(patch(3..7, "הרבר", "הדבר")).expect("takes it");
        let printed = layer.apply(&id(), "כל הרבר הזה", Showing::AsPrinted);
        assert_eq!(printed.text, "כל הרבר הזה");
        assert!(printed.applied.is_empty());
        assert_eq!(printed.noted.len(), 1);
        assert_eq!(
            (printed.noted[0].from_char, printed.noted[0].to_char),
            (3, 7)
        );
    }

    #[test]
    fn a_replacement_of_a_different_length_does_not_move_the_next_correction() {
        let mut layer = Layer::nowhere();
        layer.add(patch(0..2, "אב", "אבבב")).expect("takes it");
        layer.add(patch(4..6, "דה", "ד")).expect("takes it");
        let corrected = layer.apply(&id(), "אב גדה ו", Showing::Fixed);
        assert_eq!(corrected.text, "אבבב גד ו");
        // The second mark is placed in the text that came back, not in the one
        // it was made against — the window draws the first of those. It starts
        // at 6 and it was made at 4, because the correction before it grew.
        assert_eq!(corrected.applied[1].from_char, 6);
        assert_eq!(corrected.applied[1].to_char, 7);
    }

    #[test]
    fn a_second_correction_on_a_corrected_line_names_the_file_and_not_the_screen() {
        // The reader is looking at words that are not in the sefer, because
        // the first correction put them there. A patch made from that screen
        // has to be expressed against the file, or it lands wherever the first
        // correction's length difference put it.
        let mut layer = Layer::nowhere();
        layer.add(patch(0..2, "אב", "אבבב")).expect("takes it");
        let corrected = layer.apply(&id(), "אב גדה", Showing::Fixed);
        assert_eq!(corrected.text, "אבבב גדה");

        // `גדה` is at 5..8 on the screen and at 3..6 in the file.
        assert_eq!(corrected.base_span(5, 8), Some(3..6));
        // And a selection over the correction itself has no answer: those
        // letters are not in the file at all.
        assert_eq!(corrected.base_span(0, 4), None);
        assert_eq!(corrected.base_span(2, 6), None);
        assert!(corrected.covering(1, 2).is_some());
    }

    #[test]
    fn a_segment_with_no_corrections_costs_nothing_and_says_so() {
        let layer = Layer::nowhere();
        let corrected = layer.apply(&id(), "כל הרבר הזה", Showing::Fixed);
        assert!(corrected.is_untouched());
        assert_eq!(corrected.text, "כל הרבר הזה");
    }
}
