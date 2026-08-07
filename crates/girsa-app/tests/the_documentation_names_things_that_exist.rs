//! Every command the documentation tells you to run is a command that exists,
//! and every file it links to is a file in this repository.
//!
//! # Why this is a test
//!
//! `docs/start-here.md` is the five-minute walkthrough that B36 called *"the
//! entire product, and it is written down nowhere a stranger will find it."*
//! Its **first step** told the stranger to run
//!
//! ```text
//! cargo run -p girsa-link --bin girsa-link-inbound
//! ```
//!
//! and there has never been a binary by that name in this tree. Twelve lines
//! later it linked `from-word.md`, which lives in Ksav. `docs/README.md` linked
//! `../../Ksav/docs/`, a path outside the repository — fine on the machine that
//! has both checkouts side by side, which is one machine, and broken for
//! everybody reading it on GitHub.
//!
//! None of those three is a hard thing to notice. All three survived because
//! nothing looked. Renaming a binary is a compile error everywhere in the tree
//! except in the one place a reader meets it first, and a documentation file is
//! the only artifact here whose references are not checked by anything at all.
//!
//! # Why it lives in `girsa-app`
//!
//! Because `docs/` is the reader's directory and `girsa-app` is the crate that
//! generates the one page in it that already had a gate — `girsa-card` writes
//! `docs/shortcuts.md`. There is no workspace-level crate to hang a
//! repository-level check on, and inventing one to hold forty lines would be
//! its own finding. If a third repository-wide check ever wants a home, that is
//! the moment to make one.
//!
//! # What it does not check
//!
//! Prose. Numbers in prose especially — *"a window and fifty commands"* was
//! wrong by 50 at the time this was written, and no reference-checker would
//! ever have said so. That is a separate gate and a separate argument.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// The repository root, from this crate's manifest.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| panic!("the repository root resolves: {e}"))
}

/// Every markdown file a reader or a builder is pointed at.
///
/// The three reader pages, the two builder documents, and the specification.
/// Not `target/`, and not anything a tool wrote.
fn documents(root: &Path) -> Vec<PathBuf> {
    let mut found = vec![
        root.join("README.md"),
        root.join("spec.md"),
        root.join("BUILDER.md"),
    ];
    let docs = root.join("docs");
    let mut pages: Vec<PathBuf> = std::fs::read_dir(&docs)
        .unwrap_or_else(|e| panic!("docs/ reads: {e}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    pages.sort();
    found.extend(pages);
    found.retain(|path| path.exists());
    assert!(
        found.len() >= 6,
        "expected the six documents at least, found {}: {found:?}",
        found.len()
    );
    found
}

/// Fenced-code text with the fences removed is still the text; this reads the
/// whole file, because a broken command is as broken in prose as in a fence.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()))
}

/// What comes after `flag` on each occurrence, up to the next whitespace.
///
/// Deliberately not a shell parser. Every invocation in these files is one
/// line of `cargo run`, and a parser that understood more would be a second
/// implementation of something no reader is running.
fn arguments_to(text: &str, flag: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(at) = rest.find(flag) {
        rest = &rest[at + flag.len()..];
        let word: String = rest
            .trim_start_matches([' ', '\t'])
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !word.is_empty() {
            found.insert(word);
        }
    }
    found
}

/// The file stems of `crates/*/<dir>/*.rs`, which is how every binary and every
/// example in this workspace is declared — there is no `[[bin]]` table anywhere.
fn targets(root: &Path, dir: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let crates = std::fs::read_dir(root.join("crates")).unwrap_or_else(|e| panic!("crates/: {e}"));
    for krate in crates.filter_map(Result::ok) {
        let Ok(entries) = std::fs::read_dir(krate.path().join(dir)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    found.insert(stem.to_string());
                }
            }
        }
    }
    assert!(
        !found.is_empty(),
        "no {dir} targets found at all — the walk is wrong, not the docs"
    );
    found
}

