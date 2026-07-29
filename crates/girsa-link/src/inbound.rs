//! The other half of every segment's links, cached where they land.
//!
//! An edge is stored **once, in the shard of the work it points from**
//! (spec.md §8.2), and which end that is was decided by whoever wrote the row.
//! Berakhot's own shard holds the 51,927 edges from Berakhot out to its
//! commentaries; the 18,806 edges the Mishnah Berurah makes onto the Shulchan
//! Arukh are in the **Mishnah Berurah's** shard, not the Shulchan Arukh's.
//!
//! So *what links to this segment* has, until now, been answered by reading the
//! shards of every work the companions cache says is joined to this one — up to
//! 200 files, some of them 16 MB. That is affordable for a sidebar drawn once
//! when a line is selected. It is not affordable for W28, where following a
//! chain means asking the same question again at every hop, of a different work
//! each time.
//!
//! This is the same trade `touching.jsonl` already made and won: walk the graph
//! **once**, and write each edge a second time into the file of the work its
//! far end lands in.
//!
//! ```jsonl
//! corpus/links/shulchan-arukh/orach-chayim/inbound.jsonl
//! {"from":"girsa:mishnah-berurah/58:1#1","to":"girsa:shulchan-arukh/orach-chayim/58:1#1",…}
//! ```
//!
//! Identical rows to `edges.jsonl`, read by the same reader
//! ([`crate::store::read_edges`]), so the two halves of a segment's links
//! cannot come to mean different things.
//!
//! # An edge inside one work is not written here
//!
//! Its own shard already holds it, and a caller that reads both files wants
//! their union to be *the edges touching this work, each once*. Writing a
//! same-work edge into both would show every one of the Tur's 1,061 internal
//! links twice, in a list the reader has no way to tell is doubled.
//!
//! # It is a cache and it is allowed to be missing
//!
//! spec.md §4.1: the text files are the truth and anything faster is
//! rebuildable. Delete it and run `girsa-link-types` again. What must never
//! happen is a caller reading a missing cache as **an answer** — *nothing links
//! here* and *I have not been told what does* are different statements, which
//! is why [`built`] exists and why every caller of it says which one it saw.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use girsa_corpus::import::slug_dir;

use crate::store::Row;
use crate::Edge;

/// Where a work's incoming edges live.
#[must_use]
pub fn inbound_path(root: &Path, slug: &str) -> PathBuf {
    slug_dir(&root.join("links"), slug).join("inbound.jsonl")
}

/// Whether the cache has been built at all.
///
/// Asked of the tree, not of one work: a work with no `inbound.jsonl` in a
/// tree that has them is a work nothing links to, and the same work in a tree
/// that has none is a question nobody has answered. The marker is written by
/// [`Writer::flush`] on its first run.
#[must_use]
pub fn built(root: &Path) -> bool {
    root.join("links/inbound.built").is_file()
}

/// Collects edges by the work their far end lands in, and writes one file per
/// work.
///
/// The same discipline as [`crate::store::Writer`] and for the same reason: a
/// run is many flushes, so a file is appended to within a run — and appending
/// to what the **last** run left would silently double every incoming link.
#[derive(Debug, Default)]
pub struct Writer {
    by_work: BTreeMap<String, String>,
    written: usize,
    opened: BTreeSet<String>,
    /// Edges whose two ends are in one work, which its own shard already holds.
    internal: usize,
}

impl Writer {
    /// Record an edge against the work its **`to`** end lands in.
    ///
    /// Takes the row as it was read rather than a parsed [`Edge`]: this runs
    /// four million times over text that came off disk, and re-serialising a
    /// parsed edge would spend the walk's whole budget proving the text equals
    /// itself.
    pub fn push_row(&mut self, from_work: &str, to_work: &str, line: &str) {
        if from_work == to_work {
            self.internal += 1;
            return;
        }
        let body = self.by_work.entry(to_work.to_string()).or_default();
        body.push_str(line.trim_end());
        body.push('\n');
        self.written += 1;
    }

    /// Record a parsed edge. For callers that have one — tests, and anything
    /// building a small graph in memory.
    pub fn push(&mut self, edge: &Edge) {
        let (from_work, to_work) = (edge.from.from.work(), edge.to.from.work());
        let Ok(line) = serde_json::to_string(&Row::of(edge)) else {
            return;
        };
        self.push_row(from_work, to_work, &line);
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.written
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.written == 0
    }

    /// Edges skipped because both ends are in one work, which is not a loss —
    /// that work's own shard holds them.
    #[must_use]
    pub const fn internal(&self) -> usize {
        self.internal
    }

    /// How many bytes are being held, so a caller can flush before memory
    /// becomes the reason the run did not finish.
    #[must_use]
    pub fn buffered_bytes(&self) -> usize {
        self.by_work.values().map(String::len).sum()
    }

