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
//! {"a":"girsa:bavli/berakhot/2a:1#1","t":"comments-on","w":["bavli/rashi-on-berakhot","bavli/tosafot-on-berakhot"]}
//! ```
//!
//! One row per (endpoint, type) an edge puts on this work, from **both** ends —
//! which is the whole point, and why it is not simply a copy of `edges.jsonl` — and
//! since W31 the row also names **which seforim** those links came from.
//!
//! # `w` — which sefer the link came from (W31)
//!
//! Filed from OtzariaSonim, the keypad-phone reader, on its first day against this
//! corpus. The row used to be endpoint plus type, and **no consumer asks whether a
//! segment has a `comments-on`.** They ask whether it has one *from the mefarshim
//! currently selected* — the per-book commentator filter, which is also spec.md
//! §8.5's lenses and §8.4's gutter density map. That question needs the source
//! work, and it was the one field not stored, so the only way to answer was the
//! file this one exists to avoid:
//!
//! | Shulchan Arukh, Orach Chayim | size | answers *which segments light up?* |
//! |---|---|---|
//! | `touching.jsonl` | 1.15 MB | no — no work slug |
//! | `inbound.jsonl` | 27.3 MB | yes, by reading all 156,076 edges |
//!
//! On a phone with a 192 MB heap that is the difference between opening שולחן ערוך
//! and an `OutOfMemoryError`.
//!
//! **Grouped onto the row, and that was measured.** One row per work is the obvious
//! encoding; it took the summary layer from 261 MB to **636 MB, which is 92.4% of
//! the 0.69 GB of `inbound.jsonl` this file exists to avoid.** A summary the size of
//! the thing it summarises is not a summary. With the works on one row the anchor
//! and the type are written once for the two hundred seforim that comment on a
//! se'if.
//!
//! The field is **optional on read**: files exist without it, and a reader that
//! refused them would mean a re-import to open a sefer. A row that does not say
//! comes back with an empty list — *this row does not record which sefer* — and
//! never as a guess.
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
    /// The seforim at the **other** end — the ones whose links these are (W31).
    ///
    /// A list, and grouped onto the `(anchor, type)` row rather than given a row
    /// each. One row per work was the obvious encoding and it was measured: the
    /// summary layer went from 261 MB to **636 MB, which is 92.4% of the 0.69 GB of
    /// `inbound.jsonl` it exists to avoid.** A summary the size of the thing it
    /// summarises is not a summary. Grouped, the anchor and the type are written
    /// once for the two hundred seforim that comment on a se'if.
    ///
    /// Empty in files written before W31, and empty means *this row does not say* —
    /// never *nothing comments here*.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub w: Vec<String>,
}

