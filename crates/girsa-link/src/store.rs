//! Edges on disk, sharded by the work they come from.
//!
//! Same rule as the text (spec.md §4.1): the files are the truth and anything
//! faster is a rebuildable cache. So an edge is a line of JSON naming both its
//! ends by permanent id —
//!
//! ```jsonl
//! {"from":"girsa:mishnah/berakhot/1:1#1","to":"girsa:rambam-on-mishnah/berakhot/1:1#5","type":"comments-on","method":"sefaria-seed","label":"commentary"}
//! ```
//!
//! — which is greppable, diffable, and survives both a correction to the text
//! and an upstream re-segmentation, because a segment id survives both.
//!
//! # Stored once, in the direction it was written
//!
//! spec.md §8.2: *directed, inverse derived and never stored twice*. An edge
//! lives in the shard of the work it points **from**; asking "who comments on
//! this se'if" is the reverse direction and is answered from an index built
//! over the shards, not from a second copy that could disagree with the first.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use girsa_corpus::import::slug_dir;

use crate::{Anchor, Edge, EdgeType, Method};

/// Where a work's outgoing edges live.
///
/// `corpus/links/<slug>/edges.jsonl`, mirroring `corpus/works/<slug>/`, so the
/// two halves of a sefer sit at the same address under different roots.
#[must_use]
pub fn edges_path(root: &Path, slug: &str) -> PathBuf {
    slug_dir(&root.join("links"), slug).join("edges.jsonl")
}

/// One edge, as it is written down.
///
/// Both ends are the **text** of a segment id, because that is how an anchor
/// travels everywhere else in the system — into Ksav documents, into patch
/// files. One shape, not two.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Row {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub method: String,
    /// What the corpus called it, verbatim — including the empty string, which
    /// is what three quarters of them say (T5).
    pub label: String,
}

impl Row {
    #[must_use]
    pub fn of(edge: &Edge) -> Self {
        Self {
            from: edge.from.to_string(),
            to: edge.to.to_string(),
            edge_type: edge.edge_type.as_str().to_string(),
            method: edge.method.as_str().to_string(),
            label: edge.source_label.clone(),
        }
    }
}

/// Collects edges and writes one file per source work.
///
/// Buffered by work rather than written per edge: the graph is millions of
/// edges over thousands of works, and opening a file per edge is the difference
/// between an import that finishes and one that does not.
///
/// # Running it twice
///
/// A run is bounded by the buffer, not by the graph: `flush` is called many
/// times per import, so a shard is written to more than once and cannot simply
/// be truncated each time. But a **new** `Writer` is a new run, and appending to
/// what the last run left would silently double every edge — a link graph twice
/// its own size, with every commentary showing twice and no error anywhere.
///
/// So each shard is truncated the first time *this* writer touches it and
/// appended to afterwards. Re-running the import is then the same as running it
/// once, which is what "a command someone else can run" has to mean.
#[derive(Debug, Default)]
pub struct Writer {
    by_work: BTreeMap<String, String>,
    written: usize,
    /// Shards this writer has already opened, and so must not truncate again.
    opened: BTreeSet<String>,
}

impl Writer {
    pub fn push(&mut self, edge: &Edge) {
        let slug = edge.from.from.work().to_string();
        let Ok(line) = serde_json::to_string(&Row::of(edge)) else {
            return;
        };
        let body = self.by_work.entry(slug).or_default();
        body.push_str(&line);
        body.push('\n');
        self.written += 1;
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.written
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.written == 0
    }

    /// How many bytes are being held, so a caller can flush before memory
    /// becomes the reason the import did not finish.
    #[must_use]
    pub fn buffered_bytes(&self) -> usize {
        self.by_work.values().map(String::len).sum()
    }

    /// Write everything held to disk and forget it.
    ///
    /// The first flush that touches a work **replaces** that work's shard; every
    /// flush after it in the same run appends. See the note on [`Writer`].
    ///
    /// # Errors
    ///
    /// If a shard cannot be created or written to.
    pub fn flush(&mut self, root: &Path) -> Result<(), std::io::Error> {
        for (slug, body) in std::mem::take(&mut self.by_work) {
            let path = edges_path(root, &slug);
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
            file.write_all(body.as_bytes())?;
        }
        Ok(())
    }
}

/// Read one work's outgoing edges back.
///
/// # Errors
///
/// If the shard exists and cannot be read. A work with no shard has no
/// outgoing edges, which is not an error — most works are cited more than they
/// cite.
pub fn read_back(root: &Path, slug: &str) -> Result<Vec<Edge>, std::io::Error> {
    read_edges(&edges_path(root, slug))
}

