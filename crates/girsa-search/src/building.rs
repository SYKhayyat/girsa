//! Indexing **one work** — and the reason that is a function.
//!
//! `girsa-index build` throws the index away and walks every root, which is
//! four minutes over 5,000,545 segments and exactly right for a rebuildable
//! cache of the corpus. It is absurd for a note you finished writing ten
//! seconds ago. A note is a sefer (spec.md §11) and a sefer you dropped on the
//! window is a sefer, and both of them were searchable *as of the last build* —
//! so the honest sentence in the results header was **"3 notes since the index
//! was built"**, and the only way to make it stop saying that was to spend four
//! minutes re-reading Shas.
//!
//! Nothing about tantivy required that. A work has been the unit of replacement
//! since W11: the first time a [`Writer`] is given a segment of some work it
//! deletes every segment of that work already in the index, because
//! `girsa-import` rewrites `segments.jsonl` wholesale and an append would have
//! doubled every hit. That rule is a full rebuild's safety net and it is also,
//! read the other way, an incremental update — one work in, the old copy out,
//! nothing else touched.
//!
//! # Why it had to become one function first
//!
//! The body was inside `girsa-index`'s build loop, and it is not three lines: a
//! page of a scan is indexed from the reading and not from the segment, the
//! reader's corrections are applied over a [`Standing`], the link-type masks are
//! read per work and **refused** when they were built against a different
//! segmentation, and the wordless count is asked of what actually went in. An
//! `absorb` that reimplemented any of that would be a second indexer, and the
//! failure would be silent — a note indexed one way and Shas the other, agreeing
//! until the day they did not. So the loop calls this, and so does [`absorb`],
//! and there is one description of what a work in the index looks like.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use girsa_corpus::import::{ImportedWork, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::standing::{redirected_here, Standing};

use crate::corrected::Corrections;
use crate::index::{IndexError, SearchIndex, Writer};

/// What the link-type cache had to say about this work.
///
/// Three states and not two, because the third is the one that would be silent:
/// masks written against a segmentation this work no longer has. Every mask
/// after an inserted se'if is about the line above it, and the facet column
/// would look exactly like a good one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Masks {
    /// Read, and used.
    Known,
    /// `girsa-link-types` has not run. The link facet says so.
    Unbuilt,
    /// Built for a different number of segments, and therefore refused.
    NotThisSegmentation { held: usize, wanted: usize },
}

/// What indexing one work did.
#[derive(Debug, Clone, Default)]
pub struct Done {
    pub segments: usize,
    pub headings: usize,
    /// Segments with no words at all once normalized — an empty heading, or a
    /// page nobody has OCR'd. Asked of **what went in**: for a corrected line
    /// that is the corrected words, because what is findable is what was
    /// indexed.
    pub wordless: usize,
    pub pages_read: usize,
    pub pages_unread: usize,
    pub scanned_words: usize,
    /// Segments indexed as the reader corrected them rather than as the corpus
    /// has them (W20).
    pub corrected: usize,
    /// Corrections whose words this work no longer has, so nothing was applied.
    pub stale_fixes: usize,
    pub masks: Option<Masks>,
    /// Lines a caller should print. A scan reading that would not parse costs
    /// one page and may not cost it silently, and this is a library — so what
    /// would have been an `eprintln!` is handed back to whoever has a screen.
    pub trouble: Vec<String>,
}

impl Done {
    fn masked(&self) -> bool {
        self.masks == Some(Masks::Known)
    }

    /// Whether the link facet can be believed for this work.
    #[must_use]
    pub fn has_link_types(&self) -> bool {
        self.masked()
    }
}

