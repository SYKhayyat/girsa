//! The manifests, checked the way the code is.
//!
//! Two facts about this repository were true, load-bearing, and asserted by
//! nothing.
//!
//! 1. **A clone of it builds.** It did not. The shared crates were
//!    `path = "../sefer-crates/crates/…"` — a sibling of *this checkout's root*
//!    — so `git clone girsa && cargo build` failed inside `cargo metadata`,
//!    before a compiler ran, with `os error 3` naming a directory the reader had
//!    never heard of. There is no submodule, no `[patch]`, no vendor directory.
//!    Every CI job carried a second `actions/checkout` purely to fake the desk
//!    layout, which is what a load-bearing workaround looks like, and the only
//!    note on the subject anywhere said *"cloning Girsa alone will not build"*
//!    as though that were a property of the world rather than of one manifest.
//!
//! 2. **One product compiles one sefer-crates.** With path dependencies that was
//!    free — one directory, one copy, nothing to keep in step. Pinning by commit
//!    buys the clone, and the bill is that the SHA is written out where it can
//!    drift. The desktop binary links the window and the Tauri shell together,
//!    so two revs would put two `girsa-post`s in one process: the loopback and
//!    the deep-link parser disagreeing about the wire between them, which is the
//!    failure the shared crate exists to prevent, arriving through the fix for
//!    something else.
//!
//! Ksav made this move first and wrote `engine/tests/manifests.rs` for it. This
//! is that fence, over this tree. It is a *port* and not a shared crate on
//! purpose: what it asserts is a fact about a directory layout, and there is no
//! directory layout the two repositories share.
//!
//! # What this reads, and what it does not
//!
//! Line scanning with comments stripped, not a TOML parse. The manifests discuss
//! their own subject matter at length, and prose *about* a path dependency must
//! not be read as one; a `toml` dependency added to assert six lines would be a
//! worse trade than the parse is worth. The cost is that a dependency written as
//! a multi-line `[dependencies.girsa-post]` table would be invisible here —
//! which is why the shared crates are also **counted**, so one written in a
//! shape this file cannot read fails the count rather than passing silently.

// A panic in a test is a failure report. The workspace denies these in
// library code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

const SEFER_CRATES: &str = "https://github.com/SYKhayyat/sefer-crates";

/// The six crates that come from the other repository.
const SHARED: [&str; 6] = [
    "girsa-cite",
    "girsa-hebrew",
    "girsa-ksav",
    "girsa-post",
    "girsa-ref",
    "girsa-source",
];

/// The repository root: this crate is `crates/girsa-app`, so two levels up.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| panic!("the repository root resolves: {e}"))
}

