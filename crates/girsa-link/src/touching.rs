//! Which kinds of link touch each segment — the graph read from the segment's
//! side (BUILDER.md W14, spec.md §9.8).
//!
//! §9.8 asks results to carry a **link type** facet: *of these 300 hits, 120
//! are in segments something comments on*. That question is asked of a segment
//! and the graph is not stored that way. spec.md §8.2: an edge is *directed,
//! inverse derived and never stored twice*, and W8 stored each one in the shard
//! of the work it points **from**. So Berakhot's own shard holds the handful of
//! edges Berakhot makes, and the two million edges that land **on** Berakhot are
//! scattered across every shard in the corpus.
//!
//! Deriving the inverse per query would mean reading all 691 MB of the graph to
//! draw one facet row. So it is derived **once**, here, and written beside the
//! edges:
//!
//! ```jsonl
//! corpus/links/bavli/berakhot/touching.jsonl
//! {"a":"girsa:bavli/berakhot/2a:1#1","t":"comments-on"}
//! ```
//!
//! One row per (endpoint, type) an edge puts on this work, from **both** ends —
//! which is the whole point, and why it is not simply a copy of `edges.jsonl`.
//!
//! # It is a cache, and it may be missing
//!
//! spec.md §4.1: the files are the truth and anything faster is rebuildable.
//! Delete this and run `girsa-link-types` again. What must never happen is the
//! facet quietly reading a **zero** off an index built without it — *no links
//! of that kind* and *nobody worked out the link types* are different
//! statements, and the index records which one it is (see
//! `girsa_search::index::BuildReport`).
//!
//! # The anchor is written out, not resolved to segments
//!
//! An endpoint can be a run — `Rashi on Berakhot 2a` covers a daf — and turning
//! a run into the segments it names needs that work's segments in reading
//! order, which the walker does not have and the indexer does. So the anchor is
//! written as it stands and expanded by [`span_of`] at the one place that knows.
//! Storing the expansion instead would also mean storing **positions**, and a
//! position is the one thing this project may not write down (spec.md §3).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use girsa_corpus::import::slug_dir;
use girsa_corpus::segment::SegmentId;

use crate::{Anchor, Edge, EdgeType};

/// Where a work's incoming-and-outgoing link types live.
#[must_use]
pub fn types_path(root: &Path, slug: &str) -> PathBuf {
    slug_dir(&root.join("links"), slug).join("touching.jsonl")
}

/// The positions an anchor names in the work it belongs to, in reading order.
///
/// The one implementation of *which segments does this endpoint cover*. Both
/// the reading pane (`girsa_app::beside`) and the indexer ask it, and two
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

/// One end of one edge, as it is written down.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Row {
    /// The anchor, in the text every id travels as.
    pub a: String,
    /// The edge type, as [`EdgeType::as_str`] writes it.
    pub t: String,
}

/// Collects both ends of every edge and writes one file per work touched.
///
/// The same discipline as [`crate::store::Writer`], for the same reason: a run
/// is many flushes, so a shard is appended to within a run — and appending to
/// what the **last** run left would double every count silently.
#[derive(Debug, Default)]
pub struct Writer {
    by_work: BTreeMap<String, BTreeSet<String>>,
    written: usize,
    opened: BTreeSet<String>,
}

impl Writer {
    /// Record what this edge puts on each of its two ends.
    ///
    /// An edge from A to B is a fact about A **and** a fact about B. A facet
    /// built from the stored direction alone would tell a reader that nothing
    /// comments on Berakhot.
    pub fn push(&mut self, edge: &Edge) {
        for anchor in [&edge.from, &edge.to] {
            self.record(anchor.from.work(), &anchor.to_string(), edge.edge_type);
        }
    }

    /// One end, by the work it lands in.
    ///
    /// Deduplicated within the buffer: thousands of edges land on one daf and
    /// they are the same row. What survives across a flush is deduplicated on
    /// read instead — see [`read_back`].
    pub fn record(&mut self, slug: &str, anchor: &str, edge_type: EdgeType) {
        let Ok(line) = serde_json::to_string(&Row {
            a: anchor.to_string(),
            t: edge_type.as_str().to_string(),
        }) else {
            return;
        };
        if self
            .by_work
            .entry(slug.to_string())
            .or_default()
            .insert(line)
        {
            self.written += 1;
        }
    }

