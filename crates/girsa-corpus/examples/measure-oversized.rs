//! Count the segments that name a volume rather than a place (B12).
//!
//! ```sh
//! cargo run --release -p girsa-corpus --example measure-oversized -- corpus
//! ```
//!
//! The audit that prompted [`girsa_corpus::oversized`] measured this by hand and
//! got 5,733 segments over 10,000 characters, 119 over 50,000, 19 over 200,000, a
//! largest of 1,275,307 and 926 works affected. **This is that measurement, as a
//! command anybody can re-run** — because a headline number nobody can reproduce is
//! a number that drifts, and this project's own standard is that the link table's
//! six lines sum exactly to 5,108,893.
//!
//! It reads the corpus as it is on disk, so run before a re-import it reports the
//! problem and run after one it reports zero.

// A tool that prints a report.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::io::BufRead;
use std::path::{Path, PathBuf};

use girsa_corpus::oversized::Tally;

fn main() -> std::process::ExitCode {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "corpus".to_string()),
    );
    let works = root.join("works");
    if !works.is_dir() {
        eprintln!("no corpus at {}", works.display());
        return std::process::ExitCode::from(2);
    }

    let mut tally = Tally::default();
    let mut segments = 0usize;
    let mut files = 0usize;
    let mut widest: Vec<(usize, String)> = Vec::new();
    walk(&works, &mut |path| {
        files += 1;
        let Ok(file) = std::fs::File::open(path) else {
            return;
        };
        let slug = path
            .parent()
            .and_then(|p| p.strip_prefix(&works).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            let id = value.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let text = value.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let characters = text.chars().count();
            segments += 1;
            tally.saw(id, characters, &slug);
            if characters > girsa_corpus::oversized::NAMES_A_PLACE * 20 {
                widest.push((characters, id.to_string()));
            }
        }
        if files % 500 == 0 {
            eprint!("\r  {files} works, {segments} segments");
        }
    });

    println!("\n{files} works · {segments} segments");
    if tally.is_empty() {
        println!(
            "no segment is over {} characters — every permanent id names a place",
            girsa_corpus::oversized::NAMES_A_PLACE
        );
        return std::process::ExitCode::SUCCESS;
    }
    println!("segments too long to name a place:");
    for line in tally.said() {
        println!("{line}");
    }
    widest.sort_by_key(|(characters, _)| std::cmp::Reverse(*characters));
    println!("\nthe worst of them:");
    for (characters, id) in widest.iter().take(10) {
        println!("  {characters:>9}  {id}");
    }
    std::process::ExitCode::SUCCESS
}

fn walk(dir: &Path, each: &mut impl FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, each);
        } else if path.file_name().is_some_and(|n| n == "segments.jsonl") {
            each(&path);
        }
    }
}