/// Resolve `.` and `..` textually.
///
/// Not `canonicalize`: a path pointing outside the repository may or may not
/// exist on the machine running the test, and *"it does not exist here"* is a
/// different failure from *"it points outside"* — which is the one worth naming.
fn normalise(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Every file called `name` in the tree, minus build output and other
/// ecosystems' dependencies.
/// Directories these walks do not descend into, and the reason is not the same
/// for both halves of the list.
///
/// `target`, `node_modules` and `.git` are build output and were always here.
/// The other four are the reader's data — the corpus, the derived indices, the
/// personal layer — every one of them gitignored, none of them able to hold a
/// `Cargo.toml` or a `.rs` file belonging to this workspace.
///
/// They were not skipped, and it does not show on CI, which has no corpus. It
/// shows on the machine of the one person who has one: the four tests in this
/// file that walk the tree each stat their way through **11 GB** first. On
/// 14 August the same gate, on the same tree, ran in **91s** with a warm file
/// cache and **1107s** an hour later after a release build had evicted it —
/// 12× for a walk over Torah text that no test in here reads. All four went
/// past the 60-second mark the test harness warns at; the whole file now runs
/// in **6.65s**, and the warm-or-cold question stops applying because the
/// corpus is no longer walked at all.
///
/// That is a gate a contributor is told to run before every commit, and
/// `docs/your-first-change.md` makes it step 0. A check nobody will sit through
/// is a check that stops being run, which is the argument `tools/verify.mjs`
/// already exists to make.
fn is_not_ours(base: &str) -> bool {
    matches!(
        base,
        "target" | "node_modules" | ".git" | "corpus" | "index" | "personal" | "data"
    )
}

fn files(root: &Path, name: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, name: &str, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let base = entry.file_name();
            let base = base.to_string_lossy();
            if path.is_dir() {
                if is_not_ours(&base) {
                    continue;
                }
                walk(&path, name, out);
            } else if base == name {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, name, &mut out);
    out.sort();
    out
}

/// Every `.rs` file in the tree. `files` matches a whole name; this matches an
/// extension, which is the only difference and not worth a parameter.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let base = entry.file_name();
            let base = base.to_string_lossy();
            if path.is_dir() {
                if is_not_ours(&base) {
                    continue;
                }
                walk(&path, out);
            } else if base.ends_with(".rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

/// A manifest with `#` comments removed.
fn uncommented(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()))
        .lines()
        .map(|line| match line.find('#') {
            // Inside a string a `#` is content, not a comment. Keeping the line
            // whole when the quotes are unbalanced is the only way to be sure
            // without a parser.
            Some(_) if line.matches('"').count() % 2 == 1 => line.to_string(),
            Some(i) => line[..i].to_string(),
            None => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The value of `key = "…"` on a dependency line.
fn field(line: &str, key: &str) -> Option<String> {
    let at = line.find(&format!("{key} = \""))?;
    let rest = &line[at + key.len() + 4..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Every shared-crate dependency line that carries its own source, as
/// (manifest, crate, the rest).
///
/// # What is skipped, and why it took a rewrite to notice
///
/// A member crate writes `girsa-ref.workspace = true`, which **inherits** the
/// root's pin and is the shape that makes one product compile one sefer-crates
/// in the first place. It carries no `git` and no `rev` of its own and must not.
///
/// The first version of this scan split on `=` and asked whether the left side
/// was a shared crate's name. `girsa-ref.workspace` is not `girsa-ref`, so every
/// dotted line fell through silently — and `app/src-tauri/Cargo.toml` writes the
/// same inheritance as `girsa-ref = { workspace = true }`, whose left side *is*.
/// So this fence went in **red**, asserting that a correctly inherited pin was
/// not a git dependency, and nobody ran it: the exact shape of the report it was
/// written to answer.
///
/// It now reads both spellings, skips inheritance in either, and asserts
/// separately that the root's table — the one place the pin can live — has an
/// entry for every shared crate.
fn shared_lines(root: &Path) -> Vec<(PathBuf, String, String)> {
    let mut out = Vec::new();
    for manifest in files(root, "Cargo.toml") {
        for line in uncommented(&manifest).lines() {
            let line = line.trim();
            let Some((name, rest)) = line.split_once('=') else {
                continue;
            };
            // `girsa-ref` and `girsa-ref.workspace` are the same dependency.
            let name = name.trim().split('.').next().unwrap_or_default().trim();
            if !SHARED.contains(&name) {
                continue;
            }
            let rest = rest.trim();
            // Inheritance, in either spelling. The pin it inherits is checked
            // where it lives.
            if rest == "true" || rest.contains("workspace = true") {
                continue;
            }
            out.push((manifest.clone(), name.to_string(), rest.to_string()));
        }
    }
    out
}

/// The root's `[workspace.dependencies]` entry for each shared crate.
fn workspace_table(root: &Path) -> Vec<(String, String)> {
    let body = uncommented(&root.join("Cargo.toml"));
    let mut out = Vec::new();
    let mut section = String::new();
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line.to_string();
            continue;
        }
        if section != "[workspace.dependencies]" {
            continue;
        }
        let Some((name, rest)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if SHARED.contains(&name) {
            out.push((name.to_string(), rest.trim().to_string()));
        }
    }
    out
}

#[test]
fn no_path_dependency_escapes_the_repository() {
    let root = repo();
    for manifest in files(&root, "Cargo.toml") {
        let dir = manifest.parent().expect("a manifest has a directory");
        for line in uncommented(&manifest).lines() {
            let line = line.trim();
            // `path = "src/lib.rs"` under `[lib]`/`[[bin]]` is a target, not a
            // dependency. Dependency lines are inline tables.
            if !line.contains("path = \"") || !line.contains('{') {
                continue;
            }
            let value = field(line, "path").expect("the path field parses");
            let resolved = normalise(&dir.join(&value));
            assert!(
                resolved.starts_with(&root),
                "{}: `path = \"{value}\"` resolves to {}, outside the repository.\n\
                 A clone would fail in `cargo metadata`, before a compiler runs, naming\n\
                 a directory the reader has never heard of — which is exactly how this\n\
                 repository could not build itself. Depend on it by git and rev instead;\n\
                 see the note above the girsa dependencies in Cargo.toml, and\n\
                 .cargo/config.toml for editing both halves at once.",
                manifest.strip_prefix(&root).unwrap_or(&manifest).display(),
                resolved.display(),
            );
        }
    }
}

#[test]
fn one_product_compiles_one_sefer_crates() {
    let root = repo();
    let found = shared_lines(&root);
    // Every shared crate has an entry in the root table, which is the one place
    // a pin can live now that every member inherits.
    let table = workspace_table(&root);
    let named: BTreeSet<&str> = table.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        named.len(),
        SHARED.len(),
        "expected all six shared crates in [workspace.dependencies], found {named:?}.\n\
         If one was added, removed, or rewritten as a `[workspace.dependencies.girsa-…]`\n\
         table (which the scan in this file cannot see), update `SHARED`\n\
         deliberately — silence here is what let the last one through.",
    );
    // And every source-carrying line anywhere in the tree is checked below,
    // including the root's own.
    assert!(
        found.len() >= SHARED.len(),
        "the root table is the only place these are pinned, and {} of {} lines \
         carry a source",
        found.len(),
        SHARED.len(),
    );

    let mut revs = BTreeSet::new();
    let mut versions = BTreeSet::new();
    for (manifest, name, rest) in &found {
        let shown = manifest.strip_prefix(&root).unwrap_or(manifest).display();
        let git = field(rest, "git").unwrap_or_else(|| {
            panic!("{shown}: {name} is not a git dependency — see the note in Cargo.toml")
        });
        assert_eq!(
            git, SEFER_CRATES,
            "{shown}: {name} points at {git}, not the shared repository",
        );
        let rev = field(rest, "rev")
            .unwrap_or_else(|| panic!("{shown}: {name} names no rev, so it is not pinned"));
        assert_eq!(
            rev.len(),
            40,
            "{shown}: {name} is pinned to `{rev}`, which is not a full commit SHA.\n\
             A branch or a short rev is not a pin.",
        );
        assert!(rev.chars().all(|c| c.is_ascii_hexdigit()));
        revs.insert(rev);

        // The exact-version requirement is kept beside the rev on purpose: a
        // commit whose manifests say a different version should be a resolution
        // error, not a surprise at the first behaviour difference.
        let version = field(rest, "version")
            .unwrap_or_else(|| panic!("{shown}: {name} has no version requirement"));
        assert!(
            version.starts_with('='),
            "{shown}: {name} requires `{version}`, which is a range, not a pin",
        );
        versions.insert(version);
    }
    assert_eq!(
        revs.len(),
        1,
        "the shared crates are pinned to {} different commits: {revs:?}",
        revs.len()
    );
    assert_eq!(versions.len(), 1, "one version, not {versions:?}");
}

#[test]
fn the_lock_files_record_the_pin() {
    let root = repo();
    let (_, _, rest) = shared_lines(&root)
        .into_iter()
        .find(|(_, n, _)| n == "girsa-source")
        .expect("the workspace declares girsa-source");
    let rev = field(&rest, "rev").expect("girsa-source is pinned by rev");

    let locks = files(&root, "Cargo.lock");
    assert!(!locks.is_empty(), "the lock files are committed");
    let mut seen = 0;
    for lock in locks {
        let text = std::fs::read_to_string(&lock).expect("a lock file is readable");
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("source = \"git+") || !line.contains("sefer-crates") {
                continue;
            }
            seen += 1;
            assert!(
                line.contains(&rev),
                "{}: a shared crate is locked to a commit the manifests do not name.\n\
                 Locked: {line}\n\
                 Pinned: {rev}\n\
                 Run `cargo update` after bumping the rev.",
                lock.strip_prefix(&root).unwrap_or(&lock).display(),
            );
        }
    }
    assert!(
        seen > 0,
        "no lock file records a sefer-crates git source — the pin is not reaching \
         resolution, so this test would pass by finding nothing"
    );
}

/// The workflow does not carry a checkout that the manifests no longer need.
///
/// This is the other half of the same change and it is worth asserting, because
/// a leftover `actions/checkout` of sefer-crates is not an error — it is a
/// slower CI that still works, which is exactly the kind of thing nobody
/// removes. It also carried `SEFER_CRATES_REF`, a second pin three files away
/// from the one in `Cargo.toml`, and two pins that must agree with nothing
/// between them is the shape this whole sweep is named after.
#[test]
fn ci_does_not_fake_a_desk_layout_any_more() {
    let root = repo();
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .unwrap_or_else(|e| panic!("ci.yml reads: {e}"));
    let code: String = ci
        .lines()
        .map(|l| l.split_once('#').map_or(l, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !code.contains("SEFER_CRATES_REF"),
        "ci.yml still pins sefer-crates itself. The rev is in Cargo.toml; a \
         second pin in a workflow is one more thing to keep in step."
    );
    assert!(
        !code.contains("repository: SYKhayyat/sefer-crates"),
        "ci.yml still checks out sefer-crates. Cargo fetches the pinned commit \
         itself now — the checkout existed only to fake a sibling directory."
    );
}

/// No step hands the shell a script that starts on one line and ends on another.
///
/// This is a guard for a bug that has already been paid for. The `nixos` step
/// passed its whole body to the container as `sh -euc '...'`, the body ran to
/// fifty lines, and one of those lines was a comment containing the word
/// `job's`. The apostrophe closed the string. Everything after it left the
/// container and ran on the host, which has no Nix, and the job reported
/// `nix: command not found` — after the fifteen-minute build it had already
/// done correctly.
///
/// What makes it worth a test rather than a fix is that nothing about the
/// failure points at the cause. The YAML is well formed, the indentation is
/// right, the line that fails is nowhere near the line that broke it, and the
/// build succeeds first so the log is thousands of lines long. `nixos-window.sh`
/// had written the lesson in its own header one level down — *a quoted script
/// inside a quoted script inside a YAML block is three levels of escaping and
/// one of them is always wrong* — and the workflow that called it did the thing
/// anyway.
///
/// So the construct is banned rather than the apostrophe. A line ending in an
/// opening quote is the only way a shell body spans lines here, and the
/// alternative is a file: `tools/nixos-ci.sh`, where an apostrophe is an
/// apostrophe. Ordinary prose apostrophes in YAML comments are untouched by
/// this, which is why the rule is about where the quote sits and not about how
/// many there are.
#[test]
fn no_workflow_step_opens_a_quote_and_finishes_it_on_another_line() {
    let root = repo();
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .unwrap_or_else(|e| panic!("ci.yml reads: {e}"));
    let hanging: Vec<String> = ci
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim_end();
            trimmed.ends_with('\'') && trimmed[..trimmed.len() - 1].ends_with(' ')
        })
        .map(|(nth, line)| format!("  line {}: {}", nth + 1, line.trim()))
        .collect();
    assert!(
        hanging.is_empty(),
        "ci.yml opens a single-quoted shell body and continues it on the next \
         line:\n{}\nOne apostrophe in one comment inside that body closes it \
         early, and what follows runs somewhere else entirely — which is how \
         the nixos job spent fifteen minutes building correctly and then said \
         `nix: command not found`. Put the script in a file under tools/ and \
         name the file.",
        hanging.join("\n")
    );
}

/// The leaf has nothing under it, which is what stops it becoming the basement.
///
/// The 9 August report's §5 finding: *"`girsa-corpus` has become the workspace
/// basement: 886 lines of `argv`/`said`/`roots`/`csv` live in the ingest crate
/// because the ingest crate is the one everything can `use`."*
///
/// `girsa-plain` is the answer, and the answer only works while it stays a leaf.
/// The moment it may `use` a girsa crate, it is a place things can be pushed
/// *down* into rather than a place with a subject — and the whole failure is
/// that a crate everything depends on collects whatever has nowhere else to go.
///
/// So the rule is mechanical: no dependency of `girsa-plain` may be named
/// `girsa-*`. A `thiserror` or a `serde` is somebody else's crate and says
/// nothing about this repository's shape.
#[test]
fn the_leaf_crate_stays_a_leaf() {
    let manifest = repo().join("crates/girsa-plain/Cargo.toml");
    let body = uncommented(&manifest);
    let mut section = String::new();
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line.to_string();
            continue;
        }
        if !section.contains("dependencies") {
            continue;
        }
        let Some((name, _)) = line.split_once('=') else {
            continue;
        };
        assert!(
            !name.trim().starts_with("girsa"),
            "girsa-plain depends on `{}`.\n\
             It is the crate with nothing under it, and that is the only thing keeping it\n\
             from becoming the next `girsa-corpus` — the one everything can `use`, so the\n\
             one everything with nowhere else to go ends up in. If something here needs a\n\
             girsa crate, it is not plain and belongs where its subject is.",
            name.trim(),
        );
    }
}

