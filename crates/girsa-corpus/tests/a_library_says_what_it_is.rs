//! A tree of seforim is asked where it came from. It is not assumed.
//!
//! # The bug this is about
//!
//! `read_otzaria` used to stamp **every** work it walked with
//!
//! ```text
//! edition:    "Otzaria"
//! provenance: "https://github.com/Sivan22/otzaria-library"
//! license:    "Unlicense"
//! ```
//!
//! which was true of the only tree it had ever been pointed at, and became
//! false the moment a second one existed. Put someone else's seforim in a
//! directory of that shape and the shelf records, as fact, that they came from
//! a repository they did not come from under a licence they are not under.
//!
//! spec.md §13 asks a work to be able to say where its text is from. A field
//! that answers that question wrongly is worse than one that does not answer:
//! `None` is a thing a reader can act on, and a confident wrong licence is not.
//!
//! So a library declares itself in a `library.json` at its root, and a tree
//! that declares nothing gets no claim made on its behalf — with exactly one
//! exception, which is a tree **positively identified** as Sivan22's by the
//! `metadata.json` and `אוצריא/` that only it has. That is a recognition rather
//! than a default, which is the whole difference.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;

use girsa_corpus::work::Library;

/// A directory of its own per test, named for the test, built fresh each run.
///
/// `std::env::temp_dir()` and not a crate: it is what the rest of this crate's
/// tests already do, and one more dependency to make a folder is not a trade.
fn dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("girsa-library-{name}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("a temporary directory");
    path
}

#[test]
fn a_tree_that_declares_itself_is_believed() {
    let d = dir("declares");
    fs::write(
        d.as_path().join("library.json"),
        r#"{"edition":"OtzarLib","provenance":"https://github.com/YairDaniel123/OtzarLib"}"#,
    )
    .expect("write");
    let library = Library::at(d.as_path());
    let version = library.version().expect("a declared library has a version");
    assert_eq!(version.edition, "OtzarLib");
    assert_eq!(
        version.provenance.as_deref(),
        Some("https://github.com/YairDaniel123/OtzarLib")
    );
    assert_eq!(
        version.license, None,
        "a library that states no licence must not be given one"
    );
}

#[test]
fn the_otzaria_library_is_recognised_without_a_file() {
    // The tree Girsa has always read. Nothing about it has changed and nobody
    // has to add a file to it for the shelf to keep saying what it always said.
    let d = dir("otzaria");
    fs::write(d.as_path().join("metadata.json"), "{}").expect("write");
    fs::create_dir(d.as_path().join("אוצריא")).expect("mkdir");
    let library = Library::at(d.as_path());
    let version = library.version().expect("recognised");
    assert_eq!(version.edition, "Otzaria");
    assert_eq!(
        version.provenance.as_deref(),
        Some("https://github.com/Sivan22/otzaria-library")
    );
    // And it states **no licence**. The Unlicense is on `Sivan22/otzaria`, the
    // application; `Sivan22/otzaria-library`, where the seforim are, carries no
    // LICENSE file, no SPDX id and no terms in its README. Recognising a tree
    // says which library it is, and that is all it says.
    assert_eq!(version.license, None);
}

#[test]
fn a_tree_that_says_nothing_has_nothing_said_about_it() {
    // The one that matters. This is the shape OtzarLib arrives in — a
    // directory of `.txt` files and no claims — and the old code called it
    // Unlicense.
    let d = dir("silent");
    fs::create_dir(d.as_path().join("ספרים")).expect("mkdir");
    assert!(
        Library::at(d.as_path()).version().is_none(),
        "an unidentified tree gets no edition, no provenance and above all no licence"
    );
}

#[test]
fn a_declaration_beats_the_recognition() {
    // A tree that looks like Otzaria's and says it is something else is
    // something else. Otherwise the recognition becomes a default again for
    // anybody who copied the layout.
    let d = dir("beats");
    fs::write(d.as_path().join("metadata.json"), "{}").expect("write");
    fs::create_dir(d.as_path().join("אוצריא")).expect("mkdir");
    fs::write(
        d.as_path().join("library.json"),
        r#"{"edition":"Mine","license":"CC0-1.0"}"#,
    )
    .expect("write");
    let library = Library::at(d.as_path());
    let version = library.version().expect("declared");
    assert_eq!(version.edition, "Mine");
    assert_eq!(version.license.as_deref(), Some("CC0-1.0"));
}

#[test]
fn a_library_json_that_will_not_parse_is_not_quietly_ignored() {
    // Silence here would put the tree back on the recognition path, or on no
    // path at all, and either way the reason would be invisible. A library that
    // tried to say something and failed says nothing — but the caller can tell
    // the difference.
    let d = dir("broken");
    fs::write(d.as_path().join("library.json"), "{not json").expect("write");
    let library = Library::at(d.as_path());
    assert!(library.version().is_none());
    assert!(
        library.trouble().is_some(),
        "the unreadable declaration is reported rather than swallowed"
    );
}
