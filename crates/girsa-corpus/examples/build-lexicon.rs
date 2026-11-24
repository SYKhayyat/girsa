//! Build the resolver's lexicon out of Sefaria's schemas.
//!
//! ```sh
//! cargo run --release -p girsa-corpus --example build-lexicon -- \
//!     corpus/sefaria/schemas corpus/lexicon.tsv
//! ```
//!
//! spec.md §4.3 says the resolver is *"seeded from Sefaria's `titles` and
//! `match_templates`, which is most of the lexicon for free"*.
//! `girsa_corpus::lexicon::from_schemas` is what collects it — every way each of 6,595 works is written, in both
//! languages, from the files that already list them — and this example is the
//! way to regenerate that one file **without a full import**.
//!
//! `girsa-import` writes the same file as part of an import, because
//! `girsa-link-import` cannot run without it and an example is not a step
//! anybody can be relied on to have run (#65). Both call the one function; there
//! is no second copy of the collection here, which is what this file's own
//! thirty-seven-line copy of `slug_of` taught the hard way.

use std::fs;
use std::path::Path;

// This file used to hold the collection itself — the walk of `schemas/`, the
// variant lists, the markup stripping, and (before that) a byte-identical
// second copy of `slug_of`, under a doc comment on the original saying why a
// second copy must not exist. The comment was right and the copy was under it.
//
// All of it is `girsa_corpus::lexicon::from_schemas` now, because
// `girsa-link-import` cannot start without the file this produced and an
// example is not a step in anybody's order (#65). What is left here is an
// argument and a write.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(schema_dir), Some(out)) = (args.next(), args.next()) else {
        eprintln!("usage: build-lexicon <schemas-dir> <output.tsv>");
        std::process::exit(2);
    };

    let built = girsa_corpus::lexicon::from_schemas(Path::new(&schema_dir))?;
    fs::write(Path::new(&out), &built.tsv)?;

    eprintln!(
        "{} works, {} spellings -> {out}",
        built.works,
        built.tsv.lines().count() - 2
    );
    if built.skipped > 0 {
        // Loud. A schema that failed to parse is a sefer whose citations will
        // not resolve, and a silent count of those is how a resolver ends up
        // mysteriously bad at one shelf.
        eprintln!(
            "SKIPPED {} schema files that would not parse",
            built.skipped
        );
    }
    Ok(())
}