/// Index every segment of one work, replacing whatever was there.
///
/// `imported` is what [`girsa_corpus::import::read_back`] returned; it is a
/// parameter rather than read here because a full build already has it and
/// reading five million segments twice is not a rounding error.
///
/// # Errors
///
/// If tantivy will not take a document. Nothing is committed here — the caller
/// decides when, because a full build commits every quarter-million segments
/// and an update commits once.
pub fn one_work(
    writer: &mut Writer,
    root: &Path,
    imported: &ImportedWork,
    corrections: &Corrections,
) -> Result<Done, IndexError> {
    let slug = imported.work.slug.as_str();
    let mut done = Done::default();

    let ids: Vec<SegmentId> = imported.segments.iter().map(|s| s.id.clone()).collect();
    let by_segment = match girsa_link::touching::read(root, slug, &ids) {
        girsa_link::touching::Touching::Known(masks) => {
            done.masks = Some(Masks::Known);
            masks
        }
        girsa_link::touching::Touching::Unbuilt => {
            done.masks = Some(Masks::Unbuilt);
            vec![Default::default(); ids.len()]
        }
        girsa_link::touching::Touching::NotThisSegmentation { held, wanted } => {
            done.masks = Some(Masks::NotThisSegmentation { held, wanted });
            done.trouble.push(format!(
                "{slug}: link-type masks are for {held} segments and this work has {wanted} — \
                 not read. Run girsa-link-types."
            ));
            vec![Default::default(); ids.len()]
        }
    };

    // What somebody has read off the pages of this sefer, if it is a scan
    // (W26). Read once per work rather than once per page, and **corrections
    // applied**, because a reader who fixed a misread word and then cannot find
    // it has been given a correction that only corrects the display.
    let (words, trouble) = girsa_scan::Words::open(root, slug);
    done.trouble
        .extend(trouble.into_iter().map(|line| format!("{slug}: {line}")));
    let mut page = 0;

    // What you have corrected in this sefer, if anything (W20). Both halves of
    // the evidence for a `Standing` are gathered per work and only for a work a
    // correction actually touches, so a shelf of 7,189 works pays for this on
    // the handful somebody has edited: the live names, so a cut can be told
    // from an insertion, and `redirects.jsonl` backwards, so a name upstream
    // moved leads home.
    let fixes_here = corrections.touch(slug);
    let (live, back): (BTreeSet<SegmentId>, BTreeMap<SegmentId, Vec<SegmentId>>) = if fixes_here {
        (
            ids.iter().cloned().collect(),
            redirected_here(&imported.redirects),
        )
    } else {
        (BTreeSet::new(), BTreeMap::new())
    };

    for (at, segment) in imported.segments.iter().enumerate() {
        let kinds: Vec<girsa_link::EdgeType> =
            by_segment.get(at).copied().unwrap_or_default().kinds();
        // A page of a scan is counted through the pages, never read off the
        // segment's ordinal — splitting one mints `#47.1` and the arithmetic
        // would quietly slip by one from there
        // (`girsa_app::scanning::page_of_id`, and W6 underneath it).
        let read = if segment.kind == SegmentKind::Page {
            page += 1;
            words.page(page)
        } else {
            None
        };
        // A page of a scan is corrected on the photograph, by ink, and
        // `words.page` above has already applied those. The overlay here is the
        // other kind — a span of characters in a line — and the two never meet
        // on one segment.
        let corrected = if fixes_here && read.is_none() {
            let standing = Standing::derived(
                &segment.id,
                |name| live.contains(name),
                |name| back.get(name).cloned().unwrap_or_default(),
            );
            corrections.text(&standing, &segment.text)
        } else {
            None
        };
        if let Some(reading) = &corrected {
            if reading.applied > 0 {
                done.corrected += 1;
            }
            done.stale_fixes += reading.stale;
        }
        match (&read, &corrected) {
            (Some(read), _) => writer.add_page(segment, &kinds, read),
            (None, Some(reading)) => writer.add_saying(segment, &kinds, &reading.text),
            (None, None) => writer.add(segment, &kinds),
        }?;
        if let Some(read) = &read {
            done.pages_read += 1;
            done.scanned_words += read.words.len();
        }
        let indexed = corrected
            .as_ref()
            .map_or(segment.text.as_str(), |reading| reading.text.as_str());
        if read.is_none() && girsa_hebrew::normalize(indexed).is_empty() {
            done.wordless += 1;
        }
        if segment.kind == SegmentKind::Page && read.is_none() {
            done.pages_unread += 1;
        }
        if segment.kind == SegmentKind::Heading {
            done.headings += 1;
        }
    }
    done.segments = imported.segments.len();
    Ok(done)
}

/// Why one work could not be taken into an existing index.
#[derive(Debug, thiserror::Error)]
pub enum AbsorbError {
    #[error("{0}")]
    Index(#[from] IndexError),
    #[error("{0}")]
    Read(#[from] girsa_corpus::import::ImportError),
}

/// Take one work into an index that already exists, and commit.
///
/// The whole of *your writing is searchable without a rebuild*. A note is
/// typically one segment and a handout is a few dozen, so this is milliseconds
/// against four minutes — and it is the same [`one_work`] a full build runs, so
/// a note and Shas cannot be indexed by two different descriptions of what a
/// document is.
///
/// The index is **reloaded** before returning: a caller that writes a note and
/// then searches for it in the next breath is the whole point, and an
/// uncommitted or unreloaded index would answer *no* to a question it already
/// knows the answer to.
///
/// # What it writes down, and why that is not the index's stamp
///
/// A successful absorb appends the work to `girsa_note::since::ABSORBED_NAME`
/// beside the index. Without it the reader is told, in the same breath as the
/// hit they just got back, that what they wrote is not searchable yet — because
/// *what the index has not seen* was answered by comparing the note's file
/// against when the index was **built**, and this function does not build it.
///
/// Touching the build stamp instead would be one line and would be wrong: the
/// stamp answers for corrections too, and the corrections in the index would
/// still be the old words. The record is per work for that reason.
///
/// It is written **after** the commit and the reload, so a failure anywhere
/// above leaves the reader with an over-report, which is the state they can see
/// through, rather than a silence they cannot.
///
/// # Errors
///
/// If the work will not read back off disk, or tantivy will not take it.
pub fn absorb(
    index: &SearchIndex,
    root: &Path,
    slug: &str,
    corrections: &Corrections,
) -> Result<Done, AbsorbError> {
    let imported = girsa_corpus::import::read_back(root, slug)?;
    let mut writer = index.writer()?;
    let done = one_work(&mut writer, root, &imported, corrections)?;
    writer.commit()?;
    index.reload()?;
    // An index with no directory is one built in memory for a test; there is
    // nowhere to write the record and nothing that would read it.
    if let Some(dir) = index.path() {
        girsa_note::since::absorbed(dir, root, slug);
    }
    Ok(done)
}

/// Take a work **out** of an index, because it is not on the shelf any more.
///
/// The other half of [`absorb`], and it is not symmetrical with it: a work that
/// has been deleted has no `segments.jsonl` to read back, so there is nothing
/// to hand `one_work` and the delete-then-add rule never fires. A note you threw
/// away would otherwise stay findable until the next full build, which is the
/// same gap this module closes pointing the other way — and worse, because a
/// hit on it opens a sefer that is not there.
///
/// # Errors
///
/// If tantivy will not commit.
pub fn forget(index: &SearchIndex, slug: &str) -> Result<(), IndexError> {
    let mut writer = index.writer()?;
    writer.forget(slug);
    writer.commit()?;
    index.reload()?;
    // And the absorb record with it — see `absorb`. A record left behind would
    // outlive the thing it describes, and a note written again under the same
    // name would inherit a claim about a different note.
    if let Some(dir) = index.path() {
        girsa_note::since::forgotten(dir, slug);
    }
    Ok(())
}
