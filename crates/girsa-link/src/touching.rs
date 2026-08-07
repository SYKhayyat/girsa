//! Which kinds of link touch each segment — the graph read from the segment's
//! side (BUILDER.md W14, spec.md §9.8).
//!
//! §9.8 asks results to carry a **link type** facet: *of these 300 hits, 120 are
//! in segments something comments on*. That question is asked of a segment and
//! the graph is not stored that way. spec.md §8.2: an edge is *directed, inverse
//! derived and never stored twice*, and W8 stored each one in the shard of the
//! work it points **from**. So Berakhot's own shard holds the handful of edges
//! Berakhot makes, and the two million edges that land **on** Berakhot are
//! scattered across every shard in the corpus.
//!
//! Deriving the inverse per query would mean reading all 665 MB of the graph to
//! draw one facet row. So it is derived **once** and written beside the edges,
//! as one 16-bit mask per segment in reading order:
//!
//! ```text
//! corpus/links/bavli/berakhot/touching.bits
//! ```
//!
//! ## Nine bits, and it used to be a nine-bit answer written as prose
//!
//! This file was `touching.jsonl` until 6 August 2026 — one JSON row per
//! `(endpoint, type)`, plus a list of every sefer at the other end:
//!
//! ```jsonl
//! {"a":"girsa:bavli/berakhot/2a:1#1","t":"comments-on","w":["bavli/rashi-on-berakhot", …]}
//! ```
//!
//! **449 MB across 6,268 files**, and its only consumer destructured it as
//! `(anchor, edge_type, _)` — throwing the `w` list away — to produce a
//! `Vec<BTreeSet<EdgeType>>` that is consumed once, at index-build time, and is
//! **nine bits per segment**. Shulchan Arukh, Orach Chayim is 4,171 se'ifim:
//! 4.14 MB of rows to say 4,171 numbers. It is now 8.4 KB, which is the
//! numbers.
//!
//! ## Where the `w` field went, and why that is not a loss
//!
//! `w` was W31, filed from OtzariaSonim — a keypad-phone reader with a 192 MB
//! heap — because the question a reader actually asks is not *does this segment
//! have a `comments-on`* but *does it have one from the mefarshim I ticked*, and
//! answering that meant reading `inbound.jsonl`: 27.3 MB and 156,076 rows for
//! Orach Chayim. That was true when W31 was written and it is not true now.
//! W28's landing index sorts `inbound.jsonl` by where its rows land and writes
//! `inbound.idx` beside it, so *which works comment on this place* is a seek and
//! a few kilobytes — **4,171 places, not 159,273 rows**. The 12× file that
//! existed to avoid a read that no longer happens is the wrong side of that
//! trade.
//!
//! Girsa's own answer to the same question never used `w` at all:
//! `girsa_app::mefarshim` reads `inbound.jsonl`, and its module note records
//! what the phone was doing all along — *"a bitmap per commentator, one bit per
//! line"*. Building an external reader's cache is defensible. Building it twelve
//! times larger than the shape that reader already uses is not.
//!
//! ## The fingerprint, and why a stale mask must not be read
//!
//! A mask is **positional**, and that is a real hazard this file did not have
//! when it was keyed by anchor: a stale anchor file is merely incomplete, and a
//! stale mask lights up the wrong lines. So the header carries the number of
//! segments and a fingerprint of the ids it was built against, and
//! [`read`] refuses a file that does not match rather than returning it.
//! `girsa-index` reports that refusal in its build report, the same way it
//! reports the file being absent.
//!
//! This is `girsa_lane::vectors`' rule, which is the best paranoia in this
//! repository: *the same model at a different width is also another model.* Two
//! segmentations produce masks where the arithmetic runs happily and the facet
//! column looks exactly like a good one.
//!
//! # It is a cache, and it may be missing
//!
//! spec.md §4.1: the files are the truth and anything faster is rebuildable.
//! Delete this and run `girsa-link-types` again. What must never happen is the
//! facet quietly reading a **zero** off an index built without it — *no links of
//! that kind* and *nobody worked out the link types* are different statements,
//! and the index records which one it is (see
//! `girsa_search::index::BuildReport`).

use std::fs;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use girsa_corpus::import::slug_dir;
use girsa_corpus::segment::SegmentId;

use crate::{Anchor, EdgeType};

/// The bytes every mask file begins with. The trailing digit is the format:
/// change the layout, change the digit, and every older file is refused by the
/// same code path that refuses a stale one.
const MAGIC: &[u8; 16] = b"girsa-touching-1";

