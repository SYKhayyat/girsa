//! Walk the graph once and write the two caches that read it from the other
//! side.
//!
//! ```sh
//! cargo run --release -p girsa-link --bin girsa-link-types -- corpus
//! ```
//!
//! spec.md §8.2 stores an edge once, in the direction it was written — so the
//! two million edges that land **on** Berakhot are scattered across every shard
//! in the corpus and none of them are in Berakhot's own. Two different features
//! need that reversed, and neither can afford to read 665 MB to answer one
//! question:
//!
//! - **`inbound.jsonl`** — the edges themselves, filed under the work their far
//!   end lands in, for W28's chain tracing, which asks *what links here* again
//!   at every hop. Sorted by where its rows land and indexed, so a panel reads
//!   the kilobytes for one place. See [`girsa_link::inbound`].
//! - **`touching.bits`** — which *kinds* of link touch each segment, for §9.8's
//!   link-type facet: one 16-bit mask per segment in reading order. See
//!   [`girsa_link::touching`].
//!
//! One walk writes the first, because the walk is the expensive part: three
//! minutes over 5,790 shards, and doing it twice would be six. The masks are a
//! third pass over what the walk produced, and they have to be, because a mask
//! is **positional** and the walk does not know a work's reading order — only
//! its ids. That is `girsa_corpus::import::ordered_ids`, one work at a time.
//!
//! # The masks used to be 449 MB of prose
//!
//! `touching.jsonl` was one JSON row per `(endpoint, type)` plus the list of
//! every sefer at the other end — 448.7 MB over 6,268 files, read once, at
//! index-build time, to produce nine bits per segment. Orach Chayim's was 4.14
//! MB to say 4,171 numbers; it is now 8.4 KB. The `w` list existed so a phone
//! reader could ask *which of my mefarshim speak here* without reading
//! `inbound.jsonl`, and W28's landing index has since made that a seek. See the
//! module note on `girsa_link::touching`.
//!
//! # They are caches and they are allowed to be missing
//!
//! Same rule as `girsa-companions` (spec.md §4.1): delete them and run this
//! again. What a reader must never do is read a missing cache as a **zero** —
//! *no links of that kind* and *nobody worked out the link types* are different
//! statements, and `girsa-index build` and `girsa_link::inbound::built` each
//! record which one they saw.

// A tool that prints a report. The library it calls does not print.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use girsa_link::store::Row;
use girsa_link::EdgeType;

/// Flush the inbound cache when this many bytes are held. Bytes rather than
/// rows, because a row here is a whole edge and not a two-field summary — the
/// finished cache is the size of the graph again.
const FLUSH_BYTES: usize = 256 * 1024 * 1024;

