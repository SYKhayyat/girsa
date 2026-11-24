//! AUDIT-lamdan-2026-08-30 item 9: the list a reader sees is short unless one
//! program has been run, and nothing checked that following the documentation
//! runs it.
//!
//! `girsa-companions` is the step that turns four million edges into *which
//! seforim are worth opening beside which*. It is a cache and it is allowed to
//! be missing — the shelf says so rather than pretending (see
//! `girsa_app::links::Touching::incoming_unknown`) — which is exactly why the
//! cache's absence is invisible: a shelf that never ran the step reads the same
//! as a sefer nobody wrote a commentary on.
//!
//! # What this holds
//!
//! Two things, and they are the two halves the audit asked for:
//!
//! 1. **The step is reachable from the code that documents it.** The binary this
//!    runs is the one `tools/build-a-shelf.mjs` names and `docs/tools.md` lists —
//!    `the_shelf_build_order_is_written_once.rs` holds that half — and it runs
//!    here against a real fixture shelf, not a table.
//! 2. **What it buys is real.** Before it, the picker offers only what a schema
//!    declares; after it, it offers the seforim the graph joins. So a shelf
//!    built through the documented path presents a non-empty list of the second
//!    kind, and this asserts a name, not a count.
//!
//! The fixture is the tree's own: `girsa_fixture::linked()` is a shelf built by
//! the real importer from a `links0.csv`, per-process and per-binary, so running
//! a binary that writes into it cannot disturb another test file.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use girsa_app::shelf::Companion;
use girsa_app::Shelf;

const GEMARA: &str = "bavli/berakhot";

fn offered(root: &Path, personal: &Path) -> Vec<Companion> {
    let shelf = Shelf::open(root, personal).expect("the fixture shelf opens");
    shelf.companions(GEMARA)
}

fn names(offered: &[Companion]) -> Vec<&str> {
    offered.iter().map(|c| c.slug.as_str()).collect()
}

#[test]
fn the_step_writes_what_no_schema_declares_and_the_picker_offers_it() {
    let fixture = girsa_fixture::linked();
    let (root, personal) = (fixture.root(), fixture.personal());
    let cache = root.join("links/companions.jsonl");
    assert!(
        !cache.exists(),
        "the fixture is a shelf nobody has run the step on, and if it carried the \
         cache this test would pass over a step that does nothing"
    );

    let before = offered(root, personal);
    // Only what the corpus declares: a sefer's schema names its commentaries and
    // nothing names the sefer that quotes it eight hundred times.
    assert!(
        !before.is_empty(),
        "the fixture declares a commentary on Berakhot, or the rest of this proves nothing"
    );

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_girsa-companions"))
        .arg(root)
        .status()
        .expect("the documented step runs");
    assert!(status.success(), "girsa-companions over the fixture shelf");
    assert!(
        cache.exists(),
        "it writes links/companions.jsonl, where the docs say"
    );

    let after = offered(root, personal);
    assert!(
        after.len() > before.len(),
        "the step buys seforim the schemas do not declare: {} before, {} after — {:?}",
        before.len(),
        after.len(),
        names(&after)
    );

    // Named, not only counted. The fixture's graph joins Berakhot to the works
    // whose rows point at it, and at least one of them declares nothing.
    let (was, now) = (names(&before), names(&after));
    let gained: Vec<&str> = now
        .iter()
        .copied()
        .filter(|slug| !was.contains(slug))
        .collect();
    assert!(
        !gained.is_empty(),
        "the seforim the step picked up are named: {now:?}"
    );

    // And it is a cache, so opening the shelf again sees the same list without
    // the step: the file is the contract, not the process that wrote it.
    assert_eq!(
        names(&after),
        names(&offered(root, personal)),
        "reopening the shelf reads the same cache"
    );
}