/// The workspace's package names, which are its directory names under `crates/`
/// plus the shell.
fn packages(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::from(["girsa-shell".to_string()]);
    let crates = std::fs::read_dir(root.join("crates")).unwrap_or_else(|e| panic!("crates/: {e}"));
    for krate in crates.filter_map(Result::ok) {
        if krate.path().join("Cargo.toml").exists() {
            if let Some(name) = krate.file_name().to_str() {
                found.insert(name.to_string());
            }
        }
    }
    found
}

#[test]
fn every_bin_the_documentation_tells_you_to_run_exists() {
    let root = repo();
    let bins = targets(&root, "src/bin");
    let mut wrong = Vec::new();
    for page in documents(&root) {
        for named in arguments_to(&read(&page), "--bin ") {
            if !bins.contains(&named) {
                wrong.push(format!("{}: --bin {named}", page.display()));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the documentation runs binaries that do not exist:\n  {}\n\nThe binaries \
         that do: {bins:?}",
        wrong.join("\n  ")
    );
}

#[test]
fn every_example_the_documentation_tells_you_to_run_exists() {
    let root = repo();
    let examples = targets(&root, "examples");
    let mut wrong = Vec::new();
    for page in documents(&root) {
        for named in arguments_to(&read(&page), "--example ") {
            if !examples.contains(&named) {
                wrong.push(format!("{}: --example {named}", page.display()));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the documentation runs examples that do not exist:\n  {}\n\nThe examples \
         that do: {examples:?}",
        wrong.join("\n  ")
    );
}

#[test]
fn every_package_the_documentation_names_is_in_the_workspace() {
    let root = repo();
    let known = packages(&root);
    let mut wrong = Vec::new();
    for page in documents(&root) {
        for named in arguments_to(&read(&page), "-p ") {
            // `-p` is also a flag on other programs quoted in these files.
            // Only ours are claims about this workspace.
            if named.starts_with("girsa-") && !known.contains(&named) {
                wrong.push(format!("{}: -p {named}", page.display()));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the documentation names packages the workspace does not have:\n  {}",
        wrong.join("\n  ")
    );
}

/// Relative links, resolved from the page they are written on.
///
/// Two rules, and the second is the one `docs/README.md` broke: the target has
/// to exist, **and** it has to be inside this repository. A `../../Ksav/…` link
/// resolves on the machine with both checkouts and nowhere else, which makes it
/// a link that is correct for its author and broken for its reader.
#[test]
fn every_relative_link_points_inside_this_repository() {
    let root = repo();
    let mut wrong = Vec::new();
    for page in documents(&root) {
        let here = page.parent().unwrap_or(&root).to_path_buf();
        for target in links(&read(&page)) {
            let joined = normalise(&here.join(&target));
            if !joined.starts_with(&root) {
                wrong.push(format!(
                    "{}: [{target}] leaves the repository",
                    page.display()
                ));
            } else if !joined.exists() {
                wrong.push(format!("{}: [{target}] is not there", page.display()));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the documentation links to things that are not here:\n  {}",
        wrong.join("\n  ")
    );
}

/// The targets of `[text](target)`, minus the ones that are not paths.
fn links(text: &str) -> Vec<String> {
    let bytes: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != '[' {
            i += 1;
            continue;
        }
        let Some(close) = (i..bytes.len()).find(|&j| bytes[j] == ']') else {
            break;
        };
        if close + 1 >= bytes.len() || bytes[close + 1] != '(' {
            i = close + 1;
            continue;
        }
        let Some(end) = (close + 2..bytes.len()).find(|&j| bytes[j] == ')') else {
            break;
        };
        let target: String = bytes[close + 2..end].iter().collect();
        i = end + 1;
        // Not paths: the web, an anchor on this page, a mail address, and the
        // one shape markdown uses for a title after the URL.
        let target = target.split_whitespace().next().unwrap_or("").to_string();
        if target.starts_with("http") || target.starts_with('#') || target.starts_with("mailto:") {
            continue;
        }
        let target = target.split('#').next().unwrap_or("").to_string();
        if target.is_empty() {
            continue;
        }
        found.push(target);
    }
    found
}

/// `..` resolved lexically, because the target may not exist and
/// `canonicalize` needs it to.
fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}
