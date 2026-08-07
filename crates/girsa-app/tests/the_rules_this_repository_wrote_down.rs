//! Invariants this repository states in doc comments, checked.
//!
//! # The diagnosis this file is the answer to
//!
//! The 6 August 2026 report's central finding was not any one defect. It was
//! that **this codebase reliably wins the argument it is having, and reliably
//! does not notice it has had the argument before** — and its evidence was a
//! table of ten doc comments, each stating an invariant, each sitting next to a
//! caller that violated it. Its conclusion:
//!
//! > The design lives in prose, and prose is not checkable. Every invariant in
//! > the table above was written down beautifully — and writing it down was
//! > mistaken for enforcing it.
//!
//! So the highest-leverage change in the repository is not any individual fix.
//! It is turning those doc comments into tests. This is where they go.
//!
//! # Why these are source scans, and when a source scan is the wrong tool
//!
//! Most invariants here are better expressed as types or as behaviour, and where
//! they can be, they are — `Bases::orient` is private so `girsa-link-orient`
//! cannot count without stamping; `Touching` has three variants so no caller can
//! read a missing cache as a zero. Those need no test in this file.
//!
//! What is left is the shape a type cannot hold: **"there is one implementation
//! of this."** Nothing in Rust says *no second function anywhere may be named
//! `slug_of`*, and every instance in that table was exactly a second
//! implementation. A scan over the tracked source is blunt, and it is what
//! catches a copy the day it is pasted rather than in an audit six weeks later.
//!
//! Each check names the doc comment it enforces, so a reader who trips one can
//! go and read the argument rather than guess at it.

use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| panic!("the repository root resolves: {e}"))
}

/// Every `.rs` file in the workspace and the shell, with its repo-relative path.
///
/// Not `target/`, and not the sibling checkouts — `sefer-crates` is a different
/// repository with its own rules, and a rule this one wrote down does not bind
/// it.
fn sources(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut stack = vec![root.join("crates"), root.join("app/src-tauri/src")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|x| x == "rs") {
                let body = std::fs::read_to_string(&path).unwrap_or_default();
                let named = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                found.push((named, body));
            }
        }
    }
    assert!(
        found.len() > 50,
        "only {} source files found — the walk is wrong, and a check that cannot \
         find what it checks is the one thing this repository forbids by name",
        found.len()
    );
    found
}

/// This file, which names every signature it forbids and would otherwise be
/// the only violation of every rule in it.
const SELF: &str = "the_rules_this_repository_wrote_down.rs";

/// Where a function of this name is defined, across the tree.
fn defined_in(root: &Path, signature: &str) -> Vec<String> {
    sources(root)
        .into_iter()
        .filter(|(named, body)| !named.ends_with(SELF) && body.contains(signature))
        .map(|(named, _)| named)
        .collect()
}

#[test]
fn slug_of_has_one_implementation() {
    // `girsa_corpus::work::slug_of`:
    //
    //   **This is the same function the lexicon is built with**, and it has to
    //   be: the lexicon maps a citation onto a slug and the importer names
    //   segments after one, so a second implementation that drifted by a hyphen
    //   would resolve citations onto works that do not exist.
    //
    // `examples/build-lexicon.rs` was a second implementation — byte-identical,
    // thirty-seven lines, directly beneath that sentence. So was the sentence.
    let root = repo();
    let mut found = defined_in(&root, "fn slug_of(");
    found.sort();
    assert_eq!(
        found,
        vec!["crates/girsa-corpus/src/work.rs".to_string()],
        "a second `slug_of`. The lexicon and the importer have to spell a work \
         the same way; see the doc comment on the original."
    );
}

#[test]
fn find_index_has_one_implementation() {
    // `girsa_note::since::index_candidates`:
    //
    //   Shared, so a search panel that finds an index and a `girsa-read` that
    //   does not cannot be two answers to one question.
    //
    // `app/src-tauri/src/lib.rs:855` was a second one — **forty lines above a
    // call to the shared one, in the same file** — with the same three
    // candidates in the same order and a different accept predicate. It took
    // only `girsa-cache.json`; the shared one also takes a bare tantivy
    // `meta.json`. So a directory `girsa-read` called an index, the window
    // called *no search index*.
    let root = repo();
    let mut found = defined_in(&root, "fn find_index(");
    found.sort();
    assert_eq!(
        found,
        vec!["crates/girsa-note/src/since.rs".to_string()],
        "a second `find_index`. Two of these had two accept predicates and          disagreed about whether a directory was an index."
    );
}

#[test]
fn a_prepared_query_is_never_rebuilt_to_be_asked_again() {
    // `girsa_search::index::Prepared`:
    //
    //   Hits, a total and the facet counts are three questions about **one**
    //   query, and a facet computed from a differently-built copy of it would
    //   be a column of numbers that did not add up to the header.
    //
    // `Bar::literally` built one for the facets and let `search_in` build a
    // second, private one for the hits. `Bar::smartly` was worse: it called
    // `prepare_widened(&answered.widened)` — rebuilding off the field whose own
    // doc comment says the facets must come from *"the search that ran rather
    // than one built afterwards to look like it."*
    //
    // The seam is `SearchIndex::found_with`, which takes a `Prepared`. So: no
    // caller of `prepare` may also call a `search_*` that prepares privately,
    // and the way to hold that is that `bar.rs` does not name those functions
    // at all any more.
    let root = repo();
    // Comments stripped, because the note above the fixed call site names the
    // call it replaced — and a check that a *comment* cannot mention what it
    // forbids is a check that makes the code harder to explain.
    let bar: String = std::fs::read_to_string(root.join("crates/girsa-search/src/bar.rs"))
        .unwrap_or_else(|e| panic!("bar.rs reads: {e}"))
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    for rebuilds in ["search_in(", "search_widened_in(", "prepare_widened("] {
        assert!(
            !bar.contains(rebuilds),
            "`bar.rs` calls `{rebuilds}`, which prepares a query privately — and it              already holds a `Prepared` for the facets. Two builds of one query is              two chances for a facet column not to add up to the header above it.              Use `SearchIndex::found_with`."
        );
    }
}
