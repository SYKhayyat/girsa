//! Every `spec.md §N`, every `W<n>` and every `B<n>` a comment cites is a
//! section, an order or an order-number that exists in this repository.
//!
//! # Why this is a test
//!
//! This tree argues with itself in citations. `spec.md §9.4` is why a search
//! widens nothing; `W28` is why a chain walk refuses rather than guesses; `B12`
//! is why a 1.2 MB segment was cut into places. Two thousand of those
//! references are the connective tissue between a line of code and the argument
//! for it — and until this file, **not one of them was read by anything except
//! a person**.
//!
//! The audit that asked for this (item 11) named the failure exactly: a section
//! that is renumbered, or an order that is renamed, leaves every comment
//! pointing at thin air, and thin air reads exactly like a citation. Item 13 was
//! the same defect caught in the wild — `girsa-mcp` opened with *"spec.md §12
//! and BUILDER.md W28: MCP on both ends"*, and §12 is **Architecture**, which
//! does not mention MCP once. Somebody wrote a true sentence with a false
//! address, which is this repository's whole subject.
//!
//! Writing this test found two more of them on the first run, which is the
//! argument for it better than any paragraph:
//!
//! - `W29` — a `.ksav` on the shelf keeps its footnotes, headings, lists and
//!   tables — is cited four times in `girsa-corpus` and **had no section in
//!   `BUILDER.md` at all**. The order landed on 29 July 2026 (commit
//!   `30a346e`) and its ledger entry was never written, so the citation pointed
//!   at the gap between W28 and W30. The section is there now.
//! - Two comments in `the_rules_this_repository_wrote_down.rs` attributed their
//!   four rules to `§19`, which is a section of nothing in this tree. The
//!   paragraph they mean is the lamdan report's *"turn four doc comments into
//!   tests"*, and they say that now.
//!
//! # What it does not check
//!
//! That the section *says* what the comment claims it says. `§12` was a real
//! section; the defect was that it was the wrong one. A machine can know that a
//! section exists and cannot know that it is the right section — item 13 is
//! fixed by hand and can only be re-broken by a reader who does not check. What
//! this test guarantees is the weaker, cheap, previously-absent half: a
//! renumbering cannot silently orphan a thousand citations again.
//!
//! Prose is out of scope. `docs/`, `BUILDER.md` and `spec.md` cite each other
//! constantly and their `§N` are relative to whichever document is speaking;
//! the subject here is code comments, where a citation is either `spec.md §N`,
//! `BUILDER.md §0.N`, `W<n>` or `B<n>` and never a guess.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The repository root, from this crate's manifest.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| panic!("the repository root resolves: {e}"))
}

/// Every source file whose comments are citations: the workspace, the shell,
/// and the window.
///
/// Not `target/`, and not the sibling checkouts — `sefer-crates` is a different
/// repository with a different `spec.md`, and a citation of `§7` in it is not a
/// citation of ours.
fn sources(root: &Path) -> Vec<PathBuf> {
    let scanner = Path::new(file!()).file_name();
    let mut found = Vec::new();
    for (dir, extensions) in [
        (root.join("crates"), &["rs"][..]),
        (root.join("app/src-tauri/src"), &["rs"][..]),
        (root.join("app/src"), &["ts"][..]),
        (root.join("app/test"), &["mjs"][..]),
        (root.join("tools"), &["mjs"][..]),
    ] {
        walk(&dir, extensions, &mut found);
    }
    // This file is the one cit*or* that would make the scan red by existing:
    // its subject is citations, so it names the false ones on purpose. Skipping
    // it here is the honest exclusion, and it is one line rather than a list of
    // allowed strings.
    found.retain(|path| path.file_name() != scanner);
    found.sort();
    found
}

fn walk(dir: &Path, extensions: &[&str], into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "node_modules" || n == "target")
            {
                continue;
            }
            walk(&path, extensions, into);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| extensions.contains(&ext))
        {
            into.push(path);
        }
    }
}

/// The comment half of one line: everything from the first `//`.
///
/// Comments are where citations live, and reading code as well as comments
/// would make `use base64::… as B64` a citation of order 64. A `//` inside a
/// string literal is not a comment and is read as one here; that is a false
/// positive in the safe direction — it can only make this test notice something
/// that a reader would also notice.
fn comment_on(line: &str) -> &str {
    let trimmed = line.trim_start();
    // Inside a block comment, where the `//` of an opening line is long gone.
    // `///` and `//!` are line comments and are handled by the case below.
    if trimmed.starts_with('*') && !trimmed.starts_with("*/") {
        return trimmed;
    }
    match line.find("//") {
        Some(at) => &line[at..],
        None => "",
    }
}

