//! Numbers the documentation states about this repository are numbers this
//! repository has.
//!
//! # Why
//!
//! The README is 3,029 lines, the highest-churn file in the tree, and
//! hand-maintained. It said *"a window and fifty commands"* while
//! `grep -c '#[tauri::command]'` returned **100** — a claim wrong by 50, in the
//! sentence that carries the shell's whole thesis, sitting there for weeks. The
//! same file says the shortcut card is *"generated from the source, so it cannot
//! drift"*, which is true, and which nothing checked until
//! `tools/check-card.sh`.
//!
//! Its sibling test — `the_documentation_names_things_that_exist` — closes the
//! other half of this and says in its own note that it cannot close this half:
//! no reference-checker would ever have found *fifty*.
//!
//! # How a number gets into this test
//!
//! By being marked in the README, right after it:
//!
//! ```text
//! a window and **100**<!--=commands--> commands
//! ```
//!
//! An HTML comment renders as nothing, so a reader sees the sentence and this
//! test sees the claim. Every marker below has to resolve, **and every marker in
//! the README has to be one this test knows** — a name nobody measures is a
//! number nobody checks wearing the costume of one.
//!
//! **A number spelled as a word cannot be marked**, and that is not a limitation
//! of the parser. `**Eleven**<!--=crates-->` fails this test on its first run,
//! by design, and it is worth knowing that the claim which started all of this
//! was *"a window and **fifty** commands"* — a word, unsearchable, and wrong by
//! 50 for weeks. Digits, or it is prose.
//!
//! # What does not belong here
//!
//! Numbers about the corpus. *4,171 se'ifim* and *5,000,545 segments* are facts
//! about a 3.4 GB download this machine may not have, and a check that quietly
//! passed because it could not find the corpus is the failure
//! `tools/check-ksav-fixture.sh:41` refuses by name. Those live in the
//! `#[ignore]`d reproductions, where they read as `10 ignored` rather than as
//! ten green ticks.
//!
//! Numbers about the *past*, either. *"27m48s against 2m44s"* is a measurement
//! taken once, on a cold CI cache, and it is history rather than a property of
//! this tree. Only what can be recounted here is marked.
//!
//! # Which pages
//!
//! `PAGES`, and it grew from one. `docs/tools.md` opened with **"sixteen
//! binaries and fifteen examples"** while there were seventeen examples: two
//! were added, each in a commit that edited *that page* to give the new one a
//! runnable line, and neither touched the sentence counting them. Wrong twice
//! in three weeks, in the page whose own second paragraph argues that a number
//! spelled as a word is unfalsifiable and that this repository marks them
//! instead. It had made the argument and not taken its own advice.
//!
//! The file and `tools/readme-numbers.sh` keep their names. The README is
//! still where all but two of the markers live, `docs/the-record.md` and
//! `docs/the-third-sitting.md` quote the old name as history, and a rename
//! that rewrites the record to match today is the one edit those pages must
//! never take.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The pages whose marked numbers are checked, and the list
/// `tools/readme-numbers.sh` writes back to. Both halves read this order.
///
/// **`spec.md` is here and it is the point of #34.** The README's counts were
/// honest because a script regenerates them and this test fails on drift, while
/// the highest-authority document in the repository — the one a work order is
/// checked against — carried counts inside its own tables that nothing counted.
/// The inverse of its weight: the more a document is trusted, the fewer people
/// recompute what it says.
///
/// What can move here is what has a *measurer*, and the two kinds are different
/// enough to be named. A count of this tree (`search-modes`) is recounted in
/// `measurements` like the README's. A count of a **corpus** cannot be recounted
/// by a checkout at all, so its measurer is the program that runs on a shelf —
/// and the guard is that the program reads the number out of this document
/// rather than remembering it (`MEASURED_ON_A_SHELF`).
const PAGES: &[&str] = &["README.md", "docs/tools.md", "spec.md"];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| panic!("the repository root resolves: {e}"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()))
}

fn lines_of(path: &Path) -> usize {
    read(path).lines().count()
}

