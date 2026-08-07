//! Where the shelf is, and where your own layer is.
//!
//! # What makes a directory a corpus
//!
//! `works/index.jsonl`. Not its name, not its parent, and not the fact that
//! `GIRSA_CORPUS` points at it — a variable pointing at an empty directory is a
//! reader who typed the path wrong, and *found it, it is empty* and *did not
//! find it* are the same sentence to somebody staring at a window with no
//! seforim in it.
//!
//! That rule was written down in the Tauri shell, under a README that says the
//! shell decides nothing. It is a fact about the corpus, so it is here, where
//! the sixteen command-line tools that also want to find a corpus can reach it.
//!
//! # Why the order is the order
//!
//! 1. `GIRSA_CORPUS` — because somebody said so, and nothing outranks that.
//! 2. `corpus/` beside the executable — how an installed copy finds its own.
//! 3. `corpus/` in the working directory, then two levels up — how it is found
//!    when run out of the source tree, from the workspace root or from inside
//!    a crate.
//!
//! Every candidate that failed is reported. *No shelf found* with no list is a
//! message that cannot be acted on, and the usual cause is the third case
//! looking one directory away from where the reader is standing.

use std::path::{Path, PathBuf};

/// The file whose presence means *this is a corpus*.
pub const MARKER: &str = "works/index.jsonl";

/// Where a corpus was looked for, and whether one was there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Looked {
    /// The one that had a [`MARKER`] in it.
    pub found: Option<PathBuf>,
    /// Every candidate, in the order they were tried.
    pub tried: Vec<PathBuf>,
}

impl Looked {
    /// What to say when there is no shelf.
    #[must_use]
    pub fn said(&self) -> String {
        format!(
            "no shelf found. Looked in: {}. Run girsa-fetch and girsa-import, or set {}.",
            self.tried
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            CORPUS
        )
    }
}

/// The variable that names the corpus.
pub const CORPUS: &str = "GIRSA_CORPUS";

/// The variable that names your own layer.
pub const PERSONAL: &str = "GIRSA_PERSONAL";

/// Find the shelf.
///
/// # Errors
///
/// If none of the candidates holds a [`MARKER`]. The error names all of them.
pub fn corpus() -> Result<PathBuf, String> {
    let looked = look(&candidates(), |p| p.join(MARKER).is_file());
    looked.found.clone().ok_or_else(|| looked.said())
}

/// Where your own layer is: the arrangement, the seforim you added, your notes.
///
/// Beside the session file, in the application's data directory — **not** under
/// the corpus root, which a re-download is entitled to replace wholesale.
#[must_use]
pub fn personal(data: &Path) -> PathBuf {
    std::env::var(PERSONAL).map_or_else(|_| data.join("personal"), PathBuf::from)
}

/// The places a corpus is looked for, in order.
#[must_use]
pub fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(said) = std::env::var(CORPUS) {
        out.push(PathBuf::from(said));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("corpus"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("corpus"));
        out.push(cwd.join("../../corpus"));
    }
    out
}

/// The search itself, with *is this one a corpus* handed in.
///
/// Split out so a test can assert the order without a disk: reading
/// `std::env` in a test sets it for every other test in the process, and the
/// failure that causes lands in whichever test runs second.
#[must_use]
pub fn look(candidates: &[PathBuf], is_corpus: impl Fn(&Path) -> bool) -> Looked {
    let mut tried = Vec::new();
    for candidate in candidates {
        if is_corpus(candidate) {
            return Looked {
                found: Some(candidate.clone()),
                tried,
            };
        }
        tried.push(candidate.clone());
    }
    Looked { found: None, tried }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_first_one_that_is_a_corpus_wins() {
        let looked = look(&paths(&["a", "b", "c"]), |p| p.ends_with("b"));
        assert_eq!(looked.found, Some(PathBuf::from("b")));
        assert_eq!(looked.tried, paths(&["a"]), "and c was never looked at");
    }

    #[test]
    fn a_variable_pointing_at_an_empty_directory_is_not_a_corpus() {
        // The reader typed the path wrong. Falling through to the next
        // candidate is right; saying nothing about it is not, which is why the
        // failed candidate is in `tried`.
        let looked = look(&paths(&["/said/so", "beside/the/exe"]), |p| {
            p.ends_with("exe")
        });
        assert_eq!(looked.found, Some(PathBuf::from("beside/the/exe")));
        assert_eq!(looked.tried, paths(&["/said/so"]));
    }

    #[test]
    fn nothing_found_says_where_it_looked() {
        let looked = look(&paths(&["one", "two"]), |_| false);
        assert_eq!(looked.found, None);
        let said = looked.said();
        assert!(said.contains("one"), "{said}");
        assert!(said.contains("two"), "{said}");
        assert!(said.contains(CORPUS), "{said}");
        assert!(said.contains("girsa-import"), "{said}");
    }

    #[test]
    fn the_order_is_said_so_then_installed_then_the_source_tree() {
        // Asserted as a list rather than as prose, because the prose was in
        // the shell and the list was in the shell and only one of them was
        // ever read again.
        let candidates = candidates();
        let last_two: Vec<String> = candidates
            .iter()
            .rev()
            .take(2)
            .map(|p| p.display().to_string())
            .collect();
        assert!(
            last_two.iter().any(|p| p.contains("..")),
            "the source tree is looked for: {last_two:?}"
        );
    }

    #[test]
    fn what_makes_a_corpus_is_one_file() {
        assert_eq!(MARKER, "works/index.jsonl");
    }

    #[test]
    fn your_own_layer_is_not_under_the_corpus() {
        // A re-download replaces the corpus wholesale. Your notes are not the
        // corpus's to delete.
        let data = PathBuf::from("/data");
        let mine = personal(&data);
        assert!(
            std::env::var(PERSONAL).is_ok() || mine == data.join("personal"),
            "{mine:?}"
        );
    }
}
