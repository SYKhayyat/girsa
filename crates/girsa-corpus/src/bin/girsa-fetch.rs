//! Fetch the Sefaria export.
//!
//! ```sh
//! cargo run --release -p girsa-corpus --bin girsa-fetch -- ./corpus/sefaria
//! ```
//!
//! Interrupt it whenever. Run it again and it picks up where it stopped: the
//! plan is cached and every file already on disk at its stated size is skipped.
//! Nothing on disk is ever half-written, so the shelves that have landed are
//! readable while the rest is still coming.

use std::path::PathBuf;
use std::process::ExitCode;

use girsa_corpus::argv::{self, Argv};
use girsa_corpus::fetch;

/// How many at once, where nobody says.
const THREADS: usize = 12;

const USAGE: &str = "\
usage: girsa-fetch <corpus> [--threads N]

  Fetches ~2.2 GB: Hebrew merged.json, every schema, the link CSVs.
  Resumable — interrupt and re-run.";

fn main() -> ExitCode {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&typed) {
        return argv::asked(USAGE);
    }
    let args = match Argv::of(typed, &[], &["--threads"]) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return argv::refuse(USAGE);
        }
    };
    let Some(root) = args.word(0).map(PathBuf::from) else {
        return argv::refuse(USAGE);
    };
    // A named option now, rather than a second bare positional that silently
    // fell back to 12 when it did not parse — so `girsa-fetch corpus 8` is
    // still read, and `girsa-fetch corpus --threads eight` says so.
    let threads: usize = match args.number("--threads") {
        Ok(Some(threads)) => threads,
        Ok(None) => args
            .word(1)
            .and_then(|word| word.parse().ok())
            .unwrap_or(THREADS),
        Err(e) => {
            eprintln!("{e}");
            return argv::refuse(USAGE);
        }
    };

    let plan = match fetch::plan(&root) {
        Ok(plan) => plan,
        Err(e) => {
            eprintln!("could not work out what to fetch: {e}");
            return ExitCode::FAILURE;
        }
    };

    let (already, total) = fetch::landed(&root, &plan);
    eprintln!(
        "plan: {total} files, {:.1} GB, across {:?}",
        plan.total_bytes() as f64 / 1_073_741_824.0,
        fetch::sections(&plan)
    );
    eprintln!("on disk already: {already}");

    match fetch::run(&root, &plan, threads) {
        Ok(0) => {
            eprintln!("complete — {total} files under {}", root.display());
            ExitCode::SUCCESS
        }
        Ok(failed) => {
            // Loudly, and with a failing exit code. A partial corpus reported as
            // a success is how an import silently ends up missing seforim, and
            // the whole of BUILDER.md §0.2 is what that looks like later.
            eprintln!();
            eprintln!("INCOMPLETE — {failed} of {total} files did not arrive.");
            eprintln!("Re-run to retry only those; everything else is already on disk.");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("fetch failed: {e}");
            ExitCode::FAILURE
        }
    }
}