/// Every `.rs` line under `dir`, at any depth.
///
/// **The directory, not one file**, which is the same correction `commands` took
/// above and for the same reason: #68 moved the shell's commands out of `lib.rs`,
/// and a measurement of that one file answered *how big is the biggest file* while
/// the README said *how big is the shell*. It read 591 against a shell of 6,9xx.
fn lines_in(dir: &Path) -> usize {
    let mut total = 0;
    let Ok(entries) = std::fs::read_dir(dir) else {
        panic!("{}: not a directory", dir.display());
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            total += lines_in(&path);
        } else if path.extension().is_some_and(|x| x == "rs") {
            total += lines_of(&path);
        }
    }
    total
}

/// Files matching `crates/*/<tail>`, where `<tail>` may end in `*.rs`.
fn under_crates(root: &Path, dir: &str, extension: &str) -> usize {
    let mut found = 0;
    let crates = std::fs::read_dir(root.join("crates")).unwrap_or_else(|e| panic!("crates/: {e}"));
    for krate in crates.filter_map(Result::ok) {
        let Ok(entries) = std::fs::read_dir(krate.path().join(dir)) else {
            continue;
        };
        found += entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == extension))
            .count();
    }
    found
}

fn files_in(dir: &Path, extension: &str) -> usize {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == extension))
        .count()
}

/// Every number this test knows how to recount, by the name it is marked with.
fn measurements(root: &Path) -> BTreeMap<&'static str, usize> {
    let shell = root.join("app/src-tauri/src");
    let mut out = BTreeMap::new();
    // **The prefix, not the whole attribute.** This matched `#[tauri::command]`
    // exactly, and the day every command in the shell became
    // `#[tauri::command(async)]` — so that one blocked call could not hold the
    // window still (finding 22) — the count fell from 132 to the 3 that stayed
    // blocking. Which is this test working: it said the README was wrong by
    // 129 rather than letting the number rot. But the README was right and the
    // *measurement* had gone stale, which is the one failure mode a test that
    // recounts a claim has, and the fix belongs here.
    // …and **as an attribute**, which means at the head of a line. Counting
    // occurrences anywhere in the file counts the two in the module's own
    // header, which is a doc comment about the attribute rather than a use of
    // it — the shell would gain two commands by explaining itself.
    // …and **over the directory**, not one file: see [`commands_in`] (#72).
    out.insert("commands", commands_in(&root.join("app/src-tauri/src")));
    out.insert("shell-lines", lines_in(&shell));
    out.insert(
        "crates",
        std::fs::read_dir(root.join("crates"))
            .unwrap_or_else(|e| panic!("crates/: {e}"))
            .filter_map(Result::ok)
            .filter(|e| e.path().join("Cargo.toml").is_file())
            .count(),
    );
    out.insert("bins", under_crates(root, "src/bin", "rs"));
    // The other half of what `docs/tools.md` opens by counting. Same shape as
    // `bins` one directory over, and the reason it is here is that the word
    // *fifteen* on that page had been wrong since the fifteenth example was
    // joined by a sixteenth.
    out.insert("examples", under_crates(root, "examples", "rs"));
    // The checks that read this repository's own source. Counted rather than
    // typed, because a file whose whole subject is *the rule was written down
    // and nothing enforced it* should not carry a number nothing enforces.
    out.insert(
        "rules",
        read(&root.join("crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs"))
            .matches("#[test]")
            .count(),
    );
    out.insert("window-modules", files_in(&root.join("app/src"), "ts"));
    out.insert("styles-lines", lines_of(&root.join("app/src/styles.css")));
    // spec.md §9.3's own heading, and the M3 milestone that repeats it. The
    // five are `girsa_search::Mode`'s variants, so a sixth mode added to the
    // enum is a spec that is now wrong — which is the number this test exists
    // to hold, and the reason it counts the enum rather than the prose.
    out.insert(
        "search-modes",
        variants_in(&root.join("crates/girsa-search/src/lib.rs"), "Mode"),
    );
    out
}