    /// How many rows are being held, so a caller can flush before memory
    /// becomes the reason the run did not finish.
    #[must_use]
    pub fn buffered(&self) -> usize {
        self.by_work.values().map(BTreeSet::len).sum()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.written
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.written == 0
    }

    /// Write everything held and forget it.
    ///
    /// # Errors
    ///
    /// If a file cannot be created or written to.
    pub fn flush(&mut self, root: &Path) -> Result<(), std::io::Error> {
        for (slug, rows) in std::mem::take(&mut self.by_work) {
            let path = types_path(root, &slug);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let first_touch = self.opened.insert(slug);
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(!first_touch)
                .write(first_touch)
                .truncate(first_touch)
                .open(&path)?;
            let mut body = String::new();
            for row in rows {
                body.push_str(&row);
                body.push('\n');
            }
            file.write_all(body.as_bytes())?;
        }
        Ok(())
    }
}

/// Read one work's link types back, deduplicated.
///
/// # Errors
///
/// If the file exists and cannot be read. A work with no file is a work no
/// edge touches, which is not an error — and is **not** what the facet reports
/// when the cache was never built. That difference is the index's to keep.
pub fn read_back(root: &Path, slug: &str) -> Result<Vec<(Anchor, EdgeType)>, std::io::Error> {
    let path = types_path(root, slug);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let body = fs::read_to_string(&path)?;
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    let mut out = Vec::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(row) = serde_json::from_str::<Row>(line) else {
            continue;
        };
        if !seen.insert((row.a.clone(), row.t.clone())) {
            continue;
        }
        let Some(anchor) = parse_anchor(&row.a) else {
            continue;
        };
        let Some(edge_type) = type_named(&row.t) else {
            continue;
        };
        out.push((anchor, edge_type));
    }
    Ok(out)
}

/// Every kind of link touching each segment of a work, in reading order.
///
/// Given the work's segments as the indexer holds them. A segment no edge
/// touches gets an empty set, not a missing entry — the caller is indexing
/// every segment either way.
#[must_use]
pub fn by_segment(rows: &[(Anchor, EdgeType)], ordered: &[SegmentId]) -> Vec<BTreeSet<EdgeType>> {
    let position: std::collections::HashMap<&SegmentId, usize> =
        ordered.iter().enumerate().map(|(i, id)| (id, i)).collect();
    let mut out = vec![BTreeSet::new(); ordered.len()];
    for (anchor, edge_type) in rows {
        let Some(range) = span_of(anchor, |id| position.get(id).copied()) else {
            continue;
        };
        for slot in out.get_mut(range).unwrap_or_default() {
            slot.insert(*edge_type);
        }
    }
    out
}

/// `girsa:x/1:1#1-girsa:x/1:3#3` → a run; a single id → a point.
fn parse_anchor(text: &str) -> Option<Anchor> {
    match text.split_once("-girsa:") {
        Some((from, to)) => Some(Anchor::span(
            from.parse().ok()?,
            format!("girsa:{to}").parse().ok()?,
        )),
        None => Some(Anchor::point(text.parse().ok()?)),
    }
}