/// One thing a comment points at.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Citation {
    /// A section of `spec.md` or of `BUILDER.md`, by number: `9.4`, `0.3`.
    Section { in_builder: bool, number: String },
    /// A work order in `BUILDER.md`: `W28`.
    Order(String),
    /// An order in the round of 30 July 2026: `B13`.
    Request(String),
}

impl std::fmt::Display for Citation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Citation::Section { in_builder, number } => {
                let doc = if *in_builder { "BUILDER.md" } else { "spec.md" };
                write!(f, "{doc} §{number}")
            }
            Citation::Order(n) => write!(f, "W{n}"),
            Citation::Request(n) => write!(f, "B{n}"),
        }
    }
}

/// A number that starts at `bytes[at]`: digits, then optionally `.digits`, then
/// optionally one ascii letter — `9`, `9.4`, `2.3b`.
///
/// Returns the number as written and the index after it.
fn number_at(bytes: &[u8], at: usize) -> Option<(String, usize)> {
    let mut end = at;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == at {
        return None;
    }
    if end + 1 < bytes.len() && bytes[end] == b'.' && bytes[end + 1].is_ascii_digit() {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end < bytes.len() && bytes[end].is_ascii_lowercase() {
        end += 1;
    }
    Some((String::from_utf8_lossy(&bytes[at..end]).into_owned(), end))
}

/// Whether the byte before `at` is part of a longer word, in which case this is
/// not the start of a citation (`ROW30`, `tomorrow13`).
fn is_word_start(bytes: &[u8], at: usize) -> bool {
    at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_')
}

/// Every citation in one comment.
///
/// The document a bare `§` belongs to is read from the comment around it:
/// `BUILDER.md §0.3` names its own document, everything else is `spec.md`, and
/// a section numbered `0.x` is `BUILDER.md`'s because `spec.md` has no section
/// zero.
fn citations_in(comment: &str) -> Vec<Citation> {
    let bytes = comment.as_bytes();
    let mut found = Vec::new();
    for (at, character) in comment.char_indices() {
        if character == '§' || character == 'W' || character == 'B' {
            if !is_word_start(bytes, at) {
                continue;
            }
            if let Some((number, _)) = number_at(bytes, at + character.len_utf8()) {
                let citation = if character == '§' {
                    // The document a `§` belongs to, from the word in front of
                    // it. `BUILDER.md §0.3` names its own document; a `§2`
                    // quoted inside a sentence *about* BUILDER.md is still
                    // spec.md's, so the mention has to be the one touching the
                    // sign and not the one nearest it.
                    let before = &comment[..at];
                    // Whichever document was named most recently, and only if
                    // it is touching the sign: `BUILDER.md §0.3` is
                    // BUILDER.md's, and `(BUILDER.md W14, spec.md §9.8)` is
                    // spec.md's because spec.md is the nearer of the two.
                    let builder_at = before.rfind("BUILDER.md");
                    let spec_at = before.rfind("spec.md");
                    let touching = builder_at.is_some_and(|found_at| {
                        spec_at.is_none_or(|nearest| nearest < found_at)
                            && at - (found_at + "BUILDER.md".len()) <= 20
                    });
                    let in_builder = touching || number.starts_with('0');
                    Citation::Section { in_builder, number }
                } else if character == 'W' {
                    Citation::Order(number)
                } else {
                    Citation::Request(number)
                };
                found.push(citation);
            }
        }
    }
    found
}

/// The numbered sections of a document: `## 9. Searching`, `### 9.4 Morphology`
/// and `### 0.1 STOP AND ASK` all name a section.
///
/// A heading is a line that starts with `#` and whose first word is a number.
/// Anything else — `## Tier 6 — Corrections`, `## Sources` — names nothing by
/// number and cannot be cited by one.
fn sections_in(document: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in document.lines() {
        let Some(heading) = line.trim_start().strip_prefix('#') else {
            continue;
        };
        let heading = heading.trim_start_matches('#').trim_start();
        let bytes = heading.as_bytes();
        if let Some((number, _)) = number_at(bytes, 0) {
            found.insert(number);
        }
    }
    found
}

/// The work orders a document declares: `### W28 · Chain tracing`.
fn orders_in(document: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in document.lines() {
        let Some(marker) = line.find("W") else {
            continue;
        };
        if !is_word_start(line.as_bytes(), marker) {
            continue;
        }
        if let Some((number, _)) = number_at(line.as_bytes(), marker + 1) {
            found.insert(number);
        }
    }
    found
}

/// The orders of the 30 July 2026 round that this tree has a name for: the
/// register row `| B13 | … |` and the status line `**B19 · …**`.
fn requests_in(document: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in document.lines() {
        let trimmed = line.trim_start();
        let cell = trimmed
            .strip_prefix("| B")
            .or_else(|| trimmed.strip_prefix("**B"));
        let Some(cell) = cell else {
            continue;
        };
        let bytes = cell.as_bytes();
        if let Some((number, _)) = number_at(bytes, 0) {
            // A register row ends its number at the cell boundary; the `**` form
            // is followed by ` · `. Either way the number is what was parsed,
            // and a letter suffix (`4a`, `10a`) is part of it.
            found.insert(number);
        }
    }
    found
}

/// Every citation in the tree, with where it was found.
fn cited(root: &Path) -> Vec<(String, usize, Citation)> {
    let mut found = Vec::new();
    for path in sources(root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let shown = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        for (index, line) in text.lines().enumerate() {
            for citation in citations_in(comment_on(line)) {
                found.push((shown.clone(), index + 1, citation));
            }
        }
    }
    found
}

/// What does not resolve, as `path:line  citation`, one per line.
fn unresolved(root: &Path, spec: &BTreeSet<String>, builder: &BTreeSet<String>) -> Vec<String> {
    let orders = orders_in(&read(root, "BUILDER.md"));
    let requests = requests_in(&read(root, "BUILDER.md"));
    let mut wrong = Vec::new();
    for (path, line, citation) in cited(root) {
        let resolves = match &citation {
            Citation::Section { in_builder, number } => {
                if *in_builder {
                    builder.contains(number)
                } else {
                    spec.contains(number)
                }
            }
            Citation::Order(number) => orders.contains(number),
            Citation::Request(number) => requests.contains(number),
        };
        if !resolves {
            wrong.push(format!("{path}:{line}  {citation}"));
        }
    }
    wrong.sort();
    wrong.dedup();
    wrong
}

fn read(root: &Path, name: &str) -> String {
    std::fs::read_to_string(root.join(name)).unwrap_or_else(|e| {
        panic!("{name} reads, because every citation in the tree is resolved against it: {e}")
    })
}

#[test]
fn every_section_a_comment_cites_is_a_section() {
    let root = repo();
    let spec = sections_in(&read(&root, "spec.md"));
    let builder = sections_in(&read(&root, "BUILDER.md"));
    // A floor, so that a document that lost its headings to a bad merge cannot
    // make every citation resolve by making none of them checkable.
    assert!(
        spec.len() > 40 && builder.len() > 3 && spec.contains("1") && builder.contains("0.3"),
        "the sections of spec.md and BUILDER.md were read: {} spec, {} builder",
        spec.len(),
        builder.len()
    );

    let wrong = unresolved(&root, &spec, &builder);
    assert!(
        wrong.is_empty(),
        "a comment cites a section that is not in the document it names:\n  {}\n\n\
         Either the citation moved or the section did. A renumbered section is the \
         one edit that cannot be caught by a compiler — this is what catches it.",
        wrong.join("\n  ")
    );

    // And the guard has a subject. `spec.md` §9.4 is cited from the search crate,
    // the ladder and the shell, and it is the section the coverage manifest
    // breaks to prove this test can fail; if it stops being cited, the check
    // above still passes and stops meaning anything.
    let cited = cited(&root);
    assert!(
        cited.iter().any(|(_, _, citation)| matches!(
            citation,
            Citation::Section { in_builder: false, number } if number == "9.4"
        )),
        "a comment cites spec.md §9.4 somewhere in the tree"
    );
}

#[test]
fn every_work_order_a_comment_cites_is_a_work_order() {
    let root = repo();
    let orders = orders_in(&read(&root, "BUILDER.md"));
    // `W28` and `W30` are cited in a hundred places and both exist; a document
    // read as empty would pass every check below it.
    assert!(
        orders.len() > 30 && orders.contains("28") && orders.contains("44"),
        "BUILDER.md's work orders were read: {} of them",
        orders.len()
    );

    let mut wrong = Vec::new();
    for (path, line, citation) in cited(&root) {
        if let Citation::Order(number) = &citation {
            if !orders.contains(number) {
                wrong.push(format!("{path}:{line}  {citation}"));
            }
        }
    }
    wrong.sort();
    wrong.dedup();
    assert!(
        wrong.is_empty(),
        "a comment cites a work order that BUILDER.md does not have:\n  {}\n\n\
         W29 is how this was found: an order that landed on 29 July 2026 with no \
         ledger entry, cited four times from the code that implemented it.",
        wrong.join("\n  ")
    );
}

#[test]
fn every_order_number_a_comment_cites_is_in_the_register() {
    let root = repo();
    let requests = requests_in(&read(&root, "BUILDER.md"));
    assert!(
        requests.len() > 20 && requests.contains("13") && requests.contains("36"),
        "the register of orders was read: {} of them",
        requests.len()
    );

    let mut wrong = Vec::new();
    for (path, line, citation) in cited(&root) {
        if let Citation::Request(number) = &citation {
            if !requests.contains(number) {
                wrong.push(format!("{path}:{line}  {citation}"));
            }
        }
    }
    wrong.sort();
    wrong.dedup();
    assert!(
        wrong.is_empty(),
        "a comment cites an order the register does not name:\n  {}\n\n\
         A `Bn` with no row is a reference a reader cannot follow anywhere in this \
         tree — the order documents that defined them were deleted, which is why \
         the register exists.",
        wrong.join("\n  ")
    );
}

#[test]
fn the_scan_notices_a_citation_that_points_at_nothing() {
    // The half that makes the three above worth having. A scan that finds
    // nothing because it looks for nothing passes every check in this file, and
    // a gate that cannot fail is the shape this repository refuses by name.
    let fake = "//! spec.md §99 and BUILDER.md W999, and the §0.9 nobody wrote (B999).";
    let found = citations_in(fake);
    let spec = sections_in(&read(&repo(), "spec.md"));
    let builder = sections_in(&read(&repo(), "BUILDER.md"));
    assert_eq!(found.len(), 4, "four citations in that line: {found:?}");
    for citation in &found {
        let resolves = match citation {
            Citation::Section { in_builder, number } => {
                if *in_builder {
                    builder.contains(number)
                } else {
                    spec.contains(number)
                }
            }
            // Neither W999 nor B999 exists, and both are asserted below.
            Citation::Order(_) | Citation::Request(_) => false,
        };
        assert!(!resolves, "{citation} resolved, so the scan is not reading");
    }
    assert!(found.contains(&Citation::Section {
        in_builder: true,
        number: "0.9".into()
    }));

    // And the other direction: a citation that does exist is not reported, so a
    // red test above is a real dangling reference and not a scan that reports
    // everything.
    let real = "//! The pane is where spec.md §9.9 says it is, per BUILDER.md §0.2 and W28.";
    let found = citations_in(real);
    assert_eq!(found.len(), 3, "three citations in that line: {found:?}");
    for citation in &found {
        let resolves = match citation {
            Citation::Section { in_builder, number } => {
                if *in_builder {
                    builder.contains(number)
                } else {
                    spec.contains(number)
                }
            }
            Citation::Order(number) => orders_in(&read(&repo(), "BUILDER.md")).contains(number),
            Citation::Request(_) => true,
        };
        assert!(resolves, "{citation} did not resolve, and it exists");
    }
}

#[test]
fn the_mcp_end_cites_the_milestone_that_names_mcp() {
    // Item 13 of the lamdan report, which the section checks above cannot
    // reach: `girsa-mcp` opened with *"spec.md §12 and BUILDER.md W28"*, §12
    // exists, and §12 is Architecture and says nothing about MCP. A number
    // that resolves is not a citation that points at the right thing, so the
    // one case where a reader caught it is held by hand.
    //
    // Both halves are read from the documents rather than written here: the
    // milestone the header names has to be a milestone, and it has to be the
    // one whose own words name MCP.
    let root = repo();
    let spec = read(&root, "spec.md");
    let m8 = spec
        .lines()
        .find(|line| line.starts_with("**M8 —"))
        .unwrap_or_else(|| panic!("spec.md §15 names a milestone M8"));
    assert!(
        m8.contains("MCP"),
        "M8 is the milestone the MCP end belongs to, and this is what it says: {m8}"
    );

    let header = read(&root, "crates/girsa-mcp/src/lib.rs");
    let opening: String = header
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("//!"))
        .take_while(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        opening.contains("M8"),
        "the MCP end names the milestone that specifies it:\n{opening}"
    );
}