    /// Write everything held and forget it, and mark the tree as built.
    ///
    /// # Errors
    ///
    /// If a file cannot be created or written to.
    pub fn flush(&mut self, root: &Path) -> Result<(), std::io::Error> {
        let links = root.join("links");
        if !links.is_dir() {
            fs::create_dir_all(&links)?;
        }
        for (slug, body) in std::mem::take(&mut self.by_work) {
            let path = inbound_path(root, &slug);
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
        // Last, so a run killed part-way leaves the tree marked unbuilt rather
        // than marked built and half full.
        fs::write(
            links.join("inbound.built"),
            "GENERATED by girsa-link-types. A cache: delete this tree's \
             inbound.jsonl files and this marker together, and run it again.\n",
        )
    }
}

/// Read the edges that land **on** one work.
///
/// # Errors
///
/// If the file exists and cannot be read. A work with no file is a work
/// nothing links to — provided [`built`] is true, which is the caller's to
/// check.
pub fn read_back(root: &Path, slug: &str) -> Result<Vec<Edge>, std::io::Error> {
    crate::store::read_edges(&inbound_path(root, slug))
}

/// Every edge touching a work, each exactly once: the ones it makes and the
/// ones made onto it.
///
/// `None` for the incoming half when the cache has not been built, so a caller
/// reports *not known* rather than *none*.
///
/// # Errors
///
/// If either file exists and cannot be read.
pub fn touching_work(root: &Path, slug: &str) -> Result<(Vec<Edge>, bool), std::io::Error> {
    let mut edges = crate::store::read_back(root, slug)?;
    if !built(root) {
        return Ok((edges, false));
    }
    edges.extend(read_back(root, slug)?);
    Ok((edges, true))
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::{store, Anchor, EdgeType, Method};
    use girsa_corpus::segment::{Ordinal, SegmentId};

    fn id(work: &str, n: u32) -> SegmentId {
        SegmentId::new(work, vec!["1".into(), n.to_string()], Ordinal::root(n))
    }

    fn edge(from: &str, to: &str) -> Edge {
        Edge {
            from: Anchor::point(id(from, 1)),
            to: Anchor::point(id(to, 5)),
            edge_type: EdgeType::CommentsOn,
            method: Method::SefariaSeed,
            source_label: "commentary".into(),
        }
    }

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn an_edge_is_readable_from_the_end_it_was_not_stored_under() {
        // The whole reason this file exists. The Mishnah Berurah's shard holds
        // 18,806 edges onto the Shulchan Arukh; the Shulchan Arukh's own shard
        // holds none of them, and *what does this se'if answer to* is asked
        // from the Shulchan Arukh's side every time.
        let root = dir("girsa-inbound-far-end");
        let mut writer = Writer::default();
        writer.push(&edge("mishnah-berurah", "shulchan-arukh/orach-chayim"));
        writer.flush(&root).expect("writes");

        let onto = read_back(&root, "shulchan-arukh/orach-chayim").expect("reads");
        assert_eq!(onto.len(), 1);
        assert_eq!(onto[0].from.from.work(), "mishnah-berurah");
        assert!(
            read_back(&root, "mishnah-berurah")
                .expect("reads")
                .is_empty(),
            "the end it was stored under keeps it in edges.jsonl, not here"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_edge_inside_one_work_is_not_counted_twice() {
        // The Tur makes 1,061 links to itself. Its own shard holds them, so a
        // caller that reads both files would show every one of them twice with
        // nothing on the row to say so.
        let root = dir("girsa-inbound-internal");
        let mut store_writer = store::Writer::default();
        let mut writer = Writer::default();
        let internal = edge("tur", "tur");
        store_writer.push(&internal);
        writer.push(&internal);
        store_writer.flush(&root).expect("writes");
        writer.flush(&root).expect("writes");

        assert_eq!(writer.internal(), 1);
        let (all, known) = touching_work(&root, "tur").expect("reads");
        assert!(known);
        assert_eq!(all.len(), 1, "once, from its own shard");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_tree_with_no_cache_says_so_rather_than_saying_none() {
        // *Nothing links here* and *I have not been told what does* are
        // different statements. A trace that read the second as the first would
        // report a sefer as a dead end because a batch job had not been run.
        let root = dir("girsa-inbound-unbuilt");
        let mut store_writer = store::Writer::default();
        store_writer.push(&edge("mishnah-berurah", "shulchan-arukh/orach-chayim"));
        store_writer.flush(&root).expect("writes");

        assert!(!built(&root));
        let (edges, known) = touching_work(&root, "shulchan-arukh/orach-chayim").expect("reads");
        assert!(!known, "the incoming half is unknown, not empty");
        assert!(edges.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn running_it_twice_does_not_double_the_incoming_half() {
        // The bug the link importer had once and the type walker had once: a
        // run is many flushes, so a file is appended to within a run, and
        // appending to the last run's file doubles every row with no error.
        let root = dir("girsa-inbound-rerun");
        for _ in 0..2 {
            let mut writer = Writer::default();
            writer.push(&edge("a", "b"));
            writer.flush(&root).expect("writes");
            writer.push(&edge("c", "b"));
            writer.flush(&root).expect("writes again");
        }
        assert_eq!(read_back(&root, "b").expect("reads").len(), 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_row_is_the_same_row_the_outgoing_shard_holds() {
        // Both halves are read by `store::read_edges`, so a blank label still
        // reads back blank here (T5) and a run endpoint still reads back a run.
        let root = dir("girsa-inbound-same-row");
        let mut written = edge("a", "b");
        written.source_label = String::new();
        written.edge_type = EdgeType::References;
        written.to = Anchor::span(id("b", 5), id("b", 9));

        let mut writer = Writer::default();
        writer.push(&written);
        writer.flush(&root).expect("writes");
        let back = read_back(&root, "b").expect("reads");
        assert_eq!(back, vec![written]);
        let _ = fs::remove_dir_all(&root);
    }
}