/// Where a work's per-segment link-type masks live.
#[must_use]
pub fn bits_path(root: &Path, slug: &str) -> PathBuf {
    slug_dir(&root.join("links"), slug).join("touching.bits")
}

/// The file this replaced, so a run can delete what it superseded.
#[must_use]
pub fn superseded_path(root: &Path, slug: &str) -> PathBuf {
    slug_dir(&root.join("links"), slug).join("touching.jsonl")
}

/// Every kind of link touching one segment, in sixteen bits.
///
/// Nine kinds and a `u16`, so a tenth takes a bit no file has ever set rather
/// than a format change. Bit order is [`EdgeType::ALL`] and is the wire format.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mask(u16);

impl Mask {
    /// Nothing touches this segment — which is a statement, and different from
    /// nobody having worked the masks out. That second one is [`Touching`].
    pub const NONE: Self = Self(0);

    #[must_use]
    pub const fn contains(self, kind: EdgeType) -> bool {
        self.0 & kind.bit() != 0
    }

    pub const fn insert(&mut self, kind: EdgeType) {
        self.0 |= kind.bit();
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The kinds set, in [`EdgeType::ALL`] order.
    #[must_use]
    pub fn kinds(self) -> Vec<EdgeType> {
        EdgeType::ALL
            .into_iter()
            .filter(|kind| self.contains(*kind))
            .collect()
    }

    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }
}

impl FromIterator<EdgeType> for Mask {
    fn from_iter<I: IntoIterator<Item = EdgeType>>(kinds: I) -> Self {
        let mut mask = Self::NONE;
        for kind in kinds {
            mask.insert(kind);
        }
        mask
    }
}

/// What is on disk for one work.
///
/// Three answers and not two, for the reason spec.md §4.1 gives: a caller that
/// cannot tell *nothing comments here* from *nobody has worked it out* will
/// eventually print the first when it means the second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Touching {
    /// No file. `girsa-link-types` has not run for this work.
    Unbuilt,
    /// A file built against a different segmentation, and therefore not read.
    ///
    /// Positional data outlives the positions it was written for. Both counts
    /// are carried so the report can say which way it drifted.
    NotThisSegmentation { held: usize, wanted: usize },
    /// One mask per segment, in reading order.
    Known(Vec<Mask>),
}

// Deliberately no `or_default()` here. Both callers `match` on all three
// variants and each does something different with the two that are not
// `Known` — one prints the work and the command to fix it, the other panics
// because it built the file a moment earlier. A convenience that collapsed the
// three into "empty" would be the exact silent zero this enum exists to stop.

/// A fingerprint of a work's segment ids, in reading order.
///
/// FNV-1a over the ids as they are written, with the count folded in. Not a
/// cryptographic hash and not trying to be: it is here to catch a re-import,
/// not an adversary.
#[must_use]
pub fn fingerprint(ordered: &[SegmentId]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x1000_0000_01b3;
    let mut hash = OFFSET;
    let mut byte = |b: u8| {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(PRIME);
    };
    for id in ordered {
        for b in id.to_string().as_bytes() {
            byte(*b);
        }
        byte(b'\n');
    }
    for b in (ordered.len() as u64).to_le_bytes() {
        byte(b);
    }
    hash
}

/// Write one work's masks.
///
/// # Errors
///
/// If the directory cannot be made or the file cannot be written.
pub fn write(
    root: &Path,
    slug: &str,
    ordered: &[SegmentId],
    masks: &[Mask],
) -> std::io::Result<()> {
    let path = bits_path(root, slug);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut body = Vec::with_capacity(MAGIC.len() + 12 + masks.len() * 2);
    body.extend_from_slice(MAGIC);
    body.extend_from_slice(&(masks.len() as u32).to_le_bytes());
    body.extend_from_slice(&fingerprint(ordered).to_le_bytes());
    for mask in masks {
        body.extend_from_slice(&mask.0.to_le_bytes());
    }
    fs::write(&path, body)
}

