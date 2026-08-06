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
use girsa_corpus::standing::Standing;

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
    read_edges_landing(path, &Landing::everything())
}

/// One row, understood. `None` for a line that will not parse, which costs that
/// edge and not the file.
fn edge_of(line: &str) -> Option<Edge> {
    let row = serde_json::from_str::<Row>(line).ok()?;
    let (from, to) = (parse_anchor(&row.from)?, parse_anchor(&row.to)?);
    Some(Edge {
        from,
        to,
        edge_type: EdgeType::from_sefaria(&row.label),
        method: if row.method == Method::OtzariaSeed.as_str() {
            Method::OtzariaSeed
        } else {
            Method::SefariaSeed
        },
        source_label: row.label,
    })
}

/// Which rows are worth building an [`Edge`] out of.
///
/// The panel reads a shard, turns every row into an `Edge`, and keeps the ones
/// that name the line you are on: 63 of 159,273 for a se'if of Orach Chayim. The
/// other 159,210 cost a JSON parse, two [`girsa_corpus::segment::SegmentId`] parses and three
/// allocations each, and are then dropped. This is the gate that lets them be
/// skipped as text.
///
/// # It is deliberately generous
///
/// A row this admits is still tested by [`Anchor::names`] afterwards, so a
/// **false positive costs one parse** and changes no answer. A false negative
/// loses a link silently, which is the failure this whole crate is arranged
/// against. Every judgement call below is therefore resolved towards admitting.
///
/// # What it looks for
///
/// The ordinal, spelled the way a row spells it. An id's text ends at the
/// ordinal, and [`girsa_corpus::segment::SegmentId::is_well_formed`] bans `#` from the work slug and
/// from every section name, so the ordinal is whatever follows the last `#` —
/// and it is followed either by the closing quote of the JSON string, or by `-`
/// where the id is the near end of a run. So `#7"` and `#7-`, which cannot be
/// satisfied by `#7.1` or by `#17`.
///
/// **The ordinal and not the whole id**, because the path is the human address
/// and the ordinal is the durable name (spec.md §3): an edge written down before
/// upstream re-sectioned a work still carries the old address. The work is not
/// compared at all, because the end being gated on is always in this file's own
/// work — a shard holds the edges a work points *from*, and `inbound.jsonl` the
/// ones that land *in* it.
///
/// **And every row carrying `-girsa:` is admitted whatever it says**, because a
/// run covers what sorts between its ends and need not name this place at
/// either of them. That is 2,041 rows of Orach Chayim's 159,273 — 1.3%, and the
/// price of not having to think about whether a daf-wide citation was missed.
///
/// # Searched over the whole line, not over the `to` field
///
/// Finding a field would mean scanning to its closing quote, and a Sefaria
/// section name really can carry an ASCII `"` — Hebrew abbreviations are full of
/// them — which JSON escapes and a naive scan would stop at. Getting that wrong
/// drops rows. Searching the raw line cannot: at worst the ordinal matches on
/// the other end of the edge, and that row gets parsed and then rejected on the
/// merits.
#[derive(Debug, Clone)]
pub struct Landing {
    /// `#7"` and `#7-`, for every name the place answers to.
    needles: Vec<String>,
    /// Read everything, because something has moved and the gate cannot know
    /// where without looking. See [`Landing::also_moved`].
    everything: bool,
}

impl Landing {
    /// The rows that might name this place.
    #[must_use]
    pub fn naming(standing: &Standing) -> Self {
        let mut needles = Vec::with_capacity(standing.len() * 2);
        for name in standing.names() {
            let ordinal = name.ordinal().to_string();
            needles.push(format!("#{ordinal}\""));
            needles.push(format!("#{ordinal}-"));
        }
        Self {
            needles,
            everything: false,
        }
    }

    /// No gate at all — build every row.
    #[must_use]
    pub fn everything() -> Self {
        Self {
            needles: Vec::new(),
            everything: true,
        }
    }

    /// Also admit the edge a repair moved, named as it is filed —
    /// `"{from} → {to}"` of the edge as it was shipped.
    ///
    /// A [`crate::repair::Repair::Reanchored`] re-points an edge, so one whose
    /// stored ends are nowhere near this line can have been moved **onto** it by
    /// hand. Gating without this would silently drop exactly the links a reader
    /// edited themselves, which would be the worst possible thing to lose.
    /// Every ordinal in the filed name is added, because a repair may move
    /// either end.
    pub fn also_moved(&mut self, name: &str) {
        let mut rest = name;
        while let Some(at) = rest.find('#') {
            rest = &rest[at + 1..];
            let ordinal: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if ordinal.is_empty() {
                continue;
            }
            self.needles.push(format!("#{ordinal}\""));
            self.needles.push(format!("#{ordinal}-"));
        }
    }

