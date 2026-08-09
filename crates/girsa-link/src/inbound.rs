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

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use girsa_corpus::import::slug_dir;
use girsa_corpus::segment::Ordinal;
use girsa_corpus::standing::Standing;

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

/// Where a work's landing index lives, beside the rows it indexes.
#[must_use]
pub fn landing_path(root: &Path, slug: &str) -> PathBuf {
    slug_dir(&root.join("links"), slug).join("inbound.landing")
}

/// Which rows of a sorted `inbound.jsonl` land where.
///
/// # Why a second file rather than a cleverer first one
///
/// After the text gate, opening a panel costs what it costs to read the rows —
/// 95 ms a line for Orach Chayim's 27 MB, against a 311 ms panel — and nothing
/// done to the rows once they are in hand can get under that. The only way down
/// is to stop reading rows that cannot matter, and that means knowing where they
/// are before opening the file.
///
/// So `inbound.jsonl` is **sorted**: the runs first, then the points in landing
/// order. Sorting is what makes this index small. Rows landing on one segment
/// become contiguous, so the index holds one entry per **distinct landing
/// place** — 4,171 for Orach Chayim, against 159,273 rows — and a lookup is a
/// `binary_search` over a slice in memory, not a hand-rolled seek over a file,
/// which is the kind of thing that goes subtly wrong and loses links quietly.
///
/// ```jsonl
/// {"runs":352104}
/// {"at":[1],"from":352104,"len":1871}
/// {"at":[2],"from":353975,"len":902}
/// ```
///
/// The first line is where the runs end. A run covers what sorts between its
/// ends and so lands on places it does not name — there is no one ordinal to
/// file it under — so all of them sit in a block at the head and are read every
/// time. That is 1.3% of the rows and it is why the head is where they go.
///
/// # It is a cache of a cache, and may be missing
///
/// spec.md §4.1. No index, or a tree whose rows were never sorted, means the
/// text gate does the work instead — slower, and the same answers, because both
/// paths hand what they find to the same [`Anchor::names`] test. That is the
/// only reason a second path is allowed to exist here: it can differ in speed
/// and it cannot differ in answers.
#[derive(Debug, Clone, Default)]
pub struct Landings {
    /// Byte length of the leading block of runs.
    runs: u64,
    /// Landing ordinal, and the byte range of the rows landing there. Sorted by
    /// ordinal, which is what makes the lookup a binary search.
    places: Vec<(Ordinal, u64, u64)>,
}

/// One line of the index.
#[derive(serde::Serialize, serde::Deserialize)]
struct Line {
    /// Serialised as the dotted sequence's own numbers — `[7,1]` for `#7.1` —
    /// because that is what an `Ordinal` is, and a cache should not invent a
    /// second spelling of a thing that already has one.
    at: Ordinal,
    from: u64,
    len: u64,
}

/// The first line of the index.
#[derive(serde::Serialize, serde::Deserialize)]
struct Head {
    runs: u64,
}

impl Landings {
    /// Read one work's index, if it has one.
    #[must_use]
    pub fn of(root: &Path, slug: &str) -> Option<Self> {
        let body = fs::read_to_string(landing_path(root, slug)).ok()?;
        let mut lines = body.lines();
        let head: Head = serde_json::from_str(lines.next()?).ok()?;
        let mut places = Vec::new();
        for line in lines.filter(|l| !l.trim().is_empty()) {
            let row: Line = serde_json::from_str(line).ok()?;
            places.push((row.at, row.from, row.len));
        }
        // A file whose entries are not in order would make the binary search
        // below quietly wrong, which is worse than having no index at all.
        if places.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
            return None;
        }
        Some(Self {
            runs: head.runs,
            places,
        })
    }

    /// The byte ranges worth reading for a place, runs included.
    #[must_use]
    pub fn ranges_for(&self, standing: &Standing) -> Vec<(u64, u64)> {
        let mut out = Vec::with_capacity(standing.len() + 1);
        if self.runs > 0 {
            out.push((0, self.runs));
        }
        for name in standing.names() {
            if let Ok(at) = self
                .places
                .binary_search_by(|(ordinal, _, _)| ordinal.cmp(name.ordinal()))
            {
                let (_, from, len) = self.places[at];
                out.push((from, len));
            }
        }
        out
    }
}

