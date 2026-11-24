//! The order the shelf is built in is written down in four venues, and the
//! one that performs it is a program.
//!
//! AUDIT-lamdan-2026-08-30 item 7 counted them: `tools/build-a-shelf.mjs`,
//! `docs/tools.md`, `docs/the-libraries.md` and `docs/troubleshooting.md`, all
//! describing six steps. Prose that restates a list is a second copy of it, and
//! the copy that is not edited drifts — the audit already found one that had.
//! Two ways to end that: stop restating it, or check the restatements. A reader
//! who wants to do the steps by hand needs the list in prose, so this does the
//! second half: **the program is the order, and the prose has to agree with it,
//! in sequence.**
//!
//! What this catches is the shape the audit reported, which no spell-check
//! would. A step inserted into the program and not the docs, a step dropped from
//! a block that still says "the six below", a list that stops before the step
//! everybody forgets (`girsa-companions`), and a reordering. It does not judge
//! the words around the commands: what a step is *for* is one place's job and
//! that place is `docs/tools.md`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root resolves")
}

fn read(root: &Path, name: &str) -> String {
    std::fs::read_to_string(root.join(name)).unwrap_or_else(|e| panic!("{name} reads: {e}"))
}

/// The binaries `tools/build-a-shelf.mjs` runs, in the order it runs them.
///
/// Read out of the program rather than listed here, so the program is the one
/// copy. A `tool("girsa-x")` whose line starts with `run(` or `runStep(` is a
/// step; the crate map at the top of the file names the same binaries and is
/// not a step, which is why the check is on the call and not on the name.
fn performed(shelf: &str) -> Vec<String> {
    let mut steps: Vec<(usize, String)> = Vec::new();
    for (index, line) in shelf.lines().enumerate() {
        let trimmed = line.trim_start();
        if !(trimmed.starts_with("run(") || trimmed.starts_with("runStep(")) {
            continue;
        }
        if let Some(at) = line.find("tool(\"girsa-") {
            let rest = &line[at + "tool(\"".len()..];
            if let Some(end) = rest.find('"') {
                steps.push((index, rest[..end].to_string()));
            }
        }
    }
    steps.sort();
    steps.into_iter().map(|(_, name)| name).collect()
}

/// Every `girsa-*` a document names inside a fenced `sh` block, in order.
///
/// Fenced shell blocks and not prose, because prose mentions a binary in a
/// sentence about what it does — the audit's complaint was about the *lists*,
/// where a missing line is a missing step.
fn listed(doc: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut inside = false;
    for line in doc.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            inside = trimmed == "```sh";
            continue;
        }
        if !inside {
            continue;
        }
        for tool in binaries_in(line) {
            // First appearance only. A page that explains a command twice is
            // not claiming an order twice, and a document that runs a step in
            // two places has said the same thing twice rather than contradicted
            // itself.
            if !found.contains(&tool) {
                found.push(tool);
            }
        }
    }
    found
}

/// The `girsa-…` names on one line, preferring the binary after `--bin`.
///
/// `cargo run -p girsa-link --bin girsa-link-types` names a crate and a
/// binary; the step is the binary, and reading the crate would make every
/// line's answer `girsa-link`.
fn binaries_in(line: &str) -> Vec<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    for (at, word) in words.iter().enumerate() {
        if *word == "--bin" {
            if let Some(name) = words.get(at + 1) {
                return vec![name.trim_matches('"').to_string()];
            }
        }
    }
    words
        .iter()
        .find(|word| word.starts_with("girsa-") && !word.contains('/'))
        .map(|word| vec![word.trim_matches('"').to_string()])
        .unwrap_or_default()
}

/// Whether `sub` appears in `all`, in order, allowing gaps.
fn in_order(all: &[String], sub: &[String]) -> bool {
    let mut at = 0;
    for want in sub {
        match all[at..].iter().position(|have| have == want) {
            Some(found) => at += found + 1,
            None => return false,
        }
    }
    true
}

/// The shelf steps, as the program performs them.
#[test]
fn the_program_performs_six_steps_and_this_is_their_order() {
    let steps = performed(&read(&repo(), "tools/build-a-shelf.mjs"));
    assert_eq!(
        steps,
        [
            "girsa-fetch",
            "girsa-import",
            "girsa-link-import",
            "girsa-link-types",
            "girsa-companions",
            "girsa-index",
        ],
        "the shelf build's own order, and the count its documentation states"
    );
}

/// `docs/tools.md` is the list a reader types, so it is the whole order.
#[test]
fn the_manual_list_is_the_whole_order_in_the_same_order() {
    let root = repo();
    let steps = performed(&read(&root, "tools/build-a-shelf.mjs"));
    let doc = read(&root, "docs/tools.md");
    let listed: Vec<String> = listed(&doc)
        .into_iter()
        .filter(|tool| steps.contains(tool))
        .collect();
    assert_eq!(
        listed, steps,
        "docs/tools.md's shell block is the shelf order, all of it and in order"
    );
    assert!(
        doc.contains("It runs the six below in order"),
        "and it says how many, so a step added to the program is a sentence to edit"
    );
}

/// The other venues list the part of the order they are talking about, and a
/// list that stops before the step everybody forgets is the audit's finding.
#[test]
fn every_venue_lists_its_steps_in_the_programs_order() {
    let root = repo();
    let steps = performed(&read(&root, "tools/build-a-shelf.mjs"));
    let mut wrong = Vec::new();
    for name in [
        "docs/tools.md",
        "docs/the-libraries.md",
        "docs/troubleshooting.md",
        "docs/start-here.md",
    ] {
        let doc = read(&root, name);
        let listed: Vec<String> = listed(&doc)
            .into_iter()
            .filter(|tool| steps.contains(tool))
            .collect();
        if listed.is_empty() {
            continue;
        }
        if !in_order(&steps, &listed) {
            wrong.push(format!("{name}: {listed:?} is not the program's order"));
        }
    }
    assert!(
        wrong.is_empty(),
        "a venue restates the order wrongly:\n  {}",
        wrong.join("\n  ")
    );
}

/// And none of them stops one short of the cache that goes silently short.
///
/// `girsa-companions` is the step the audit called easiest to miss: nothing
/// refuses without it, the מפרשים list is simply shorter than it should be, in
/// a way a reader reads as *this sefer has no commentaries*. A hand list that
/// reaches `girsa-link-types` and stops is that failure written down.
#[test]
fn a_list_that_reaches_the_caches_reaches_companions() {
    let root = repo();
    let mut wrong = Vec::new();
    for name in [
        "docs/tools.md",
        "docs/the-libraries.md",
        "docs/troubleshooting.md",
        "docs/start-here.md",
    ] {
        let doc = read(&root, name);
        let listed = listed(&doc);
        if listed.iter().any(|t| t == "girsa-link-types")
            && !listed.iter().any(|t| t == "girsa-companions")
        {
            wrong.push(name.to_string());
        }
    }
    assert!(
        wrong.is_empty(),
        "a venue lists the caches and not the companions step, which nothing refuses without: {wrong:?}"
    );
}