/// The variants of one `pub enum`, counted from the source.
///
/// Counted rather than reached for as `Mode::SPELLINGS.len()`, because
/// `girsa-app` deliberately does not depend on `girsa-search` — the workspace
/// is the thing that shows a search, not the thing that defines one — and a
/// dev-dependency added to make a documentation test compile would be the
/// wrong way round. A source count is what every other measurement in this file
/// already is.
///
/// The variant lines are the ones that begin with an uppercase letter inside the
/// braces; attributes, doc comments and the closing brace are stepped over. A
/// variant carrying a doc line or a `#[default]` therefore does not change the
/// count, and neither does one that carries fields — what ends the list is the
/// closing brace, not the first line that fails to end in a comma. The last
/// variant in an enum may legally have no comma, so a line that begins with a
/// capital and does not end in one is counted only when the next meaningful line
/// closes the enum; anything else is a variant with fields written across lines,
/// which this refuses to guess at rather than guessing wrong.
fn variants_in(path: &Path, name: &str) -> usize {
    let body = read(path);
    let open = format!("pub enum {name} {{");
    let Some(start) = body.find(&open) else {
        panic!("{} declares `{open}`", path.display());
    };
    // Everything between the enum's head and the brace that closes it.
    let inside: Vec<&str> = body[start + open.len()..]
        .lines()
        .map(str::trim)
        .take_while(|line| !line.starts_with('}'))
        .collect();
    let mut total = 0;
    for (i, line) in inside.iter().enumerate() {
        let first = line.chars().next().unwrap_or(' ');
        if !first.is_ascii_uppercase() {
            continue;
        }
        // A comma ends a variant; the last one in an enum may legally have none.
        if line.ends_with(',') || inside.get(i + 1).is_some_and(|next| next.starts_with('}')) {
            total += 1;
        }
    }
    total
}

/// Every `#[tauri::command]` under the shell's source directory, recursively.
///
/// This counted `lib.rs` alone, and so did `tools/readme-numbers.sh` — two lists
/// with the same blind spot, which is the one arrangement that cannot catch
/// itself. #68 then moved the first command into `commands/nav.rs`, the tree
/// gained a command, and both counters said 138 while the README said 138 too:
/// nothing to fail. Worse, the script the documentation tells you to run
/// **wrote the wrong number back**, so the value could not be corrected by the
/// tool that exists to correct it (#72).
///
/// A directory because the shell is a directory now. The attribute is matched at
/// the head of a line, for the reason in [`measurements`].
fn commands_in(dir: &Path) -> usize {
    let mut total = 0;
    let Ok(entries) = std::fs::read_dir(dir) else {
        panic!("the shell's source directory reads: {}", dir.display());
    };
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("an entry of {} reads: {e}", dir.display()))
            .path();
        if path.is_dir() {
            total += commands_in(&path);
        } else if path.extension().is_some_and(|x| x == "rs") {
            total += read(&path)
                .lines()
                .filter(|line| line.trim_start().starts_with("#[tauri::command"))
                .count();
        }
    }
    total
}

/// `**4,600**<!--=shell-lines-->` → `("shell-lines", 4600)`.
///
/// The number is whatever digits and commas run backwards from the marker,
/// through any markdown emphasis between them.
fn claims(text: &str) -> Vec<(String, usize, String)> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(open) = text[at..].find("<!--=") {
        let open = at + open;
        let Some(close) = text[open..].find("-->") else {
            break;
        };
        let name = text[open + 5..open + close].trim().to_string();
        at = open + close + 3;

        let before = &text[..open];
        let mut digits = String::new();
        for c in before.chars().rev() {
            match c {
                '0'..='9' | ',' => digits.insert(0, c),
                '*' | '`' | '_' if digits.is_empty() => {}
                _ => break,
            }
        }
        let cleaned: String = digits.chars().filter(char::is_ascii_digit).collect();
        let line = before
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .trim()
            .chars()
            .take(90)
            .collect();
        match cleaned.parse::<usize>() {
            Ok(n) => found.push((name, n, line)),
            Err(_) => found.push((name, usize::MAX, line)),
        }
    }
    found
}

