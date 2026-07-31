//! Turn the `comments-on` edges already on disk the right way round.
//!
//! `girsa-link-import` now orients as it writes, but the corpus on this machine
//! was imported before it did, and re-reading 672 MB of `links*.csv` to fix a
//! field that is already resolved would take an hour to arrive at the same
//! place. This reads the store, applies [`girsa_link::orient`], and writes it
//! back.
//!
//! # Why it writes to a second tree
//!
//! A flipped edge changes shard — it belongs to the commentary's file, not the
//! base's. Writing into the tree being read would let one flush truncate a
//! shard this pass has not read yet, and those edges would be gone. So the new
//! store is built beside the old one and the two are swapped at the end, which
//! also makes the whole operation reversible: the previous store is left as
//! `links.superseded` rather than deleted.
//!
//! Idempotent, because [`girsa_link::orient::Bases::orient`] is: running it
//! twice reports everything already right and writes the same bytes.
//!
//! ```text
//! girsa-link-orient [corpus_root]              # build the new store, change nothing
//! girsa-link-orient [corpus_root] --replace    # and swap it in
//! ```
//!
//! Rebuild the reverse index afterwards — `girsa-link-types` — or the reader's
//! panel keeps answering from the old one. Until it is rebuilt there is no
//! `inbound.built` marker, so the panel says it has not been told what links
//! here, which is the truth and is what `girsa_link::inbound::built` is for.

// This is a tool with a console, not a library. A failure here should say what
// happened and stop.
#![allow(clippy::expect_used, clippy::print_stderr, clippy::print_stdout)]

use std::path::{Path, PathBuf};

use girsa_corpus::work::Work;
use girsa_link::{orient, store};

const SUPERSEDED: &str = "links.superseded";

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from("corpus");
    let mut replace = false;
    for arg in args.by_ref() {
        match arg.as_str() {
            "--replace" => replace = true,
            other => root = PathBuf::from(other),
        }
    }

    let links = root.join("links");
    if !links.is_dir() {
        eprintln!(
            "{}: no links directory — run girsa-link-import first",
            links.display()
        );
        return std::process::ExitCode::FAILURE;
    }

    let works = match load_works(&root) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("could not read the work index: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let bases = orient::Bases::of(&works);
    println!(
        "{} works on the shelf, {} declaring a base text",
        works.len(),
        bases.declaring()
    );

    // Captured before anything is written, and the staging tree is not under
    // `links`, so this list cannot grow or be clobbered mid-pass.
    let shards = shards(&links);
    println!("{} shards to read", shards.len());

    // `store::Writer::flush` writes `<root>/links/<slug>/edges.jsonl`, so the
    // staging root is a directory whose `links` is where the new store goes.
    let staging_root = root.join(".oriented");
    if staging_root.exists() {
        if let Err(e) = std::fs::remove_dir_all(&staging_root) {
            eprintln!("could not clear {}: {e}", staging_root.display());
            return std::process::ExitCode::FAILURE;
        }
    }

    let mut writer = store::Writer::default();
    let mut tally = orient::Tally::default();
    let mut read = 0usize;
    let mut failed = Vec::new();

    for (i, path) in shards.iter().enumerate() {
        if i % 500 == 0 {
            eprint!("\r  {i}/{}", shards.len());
        }
        let edges = match store::read_edges(path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("\n{}: {e}", path.display());
                failed.push(path.clone());
                continue;
            }
        };
        for mut edge in edges {
            read += 1;
            tally.count(bases.orient(&mut edge));
            writer.push(&edge);
        }
        if writer.buffered_bytes() > 64 * 1024 * 1024 {
            if let Err(e) = writer.flush(&staging_root) {
                eprintln!("\ncould not write edges: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    eprintln!();
    if let Err(e) = writer.flush(&staging_root) {
        eprintln!("could not write the last of the edges: {e}");
        return std::process::ExitCode::FAILURE;
    }

    println!("{read} edges read, {} written", writer.len());
    println!("{}", tally.said());

    // A shard that would not read means edges missing from the new store. Do
    // not swap that in over a store that still has them.
    if !failed.is_empty() {
        println!(
            "{} shards could not be read; the new store is incomplete and was NOT swapped in",
            failed.len()
        );
        return std::process::ExitCode::FAILURE;
    }
    if read != writer.len() {
        println!("read {read} but wrote {} — not swapping", writer.len());
        return std::process::ExitCode::FAILURE;
    }

    let built = staging_root.join("links");
    if !replace {
        println!(
            "built {} — rerun with --replace to swap it in",
            built.display()
        );
        return std::process::ExitCode::SUCCESS;
    }

    let superseded = root.join(SUPERSEDED);
    if superseded.exists() {
        if let Err(e) = std::fs::remove_dir_all(&superseded) {
            eprintln!("could not clear {}: {e}", superseded.display());
            return std::process::ExitCode::FAILURE;
        }
    }
    if let Err(e) = std::fs::rename(&links, &superseded) {
        eprintln!("could not move the old store aside: {e}");
        return std::process::ExitCode::FAILURE;
    }
    if let Err(e) = std::fs::rename(&built, &links) {
        // Put it back. Leaving no `links` at all would be worse than leaving
        // the unoriented one.
        eprintln!("could not move the new store into place: {e}");
        let _ = std::fs::rename(&superseded, &links);
        return std::process::ExitCode::FAILURE;
    }
    let _ = std::fs::remove_dir_all(&staging_root);
    println!(
        "swapped in. the previous store is {} — delete it once you are satisfied",
        superseded.display()
    );
    println!(
        "now rebuild the reverse index:  girsa-link-types {}",
        root.display()
    );
    std::process::ExitCode::SUCCESS
}

fn load_works(root: &Path) -> Result<Vec<Work>, std::io::Error> {
    let body = std::fs::read_to_string(root.join("works/index.jsonl"))?;
    Ok(body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Work>(l).ok())
        .collect())
}

/// Every `edges.jsonl` under `links`, at any depth.
fn shards(links: &Path) -> Vec<PathBuf> {
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
