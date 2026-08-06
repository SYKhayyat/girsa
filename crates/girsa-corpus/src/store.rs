//! Holding segments, and the two ways of anchoring to them.
//!
//! Both implementations are here on purpose. [`LineIndexStore`] is Otzaria's
//! scheme — file plus line number — and it is kept, tested, and shown to be
//! broken, so that the reason for [`SegmentStore`] is a runnable fact rather
//! than a paragraph in a design document somebody stops believing in eighteen
//! months from now.
//!
//! See `tests/anchors_survive_editing.rs` for the comparison.

use std::collections::BTreeMap;

use crate::segment::{Ordinal, SegmentId};

/// What every anchoring scheme has to be able to do.
///
/// The interesting method is [`Anchors::text_at`]. An anchor is only worth
/// anything if it keeps naming the same words after the text around it is
/// edited, and that is the one thing a line number cannot do.
pub trait Anchors {
    /// How this scheme names a segment.
    type Anchor: Clone + std::fmt::Debug;

    /// The anchor for the segment currently sitting at reading position `nth`.
    fn anchor_at_position(&self, nth: usize) -> Option<Self::Anchor>;

    /// The text an anchor names, following whatever indirection the scheme has.
    fn text_at(&self, anchor: &Self::Anchor) -> Option<String>;

    /// Split the segment at reading position `nth` into two, at `at` bytes in.
    fn split_at_position(&mut self, nth: usize, at: usize);

    /// Join the segment at reading position `nth` with the one after it.
    fn merge_at_position(&mut self, nth: usize);

    /// How many segments are readable now.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ---------------------------------------------------------------------------
// The broken one
// ---------------------------------------------------------------------------

/// Otzaria's scheme: a segment is a line, and a link is a line number.
///
/// **This is kept as a counter-example, not as an option.** Every method is
/// correct as written; the defect is in the design. Splitting line 3 moves line
/// 500 to line 501, and a link recorded as "line 500" now names different
/// words — with no error, no warning, and nothing in the data to notice it by.
///
/// Mishnah Berurah is 18,120 lines. One typo fix near the front silently
/// re-points every link below it.
#[derive(Debug, Clone, Default)]
pub struct LineIndexStore {
    lines: Vec<String>,
}

impl LineIndexStore {
    #[must_use]
    pub fn new(lines: Vec<String>) -> Self {
        Self { lines }
    }
}

impl Anchors for LineIndexStore {
    /// A line number. That is the whole problem.
    type Anchor = usize;

    fn anchor_at_position(&self, nth: usize) -> Option<usize> {
        (nth < self.lines.len()).then_some(nth)
    }

    fn text_at(&self, anchor: &usize) -> Option<String> {
        self.lines.get(*anchor).cloned()
    }

    fn split_at_position(&mut self, nth: usize, at: usize) {
        let Some(line) = self.lines.get(nth).cloned() else {
            return;
        };
        let (head, tail) = line.split_at(clamp_to_char_boundary(&line, at));
        self.lines
            .splice(nth..=nth, [head.to_string(), tail.to_string()]);
    }

    fn merge_at_position(&mut self, nth: usize) {
        if nth + 1 >= self.lines.len() {
            return;
        }
        let tail = self.lines.remove(nth + 1);
        if let Some(head) = self.lines.get_mut(nth) {
            head.push_str(&tail);
        }
    }

    fn len(&self) -> usize {
        self.lines.len()
    }
}

// ---------------------------------------------------------------------------
// The real one
// ---------------------------------------------------------------------------

/// Segments held by permanent ID, with a redirect table over the top.
///
/// The map is ordered by [`SegmentId`], which orders by ordinal, which is
/// reading order — so iterating gives the sefer, and a range gives a span.
#[derive(Debug, Clone)]
pub struct SegmentStore {
    work: String,
    live: BTreeMap<SegmentId, String>,
    /// An ID that is no longer live, and the live IDs it became.
    ///
    /// A split points at its children; a merge points at whichever segment
    /// absorbed it; an upstream re-segmentation points wherever the importer
    /// worked out the text went. One mechanism, because from an anchor's point
    /// of view they are the same event: *what I named is now over there*.
    redirects: BTreeMap<SegmentId, Vec<SegmentId>>,
}

/// A redirect chain longer than this is a cycle somebody built by hand.
/// Following it forever would hang the reader rather than show them a page.
const MAX_REDIRECT_DEPTH: usize = 32;

impl SegmentStore {
    /// Import: assign every segment an ordinal, once, in reading order.
    ///
    /// **This is a first import and nothing else.** Ordinals come out of
    /// enumeration position here, which is only right when there is no earlier
    /// promise to keep; the doc comment used to say *"it happens once in the
    /// life of a work"*, which was a claim about the world rather than about the
    /// code, and `girsa-import` refuted it on every invocation. What a re-import
    /// does is [`crate::import::continuity`]'s, and it is the caller's job to
    /// route through [`crate::import::read_over`] rather than this.
    #[must_use]
    pub fn import(work: impl Into<String>, segments: Vec<(Vec<String>, String)>) -> Self {
        let work = work.into();
        let live = segments
            .into_iter()
            .enumerate()
            .map(|(i, (path, text))| {
                #[allow(clippy::cast_possible_truncation)]
                let ordinal = Ordinal::root(i as u32 + 1);
                (SegmentId::new(work.clone(), path, ordinal), text)
            })
            .collect();
        Self {
            work,
            live,
            redirects: BTreeMap::new(),
        }
    }

    /// The work this store holds.
    #[must_use]
    pub fn work(&self) -> &str {
        &self.work
    }