/// One file of edge rows, whichever file it is.
///
/// The outgoing shard and W28's inbound cache hold the same rows in the same
/// shape, and two readers that drifted would give one answer for the half of a
/// segment's links stored here and another for the half stored elsewhere.
///
/// # Errors
///
/// If the file exists and cannot be read. A file that is not there is no edges,
/// which is not an error.
pub fn read_edges(path: &Path) -> Result<Vec<Edge>, std::io::Error> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let body = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(row) = serde_json::from_str::<Row>(line) else {
            continue;
        };
        let (Some(from), Some(to)) = (parse_anchor(&row.from), parse_anchor(&row.to)) else {
            continue;
        };
        out.push(Edge {
            from,
            to,
            edge_type: EdgeType::from_sefaria(&row.label),
            method: if row.method == Method::OtzariaSeed.as_str() {
                Method::OtzariaSeed
            } else {
                Method::SefariaSeed
            },
            source_label: row.label,
        });
    }
    Ok(out)
}

/// `girsa:x/1:1#1-girsa:x/1:3#3` → a run; a single id → a point.
pub(crate) fn parse_anchor(text: &str) -> Option<Anchor> {
    match text.split_once("-girsa:") {
        Some((from, to)) => Some(Anchor::span(
            from.parse().ok()?,
            format!("girsa:{to}").parse().ok()?,
        )),
        None => Some(Anchor::point(text.parse().ok()?)),
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::{Ordinal, SegmentId};

    fn id(work: &str, seif: u32, n: u32) -> SegmentId {
        SegmentId::new(work, vec!["1".into(), seif.to_string()], Ordinal::root(n))
    }

    fn edge() -> Edge {
        Edge {
            from: Anchor::point(id("mishnah/berakhot", 1, 1)),
            to: Anchor::span(id("rambam/berakhot", 1, 5), id("rambam/berakhot", 2, 6)),
            edge_type: EdgeType::CommentsOn,
            method: Method::SefariaSeed,
            source_label: "commentary".into(),
        }
    }

    #[test]
    fn an_edge_survives_the_round_trip_to_disk() {
        let dir = std::env::temp_dir().join("girsa-link-store-test");
        let _ = fs::remove_dir_all(&dir);
        let mut writer = Writer::default();
        writer.push(&edge());
        writer.flush(&dir).expect("writes");

        let back = read_back(&dir, "mishnah/berakhot").expect("reads");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0], edge());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn running_the_import_twice_does_not_double_the_graph() {
        // A run is many flushes, so a shard is appended to within a run — and
        // appending to what the *last* run left would silently double every
        // edge. Twice the graph, every commentary showing twice, no error.
        let dir = std::env::temp_dir().join("girsa-link-store-rerun");
        let _ = fs::remove_dir_all(&dir);

        for _ in 0..2 {
            let mut writer = Writer::default();
            // Two flushes, the way a real run buffers and spills.
            writer.push(&edge());
            writer.flush(&dir).expect("writes");
            writer.push(&edge());
            writer.flush(&dir).expect("writes again");
        }

        let back = read_back(&dir, "mishnah/berakhot").expect("reads");
        assert_eq!(
            back.len(),
            2,
            "the second run replaced the first, and its own two flushes both landed"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_span_endpoint_reads_back_as_a_span() {
        // The two ids in a run are separated by a hyphen, and a slug can carry
        // a hyphen too — `shulchan-arukh`. Splitting on the first hyphen would
        // tear the work name in half and neither end would parse.
        let a = id("shulchan-arukh/orach-chayim", 1, 1);
        let b = id("shulchan-arukh/orach-chayim", 3, 3);
        let anchor = Anchor::span(a.clone(), b.clone());
        let back = parse_anchor(&anchor.to_string()).expect("parses");
        assert_eq!(back, anchor);
        assert_eq!(back.from, a);
        assert_eq!(back.to, Some(b));
    }

    #[test]
    fn a_blank_label_reads_back_as_a_blank_label() {
        // T5. Three quarters of the corpus looks like this, and a reader that
        // turned a blank into `references` on write would lose the difference
        // between "the corpus said nothing" and "the corpus said reference".
        let mut edge = edge();
        edge.source_label = String::new();
        edge.edge_type = EdgeType::References;

        let dir = std::env::temp_dir().join("girsa-link-store-blank");
        let _ = fs::remove_dir_all(&dir);
        let mut writer = Writer::default();
        writer.push(&edge);
        writer.flush(&dir).expect("writes");
        let back = read_back(&dir, "mishnah/berakhot").expect("reads");
        assert_eq!(back[0].source_label, "");
        assert_eq!(back[0].edge_type, EdgeType::References);
        let _ = fs::remove_dir_all(&dir);
    }
}
