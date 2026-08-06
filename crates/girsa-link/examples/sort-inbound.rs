//! Sort every `inbound.jsonl` by where its rows land, and index them.
//!
//! `girsa-link-types` does this at the end of a rebuild. This is the same pass
//! over a tree somebody already has, so a corpus does not have to be re-walked —
//! 4.18M edges — to get the index over rows that are already correct.
//!
//! Idempotent. Safe to run twice, and safe to interrupt: each work is rewritten
//! and then indexed, and a work with no index is read the slower way.
//!
//! ```text
//! cargo run --release --example sort-inbound -p girsa-link
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::time::Instant;

fn inbound_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            inbound_files(&path, found);
        } else if path.file_name().is_some_and(|n| n == "inbound.jsonl") {
            found.push(path);
        }
    }
}

fn main() {
    let root = std::path::Path::new("corpus/links");
    if !root.is_dir() {
        println!("no corpus/links here — nothing to sort");
        return;
    }
    let mut files = Vec::new();
    inbound_files(root, &mut files);
    println!("{} works with an inbound cache", files.len());

    let began = Instant::now();
    let (mut places, mut bytes, mut refused) = (0usize, 0u64, 0usize);
    for path in &files {
        bytes += std::fs::metadata(path).map(|m| m.len()).unwrap_or_default();
        match girsa_link::inbound::sort_and_index_at(path) {
            Ok(n) => places += n,
            Err(e) => {
                refused += 1;
                println!("  {} refused: {e}", path.display());
            }
        }
    }
    let took = began.elapsed();

    println!(
        "sorted and indexed {} works, {} landing places, {} MB, in {took:.1?}",
        files.len() - refused,
        places,
        bytes >> 20
    );
    if refused > 0 {
        println!("{refused} refused — those are read the slower way, and say so above");
    }
}