    /// Every live segment, in reading order.
    pub fn iter(&self) -> impl Iterator<Item = (&SegmentId, &str)> {
        self.live.iter().map(|(id, text)| (id, text.as_str()))
    }

    /// The live segments an ID names now, in reading order.
    ///
    /// For an ID that was never touched, itself. For one that was split, its
    /// children. For one that was merged away, whatever absorbed it.
    #[must_use]
    pub fn resolve(&self, id: &SegmentId) -> Vec<SegmentId> {
        let mut out = Vec::new();
        self.resolve_into(id, 0, &mut out);
        out
    }

    fn resolve_into(&self, id: &SegmentId, depth: usize, out: &mut Vec<SegmentId>) {
        if depth > MAX_REDIRECT_DEPTH {
            return;
        }
        if self.live.contains_key(id) {
            if !out.contains(id) {
                out.push(id.clone());
            }
            return;
        }
        if let Some(targets) = self.redirects.get(id) {
            for target in targets {
                self.resolve_into(target, depth + 1, out);
            }
        }
    }

    /// Record where a segment went, for an upstream re-segmentation.
    ///
    /// Everything anchored to the old ID keeps working, which is the promise
    /// that lets a Ksav document written last year still open.
    ///
    /// The importer's own re-segmentation rows go through
    /// [`crate::import::continuity`] and reach disk as `redirects.jsonl`; this
    /// is the in-memory half of the same fact, and [`SegmentStore::from_disk`]
    /// is what makes the two one thing rather than two.
    pub fn redirect(&mut self, from: SegmentId, to: Vec<SegmentId>) {
        self.live.remove(&from);
        self.redirects.insert(from, to);
    }

    /// Every redirect this store holds, for writing down.
    ///
    /// Without this a store round-tripped through disk lost every row it had —
    /// which is what happened for the whole of W6 through W44, because the
    /// on-disk form had no slot for them.
    pub fn redirects(&self) -> impl Iterator<Item = (&SegmentId, &[SegmentId])> {
        self.redirects
            .iter()
            .map(|(from, to)| (from, to.as_slice()))
    }

    /// A store built from what an import wrote, redirects and all.
    ///
    /// The round trip `import::write` → `import::read_back` → here is lossless,
    /// and that is the property spec.md §3 rests on: a redirect that only lives
    /// in memory absorbs an upstream re-segmentation exactly until the process
    /// exits.
    #[must_use]
    pub fn from_disk(imported: &crate::import::ImportedWork) -> Self {
        Self {
            work: imported.work.slug.clone(),
            live: imported
                .segments
                .iter()
                .map(|s| (s.id.clone(), s.text.clone()))
                .collect(),
            redirects: imported
                .redirects
                .iter()
                .filter(|row| !row.to.is_empty())
                .map(|row| (row.from.clone(), row.to.clone()))
                .collect(),
        }
    }

    /// Split a segment in two at `at` bytes in.
    ///
    /// Mints `#n.1` and `#n.2`, redirects `#n` at them, and touches nothing
    /// else in the work. Returns the children.
    pub fn split(&mut self, id: &SegmentId, at: usize) -> Vec<SegmentId> {
        let Some(text) = self.live.remove(id) else {
            return Vec::new();
        };
        let at = clamp_to_char_boundary(&text, at);
        let (head, tail) = text.split_at(at);

        let children = id.split(2);
        self.live.insert(children[0].clone(), head.to_string());
        self.live.insert(children[1].clone(), tail.to_string());
        self.redirects.insert(id.clone(), children.clone());
        children
    }

    /// Join a segment with the one after it in reading order.
    ///
    /// The earlier ID stays live and carries the combined text; the later one
    /// redirects at it. Nothing after them moves.
    pub fn merge_with_next(&mut self, id: &SegmentId) -> Option<SegmentId> {
        let next = self
            .live
            .range(id.clone()..)
            .nth(1)
            .map(|(id, _)| id.clone())?;
        let tail = self.live.remove(&next)?;
        self.live.get_mut(id)?.push_str(&tail);
        self.redirects.insert(next, vec![id.clone()]);
        Some(id.clone())
    }
}

/// Nudge a byte offset back onto a character boundary.
///
/// Hebrew is two bytes a letter, so an offset arriving from anywhere that
/// counts bytes — a UI selection, a stored span, a half of a length — lands
/// mid-character about half the time, and `str::split_at` panics when it does.
/// A correction that takes the window down is not a correction.
fn clamp_to_char_boundary(s: &str, mut at: usize) -> usize {
    at = at.min(s.len());
    while at > 0 && !s.is_char_boundary(at) {
        at -= 1;
    }
    at
}

impl Anchors for SegmentStore {
    type Anchor = SegmentId;

    fn anchor_at_position(&self, nth: usize) -> Option<SegmentId> {
        self.live.keys().nth(nth).cloned()
    }

    /// The words the anchor names, wherever they ended up.
    fn text_at(&self, anchor: &SegmentId) -> Option<String> {
        let parts = self.resolve(anchor);
        if parts.is_empty() {
            return None;
        }
        Some(
            parts
                .iter()
                .filter_map(|id| self.live.get(id))
                .cloned()
                .collect(),
        )
    }

    fn split_at_position(&mut self, nth: usize, at: usize) {
        if let Some(id) = self.anchor_at_position(nth) {
            self.split(&id, at);
        }
    }

    fn merge_at_position(&mut self, nth: usize) {
        if let Some(id) = self.anchor_at_position(nth) {
            self.merge_with_next(&id);
        }
    }

    fn len(&self) -> usize {
        self.live.len()
    }
}