/// Read one work's masks, against the segments they are about to be used with.
///
/// `ordered` is not a convenience — it is the check. A mask file names a
/// segmentation and this is where the two are made to agree.
#[must_use]
pub fn read(root: &Path, slug: &str, ordered: &[SegmentId]) -> Touching {
    let path = bits_path(root, slug);
    let Ok(body) = fs::read(&path) else {
        return Touching::Unbuilt;
    };
    let head = MAGIC.len() + 12;
    if body.len() < head || &body[..MAGIC.len()] != MAGIC {
        // A file of another format is not a file about these segments. Same
        // answer as a stale one, because the caller's move is the same: say so,
        // and rebuild.
        return Touching::NotThisSegmentation {
            held: 0,
            wanted: ordered.len(),
        };
    }
    let mut four = [0u8; 4];
    four.copy_from_slice(&body[MAGIC.len()..MAGIC.len() + 4]);
    let held = u32::from_le_bytes(four) as usize;
    let mut eight = [0u8; 8];
    eight.copy_from_slice(&body[MAGIC.len() + 4..head]);
    let stamped = u64::from_le_bytes(eight);

    if held != ordered.len() || stamped != fingerprint(ordered) || body.len() < head + held * 2 {
        return Touching::NotThisSegmentation {
            held,
            wanted: ordered.len(),
        };
    }
    let masks = body[head..head + held * 2]
        .chunks_exact(2)
        .map(|pair| Mask(u16::from_le_bytes([pair[0], pair[1]])))
        .collect();
    Touching::Known(masks)
}

/// The positions an anchor names in the work it belongs to, in reading order.
///
/// The one implementation of *which segments does this endpoint cover*. Both
/// the reading pane (`girsa_app::beside`) and the mask builder ask it, and two
/// answers that drifted would put a commentary against a line the graph does
/// not join it to.
///
/// `None` when the anchor names a segment this work does not have — never the
/// nearest one (BUILDER.md rule 6).
pub fn span_of(
    anchor: &Anchor,
    position: impl Fn(&SegmentId) -> Option<usize>,
) -> Option<RangeInclusive<usize>> {
    let from = position(&anchor.from)?;
    // A run whose far end is missing is still a run from its near end: the
    // endpoint was resolved against this work at import, and half of a known
    // answer beats none of it.
    let to = anchor.to.as_ref().and_then(&position).unwrap_or(from);
    Some(from.min(to)..=to.max(from))
}

/// Fold `(endpoint, kind)` pairs into one mask per segment, in reading order.
///
/// A segment no edge touches gets [`Mask::NONE`], not a missing entry — the
/// caller is indexing every segment either way.
#[must_use]
pub fn masks_for<'a>(
    ends: impl IntoIterator<Item = (&'a Anchor, EdgeType)>,
    ordered: &[SegmentId],
) -> Vec<Mask> {
    let position: std::collections::HashMap<&SegmentId, usize> =
        ordered.iter().enumerate().map(|(i, id)| (id, i)).collect();
    let mut out = vec![Mask::NONE; ordered.len()];
    for (anchor, kind) in ends {
        let Some(range) = span_of(anchor, |id| position.get(id).copied()) else {
            continue;
        };
        for slot in out.get_mut(range).unwrap_or_default() {
            slot.insert(kind);
        }
    }
    out
}

