//! Would a re-import today keep every permanent id on the shelf? (spec.md §3)
//!
//! ```sh
//! cargo run --release -p girsa-corpus --example measure-continuity -- corpus
//! cargo run --release -p girsa-corpus --example measure-continuity -- corpus 200
//! ```
//!
//! The importer's own report says what a run *did*. This says what a run
//! **would do**, without an Otzaria tree, without the network and without
//! overwriting anything: it re-imports each work from the corpus's own files
//! against the corpus's own previous run, and counts the names that survive.
//!
//! Every number the answer to *"is §3 actually held"* rests on is one of these,
//! and the honest form of that answer is a command anybody can re-run rather
//! than a paragraph in a design document. The expected result on an unchanged
//! shelf is **every name kept and none minted** — anything else means the
//! importer is not idempotent, which is a defect on its own.
//!
//! The optional second argument caps how many works to read; the whole corpus is
//! ~3 GB and a spot check is usually what is wanted.

// A tool that prints a report.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use girsa_corpus::import::{self, continuity::Continuity, ImportedWork, Previous, RawSegment};

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap_or_else(|| "corpus".to_string()));
    let cap: usize = args
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(usize::MAX);

    let works = root.join("works");
    if !works.is_dir() {
        eprintln!("no corpus at {}", works.display());
        return std::process::ExitCode::from(2);
    }

    let slugs = slugs(&works, cap);
    if slugs.is_empty() {
        eprintln!("no works under {}", works.display());
        return std::process::ExitCode::from(2);
    }
    eprintln!("re-importing {} works from their own files …", slugs.len());

    let started = Instant::now();
    let mut tally = Continuity::default();
    let mut unreadable = 0usize;
    let mut moved: Vec<String> = Vec::new();
    let mut unsettled: Vec<(String, usize)> = Vec::new();
    for (n, slug) in slugs.iter().enumerate() {
        let Ok(on_disk) = import::read_back(&root, slug) else {
            unreadable += 1;
            continue;
        };
        // The places the last run judged — a cut se'if is one place, not three
        // records — fed back in as though upstream had handed them over again.
        let previous = Previous::on_the_shelf(&root, slug);
        let raw: Vec<RawSegment> =
            import::continuity::places_of(&on_disk.segments, &on_disk.redirects)
                .into_iter()
                .map(|place| RawSegment {
                    path: place.path,
                    kind: place.kind,
                    text: place.text,
                })
                .collect();
        let again = ImportedWork::assemble_after(on_disk.work.clone(), raw, &previous);
        tally.absorb(&again.continuity);
        // Named, not just counted. On a shelf nothing upstream has touched the
        // right answer is zero, so any work that would mint a name is a work
        // whose files disagree with the importer about something — which is a
        // finding, and a finding with no slug attached is a number to shrug at.
        if again.continuity.minted > 0 {
            unsettled.push((slug.clone(), again.continuity.minted));
        }

        // Nothing is written. The check is that every id that was on disk still
        // names the words it named.
        //
        // Mined on both sides. 1,500 works on this shelf were imported before
        // W34 and still carry `<i data-commentator…></i>` in their text, so a
        // re-import legitimately strips markup from them — that is the import
        // doing its job, not a name moving, and counting it as one would bury
        // the thing this is looking for.
        let before: std::collections::BTreeMap<String, String> = on_disk
            .segments
            .iter()
            .map(|s| (s.id.to_string(), girsa_corpus::anchors::mine(&s.text).text))
            .collect();
        for segment in &again.segments {
            if before
                .get(&segment.id.to_string())
                .is_some_and(|was| *was != segment.text)
            {
                moved.push(segment.id.to_string());
            }
        }
        if (n + 1) % 100 == 0 || n + 1 == slugs.len() {
            eprint!("\r  {}/{} works", n + 1, slugs.len());
        }
    }
    eprintln!();

    println!();
    for line in tally.said() {
        println!("{line}");
    }
    if unreadable > 0 {
        println!("  would not read     {unreadable} works");
    }
    println!(
        "  took               {:.1}s",
        started.elapsed().as_secs_f64()
    );
    if !unsettled.is_empty() {
        println!("\nworks that would mint a name on a shelf nothing has touched:");
        for (slug, n) in unsettled.iter().take(20) {
            println!("  {n:>4}  {slug}");
        }
        if unsettled.len() > 20 {
            println!("  … and {} more", unsettled.len() - 20);
        }
    }

    if moved.is_empty() {
        println!(
            "\nno permanent id would change the words it names. spec.md §3 holds \
             across a re-import."
        );
        return std::process::ExitCode::SUCCESS;
    }
    println!("\n{} ids would name different words:", moved.len());
    for id in moved.iter().take(10) {
        println!("  {id}");
    }
    println!(
        "\nThis is T1 at import granularity and it is the failure the whole \
         ordinal scheme exists to prevent — the wrong text, silently."
    );
    std::process::ExitCode::FAILURE
}

/// Every work slug under `works/`, by the directory holding a `segments.jsonl`.
fn slugs(works: &Path, cap: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![works.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= cap {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "segments.jsonl") {
                if let Some(slug) = dir
                    .strip_prefix(works)
                    .ok()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                {
                    out.push(slug);
                }
            }
        }
    }
    out.sort();
    out.truncate(cap);
    out
}