/// Collects edges by the work their far end lands in, and writes one file per
/// work.
///
/// The same discipline as [`crate::store::Writer`] and for the same reason: a
/// run is many flushes, so a file is appended to within a run — and appending
/// to what the **last** run left would silently double every incoming link.
///
/// That paragraph used to sit above a second copy of it. It is
/// [`crate::shards`] now, and this struct is what is genuinely different: which
/// end of an edge decides the file, and the marker written at the end.
#[derive(Debug, Default)]
pub struct Writer {
    shards: crate::shards::Shards,
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
        self.shards.add(to_work, line);
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
        self.shards.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.shards.len() == 0
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
        self.shards.buffered_bytes()
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
        self.shards.flush(root, inbound_path)?;
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

/// The same, keeping only the rows that might land on one place.
///
/// This file is the reason the gate exists. It holds every edge in the corpus
/// that lands anywhere in the work — 159,273 of them for Orach Chayim — and a
/// reader standing on a line wants the sixty-odd that name it. Building the
/// other 159,210 and dropping them was 43% of the wait before a panel appeared.
/// See [`crate::store::Landing`], which is generous on purpose.
///
/// # Errors
///
/// If the file exists and cannot be read.
pub fn read_landing(
    root: &Path,
    slug: &str,
    wanted: &crate::store::Landing,
) -> Result<Vec<Edge>, std::io::Error> {
    crate::store::read_edges_landing(&inbound_path(root, slug), wanted)
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

/// Sort one work's rows by where they land, and write the index beside them.
///
/// Idempotent: sorting a sorted file gives the same file, so this can be run
/// over a tree as many times as somebody likes. Returns how many places the
/// index names.
///
/// # Errors
///
/// If the file cannot be rewritten, or the index cannot be written. A work with
/// no inbound rows is not an error; it is a work nothing links to.
pub fn sort_and_index(root: &Path, slug: &str) -> Result<usize, std::io::Error> {
    sort_and_index_at(&inbound_path(root, slug))
}

/// The same, for a caller walking the tree and holding the file rather than the
/// slug — `girsa-link-types` after a rebuild, and `--example sort-inbound` over
/// a tree somebody already has.
///
/// # Errors
///
/// If the file cannot be rewritten, or the index cannot be written.
pub fn sort_and_index_at(path: &Path) -> Result<usize, std::io::Error> {
    let Ok(body) = fs::read_to_string(path) else {
        return Ok(0);
    };

    let mut runs: Vec<&str> = Vec::new();
    let mut points: Vec<(Ordinal, &str)> = Vec::new();
    let mut unreadable: Vec<&str> = Vec::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        match where_it_lands(line) {
            Where::Run => runs.push(line),
            Where::At(ordinal) => points.push((ordinal, line)),
            Where::Unreadable => unreadable.push(line),
        }
    }
    // Stable, so two rows landing in the same place keep the order the walk
    // gave them and a re-sort is not a diff.
    points.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::with_capacity(body.len());
    for line in &runs {
        out.push_str(line);
        out.push('\n');
    }
    let runs_bytes = out.len() as u64;

    let mut places: Vec<(Ordinal, u64, u64)> = Vec::new();
    for (ordinal, line) in &points {
        let at = out.len() as u64;
        out.push_str(line);
        out.push('\n');
        let len = out.len() as u64 - at;
        match places.last_mut() {
            Some((last, _, held)) if last == ordinal => *held += len,
            _ => places.push((ordinal.clone(), at, len)),
        }
    }
    for line in &unreadable {
        out.push_str(line);
        out.push('\n');
    }

    let mut index = serde_json::to_string(&Head { runs: runs_bytes })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    index.push('\n');
    for (at, from, len) in &places {
        let line = serde_json::to_string(&Line {
            at: at.clone(),
            from: *from,
            len: *len,
        })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        index.push_str(&line);
        index.push('\n');
    }

    // This rewrites a file in place, and the file is 27 MB of somebody's corpus.
    // Re-deriving it means walking 4.18M edges again, so the one thing that must
    // not happen is a sort that quietly drops rows. Counted rather than trusted:
    // the check is cheap and the failure it guards against is not.
    let was = body.lines().filter(|l| !l.trim().is_empty()).count();
    let is = out.lines().filter(|l| !l.trim().is_empty()).count();
    if was != is {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "{}: sorting would have written {is} rows where there were {was} — refusing",
                path.display()
            ),
        ));
    }

    // The rows first. An index naming offsets into a file that was not written
    // would send a reader to the wrong bytes; an index that is missing only
    // costs them the slower path.
    fs::write(path, out.as_bytes())?;
    fs::write(path.with_file_name("inbound.landing"), index.as_bytes())?;
    Ok(places.len())
}