/// Claims whose measurement needs `corpus/`, which is generated and gitignored.
///
/// The value is the program that measures them. `docs/tools.md` may state the
/// number; this test cannot recompute it from a checkout, so it holds the two
/// things a checkout *can* prove — that the program exists, and that
/// `tools/verify.mjs` runs it. A marker outside this list is still refused
/// outright: "nothing measures it" stays an error, it just stops being the only
/// thing a corpus-bound measurement can say.
const MEASURED_ELSEWHERE: &[(&str, &str)] = &[
    ("resolver-exact", "tools/resolver.mjs"),
    ("resolver-ambiguous", "tools/resolver.mjs"),
    ("resolver-unresolved", "tools/resolver.mjs"),
    ("resolver-citations", "tools/resolver.mjs"),
];

/// Claims about a **corpus**, and the program that measures them.
///
/// A checkout cannot recount these — `corpus/` is generated and gitignored, and
/// this project's rule is that a check silently passing because it could not
/// find its subject is worse than no check (`tools/check-ksav-fixture.sh:41`).
/// So the number stays in `spec.md`, a shelf build measures it, and what this
/// test holds is the one thing that can be held without a corpus: **that the
/// measuring program reads the number out of the document instead of carrying
/// its own copy.**
///
/// That is the whole of #34. `girsa-import` compared the import against `7576`
/// and `978` typed into its own source, so §2's table, the importer and
/// `docs/record/links.md`'s measurement were three copies of one fact with
/// nothing keeping any two of them in step — and the record's copy already said
/// something else. Now there is one copy and one reader.
const MEASURED_ON_A_SHELF: &[(&str, &str)] = &[
    ("union-works", "crates/girsa-corpus/src/bin/girsa-import.rs"),
    (
        "otzaria-only",
        "crates/girsa-corpus/src/bin/girsa-import.rs",
    ),
];

#[test]
fn every_marked_number_in_the_documentation_is_what_the_tree_measures() {
    let root = repo();
    let known = measurements(&root);

    let mut wrong = Vec::new();
    let mut seen = 0;
    for page in PAGES {
        let marked = claims(&read(&root.join(page)));
        // Per page, not over the whole set: a page that loses its markers goes
        // quiet on its own, and the two that carry them are checked for that
        // separately rather than covering for each other.
        assert!(
            !marked.is_empty(),
            "no marked numbers in {page} at all — the markers were removed and this test \
             went quiet about that page, which is the failure it exists to prevent"
        );
        seen += marked.len();
        for (name, claimed, line) in &marked {
            // A claim measured by a program that needs the corpus is still a
            // claim with a measurer — but that program cannot run here, so what
            // this test can hold is that the program exists and that the gate
            // runs it. Anything else marked is a number pretending to be
            // measured (#73).
            if let Some((_, tool)) = MEASURED_ELSEWHERE
                .iter()
                .find(|(mark, _)| *mark == name.as_str())
            {
                assert!(
                    root.join(tool).is_file(),
                    "`{name}` is measured by {tool}, and there is no {tool}"
                );
                assert!(
                    read(&root.join("tools/verify.mjs")).contains(tool),
                    "`{name}` is measured by {tool}, and the gate does not run it — a \
                     measurement nothing runs is the claim this test refuses"
                );
                continue;
            }
            // A corpus claim: the reader is what is checked, not the number.
            if let Some((_, tool)) = MEASURED_ON_A_SHELF
                .iter()
                .find(|(mark, _)| *mark == name.as_str())
            {
                let reader = root.join(tool);
                assert!(
                    reader.is_file(),
                    "`{name}` is measured by {tool}, and there is no {tool}"
                );
                assert!(
                    read(&reader).contains(&format!("spec::claim(\"{name}\")")),
                    "`{name}` is measured by {tool}, and that program does not read it out of \
                     spec.md — a copy of a number the document also states is the third copy \
                     nobody keeps in step (#34)"
                );
                continue;
            }
            let Some(measured) = known.get(name.as_str()) else {
                wrong.push(format!(
                    "`{name}` is marked in {page} and nothing measures it — add it to \
                     `measurements`, or take the marker off\n      at: {line}"
                ));
                continue;
            };
            if claimed != measured {
                wrong.push(format!(
                    "`{name}`: {page} says {claimed}, the tree has {measured}\n      at: {line}"
                ));
            }
        }
    }
    assert!(seen > 0, "no marked numbers anywhere");
    assert!(
        wrong.is_empty(),
        "the documentation states numbers this repository does not have:\n  - {}\n\nEvery one \
         of these is one `tools/readme-numbers.sh` away. The point is that nothing used to \
         say so.",
        wrong.join("\n  - ")
    );
}