/// An edge type by the name it was written under.
///
/// Deliberately **not** [`EdgeType::from_sefaria`]: that reads the corpus's own
/// label and maps three quarters of them onto `references`. This reads a name
/// this project wrote, and a name it does not know is dropped rather than
/// folded into the catch-all — a type invented on read is a claim nobody made.
#[must_use]
pub fn type_named(name: &str) -> Option<EdgeType> {
    EdgeType::ALL.into_iter().find(|t| t.as_str() == name)
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::Ordinal;

    fn id(work: &str, seif: u32, n: u32) -> SegmentId {
        SegmentId::new(work, vec!["1".into(), seif.to_string()], Ordinal::root(n))
    }

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_run_touches_every_segment_it_covers() {
        // `Rashi on Berakhot 2a` covers a daf. A facet that counted only the
        // first segment of a run would undercount most of the graph, because
        // most of Sefaria's citations are coarser than a segment.
        let ordered: Vec<SegmentId> = (1..=5).map(|n| id("bavli/berakhot", n, n)).collect();
        let anchor = Anchor::span(ordered[1].clone(), ordered[3].clone());
        let masks = masks_for([(&anchor, EdgeType::CommentsOn)], &ordered);
        let set: Vec<bool> = masks.iter().map(|m| !m.is_empty()).collect();
        assert_eq!(set, [false, true, true, true, false]);
    }

    #[test]
    fn an_anchor_this_work_does_not_have_touches_nothing() {
        let ordered: Vec<SegmentId> = (1..=3).map(|n| id("bavli/berakhot", n, n)).collect();
        let anchor = Anchor::point(id("bavli/berakhot", 9, 9));
        let masks = masks_for([(&anchor, EdgeType::CommentsOn)], &ordered);
        assert!(masks.iter().all(|m| m.is_empty()));
    }

    #[test]
    fn two_kinds_on_one_segment_are_two_bits_and_one_kind_twice_is_one() {
        // The facet counts *segments* per kind. Two hundred seforim comment on
        // Berakhot 2a:1; that is one bit, not two hundred rows.
        let ordered = vec![id("bavli/berakhot", 1, 1)];
        let a = Anchor::point(ordered[0].clone());
        let masks = masks_for(
            [
                (&a, EdgeType::CommentsOn),
                (&a, EdgeType::CommentsOn),
                (&a, EdgeType::Quotes),
            ],
            &ordered,
        );
        assert_eq!(
            masks[0].kinds(),
            vec![EdgeType::CommentsOn, EdgeType::Quotes]
        );
    }

    #[test]
    fn a_mask_file_round_trips() {
        let root = dir("girsa-touching-round-trip");
        let ordered: Vec<SegmentId> = (1..=4).map(|n| id("bavli/berakhot", n, n)).collect();
        let a = Anchor::point(ordered[2].clone());
        let masks = masks_for([(&a, EdgeType::Quotes)], &ordered);
        write(&root, "bavli/berakhot", &ordered, &masks).expect("writes");

        match read(&root, "bavli/berakhot", &ordered) {
            Touching::Known(back) => assert_eq!(back, masks),
            other => panic!("not read back: {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mask_built_for_a_different_segmentation_is_refused_rather_than_read() {
        // The one hazard a positional cache has that an anchor-keyed one does
        // not. Sefaria adds a se'if, the work is re-imported, and every mask
        // after the insertion point is now about the line above it — arithmetic
        // that runs happily and a facet column that looks exactly like a good
        // one. `girsa_lane::vectors` learned this first.
        let root = dir("girsa-touching-stale");
        let before: Vec<SegmentId> = (1..=4).map(|n| id("bavli/berakhot", n, n)).collect();
        let a = Anchor::point(before[2].clone());
        let masks = masks_for([(&a, EdgeType::Quotes)], &before);
        write(&root, "bavli/berakhot", &before, &masks).expect("writes");

        // Same count, one id different — the count alone would not catch it.
        let mut after = before.clone();
        after[1] = id("bavli/berakhot", 1, 9);
        assert_eq!(
            read(&root, "bavli/berakhot", &after),
            Touching::NotThisSegmentation { held: 4, wanted: 4 },
            "a renumbered work read its old masks"
        );

        // And a count that moved.
        let longer: Vec<SegmentId> = (1..=5).map(|n| id("bavli/berakhot", n, n)).collect();
        assert!(matches!(
            read(&root, "bavli/berakhot", &longer),
            Touching::NotThisSegmentation { .. }
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_file_is_not_an_empty_answer() {
        let root = dir("girsa-touching-absent");
        let ordered = vec![id("bavli/berakhot", 1, 1)];
        assert_eq!(read(&root, "bavli/berakhot", &ordered), Touching::Unbuilt);
    }

    #[test]
    fn the_bits_are_where_the_wire_format_says_they_are() {
        // The bit order is `EdgeType::ALL` and is a format, not an
        // implementation detail: rearranging it silently repaints every facet
        // built before the change.
        assert_eq!(EdgeType::CommentsOn.bit(), 1);
        assert_eq!(EdgeType::References.bit(), 1 << 8);
        for (at, kind) in EdgeType::ALL.into_iter().enumerate() {
            assert_eq!(kind.bit(), 1 << at, "{kind:?} moved");
        }
        let all: Mask = EdgeType::ALL.into_iter().collect();
        assert_eq!(all.kinds(), EdgeType::ALL.to_vec());
        assert_eq!(all.bits(), 0b1_1111_1111);
    }

    #[test]
    fn a_type_this_project_did_not_write_is_dropped_rather_than_guessed() {
        assert_eq!(type_named("comments-on"), Some(EdgeType::CommentsOn));
        assert_eq!(type_named("references"), Some(EdgeType::References));
        assert_eq!(type_named(""), None, "a blank is not the catch-all here");
        assert_eq!(type_named("commentary"), None, "that is Sefaria's word");
    }
}
