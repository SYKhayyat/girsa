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
const PAGES: &[&str] = &["README.md", "docs/tools.md"];

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
    let shell = root.join("app/src-tauri/src/lib.rs");
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
    out.insert(
        "commands",
        read(&shell)
            .lines()
            .filter(|line| line.trim_start().starts_with("#[tauri::command"))
            .count(),
    );
    out.insert("shell-lines", lines_of(&shell));
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
    out
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