/// An edge type by the name it was written under.
///
/// Deliberately **not** [`EdgeType::from_sefaria`]: that reads the corpus's own
/// label and maps three quarters of them onto `references`. This reads a name
/// this project wrote, and a name it does not know is dropped rather than
/// folded into the catch-all — a type invented on read is a claim nobody made.
#[must_use]
pub fn type_named(name: &str) -> Option<EdgeType> {
    [
        EdgeType::CommentsOn,
        EdgeType::Quotes,
        EdgeType::Paraphrases,
        EdgeType::Codifies,
        EdgeType::Disputes,
        EdgeType::Emends,
        EdgeType::ParallelTo,
        EdgeType::Translates,
        EdgeType::References,
    ]
    .into_iter()
    .find(|t| t.as_str() == name)
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::Method;
    use girsa_corpus::segment::Ordinal;

    fn id(work: &str, seif: u32, n: u32) -> SegmentId {
        SegmentId::new(work, vec!["1".into(), seif.to_string()], Ordinal::root(n))
    }

    fn edge(from: Anchor, to: Anchor, edge_type: EdgeType) -> Edge {
        Edge {
            from,
            to,
            edge_type,
            method: Method::SefariaSeed,
            source_label: String::new(),
        }
    }

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn an_edge_is_recorded_against_both_of_its_ends() {
        // The whole reason this file exists. Rashi's shard holds the edge;
        // Berakhot's shard holds nothing, and *"what comments on this line"* is
        // asked from Berakhot's side every time.
        let root = dir("girsa-touching-both-ends");
        let mut writer = Writer::default();
        writer.push(&edge(
            Anchor::point(id("bavli/rashi-on-berakhot", 1, 3)),
            Anchor::point(id("bavli/berakhot", 1, 1)),
            EdgeType::CommentsOn,
        ));
        writer.flush(&root).expect("writes");

        let rashi = read_back(&root, "bavli/rashi-on-berakhot").expect("reads");
        let berakhot = read_back(&root, "bavli/berakhot").expect("reads");
        assert_eq!(rashi.len(), 1, "the end it was stored under");
        assert_eq!(
            berakhot.len(),
            1,
            "the end it was not stored under — which is the one a reader asks from"
        );
        assert_eq!(berakhot[0].1, EdgeType::CommentsOn);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_run_touches_every_segment_it_covers() {
        // `Rashi on Berakhot 2a` covers a daf. A facet that counted only the
        // first segment of a run would undercount most of the graph, because
        // most of Sefaria's citations are coarser than a segment.
        let ordered: Vec<SegmentId> = (1..=5).map(|n| id("bavli/berakhot", n, n)).collect();
        let rows = vec![(
            Anchor::span(ordered[1].clone(), ordered[3].clone()),
            EdgeType::CommentsOn,
        )];
        let touched = by_segment(&rows, &ordered);
        let counts: Vec<usize> = touched.iter().map(BTreeSet::len).collect();
        assert_eq!(counts, [0, 1, 1, 1, 0]);
    }

    #[test]
    fn an_anchor_this_work_does_not_have_touches_nothing() {
        let ordered: Vec<SegmentId> = (1..=3).map(|n| id("bavli/berakhot", n, n)).collect();
        let rows = vec![(
            Anchor::point(id("bavli/berakhot", 9, 9)),
            EdgeType::CommentsOn,
        )];
        assert!(by_segment(&rows, &ordered).iter().all(BTreeSet::is_empty));
    }

    #[test]
    fn running_it_twice_does_not_double_what_is_counted() {
        // The bug the link importer already had once: a run is many flushes, so
        // a file is appended to within a run, and appending to the last run's
        // file doubles every row with no error anywhere.
        let root = dir("girsa-touching-rerun");
        for _ in 0..2 {
            let mut writer = Writer::default();
            writer.push(&edge(
                Anchor::point(id("a", 1, 1)),
                Anchor::point(id("b", 1, 1)),
                EdgeType::Quotes,
            ));
            writer.flush(&root).expect("writes");
            writer.push(&edge(
                Anchor::point(id("a", 2, 2)),
                Anchor::point(id("b", 2, 2)),
                EdgeType::Quotes,
            ));
            writer.flush(&root).expect("writes again");
        }
        assert_eq!(read_back(&root, "a").expect("reads").len(), 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_same_end_recorded_twice_is_counted_once() {
        // Berakhot 2a:1 is commented on by two hundred seforim. That is two
        // hundred rows saying `comments-on` about one segment, and the facet
        // counts **segments**, not edges.
        let root = dir("girsa-touching-dedup");
        let mut writer = Writer::default();
        for n in 1..=3u32 {
            writer.push(&edge(
                Anchor::point(id("commentary", 1, n)),
                Anchor::point(id("bavli/berakhot", 1, 1)),
                EdgeType::CommentsOn,
            ));
            // Flushed each time, so the duplicates cross a flush boundary and
            // have to be caught on read rather than in the buffer.
            writer.flush(&root).expect("writes");
        }
        let back = read_back(&root, "bavli/berakhot").expect("reads");
        assert_eq!(back.len(), 1, "one segment, one kind of link");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_type_this_project_did_not_write_is_dropped_rather_than_guessed() {
        assert_eq!(type_named("comments-on"), Some(EdgeType::CommentsOn));
        assert_eq!(type_named("references"), Some(EdgeType::References));
        assert_eq!(type_named(""), None, "a blank is not the catch-all here");
        assert_eq!(type_named("commentary"), None, "that is Sefaria's word");
    }
}
