//! #12: the tree's one strict duplicate of Rust logic, pinned.
//!
//! `tools/degraded-ocr.mjs` measures what a photograph costs OCR, and the whole
//! measurement is *is this word the word that was on the page*. So it folds
//! nikud and final forms before comparing — the same normalizing
//! `girsa_hebrew::normalize` does. It says so in its own comment: *"the same
//! normalizing `girsa-hebrew` does, spelled here because this tool is node and
//! that crate is Rust."*
//!
//! It cannot be deleted. The tool is a node script with no Rust in it, and
//! giving a measuring harness its own cargo build would mean it cannot run on a
//! machine that only has node — which is a real cost for a harness whose job is
//! to be runnable. And a second implementation that nothing compares is the
//! thing this repository keeps finding a year later.
//!
//! So it is pinned: `--words` prints what the comparison is made of, and this
//! demands it equal [`girsa_hebrew::tokenize`] on text built to break the
//! agreement — nikud, a final form at the end of a word, a geresh inside a
//! title, a verse number, a Latin word, doubled punctuation. Every one of those
//! is a case where two normalizers could differ while both looking right.
//!
//! # What this does not cover
//!
//! `kept()` — the multiset score — is arithmetic over the word lists, not
//! normalizing, and it is measured against ground truth in the same run. The
//! pixel loop is browser work and stays where it is. What is here is the half
//! that decides *what is the same word*, which is the half that has to agree.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root resolves")
}

/// The tool's own word list for one passage.
fn the_tools(text: &str) -> Vec<String> {
    let root = repo();
    let mut child = Command::new("node")
        .arg("tools/degraded-ocr.mjs")
        .arg("--words")
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("node runs the harness");
    child
        .stdin
        .as_mut()
        .expect("a pipe to write the passage into")
        .write_all(text.as_bytes())
        .expect("the passage is written");
    let out = child.wait_with_output().expect("the harness answers");
    assert!(
        out.status.success(),
        "the harness refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(ToString::to_string)
        .collect()
}

/// The crate's.
fn the_crates(text: &str) -> Vec<String> {
    girsa_hebrew::tokenize(text)
        .into_iter()
        .map(|token| token.text)
        .collect()
}

/// Passages chosen for the places two normalizers part company, not for prose.
const PASSAGES: &[&str] = &[
    // Nikud, and a word ending in a final form that has to fold.
    "בְּרֵאשִׁית בָּרָא אֱלֹהִים",
    // The same word twice, one menukad and one not, so a difference shows as two
    // entries rather than as one.
    "קוֹרִין קורין",
    // Punctuation inside and between words, a geresh inside a title, and the
    // end-of-verse mark a real page carries.
    "שֶׁאֵין לוֹ פֵּרוּשׁ׃ וְאָמַר רַבִּי יוֹחָנָן, מִשְׁנָה תּוֹרָה (ויקרא א,א)",
    // A page's furniture: a verse number, a Latin word, a hyphen, an apostrophe.
    "151 Rashi 12a-13b דַּף י\"א",
    // Nothing Hebrew at all, and the empty string.
    "12.5 kg, page 3",
    "",
];

#[test]
fn the_harness_folds_words_the_way_the_crate_does() {
    let mut wrong = Vec::new();
    for passage in PASSAGES {
        let mine = the_tools(passage);
        let theirs = the_crates(passage);
        if mine != theirs {
            wrong.push(format!(
                "{passage:?}\n  tool: {mine:?}\n  crate: {theirs:?}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "the OCR harness and `girsa_hebrew` disagree about what the same word is:\n{}",
        wrong.join("\n")
    );
}

/// And the fold really is a fold, so the test above cannot pass by both sides
/// doing nothing.
#[test]
fn the_agreed_listing_has_the_nikud_off_and_the_final_forms_folded() {
    let words = the_crates("בְּרֵאשִׁית אֱלֹהִים");
    assert_eq!(
        words,
        ["בראשית", "אלהימ"],
        "nikud off, and a final mem written as a mem"
    );
    assert_eq!(the_tools("בְּרֵאשִׁית אֱלֹהִים"), words);
}