    /// Whether a raw row is worth parsing.
    #[must_use]
    pub fn admits(&self, line: &str) -> bool {
        if self.everything {
            return true;
        }
        // A run, whichever end it names. See the note on runs above.
        if line.contains("-girsa:") {
            return true;
        }
        self.needles.iter().any(|needle| line.contains(needle))
    }
}

/// The edges of one file that might name a place, without building the rest.
///
/// Same reader and same rows as [`read_edges`] — it *is* `read_edges` with a
/// gate in front — so the two cannot come to mean different things. Handing it
/// [`Landing::everything`] gives exactly `read_edges`.
///
/// # Errors
///
/// If the file exists and cannot be read.
pub fn read_edges_landing(path: &Path, wanted: &Landing) -> Result<Vec<Edge>, std::io::Error> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let body = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        if !wanted.admits(line) {
            continue;
        }
        if let Some(edge) = edge_of(line) {
            out.push(edge);
        }
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

    /// The row `Writer` would write for an edge landing on `#n` of this work.
    ///
    /// The far end sits at an ordinal nothing else here uses, because the gate
    /// searches the whole line and would otherwise be admitting these rows on
    /// the commentary's ordinal rather than the one under test — see
    /// [`tests::the_far_end_can_admit_a_row_and_that_is_the_cheap_mistake`].
    fn row_landing_on(to: &SegmentId) -> String {
        serde_json::to_string(&Row::of(&Edge {
            from: Anchor::point(id("mishnah-berurah", 1, 50_000)),
            to: Anchor::point(to.clone()),
            edge_type: EdgeType::CommentsOn,
            method: Method::SefariaSeed,
            source_label: "commentary".into(),
        }))
        .expect("serializes")
    }

    fn standing_on(to: &SegmentId) -> girsa_corpus::standing::Standing {
        girsa_corpus::standing::Standing::just(to.clone())
    }

    #[test]
    fn an_ordinal_is_not_admitted_by_a_name_that_merely_starts_with_it() {
        // `#7` against `#7.1` — a cut piece, or a se'if upstream inserted — and
        // against `#17`. The gate matches on the ordinal *and its terminator*
        // for exactly this reason; a bare `contains("#7")` would admit both, and
        // while a false positive is only a wasted parse, being wrong here at
        // scale is the difference between skipping a shard and reading it.
        let seven = id("shulchan-arukh/orach-chayim", 1, 7);
        let wanted = Landing::naming(&standing_on(&seven));

        assert!(wanted.admits(&row_landing_on(&seven)));
        for other in [
            SegmentId::new(
                "shulchan-arukh/orach-chayim",
                vec!["1".into(), "7".into()],
                Ordinal::root(7).child(1),
            ),
            id("shulchan-arukh/orach-chayim", 1, 17),
            id("shulchan-arukh/orach-chayim", 1, 70),
        ] {
            assert!(
                !wanted.admits(&row_landing_on(&other)),
                "{other} is a different place from {seven}"
            );
        }
    }

    #[test]
    fn a_run_is_admitted_whatever_it_names() {
        // `Rashi on Berakhot 2a` covers a daf, and a run covers what sorts
        // between its ends — so it can land on this line while naming neither
        // it nor anything like it. 2,041 of Orach Chayim's 159,273 inbound rows
        // are runs; admitting all of them is the price of never having to work
        // out whether one was missed.
        let far = Anchor::span(id("bavli/berakhot", 1, 400), id("bavli/berakhot", 9, 900));
        let row = serde_json::to_string(&Row::of(&Edge {
            from: Anchor::point(id("rashi/berakhot", 1, 1)),
            to: far,
            edge_type: EdgeType::CommentsOn,
            method: Method::SefariaSeed,
            source_label: "commentary".into(),
        }))
        .expect("serializes");

        let elsewhere = id("bavli/berakhot", 5, 500);
        assert!(Landing::naming(&standing_on(&elsewhere)).admits(&row));
    }

    #[test]
    fn a_stale_address_with_the_right_ordinal_is_admitted() {
        // The ordinal is the durable name and the path is the human address
        // (spec.md §3), so an edge written down before upstream re-sectioned the
        // work carries the old address against the same ordinal. Matching the
        // whole id text would drop it — which is the failure the gate is most
        // at risk of, because it would look like a link that was simply never
        // there.
        let now = SegmentId::new(
            "shulchan-arukh/orach-chayim",
            vec!["4".into(), "2".into()],
            Ordinal::root(11),
        );
        let then = SegmentId::new(
            "shulchan-arukh/orach-chayim",
            vec!["3".into(), "9".into()],
            Ordinal::root(11),
        );
        assert_ne!(then.to_string(), now.to_string(), "the addresses differ");
        assert!(Landing::naming(&standing_on(&now)).admits(&row_landing_on(&then)));
    }

    #[test]
    fn a_link_moved_by_hand_survives_the_gate() {
        // `Repair::Reanchored` puts an edge somewhere its stored ends do not
        // say it is. The gate cannot know that without reading the rows, so the
        // filed name of every moved edge is fed back into it. Without this, the
        // links a reader edited themselves would be the ones the panel dropped.
        let stored = id("shulchan-arukh/orach-chayim", 9, 900);
        let here = id("shulchan-arukh/orach-chayim", 1, 1);
        let mut wanted = Landing::naming(&standing_on(&here));
        assert!(!wanted.admits(&row_landing_on(&stored)), "not yet");

        wanted.also_moved(&format!(
            "{} → {}",
            Anchor::point(id("mishnah-berurah", 1, 1)),
            Anchor::point(stored.clone())
        ));
        assert!(wanted.admits(&row_landing_on(&stored)));
    }

    #[test]
    fn the_far_end_can_admit_a_row_and_that_is_the_cheap_mistake() {
        // The gate searches the raw line rather than picking the `to` field out
        // of it, because a Sefaria section name can carry an ASCII `"` — Hebrew
        // abbreviations are full of them — and scanning to a closing quote would
        // stop early and drop the row. The cost of not parsing JSON to gate JSON
        // is that the *other* end's ordinal can admit a row too.
        //
        // That is the mistake worth making. This row is parsed and then rejected
        // by `names`, so the answer is unchanged and the bill is one parse. The
        // opposite trade — a tighter gate that occasionally drops a row — would
        // lose a link with nothing anywhere saying so.
        let here = id("shulchan-arukh/orach-chayim", 1, 3);
        let elsewhere = id("shulchan-arukh/orach-chayim", 9, 900);
        let row = serde_json::to_string(&Row::of(&Edge {
            // The commentary happens to sit at ordinal 3 of its own sefer.
            from: Anchor::point(id("mishnah-berurah", 1, 3)),
            to: Anchor::point(elsewhere.clone()),
            edge_type: EdgeType::CommentsOn,
            method: Method::SefariaSeed,
            source_label: "commentary".into(),
        }))
        .expect("serializes");

        let standing = standing_on(&here);
        assert!(
            Landing::naming(&standing).admits(&row),
            "admitted on the far end's ordinal"
        );
        let edge = edge_of(&row).expect("parses");
        assert!(
            !edge.to.names(&standing) && !edge.from.names(&standing),
            "and then rejected on the merits, which is the only thing that decides"
        );
    }

    #[test]
    fn the_gate_and_no_gate_read_the_same_file_the_same_way() {
        // `read_edges` *is* `read_edges_landing` with everything admitted, so
        // the two cannot drift into meaning different things — which is the same
        // argument the module note makes about the outgoing shard and the
        // inbound cache sharing one reader.
        let dir = std::env::temp_dir().join("girsa-link-store-gate");
        let _ = fs::remove_dir_all(&dir);
        let mut writer = Writer::default();
        let here = id("mishnah/berakhot", 1, 1);
        for n in 1..=20 {
            writer.push(&Edge {
                from: Anchor::point(id("mishnah/berakhot", 1, n)),
                to: Anchor::point(id("rambam/berakhot", 1, n)),
                edge_type: EdgeType::CommentsOn,
                method: Method::SefariaSeed,
                source_label: "commentary".into(),
            });
        }
        writer.flush(&dir).expect("writes");
        let path = edges_path(&dir, "mishnah/berakhot");

        let all = read_edges(&path).expect("reads");
        assert_eq!(all.len(), 20);
        let ungated = read_edges_landing(&path, &Landing::everything()).expect("reads");
        assert_eq!(ungated, all);

        let standing = standing_on(&here);
        let gated = read_edges_landing(&path, &Landing::naming(&standing)).expect("reads");
        let kept: Vec<_> = all
            .iter()
            .filter(|edge| edge.from.names(&standing))
            .cloned()
            .collect();
        assert_eq!(kept.len(), 1);
        assert_eq!(
            gated.iter().filter(|e| e.from.names(&standing)).count(),
            kept.len(),
            "the gate kept everything the real test would have"
        );
        let _ = fs::remove_dir_all(&dir);
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
