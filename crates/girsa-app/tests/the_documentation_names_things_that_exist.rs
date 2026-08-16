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
/// The pages under `docs/`, the two builder documents, the specification, and
/// the two front doors at the root. Not `target/`, and not anything a tool
/// wrote.
///
/// `CONTRIBUTING.md` is in here for the reason the whole file exists. It is the
/// first page a contributor opens, it names commands and links to a dozen other
/// pages, and it sat outside this walk for exactly as long as it took somebody
/// to notice — which is the same shape as `docs/start-here.md` opening with a
/// binary that had never been in this tree.
/// Every `.md` under a directory, at any depth.
///
/// It read `docs/` one level deep until 14 August, which was true of `docs/`
/// while `docs/` was flat. It stopped being true twice in one change: the record
/// split into `docs/record/`, twelve pages of links to the rest of the tree, and
/// `docs/images/README.md` — which had been unchecked since the day it was
/// written and is the page that tells somebody how to take a screenshot. A walk
/// that stops at the first subdirectory is a walk that checks whatever nobody
/// has filed yet.
fn walk(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            into.push(path);
        }
    }
}

fn documents(root: &Path) -> Vec<PathBuf> {
    let mut found = vec![
        root.join("README.md"),
        root.join("CONTRIBUTING.md"),
        root.join("spec.md"),
        root.join("BUILDER.md"),
    ];
    let mut pages = Vec::new();
    walk(&root.join("docs"), &mut pages);
    pages.sort();
    found.extend(pages);
    found.retain(|path| path.exists());
    assert!(
        found.len() >= 7,
        "expected the seven documents at least, found {}: {found:?}",
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

/// The targets of `[text](target)`, minus the two that name nothing in this
/// tree.
///
/// Anchors are kept here and thrown away by the caller that does not want
/// them, because the two callers want different halves of the same string and
/// scanning the brackets twice to get them would be two implementations of one
/// thing.
fn targets_in(text: &str) -> Vec<String> {
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
        // Not here: the web, a mail address, and the one shape markdown uses
        // for a title after the URL.
        let target = target.split_whitespace().next().unwrap_or("").to_string();
        if target.starts_with("http") || target.starts_with("mailto:") {
            continue;
        }
        if target.is_empty() {
            continue;
        }
        found.push(target);
    }
    found
}

/// The file half of every target that has one. An anchor on this page names no
/// file and is dropped.
fn links(text: &str) -> Vec<String> {
    targets_in(text)
        .into_iter()
        .filter_map(|target| {
            let path = target.split('#').next().unwrap_or("").to_string();
            (!path.is_empty()).then_some(path)
        })
        .collect()
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

// ---------------------------------------------------------------------------
// And the half of a link that was being thrown away
//
// `every_relative_link_points_inside_this_repository` split `page.md#section`
// on the `#` and checked the left half. The right half was dropped on the line
// above this comment's arrival, with no argument for dropping it beyond that
// resolving a path is the thing the test was about.
//
// That was survivable while four anchors existed in the whole tree. On 14
// August `docs/not-yet.md` was written — one page whose entire job is to point
// at the eight sections of `docs/record/` that say what is unfinished — and
// took the count to twenty-one. Seventeen new references, none of them checked
// by anything, in a repository whose position is that a copy nothing
// regenerates is a copy that rots.
//
// A heading is easier to break than a filename, too, and quieter. Renaming a
// file is felt: something fails to open. Reword a heading and every link into
// it lands at the top of the page instead, which reads exactly like a page
// that was always going to open there.

/// A heading's anchor, as GitHub spells it: lower case, punctuation dropped,
/// spaces to hyphens. Emphasis markers go first, since they are how the text
/// is written and not part of it.
fn slug(heading: &str) -> String {
    let kept: String = heading
        .replace(['`', '*', '_'], "")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .collect();
    kept.trim().replace(' ', "-")
}

/// Every anchor a reader could land on in one page.
///
/// A `#` line inside a fenced block is a shell comment and is counted as a
/// heading here, which can only make this check more forgiving than a browser
/// is — never less. Telling the two apart needs a fence parser, and a fence
/// parser to widen a check that is already passing is work with no finding in
/// it.
fn headings(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim_start_matches('#');
            let hashes = line.len() - rest.len();
            ((1..=6).contains(&hashes) && rest.starts_with(' ')).then(|| slug(rest.trim()))
        })
        .collect()
}