fn main() -> std::process::ExitCode {
    let Some(root) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: girsa-link-types <corpus-root>");
        return std::process::ExitCode::from(2);
    };
    let links = root.join("links");
    if !links.is_dir() {
        eprintln!(
            "no link graph at {} — run girsa-link-import first",
            links.display()
        );
        return std::process::ExitCode::FAILURE;
    }

    let started = Instant::now();
    let shards = shard_paths(&links);
    eprintln!(
        "walking {} shards under {} …",
        shards.len(),
        links.display()
    );

    let mut inbound = girsa_link::inbound::Writer::default();
    let mut edges = 0usize;
    let mut unreadable = 0usize;
    let mut unparsed = 0usize;
    let mut done = 0usize;

    for path in &shards {
        let Ok(body) = std::fs::read_to_string(path) else {
            unreadable += 1;
            continue;
        };
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(mut row) = serde_json::from_str::<Row>(line) else {
                unparsed += 1;
                continue;
            };
            row.forget_implied_type();
            let (Some(from), Some(to)) = (work_of(&row.from), work_of(&row.to)) else {
                unparsed += 1;
                continue;
            };
            // Re-serialised rather than passed through, and only because the
            // shape changed underneath: every shard on disk carries a `type`
            // that says exactly what its `label` says, and `Row` now writes one
            // only when it is a judgement. Pushing the line as it stands would
            // fill a cache **this tool owns** with 4.1M copies of it. Round-
            // tripping through `Row` costs one `to_string` per edge and buys
            // 12.3% of the file back: `inbound.jsonl` 656.4 MB → 575.6 MB, with
            // 0 of its 4,131,100 rows still carrying a type. The extra
            // serialisation did not show above this pass's own run-to-run
            // noise, which is 89–167s on one machine over one input.
            //
            // The shards keep theirs until the next import, where they simply
            // stop being written. Both read identically, which is the property
            // that made this a format change with no migration.
            let Ok(line) = serde_json::to_string(&row) else {
                unparsed += 1;
                continue;
            };
            inbound.push_row(from, to, &line);
            edges += 1;
        }
        done += 1;
        if inbound.buffered_bytes() >= FLUSH_BYTES {
            if let Err(e) = inbound.flush(&root) {
                eprintln!("cannot write: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
        if done % 500 == 0 {
            eprint!("\r  {done}/{} shards, {edges} edges", shards.len());
        }
    }
    if let Err(e) = inbound.flush(&root) {
        eprintln!("cannot write: {e}");
        return std::process::ExitCode::FAILURE;
    }
    eprintln!(
        "\r  {done}/{} shards, {edges} edges          ",
        shards.len()
    );

    // Sorted by where its rows land, and indexed, so the panel can read the
    // kilobytes that matter instead of the file that holds them. Last, because
    // it rewrites what the walk just wrote; the pass counts its own rows and
    // refuses to write a file it would have shortened.
    let mut indexed = (0usize, 0usize, 0usize);
    for path in inbound_paths(&links) {
        match girsa_link::inbound::sort_and_index_at(&path) {
            Ok(0) => {}
            Ok(places) => {
                indexed.0 += 1;
                indexed.1 += places;
            }
            Err(e) => {
                indexed.2 += 1;
                eprintln!("  {}: not indexed — {e}", path.display());
            }
        }
    }

    // The masks, which are positional and so have to come after everything that
    // could still move a row, and need a reading order the walk does not have.
    let masks = match write_masks(&root) {
        Ok(masks) => masks,
        Err(e) => {
            eprintln!("cannot write the link-type masks: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    println!("two caches written beside the edges:");
    println!("  shards read        {done}");
    println!("  edges              {edges}");
    println!(
        "  link-type masks    {} works, {} segments   ({} works have no segments on this shelf)",
        masks.works, masks.segments, masks.unreadable
    );
    if masks.superseded > 0 {
        println!(
            "  touching.jsonl     {} deleted   (the 449 MB this replaced)",
            masks.superseded
        );
    }
    println!(
        "  inbound rows       {}   ({} skipped — both ends in one work, whose own shard holds them)",
        inbound.len(),
        inbound.internal()
    );
    println!(
        "  landing index      {} works, {} places   (inbound sorted by where its rows land)",
        indexed.0, indexed.1
    );
    println!(
        "  took               {:.0}s",
        started.elapsed().as_secs_f64()
    );
    if indexed.2 > 0 {
        println!(
            "  not indexed        {}   (read the slower way, and named above)",
            indexed.2
        );
    }
    if unreadable > 0 {
        println!("  shards unreadable  {unreadable}");
    }
    if unparsed > 0 {
        println!("  rows unparsed      {unparsed}");
    }

    // A run that read nothing wrote nothing, and an index built after it would
    // show every facet row as zero. Loud, not quiet.
    if edges == 0 {
        eprintln!("\nNo edges were read. The link-type facet would be empty and wrong.");
        return std::process::ExitCode::FAILURE;
    }
    if unreadable > 0 {
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

/// What the mask pass did.
#[derive(Debug, Default)]
struct Masks {
    works: usize,
    segments: usize,
    /// Works in the catalogue whose segments could not be read. Counted rather
    /// than fatal: a catalogue row with no imported text is a work the shelf
    /// knows about and the importer has not reached, and one of those is not a
    /// reason to abandon the other seven thousand.
    unreadable: usize,
    /// `touching.jsonl` files removed. The format this replaced.
    superseded: usize,
}

/// One 16-bit mask per segment of every work, in reading order.
///
/// A third pass rather than part of the walk, and it has to be. A mask is
/// **positional**, the walk knows only ids, and reading order is
/// `segments.jsonl` — so the resolution happens where the order is, one work at
/// a time, off `inbound::touching_work`, which is the union of *what this work
/// points at* and *what points at this work* with each edge exactly once.
///
/// # Errors
///
/// Only if the catalogue itself cannot be read. A single work that cannot be
/// resolved is counted, not fatal.
fn write_masks(root: &Path) -> Result<Masks, std::io::Error> {
    let catalogue = root.join("works/index.jsonl");
    let body = std::fs::read_to_string(&catalogue)?;
    let slugs: Vec<String> = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Slug>(l).ok())
        .map(|w| w.slug)
        .collect();

    let mut out = Masks::default();
    for (done, slug) in slugs.iter().enumerate() {
        if done % 500 == 0 {
            eprint!("\r  masks: {done}/{} works", slugs.len());
        }
        let Ok(ordered) = girsa_corpus::import::ordered_ids(root, slug) else {
            out.unreadable += 1;
            continue;
        };
        if ordered.is_empty() {
            continue;
        }
        let (edges, _) = girsa_link::inbound::touching_work(root, slug).unwrap_or_default();
        // Both ends of every edge, and only the ends that land in *this* work.
        // An edge inside one work contributes both of its ends here, which is
        // what a facet asking "is anything said about this line" wants.
        let mut ends: Vec<(&girsa_link::Anchor, EdgeType)> = Vec::new();
        for edge in &edges {
            if edge.from.from.work() == slug {
                ends.push((&edge.from, edge.edge_type));
            }
            if edge.to.from.work() == slug {
                ends.push((&edge.to, edge.edge_type));
            }
        }
        let masks = girsa_link::touching::masks_for(ends, &ordered);
        girsa_link::touching::write(root, slug, &ordered, &masks)?;
        out.works += 1;
        out.segments += masks.len();

        // The old format, and only once its replacement is on disk. BUILDER.md
        // rule 3 — when a thing is replaced the old thing goes in the same
        // change — but a 4 MB file nothing reads is still better than no file
        // at all if the write above had failed.
        let superseded = girsa_link::touching::superseded_path(root, slug);
        if superseded.is_file() && std::fs::remove_file(&superseded).is_ok() {
            out.superseded += 1;
        }
    }
    eprintln!("\r  masks: {}/{} works          ", slugs.len(), slugs.len());
    Ok(out)
}

/// The one field of a catalogue row this pass needs.
#[derive(serde::Deserialize)]
struct Slug {
    slug: String,
}

/// Every `edges.jsonl` under the links tree.
fn shard_paths(links: &Path) -> Vec<PathBuf> {
    named_files(links, "edges.jsonl")
}

/// Every `inbound.jsonl` under the tree, for the sorting pass.
fn inbound_paths(links: &Path) -> Vec<PathBuf> {
    named_files(links, "inbound.jsonl")
}

fn named_files(links: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![links.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == name) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// The work slug out of a written anchor.
///
/// `girsa:bavli/berakhot/2a:1#1` → `bavli/berakhot`. A run endpoint is written
/// `<id>-girsa:<id>` and both ends of a run are in the same work, so the first
/// half answers for both. By hand rather than by parsing a `SegmentId` because
/// this runs eight million times and the answer is one `rfind`.
fn work_of(anchor: &str) -> Option<&str> {
    let one = anchor.split("-girsa:").next().unwrap_or(anchor);
    let body = one.strip_prefix("girsa:")?;
    let cut = body.rfind('/')?;
    Some(&body[..cut])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_link::{inbound, touching};

    #[test]
    fn a_slug_comes_off_an_anchor_whichever_shape_it_is_in() {
        assert_eq!(
            work_of("girsa:bavli/berakhot/2a:1#1"),
            Some("bavli/berakhot")
        );
        assert_eq!(
            work_of(
                "girsa:shulchan-arukh/orach-chayim/1:1#1-girsa:shulchan-arukh/orach-chayim/1:3#3"
            ),
            Some("shulchan-arukh/orach-chayim")
        );
        assert_eq!(work_of("not a ref"), None);
    }

    /// A two-work shelf with one edge across it, written the way the importer
    /// leaves things.
    fn a_little_shelf(root: &Path) {
        let mut catalogue = String::new();
        for (slug, count) in [("bavli/berakhot", 3), ("bavli/rashi-on-berakhot", 2)] {
            catalogue.push_str(&format!("{{\"slug\":\"{slug}\"}}\n"));
            let dir = girsa_corpus::import::work_dir(root, slug);
            std::fs::create_dir_all(&dir).expect("a work directory");
            let mut body = String::new();
            for n in 1..=count {
                body.push_str(&format!(
                    "{{\"id\":\"girsa:{slug}/2a:1#{n}\",\"kind\":\"text\",\"text\":\"…\"}}\n"
                ));
            }
            std::fs::write(dir.join("segments.jsonl"), body).expect("segments");
        }
        std::fs::create_dir_all(root.join("works")).expect("works/");
        std::fs::write(root.join("works/index.jsonl"), catalogue).expect("a catalogue");

        // Rashi's second comment, on Berakhot's third segment. Stored once, in
        // the shard of the work it points *from* (spec.md §8.2).
        let line = "{\"from\":\"girsa:bavli/rashi-on-berakhot/2a:1#2\",\
                    \"to\":\"girsa:bavli/berakhot/2a:1#3\",\"type\":\"comments-on\",\
                    \"method\":\"sefaria-seed\",\"label\":\"commentary\"}";
        let shard = girsa_corpus::import::slug_dir(&root.join("links"), "bavli/rashi-on-berakhot");
        std::fs::create_dir_all(&shard).expect("a shard directory");
        std::fs::write(shard.join("edges.jsonl"), format!("{line}\n")).expect("a shard");

        let mut into = inbound::Writer::default();
        into.push_row("bavli/rashi-on-berakhot", "bavli/berakhot", line);
        into.flush(root).expect("the inbound cache");
        for path in inbound_paths(&root.join("links")) {
            inbound::sort_and_index_at(&path).expect("the landing index");
        }
    }

    #[test]
    fn a_mask_is_set_on_both_ends_of_an_edge_and_only_where_it_lands() {
        // The whole reason this pass exists. Rashi's shard holds the edge;
        // Berakhot's shard holds nothing, and *"what comments on this line"* is
        // asked from Berakhot's side every time.
        let root = std::env::temp_dir().join("girsa-link-types-masks");
        let _ = std::fs::remove_dir_all(&root);
        a_little_shelf(&root);

        let done = write_masks(&root).expect("the masks are written");
        assert_eq!(done.works, 2);
        assert_eq!(done.segments, 5);

        let berakhot = girsa_corpus::import::ordered_ids(&root, "bavli/berakhot").expect("ids");
        let touching::Touching::Known(masks) = touching::read(&root, "bavli/berakhot", &berakhot)
        else {
            panic!("the masks just written were refused");
        };
        assert_eq!(
            masks.iter().map(|m| !m.is_empty()).collect::<Vec<_>>(),
            [false, false, true],
            "the end the edge was NOT stored under is the one a reader asks from"
        );
        assert!(masks[2].contains(EdgeType::CommentsOn));

        let rashi =
            girsa_corpus::import::ordered_ids(&root, "bavli/rashi-on-berakhot").expect("ids");
        let touching::Touching::Known(his) =
            touching::read(&root, "bavli/rashi-on-berakhot", &rashi)
        else {
            panic!("refused");
        };
        assert_eq!(
            his.iter().map(|m| !m.is_empty()).collect::<Vec<_>>(),
            [false, true]
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_pass_deletes_the_449_mb_format_it_replaced() {
        // BUILDER.md rule 3: when a thing is replaced the old thing goes in the
        // same change. A 4 MB file per work that nothing reads is exactly what
        // a `grep` six months from now presents as authoritative.
        let root = std::env::temp_dir().join("girsa-link-types-supersede");
        let _ = std::fs::remove_dir_all(&root);
        a_little_shelf(&root);
        let old = touching::superseded_path(&root, "bavli/berakhot");
        std::fs::create_dir_all(old.parent().expect("a parent")).expect("dir");
        std::fs::write(
            &old,
            "{\"a\":\"girsa:bavli/berakhot/2a:1#1\",\"t\":\"comments-on\"}\n",
        )
        .expect("the old format");

        let done = write_masks(&root).expect("the masks are written");
        assert_eq!(done.superseded, 1);
        assert!(
            !old.exists(),
            "touching.jsonl outlived the format that read it"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn one_walk_files_a_line_under_the_work_it_lands_in() {
        // The line as it came off disk, not a re-serialisation of it: the
        // inbound cache and the outgoing shard hold the same rows, and the
        // reader that parses one parses the other.
        let root = std::env::temp_dir().join("girsa-link-types-inbound");
        let _ = std::fs::remove_dir_all(&root);
        let line = r#"{"from":"girsa:mishnah-berurah/58:1#1","to":"girsa:shulchan-arukh/orach-chayim/58:1#1","type":"comments-on","method":"sefaria-seed","label":"commentary"}"#;
        let row: Row = serde_json::from_str(line).expect("parses");

        let mut inbound_writer = inbound::Writer::default();
        inbound_writer.push_row(
            work_of(&row.from).expect("a slug"),
            work_of(&row.to).expect("a slug"),
            line,
        );
        inbound_writer.flush(&root).expect("writes");

        let onto = inbound::read_back(&root, "shulchan-arukh/orach-chayim").expect("reads");
        assert_eq!(onto.len(), 1);
        assert_eq!(onto[0].from.from.work(), "mishnah-berurah");
        let _ = std::fs::remove_dir_all(&root);
    }
}
