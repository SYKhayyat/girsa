//! A name this shelf does not have is not remembered as *a sefer that is not
//! here*.
//!
//! The shelf grows in the middle of a session: `add_mine` and `write_note` put
//! a sefer on it while the window is open, and that is their whole point. If
//! the answer for a slug were cached the first time it was asked for and came
//! back empty, a sefer the reader imports would keep an empty margin — the very
//! sefer they just added and are looking at — until a restart. `sections`
//! therefore holds an answer only for a work that is actually on the shelf, and
//! a name it does not have is re-asked for every time (issue #58).
//!
//! Both facts are pinned through the cache's own signature: a hit returns the
//! same `Arc`, a miss returns a fresh one.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::Arc;

use girsa_app::shelf::Shelf;

const MISHNAH: &str = "mishnah-berakhot";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-sections-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_sefer_that_is_here_is_remembered() {
    let personal = scratch("present");
    let shelf =
        Shelf::open(girsa_fixture::linked().root(), &personal).expect("the fixture shelf opens");

    // Not an empty run: the identical-answer check below would pass about
    // nothing if the fixture built no seforim, which is the failure
    // `tools/check-ksav-fixture.sh` exists to end.
    assert!(
        !shelf.works().is_empty(),
        "the fixture shelf has no seforim"
    );

    let first = shelf.sections(MISHNAH);
    let second = shelf.sections(MISHNAH);
    assert!(
        Arc::ptr_eq(&first, &second),
        "a sefer on the shelf is read back again on every call"
    );
}

#[test]
fn a_sefer_that_is_not_here_is_not_remembered_as_absent() {
    let personal = scratch("absent");
    let shelf =
        Shelf::open(girsa_fixture::linked().root(), &personal).expect("the fixture shelf opens");

    // A name nothing on this shelf holds.
    let slug = "girsa:nonesuch";
    let first = shelf.sections(slug);
    let second = shelf.sections(slug);
    assert!(
        !Arc::ptr_eq(&first, &second),
        "an absent sefer was remembered as an absent sefer"
    );
}