/// The `(file, anchor)` of every target that names a section. An empty file is
/// an anchor on the page it is written on, which is the same claim about a
/// heading and is checked the same way.
fn anchors(text: &str) -> Vec<(String, String)> {
    targets_in(text)
        .into_iter()
        .filter_map(|target| {
            let (path, anchor) = target.split_once('#')?;
            (!anchor.is_empty()).then(|| (path.to_string(), anchor.to_string()))
        })
        .collect()
}

#[test]
fn every_anchor_points_at_a_heading_that_exists() {
    let root = repo();
    let mut checked = 0usize;
    let mut wrong = Vec::new();
    for page in documents(&root) {
        let here = page.parent().unwrap_or(&root).to_path_buf();
        for (file, anchor) in anchors(&read(&page)) {
            let target = if file.is_empty() {
                page.clone()
            } else {
                normalise(&here.join(&file))
            };
            // Whether the file is there at all, and whether it is in this
            // repository, is the test above's question and its wording is
            // better at it. This one is only about the heading.
            if target.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            let Ok(body) = std::fs::read_to_string(&target) else {
                continue;
            };
            checked += 1;
            if !headings(&body).contains(&anchor) {
                wrong.push(format!(
                    "{}: [{file}#{anchor}] is not a heading in that page",
                    page.display()
                ));
            }
        }
    }
    assert!(
        checked > 0,
        "no anchor links found in any document at all — the scan is wrong, not the docs"
    );
    assert!(
        wrong.is_empty(),
        "the documentation links to sections that are not there:\n  {}\n\nA heading \
         that was reworded takes every link into it to the top of the page, which \
         reads like a page that always opened there.",
        wrong.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// The other direction
//
// The three tests above ask *does what the documentation names exist*. The 9
// August report pointed at the test forty lines from here that asks the reverse
// question about measurements — `every_measurement_is_claimed_somewhere`, whose
// own comment is *"a measurement nobody cites is a check that runs, passes, and
// guards nothing"* — and noted that the same argument was never applied here:
//
// > `the_documentation_names_things_that_exist.rs:148-165` runs docs→bins only,
// > so a binary no document names passes. `girsa-read` and `girsa-companions`
// > have never been seen by it — and `linksview.ts:134` tells the reader, in
// > Hebrew, *"הרץ girsa-companions"*: a reading application instructing its
// > reader to go run a cargo binary.
//
// A binary nobody is told to run is not a smaller problem than a binary that
// does not exist. It is the same problem — a reader who cannot get to the thing
// — arriving from the other side, and it is the quieter one, because building
// the tool feels like shipping it.

/// A binary or example that no reader is meant to reach, and why.
///
/// Each is a claim. A tool that ends up here because writing the line was work
/// is the failure this test exists to make visible, so the reason has to be one
/// that would still be true if somebody argued with it.
const NOT_FOR_A_READER: &[(&str, &str)] = &[
    (
        "girsa-card",
        "writes docs/shortcuts.md, and the page it writes says so. A command \
         whose output is the documentation cannot be documented by its output \
         without saying it twice.",
    ),
    (
        "dev-fixtures",
        "writes public/dev/*.json for the browser build, which BUILDER.md's W9 \
         describes and which nobody runs by hand — `npm run dev` runs it.",
    ),
];

#[test]
fn every_command_this_workspace_ships_is_one_a_reader_is_told_to_run() {
    let root = repo();
    let mut told = BTreeSet::new();
    for page in documents(&root) {
        let body = read(&page);
        told.extend(arguments_to(&body, "--bin "));
        told.extend(arguments_to(&body, "--example "));
    }

    let excused: BTreeSet<&str> = NOT_FOR_A_READER.iter().map(|(name, _)| *name).collect();
    let mut silent = Vec::new();
    for name in targets(&root, "src/bin")
        .into_iter()
        .chain(targets(&root, "examples"))
    {
        if told.contains(&name) || excused.contains(name.as_str()) {
            continue;
        }
        silent.push(name);
    }
    assert!(
        silent.is_empty(),
        "built and never mentioned — no document tells a reader to run:\n  {}\n\nA \
         tool nobody is told about is a tool nobody has. Either give it a line \
         in a document, or say in NOT_FOR_A_READER why it is not for one.",
        silent.join("\n  ")
    );
}

#[test]
fn nothing_is_excused_from_that_which_does_not_exist() {
    // The exemption list is a claim about targets in this tree, so it decays
    // the same way any other hand-written list does: a tool that is renamed or
    // removed leaves its excuse behind, and the next tool with a similar name
    // inherits it silently.
    let root = repo();
    let mut all = targets(&root, "src/bin");
    all.extend(targets(&root, "examples"));
    let stale: Vec<&str> = NOT_FOR_A_READER
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !all.contains(*name))
        .collect();
    assert!(
        stale.is_empty(),
        "excused from being documented, and not in the tree: {stale:?}"
    );
    for (name, why) in NOT_FOR_A_READER {
        assert!(
            why.len() > 40,
            "{name}'s reason has to be an argument, not a shrug: {why:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// And the window is a document too
//
// Four places in `app/src/` tell the reader to go and run something:
//
// ```text
// linksview.ts  אין מטמון נכנס … הרץ girsa-link-types
// search.ts     לא נבנה — הרץ girsa-link-types ובנה אינדקס מחדש
// suspects.ts   אין תור. הרץ: cargo run … --bin girsa-suspects …
// trouble.ts    אין אינדקס חיפוש — יש לבנות אותו: girsa-index build
// ```
//
// The report is right that a reading application handing its reader a cargo
// command is a smell. It is also the honest answer for a three-minute batch over
// 665 MB that is step four of the setup — what is *not* honest is naming the
// wrong one, and `linksview.ts` named `girsa-companions` for a cache that
// `girsa-link-types` writes. `search.ts` reports the same cold cache and names
// it right, so the two sat four files apart contradicting each other.
//
// A checker cannot know which cache a Hebrew sentence is about. It can know that
// every tool the window names is a tool that exists and that `docs/tools.md`
// tells a reader how to run — which is the fence that stops the next one being
// a name nobody can act on.

/// Every `girsa-…` word in a shipped string in `app/src/`.
fn tools_the_window_names(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let src = root.join("app/src");
    let entries = std::fs::read_dir(&src).unwrap_or_else(|e| panic!("app/src: {e}"));
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "ts") {
            continue;
        }
        for line in read(&path).lines() {
            // Comments are prose about the code, including the paragraph above
            // this test explaining what the wrong name used to be.
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with('*') || code.starts_with("/*") {
                continue;
            }
            let mut rest = line;
            while let Some(at) = rest.find("girsa-") {
                rest = &rest[at..];
                let word: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                    .collect();
                rest = &rest[word.len()..];
                found.insert(word);
            }
        }
    }
    found
}

#[test]
fn every_tool_the_window_tells_you_to_run_is_one_you_can_be_told_how_to_run() {
    let root = repo();
    let mut real = targets(&root, "src/bin");
    real.extend(targets(&root, "examples"));
    let told: String = documents(&root).iter().map(|page| read(page)).collect();

    let mut wrong = Vec::new();
    for named in tools_the_window_names(&root) {
        // `girsa-corpus`, `girsa-app` and the like are crate names, and the
        // window says them in paths and messages about where things live.
        if !real.contains(&named) {
            continue;
        }
        if !told.contains(&named) {
            wrong.push(named);
        }
    }
    assert!(
        wrong.is_empty(),
        "the window tells a reader to run a command no document explains: {wrong:?}"
    );
}

/// A documented invocation that the tool it names would refuse.
///
/// # What this file checked, and what it did not
///
/// Everything above asks whether a thing a document names **exists**. Every one
/// of them passed while `docs/start-here.md` — the page whose first sentence is
/// *do this once and the rest will make sense* — opened with four commands, all
/// four of which print a usage line and do nothing:
///
/// ```text
/// cargo run -p girsa-corpus --bin girsa-fetch          # the seforim
/// cargo run -p girsa-link  --bin girsa-link-import     # the links between them
/// cargo run -p girsa-link  --bin girsa-link-types      # the caches that read them backwards
/// cargo run -p girsa-search --bin girsa-index          # the search index
/// ```
///
/// `girsa-fetch` wants a corpus. `girsa-link-import` wants a corpus **and** an
/// Otzaria tree, which is a download the reader has to make and which that page
/// never mentioned. `girsa-index` wants a subcommand before anything. And the
/// step that turns the download into a shelf — `girsa-import` — was not in the
/// list at all. A newcomer following the page got four usage lines and no
/// library. `docs/tools.md` carried the same block with the same holes, under a
/// heading that says *in this order, once, before anything else*.
///
/// The binaries all existed, so every check here was green. Naming a tool and
/// calling it correctly are two different claims, and only the first was being
/// made.
///
/// # How the requirement is known
///
/// From the tool itself. Every binary here prints `usage: girsa-x <a> [b]`, and
/// the convention is uniform: `<angled>` is required, `[bracketed]` is not. So
/// the source is scanned for that line, the `<…>` words are counted, and a
/// documented `--bin girsa-x` line has to carry at least that many words of its
/// own.
///
/// **Words, not correctness.** This cannot tell whether `corpus` is the right
/// directory, and does not try — it catches the invocation that could not
/// possibly work, which is the one that had been sitting in the onboarding.
/// `girsa-index` is the one tool whose usage is a block of subcommands rather
/// than a line, so it is checked for having a subcommand instead.
#[test]
fn every_documented_invocation_carries_the_words_its_tool_requires() {
    let root = repo();
    let wants = required_words(&root);
    let mut wrong = Vec::new();
    for page in documents(&root) {
        let text = read(&page);
        for (n, line) in continued(&text) {
            let Some((tool, given)) = invocation(&line) else {
                continue;
            };
            let Some(&want) = wants.get(tool.as_str()) else {
                continue;
            };
            if given.len() < want {
                wrong.push(format!(
                    "{}:{}: `{tool}` needs {want} word(s) and is given {}: {}",
                    page.display(),
                    n + 1,
                    given.len(),
                    line.trim(),
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} documented invocation(s) would print a usage line rather than run:\n{}\n\n\
         Each one names a tool that exists and calls it in a way it refuses.",
        wrong.len(),
        wrong.join("\n"),
    );
}

/// How many words each binary and each example requires, read from its own
/// usage line.
///
/// `girsa-index` is entered by hand at one, because its usage is a block of
/// subcommands rather than a single line and every one of them starts with a
/// verb. One word is the claim being made: a documented `girsa-index` must at
/// least say which of the five things it is doing.
///
/// # Examples too, and why they were not here
///
/// This walked for `usage: girsa-` and nothing else, so it saw the binaries and
/// none of the fifteen examples — whose usage lines begin `usage: measure`,
/// `usage: write`, `usage: build-lexicon`. Four documented `--example` lines
/// were therefore in exactly the state the `--bin` lines had been in the day
/// this test was written: they name a tool that exists, call it in a way it
/// refuses, and print a usage line instead of doing anything.
///
/// Keyed `bin:girsa-x` / `example:x`, because the two namespaces are separate —
/// there is no rule stopping a crate having both, and a collision would silently
/// hold one to the other's arguments. An example is keyed by its **file stem**,
/// which is what `--example` takes; the fifteen are checked for uniqueness by
/// this map being built from paths.
fn required_words(root: &Path) -> std::collections::BTreeMap<String, usize> {
    let mut wants = std::collections::BTreeMap::new();
    let mut stack = vec![root.join("crates")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            // An example is named by its file, and a binary by its usage line —
            // `src/bin/girsa-x.rs` agrees with `usage: girsa-x`, so the binary
            // half is left reading the line it always read.
            let example = path
                .parent()
                .is_some_and(|p| p.file_name().is_some_and(|n| n == "examples"))
                .then(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
                .flatten();
            for line in read(&path).lines() {
                let Some(rest) = line.split("usage: ").nth(1) else {
                    continue;
                };
                let mut words = rest.split_whitespace();
                let Some(named) = words.next() else { continue };
                let needed = words
                    .filter(|w| w.starts_with('<') && !w.contains("--"))
                    .count();
                let key = match &example {
                    // The usage line has to be this example's own, or a doc
                    // comment quoting another tool would set its requirement.
                    Some(stem) if named == stem => format!("example:{stem}"),
                    Some(_) => continue,
                    None if named.starts_with("girsa-") => format!("bin:{named}"),
                    None => continue,
                };
                // The first usage line wins, so `girsa-index`'s `build` line
                // sets it and its four siblings do not raise it.
                wants.entry(key).or_insert(needed);
            }
        }
    }
    wants
}

/// A document's lines, with shell continuations joined.
///
/// A `cargo run …` too long for one line is written the way anybody writes one,
/// with a trailing backslash — and a reader copies the whole block, so the
/// invocation is the joined line and not either half. Read a line at a time,
/// the first half looks like a command called with one argument and the second
/// half looks like nothing at all.
///
/// The number that comes back is the **first** line's, so a failure points at
/// where the command starts rather than at where it happened to wrap.
fn continued(text: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut held: Option<(usize, String)> = None;
    for (n, line) in text.lines().enumerate() {
        let more = line.trim_end().ends_with('\\');
        let piece = line.trim_end().trim_end_matches('\\');
        match held.take() {
            Some((first, mut so_far)) => {
                so_far.push(' ');
                so_far.push_str(piece.trim());
                if more {
                    held = Some((first, so_far));
                } else {
                    out.push((first, so_far));
                }
            }
            None if more => held = Some((n, piece.to_string())),
            None => out.push((n, line.to_string())),
        }
    }
    // A block that ends mid-continuation is still worth checking.
    if let Some(dangling) = held {
        out.push(dangling);
    }
    out
}

/// The tool and the words given to it on one documented `cargo run` line.
///
/// A word is anything after `--bin <tool>` or `--example <name>` that is not an
/// option, not a comment and not cargo's own `--` separator. `<otzaria>` counts
/// as a word: it is a placeholder the reader fills in, which is a different
/// thing from an argument that is not there at all.
///
/// Returns the key `required_words` files the tool under, so the two cannot
/// drift into looking each other up by different names.
fn invocation(line: &str) -> Option<(String, Vec<String>)> {
    let (kind, after) = match (line.split("--bin ").nth(1), line.split("--example ").nth(1)) {
        (Some(after), _) => ("bin", after),
        (None, Some(after)) => ("example", after),
        (None, None) => return None,
    };
    let mut words = after.split_whitespace();
    let tool = words.next()?.to_string();
    if kind == "bin" && !tool.starts_with("girsa-") {
        return None;
    }
    let given: Vec<String> = words
        .take_while(|w| !w.starts_with('#') && *w != ">")
        .filter(|w| !w.starts_with('-'))
        .map(ToString::to_string)
        .collect();
    Some((format!("{kind}:{tool}"), given))
}

/// A tool that writes a cache into the corpus says so, and names it.
///
/// # What went wrong
///
/// Two of the five commands `docs/start-here.md` tells a newcomer to run
/// described the wrong job in `--help`, and the page invites them to look:
/// *"every one answers `--help` if you would rather read it there than here."*
///
/// - `girsa-link-types` said *"Counts the edge types the corpus ships"*. It
///   walks 4.1 million edges and writes about 575 MB of `inbound.jsonl` plus
///   `touching.bits` into the corpus. Counting is what it prints on the way
///   past.
/// - `girsa-companions` said *"Builds the inbound half of the link graph"* —
///   which is `girsa-link-types`' job, not its own. It writes
///   `companions.jsonl`: which seforim are worth opening beside which.
///
/// Both binaries existed, both were named by a document, both took the
/// arguments their usage lines said they took, and every check above was green.
/// A reader who did the responsible thing and read `--help` before pointing a
/// tool at their disk was told what a different tool does.
///
/// # The claim this makes, and the one it does not
///
/// It makes one claim: **for each artefact below, the binary that writes it
/// names it in its own usage.** That is exact, it is not satisfiable by adding
/// a word, and it fails on both of the strings above.
///
/// It does **not** check that the rest of a usage line is true. Nothing here
/// can, and a guard that tried by matching prose against prose would pass by
/// sharing the word *link*. The table is written out rather than derived for
/// the same reason: derived, it would have to guess which string literals in a
/// binary are things it writes rather than things it reads, and a guard that
/// guesses is a guard that gets edited until it stops complaining.
///
/// A new cache with no row here is not caught. That is the honest limit, and it
/// is why the row is one line next to the code that writes the file.
#[test]
fn a_tool_that_writes_a_cache_into_the_corpus_names_it_in_its_usage() {
    // binary source, relative to `crates/` -> what it writes into the corpus.
    const WRITES: [(&str, &[&str]); 3] = [
        (
            "girsa-link/src/bin/girsa-link-types.rs",
            &["inbound.jsonl", "touching.bits"],
        ),
        (
            "girsa-app/src/bin/girsa-companions.rs",
            &["companions.jsonl"],
        ),
        (
            "girsa-link/src/bin/girsa-link-orient.rs",
            // Both, because the second is written on **every** run and the
            // first only with `--replace`. A dry run that reports *0 flipped*
            // still leaves 633 MB behind, and the usage said only that it
            // "writes a new store beside the old one".
            &["links.superseded", ".oriented"],
        ),
    ];
    let crates = repo().join("crates");
    let mut silent = Vec::new();
    for (source, artefacts) in WRITES {
        let path = crates.join(source);
        let body = read(&path);
        let usage = usage_line(&body).unwrap_or_else(|| {
            panic!("{source} has no `usage:` block, so nothing here can be checked")
        });
        for artefact in artefacts {
            if !usage.contains(artefact) {
                silent.push(format!("{source}: --help never mentions {artefact}"));
            }
        }
    }
    assert!(
        silent.is_empty(),
        "{} tool(s) write into the corpus without saying what:\n{}\n\n\
         A reader who reads --help before running a tool is told what it does to \
         their disk, or the page that told them to read --help is lying.",
        silent.len(),
        silent.join("\n"),
    );
}

/// The `USAGE` constant of a binary, whole.
///
/// From `const USAGE: &str = "` to the closing quote, so a multi-line usage is
/// one string. Reading the source rather than running the binary: this test
/// suite is compiled by the same `cargo test` that would have to build seven
/// more binaries first, and what is being asserted is what the source says.
fn usage_line(body: &str) -> Option<String> {
    let after = body.split("const USAGE: &str = \"").nth(1)?;
    let mut out = String::new();
    let mut chars = after.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                chars.next();
            }
            '"' => return Some(out),
            other => out.push(other),
        }
    }
    None
}
