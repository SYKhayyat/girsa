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
                if base == "target" || base == "node_modules" || base == ".git" {
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

/// Every shared-crate dependency line, as (manifest, crate, the rest).
fn shared_lines(root: &Path) -> Vec<(PathBuf, String, String)> {
    let mut out = Vec::new();
    for manifest in files(root, "Cargo.toml") {
        for line in uncommented(&manifest).lines() {
            let line = line.trim();
            let Some((name, rest)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if SHARED.contains(&name) {
                out.push((manifest.clone(), name.to_string(), rest.trim().to_string()));
            }
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
    let names: BTreeSet<&str> = found.iter().map(|(_, n, _)| n.as_str()).collect();
    assert_eq!(
        names.len(),
        SHARED.len(),
        "expected all six shared crates in the manifests, found {names:?}.\n\
         If one was added, removed, or rewritten as a `[dependencies.girsa-…]`\n\
         table (which the scan in this file cannot see), update `SHARED`\n\
         deliberately — silence here is what let the last one through.",
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