/// The edges landing on one place, read out of the rows that hold them.
///
/// `None` when this work has no index, or has one that does not agree with
/// itself — the caller then falls back to [`read_landing`], which is slower and
/// gives the same answers.
///
/// # It does not know about repairs
///
/// A [`crate::repair::Repair::Reanchored`] moves an edge to a place its stored
/// row does not mention, and no index over stored rows can find it. A caller
/// with any moved edge in its layer must take the gate instead —
/// `girsa_app::touching` asks `Repairs::moves_anything` and does.
#[must_use]
pub fn read_at(root: &Path, slug: &str, at: &Standing) -> Option<Vec<Edge>> {
    let index = Landings::of(root, slug)?;
    let mut file = fs::File::open(inbound_path(root, slug)).ok()?;
    let mut out = Vec::new();
    for (from, len) in index.ranges_for(at) {
        file.seek(SeekFrom::Start(from)).ok()?;
        let mut held = vec![0u8; usize::try_from(len).ok()?];
        file.read_exact(&mut held).ok()?;
        let text = String::from_utf8(held).ok()?;
        // Ranges are line-aligned by construction, so nothing here is a
        // fragment of a row.
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            if let Some(edge) = crate::store::edge_of(line) {
                out.push(edge);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod landing_tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::{Anchor, EdgeType, Method};
    use girsa_corpus::segment::SegmentId;

    const SEFER: &str = "shulchan-arukh/orach-chayim";

    fn seif(n: u32) -> SegmentId {
        SegmentId::new(SEFER, vec!["1".into(), n.to_string()], Ordinal::root(n))
    }

    fn commentary(n: u32) -> SegmentId {
        SegmentId::new(
            "mishnah-berurah",
            vec!["1".into(), n.to_string()],
            Ordinal::root(n + 9000),
        )
    }

    fn onto(to: Anchor, from: u32) -> Edge {
        Edge {
            from: Anchor::point(commentary(from)),
            to,
            edge_type: EdgeType::CommentsOn,
            method: Method::SefariaSeed,
            direction: crate::Direction::NotRecorded,
            source_label: "commentary".into(),
        }
    }

    /// An inbound file with the shape a real one has: points scattered out of
    /// order, and a handful of runs that cover places they do not name.
    fn a_cache(dir: &Path) -> Vec<Edge> {
        let mut written = Vec::new();
        let mut writer = Writer::default();
        // Out of landing order on purpose — the walk that builds this file goes
        // work by work, not place by place.
        for n in [7u32, 2, 40, 2, 13, 7, 1, 40, 2] {
            let edge = onto(Anchor::point(seif(n)), n);
            writer.push_row(
                edge.from.from.work(),
                SEFER,
                &serde_json::to_string(&crate::store::Row::of(&edge)).expect("serializes"),
            );
            written.push(edge);
        }
        // Runs: one covering the middle of the sefer, naming neither end of it.
        for (lo, hi) in [(5u32, 20u32), (30, 45)] {
            let edge = onto(Anchor::span(seif(lo), seif(hi)), lo);
            writer.push_row(
                edge.from.from.work(),
                SEFER,
                &serde_json::to_string(&crate::store::Row::of(&edge)).expect("serializes"),
            );
            written.push(edge);
        }
        writer.flush(dir).expect("writes");
        written
    }

    fn standing_on(n: u32) -> Standing {
        Standing::just(seif(n))
    }

    /// What the panel would keep, whichever way the rows arrived.
    fn kept(edges: &[Edge], at: &Standing) -> Vec<String> {
        let mut out: Vec<String> = edges
            .iter()
            .filter(|edge| edge.to.names(at))
            .map(|edge| format!("{} → {}", edge.from, edge.to))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn the_index_and_the_gate_answer_alike_for_every_place() {
        // The only thing that licenses a second read path: it may be faster and
        // it may not be different. Asked of every segment the fixture touches,
        // including the ones nothing lands on.
        let dir = std::env::temp_dir().join("girsa-inbound-landing");
        let _ = fs::remove_dir_all(&dir);
        let all = a_cache(&dir);

        let places = sort_and_index_at(&inbound_path(&dir, SEFER)).expect("sorts");
        assert_eq!(places, 5, "1, 2, 7, 13 and 40 are landed on");

        for n in 1..=46u32 {
            let at = standing_on(n);
            let gated = read_landing(&dir, SEFER, &crate::store::Landing::naming(&at))
                .expect("the gate reads");
            let indexed = read_at(&dir, SEFER, &at).expect("the index reads");
            assert_eq!(
                kept(&indexed, &at),
                kept(&gated, &at),
                "the two paths disagree about se'if {n}"
            );
            assert_eq!(
                kept(&indexed, &at),
                kept(&all, &at),
                "and about what was written in the first place"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_run_reaches_a_place_it_does_not_name() {
        // The reason the runs sit in a block at the head rather than being filed
        // under an ordinal: `5..20` lands on se'if 13 while naming neither 5 nor
        // 13. An index that filed runs by their near end would lose it.
        let dir = std::env::temp_dir().join("girsa-inbound-runs");
        let _ = fs::remove_dir_all(&dir);
        a_cache(&dir);
        sort_and_index_at(&inbound_path(&dir, SEFER)).expect("sorts");

        let at = standing_on(13);
        let found = read_at(&dir, SEFER, &at).expect("the index reads");
        assert!(
            found
                .iter()
                .any(|edge| edge.to.is_span() && edge.to.names(&at)),
            "the run covering se'if 13 is in the answer"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sorting_twice_changes_nothing() {
        // It runs over a tree somebody already has, and will be run again by the
        // next rebuild. A pass that is not idempotent would make every run a
        // diff and every diff a reason to wonder.
        let dir = std::env::temp_dir().join("girsa-inbound-twice");
        let _ = fs::remove_dir_all(&dir);
        a_cache(&dir);
        let path = inbound_path(&dir, SEFER);

        sort_and_index_at(&path).expect("sorts");
        let once = fs::read_to_string(&path).expect("reads");
        let index_once = fs::read_to_string(landing_path(&dir, SEFER)).expect("reads");
        sort_and_index_at(&path).expect("sorts again");
        assert_eq!(fs::read_to_string(&path).expect("reads"), once);
        assert_eq!(
            fs::read_to_string(landing_path(&dir, SEFER)).expect("reads"),
            index_once
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sorting_keeps_every_row() {
        // A sort that drops rows is not a sort. Rows that will not parse are
        // kept too, at the end, where the index does not point — the same rows
        // the gate would fail to understand, and no fewer.
        let dir = std::env::temp_dir().join("girsa-inbound-keeps");
        let _ = fs::remove_dir_all(&dir);
        let all = a_cache(&dir);
        let path = inbound_path(&dir, SEFER);
        let before = fs::read_to_string(&path).expect("reads");

        sort_and_index_at(&path).expect("sorts");
        let after = fs::read_to_string(&path).expect("reads");
        let mut was: Vec<&str> = before.lines().filter(|l| !l.trim().is_empty()).collect();
        let mut is: Vec<&str> = after.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(is.len(), all.len());
        was.sort_unstable();
        is.sort_unstable();
        assert_eq!(is, was, "the same rows, in a different order");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tree_with_no_index_is_read_the_slower_way_and_not_wrongly() {
        // spec.md §4.1 — this is a cache of a cache. Its absence is a speed, not
        // an answer.
        let dir = std::env::temp_dir().join("girsa-inbound-noindex");
        let _ = fs::remove_dir_all(&dir);
        let all = a_cache(&dir);
        let at = standing_on(7);
        assert!(read_at(&dir, SEFER, &at).is_none(), "nothing to read yet");

        let gated =
            read_landing(&dir, SEFER, &crate::store::Landing::naming(&at)).expect("the gate reads");
        assert_eq!(kept(&gated, &at), kept(&all, &at));
        let _ = fs::remove_dir_all(&dir);
    }
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
            direction: crate::Direction::NotRecorded,
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

/// Where one row lands, for sorting it.
enum Where {
    /// A run, which covers what sorts between its ends and so lands on places
    /// it does not name. There is no one ordinal to file it under.
    Run,
    At(Ordinal),
    /// A line that will not parse. It keeps its place at the end of the file:
    /// this is a sort, and a sort that drops rows is not one.
    Unreadable,
}

fn where_it_lands(line: &str) -> Where {
    let Ok(row) = serde_json::from_str::<Row>(line) else {
        return Where::Unreadable;
    };
    if row.to.contains("-girsa:") {
        return Where::Run;
    }
    match row.to.parse::<girsa_corpus::segment::SegmentId>() {
        Ok(id) => Where::At(id.ordinal().clone()),
        Err(_) => Where::Unreadable,
    }
}