/// A `cargo run -p X` in a doc comment names the crate that has the target.
///
/// The 9 August report, §4, on what a mechanical crate split looks like from
/// outside:
///
/// > Thirteen documented commands in `girsa-desk` cannot run —
/// > `bin/girsa-notes.rs:6-20` and `examples/write.rs:4` all say
/// > `cargo run -p girsa-app`, and the targets moved crates without their doc
/// > comments.
///
/// Fifteen, counted. Every one of them was the first thing a reader would type,
/// and every one of them fails with *no bin target named `girsa-notes`* — while
/// the README, twenty lines of it, had the right crate all along. The usual
/// failure runs the other way; this is the code's own comments lying and the
/// documentation being right.
///
/// A path is not checked against a doc comment anywhere else, so this is the
/// sweep rather than the fix: it reads every `cargo run -p <crate> --bin <name>`
/// and `--example <name>` in every source file in the tree and asserts the
/// target is where the command says it is.
#[test]
fn every_documented_command_names_the_crate_that_has_it() {
    let root = repo();
    let mut checked = 0usize;
    for source in rust_files(&root) {
        // `src/` and `examples/` only. A test file is **the record**: the
        // suite next door exists to assert that `girsa-link-inbound` has never
        // been a binary in this tree, and it has to quote the command in order
        // to say so. A rule that forbade naming a defect would forbid recording
        // it — the same partition `app/test/prohibitions.test.mjs` draws around
        // `lamdan/` and `docs/`, for the same reason.
        let shown = source.to_string_lossy().replace('\\', "/");
        if shown.contains("/tests/") {
            continue;
        }
        let body = std::fs::read_to_string(&source).unwrap_or_default();
        for line in body.lines() {
            let Some(at) = line.find("cargo run -p ") else {
                continue;
            };
            let mut words = line[at + "cargo run -p ".len()..].split_whitespace();
            let Some(krate) = words.next() else { continue };
            // A placeholder in prose about this rule — including the paragraph
            // above — is not a command anybody can run.
            if krate.contains('<') || krate.contains('`') {
                continue;
            }
            let (kind, name) = match (words.next(), words.next()) {
                (Some("--bin"), Some(name)) => ("src/bin", name),
                (Some("--example"), Some(name)) => ("examples", name),
                // `cargo run -p x -- …` runs the crate's only binary; there is
                // no name to check.
                _ => continue,
            };
            if name.contains('<') || name.contains('`') {
                continue;
            }
            let target = root
                .join("crates")
                .join(krate)
                .join(kind)
                .join(format!("{name}.rs"));
            checked += 1;
            assert!(
                target.is_file(),
                "{}:\n  {}\n names `-p {krate}`, but there is no {kind}/{name}.rs in it.\n\
                 The target moved crates and the doc comment did not. Every reader who\n\
                 copies this line gets `no bin target named `{name}``.",
                source.strip_prefix(&root).unwrap_or(&source).display(),
                line.trim().trim_start_matches("//!").trim(),
            );
        }
    }
    assert!(
        checked > 20,
        "the scan found only {checked} documented commands, which is fewer than the tree has"
    );
}

