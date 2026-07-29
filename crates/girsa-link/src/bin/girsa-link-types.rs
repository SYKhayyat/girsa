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
//! - **`touching.jsonl`** — which *kinds* of link touch each segment, for
//!   §9.8's link-type facet. Both ends of every edge, deduplicated, no far end
//!   kept. See [`girsa_link::touching`].
//! - **`inbound.jsonl`** — the edges themselves, filed under the work their far
//!   end lands in, for W28's chain tracing, which asks *what links here* again
//!   at every hop. See [`girsa_link::inbound`].
//!
//! One walk writes both, because the walk is the expensive part: three minutes
//! over 5,790 shards, and doing it twice would be six.
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

/// Flush when this many rows are held. Bounds memory rather than time: the
/// graph is four million edges and every one of them is two rows.
const FLUSH_EVERY: usize = 2_000_000;

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

    let mut writer = girsa_link::touching::Writer::default();
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
            let Ok(row) = serde_json::from_str::<Row>(line) else {
                unparsed += 1;
                continue;
            };
            // The type is read from the row's own label the same way
            // `store::read_back` reads it, so the cache says exactly what the
            // graph says (T2, T5: a blank label is `references` and is not an
            // error).
            let edge_type = EdgeType::from_sefaria(&row.label);
            let (Some(from), Some(to)) = (work_of(&row.from), work_of(&row.to)) else {
                unparsed += 1;
                continue;
            };
            writer.record(from, &row.from, edge_type);
            writer.record(to, &row.to, edge_type);
            // The line as it stands, not a re-serialisation of it: the inbound
            // cache is the same rows in the same shape, read back by the same
            // reader (see `girsa_link::inbound`).
            inbound.push_row(from, to, line);
            edges += 1;
        }
        done += 1;
        if writer.buffered() >= FLUSH_EVERY {
            if let Err(e) = writer.flush(&root) {
                eprintln!("cannot write: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
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
    if let Err(e) = writer.flush(&root).and_then(|()| inbound.flush(&root)) {
        eprintln!("cannot write: {e}");
        return std::process::ExitCode::FAILURE;
    }
    eprintln!(
        "\r  {done}/{} shards, {edges} edges          ",
        shards.len()
    );

    println!("two caches written beside the edges:");
    println!("  shards read        {done}");
    println!("  edges              {edges}");
    println!(
        "  type rows          {}   (both ends of each, deduplicated)",
        writer.len()
    );
    println!(
        "  inbound rows       {}   ({} skipped — both ends in one work, whose own shard holds them)",
        inbound.len(),
        inbound.internal()
    );
    println!(
        "  took               {:.0}s",
        started.elapsed().as_secs_f64()
    );
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

/// Every `edges.jsonl` under the links tree.
fn shard_paths(links: &Path) -> Vec<PathBuf> {
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
            } else if path.file_name().is_some_and(|n| n == "edges.jsonl") {
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

    #[test]
    fn both_ends_of_a_real_edge_land_in_their_own_works() {
        let root = std::env::temp_dir().join("girsa-link-types-bin");
        let _ = std::fs::remove_dir_all(&root);
        let mut writer = touching::Writer::default();
        let from = "girsa:bavli/rashi-on-berakhot/2a:1:3#3";
        let to = "girsa:bavli/berakhot/2a:1#1";
        writer.record(work_of(from).expect("a slug"), from, EdgeType::CommentsOn);
        writer.record(work_of(to).expect("a slug"), to, EdgeType::CommentsOn);
        writer.flush(&root).expect("writes");

        assert_eq!(
            touching::read_back(&root, "bavli/berakhot")
                .expect("reads")
                .len(),
            1
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