/// Collects both ends of every edge and writes one file per work touched.
///
/// The same discipline as [`crate::store::Writer`], for the same reason: a run
/// is many flushes, so a shard is appended to within a run — and appending to
/// what the **last** run left would double every count silently.
#[derive(Debug, Default)]
pub struct Writer {
    /// work → (anchor, type) → the seforim at the other end.
    ///
    /// A map and not a set of serialised lines since W31: the works have to be
    /// gathered onto one row before anything is written, or the grouping that keeps
    /// this file a summary cannot happen.
    by_work: BTreeMap<String, BTreeMap<(String, String), BTreeSet<String>>>,
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
        let (near, far) = (edge.from.from.work(), edge.to.from.work());
        // Each end's row names the **other** work: a row filed under the Mishnah
        // Berurah saying `mishnah-berurah` tells a reader standing in it nothing.
        self.record(near, &edge.from.to_string(), edge.edge_type, far);
        self.record(far, &edge.to.to_string(), edge.edge_type, near);
    }

    /// One end, by the work it lands in.
    ///
    /// Deduplicated within the buffer: thousands of edges land on one daf and
    /// they are the same row. What survives across a flush is deduplicated on
    /// read instead — see [`read_back`].
    pub fn record(&mut self, slug: &str, anchor: &str, edge_type: EdgeType, from_work: &str) {
        let works = self
            .by_work
            .entry(slug.to_string())
            .or_default()
            .entry((anchor.to_string(), edge_type.as_str().to_string()))
            .or_default();
        // A self-edge would say a sefer is its own source, which is true and
        // useless; the filter asks *which other sefer*, so it is left off.
        if from_work == slug {
            self.written += 1;
            return;
        }
        if works.insert(from_work.to_string()) {
            self.written += 1;
        }
    }

    /// How many rows are being held, so a caller can flush before memory
    /// becomes the reason the run did not finish.
    #[must_use]
    pub fn buffered(&self) -> usize {
        self.by_work
            .values()
            .map(|rows| {
                rows.values()
                    .map(BTreeSet::len)
                    .sum::<usize>()
                    .max(rows.len())
            })
            .sum()
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
            for ((anchor, edge_type), works) in rows {
                let Ok(line) = serde_json::to_string(&Row {
                    a: anchor,
                    t: edge_type,
                    w: works.into_iter().collect(),
                }) else {
                    continue;
                };
                body.push_str(&line);
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
pub fn read_back(
    root: &Path,
    slug: &str,
) -> Result<Vec<(Anchor, EdgeType, Vec<String>)>, std::io::Error> {
    let path = types_path(root, slug);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let body = fs::read_to_string(&path)?;
    // Merged on `(anchor, type)`, and the **works are unioned** rather than the
    // second row dropped. A run is many flushes, so one se'if's works can be split
    // across two rows of the same file — keeping the first and discarding the rest
    // would be the filter answering wrongly rather than slowly, which is the worse
    // of the two failures.
    let mut merged: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut order: Vec<(String, String)> = Vec::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(row) = serde_json::from_str::<Row>(line) else {
            continue;
        };
        let key = (row.a, row.t);
        if !merged.contains_key(&key) {
            order.push(key.clone());
        }
        merged.entry(key).or_default().extend(row.w);
    }
    let mut out = Vec::new();
    for key in order {
        let Some(works) = merged.remove(&key) else {
            continue;
        };
        let Some(anchor) = parse_anchor(&key.0) else {
            continue;
        };
        let Some(edge_type) = type_named(&key.1) else {
            continue;
        };
        out.push((anchor, edge_type, works.into_iter().collect()));
    }
    Ok(out)
}

/// Every kind of link touching each segment of a work, in reading order.
///
/// Given the work's segments as the indexer holds them. A segment no edge
/// touches gets an empty set, not a missing entry — the caller is indexing
/// every segment either way.
#[must_use]
pub fn by_segment(
    rows: &[(Anchor, EdgeType, Vec<String>)],
    ordered: &[SegmentId],
) -> Vec<BTreeSet<EdgeType>> {
    let position: std::collections::HashMap<&SegmentId, usize> =
        ordered.iter().enumerate().map(|(i, id)| (id, i)).collect();
    let mut out = vec![BTreeSet::new(); ordered.len()];
    for (anchor, edge_type, _) in rows {
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
            vec!["bavli/rashi-on-berakhot".to_string()],
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
            Vec::new(),
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
    fn a_row_says_which_sefer_the_link_came_from() {
        // W31, filed from OtzariaSonim on its first day against this corpus. A row
        // was `{"a": …, "t": "comments-on"}` — segment plus edge *type* — and no
        // consumer asks *does this segment have a `comments-on`*. They ask **does
        // it have one from the mefarshim I have selected**, which is the per-book
        // commentator filter, spec.md §8.5's lenses and §8.4's gutter density map.
        //
        // Without the source work the only way to answer was `inbound.jsonl`: on
        // Shulchan Arukh, Orach Chayim that is 27.3 MB and 156,076 edges against
        // this file's 1.15 MB. On a phone with a 192 MB heap that is the difference
        // between opening the sefer and an `OutOfMemoryError`.
        let root = dir("girsa-touching-source-work");
        let mut writer = Writer::default();
        writer.push(&edge(
            Anchor::point(id("mishnah-berurah", 1, 3)),
            Anchor::point(id("shulchan-arukh/orach-chayim", 1, 1)),
            EdgeType::CommentsOn,
        ));
        writer.flush(&root).expect("writes");

        let rows = read_back(&root, "shulchan-arukh/orach-chayim").expect("reads");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].2,
            vec!["mishnah-berurah".to_string()],
            "the row cannot say which mefaresh it came from"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_far_end_says_where_the_link_landed_not_where_it_started() {
        // Both ends get a row, and each row names the **other** work — otherwise
        // the row filed under Mishnah Berurah would say `mishnah-berurah`, which
        // tells a reader standing in it precisely nothing.
        let root = dir("girsa-touching-source-both");
        let mut writer = Writer::default();
        writer.push(&edge(
            Anchor::point(id("mishnah-berurah", 1, 3)),
            Anchor::point(id("shulchan-arukh/orach-chayim", 1, 1)),
            EdgeType::CommentsOn,
        ));
        writer.flush(&root).expect("writes");

        let mine = read_back(&root, "mishnah-berurah").expect("reads");
        assert_eq!(mine.len(), 1);
        assert_eq!(
            mine[0].2,
            vec!["shulchan-arukh/orach-chayim".to_string()],
            "a row must name the sefer at the other end"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn two_mefarshim_on_one_line_are_two_rows_and_one_mefaresh_twice_is_one() {
        // The dedup key gained a field, and this is what that has to mean: the
        // facet still counts *segments* per kind, and the filter now needs *which
        // work* — so `(anchor, type, work)` is the key. Deduplicating on
        // `(anchor, type)` would keep one of the two mefarshim and lose the other,
        // which is the filter answering wrongly rather than slowly.
        let root = dir("girsa-touching-source-dedup");
        let mut writer = Writer::default();
        for (work, seif) in [
            ("mishnah-berurah", 1),
            ("magen-avraham", 1),
            ("mishnah-berurah", 2),
        ] {
            writer.push(&edge(
                Anchor::point(id(work, seif, seif)),
                Anchor::point(id("shulchan-arukh/orach-chayim", 1, 1)),
                EdgeType::CommentsOn,
            ));
            // Flushed each time, so the duplicate crosses a flush boundary.
            writer.flush(&root).expect("writes");
        }
        let rows = read_back(&root, "shulchan-arukh/orach-chayim").expect("reads");
        // One row for the se'if, carrying both mefarshim — the grouping that keeps
        // this file a summary. Split across three flushes and merged on read.
        assert_eq!(rows.len(), 1, "one (segment, type) row: {rows:?}");
        assert_eq!(
            rows[0].2,
            vec!["magen-avraham".to_string(), "mishnah-berurah".to_string()],
            "both mefarshim, and Mishnah Berurah once"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_row_written_before_w31_still_reads() {
        // 261 MB of these files exist on disk without the field. A reader that
        // refused them would mean a re-import to open a sefer, and the honest
        // answer for a row that does not say is **`None`** — *this row does not
        // record which sefer* — rather than a guess.
        let root = dir("girsa-touching-old-row");
        let path = types_path(&root, "bavli/berakhot");
        fs::create_dir_all(path.parent().expect("a parent")).expect("dir");
        fs::write(
            &path,
            "{\"a\":\"girsa:bavli/berakhot/2a:1#1\",\"t\":\"comments-on\"}
",
        )
        .expect("writes");

        let rows = read_back(&root, "bavli/berakhot").expect("reads");
        assert_eq!(rows.len(), 1, "an older row was refused");
        assert_eq!(rows[0].1, EdgeType::CommentsOn);
        assert!(
            rows[0].2.is_empty(),
            "it does not say, and must not pretend to"
        );
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