/// The **product's** version is stated in three places, and they have to agree.
///
/// The shell crate's own `version`, `tauri.conf.json`, and `app/package.json`.
/// Each decides something a reader sees — the bundle's filename, what
/// Add/Remove Programs reports, what an update check would compare against —
/// and nothing has ever checked that they say the same thing.
///
/// **`[workspace.package]` is deliberately not one of them**, and finding that
/// out cost a broken build. Bumping it to cut a release moved every library
/// crate at once and left them requiring `=0.1.0` of each other, because those
/// pins are exact on purpose (see the scan above). Which is the answer: that
/// number versions a set of libraries pinned against one another, and these
/// three version the application somebody installs. Two questions that happen
/// to have had the same answer since the first commit.
///
/// That is the same shape as the README stating numbers nothing measured, which
/// this repository already guards. A release cut with `tauri.conf.json` at
/// 0.1.1 and the crate at 0.1.0 produces `Girsa_0.1.1_x64-setup.exe` containing
/// a binary that reports 0.1.0, and the only way anybody finds out is by
/// noticing.
#[test]
fn the_version_is_the_same_number_everywhere() {
    let root = repo();
    let read = |at: &str| {
        std::fs::read_to_string(root.join(at)).unwrap_or_else(|e| panic!("{at} reads: {e}"))
    };

    let said = |body: &str, key: &str| -> String {
        body.lines()
            .find_map(|line| {
                let line = line.trim();
                let rest = line.strip_prefix(key)?;
                // `=` in TOML, `:` in JSON — the same question asked of two
                // file formats, and not worth two readers.
                let rest = rest.trim_start();
                let rest = rest.strip_prefix('=').or_else(|| rest.strip_prefix(':'))?;
                Some(
                    rest.trim()
                        .trim_matches(|c| c == '"' || c == ',')
                        .to_string(),
                )
            })
            .unwrap_or_else(|| panic!("no {key} found"))
    };
    let found = [
        (
            "app/src-tauri/Cargo.toml",
            said(&read("app/src-tauri/Cargo.toml"), "version"),
        ),
        (
            "app/src-tauri/tauri.conf.json",
            said(&read("app/src-tauri/tauri.conf.json"), "\"version\""),
        ),
        (
            "app/package.json",
            said(&read("app/package.json"), "\"version\""),
        ),
    ];

    let first = &found[0].1;
    let disagree: Vec<String> = found
        .iter()
        .filter(|(_, v)| v != first)
        .map(|(where_, v)| format!("  {where_}: {v}"))
        .collect();
    assert!(
        disagree.is_empty(),
        "the product is {first} in app/src-tauri/Cargo.toml and something else elsewhere:\n{}\n\nA bundle \
         named for one version containing a binary that reports another is found by noticing, \
         which is not a way of being found.",
        disagree.join("\n"),
    );
}