#[test]
fn every_measurement_is_claimed_somewhere() {
    // The other direction, and it is the one that rots quietly: a measurement
    // nobody cites is a check that runs, passes, and guards nothing. Either a
    // page says it or this test should not know how to count it.
    let root = repo();
    let marked: Vec<String> = PAGES
        .iter()
        .flat_map(|page| claims(&read(&root.join(page))))
        .map(|(name, _, _)| name)
        .collect();
    let unclaimed: Vec<&str> = measurements(&root)
        .keys()
        .filter(|name| !marked.iter().any(|m| m == *name))
        .copied()
        .collect();
    assert!(
        unclaimed.is_empty(),
        "measured and never cited: {unclaimed:?} — either mark them in one of {PAGES:?} or \
         stop counting them"
    );
}

/// The rule, not the number: a command declared outside `lib.rs` is counted.
///
/// Written against a temporary tree rather than against the repository, because
/// the repository only proves the number for the arrangement it happens to be in
/// — and it was in the other arrangement when this went wrong.
#[test]
fn a_command_declared_outside_lib_rs_is_counted() {
    let dir = std::env::temp_dir().join(format!("girsa-commands-{}", std::process::id()));
    let commands = dir.join("commands");
    std::fs::create_dir_all(&commands).unwrap_or_else(|e| panic!("temp tree: {e}"));
    std::fs::write(
        dir.join("lib.rs"),
        "//! The window.\n#[tauri::command]\nfn one() {}\n#[tauri::command(async)]\nfn two() {}\n",
    )
    .unwrap_or_else(|e| panic!("lib.rs: {e}"));
    std::fs::write(
        commands.join("nav.rs"),
        "//! A domain.\n#[tauri::command]\npub(crate) fn three() {}\n",
    )
    .unwrap_or_else(|e| panic!("commands/nav.rs: {e}"));

    assert_eq!(commands_in(&dir), 3, "2 in lib.rs and 1 in commands/");
    std::fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("temp tree: {e}"));
}

/// The rule, not the number: a variant added to the enum moves the count.
///
/// `spec.md §9.3` says how many modes there are, and the modes are an enum.
/// Written against a temporary file for the reason the command counter is: this
/// repository proves today's arrangement, and the failure this guards is the
/// arrangement changing — a variant carrying an attribute, a doc line between
/// two listed variants, a `where`-clause-free body. A counter that stopped at
/// the first `#[` would say five for ever.
#[test]
fn a_variant_added_to_an_enum_is_counted() {
    let dir = std::env::temp_dir().join(format!("girsa-enums-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("temp tree: {e}"));
    let file = dir.join("lib.rs");
    std::fs::write(
        &file,
        "/// The modes.\n\
         #[derive(Debug)]\n\
         pub enum Mode {\n\
         \x20   /// Literal.\n\
         \x20   #[default]\n\
         \x20   ToratEmet,\n\
         \x20   Smart,\n\
         \x20   Regex { bare: bool },\n\
         }\n\n\
         pub enum Other {\n\
         \x20   One,\n\
         }\n",
    )
    .unwrap_or_else(|e| panic!("lib.rs: {e}"));

    assert_eq!(
        variants_in(&file, "Mode"),
        3,
        "attributes and docs stepped over"
    );
    assert_eq!(
        variants_in(&file, "Other"),
        1,
        "and the next enum is not swallowed"
    );
    std::fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("temp tree: {e}"));
}
