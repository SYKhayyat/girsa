//! The numbers `spec.md` states about a corpus, read **out of `spec.md`**.
//!
//! # Why this exists
//!
//! `girsa-import` has always ended by comparing what it imported against the
//! counts §2.3 states — *works in the union*, *works Sefaria does not supply* —
//! and it compared them against those numbers **typed into its own source**.
//! Which makes three copies of one fact: the table in `spec.md`, the literal in
//! the importer, and whatever `docs/record/links.md` measured when it was
//! written. #34 is that finding: the README's counts are regenerated and fail
//! the gate when they drift, and the highest-authority document's counts were
//! the least guarded in the tree.
//!
//! So the number lives in `spec.md`, marked the way the README's are —
//!
//! ```text
//! | 5,637 | **978**<!--=otzaria-only--> | 961 | ~7,576<!--=union-works--> |
//! ```
//!
//! — and the program that measures it reads it back rather than remembering it.
//! An HTML comment renders as nothing, so a reader sees the sentence and the
//! importer sees the claim.
//!
//! # Embedded, not read at runtime
//!
//! `include_str!`, so the document is compiled into the importer and a shelf
//! built on a machine with no checkout — the shipped case — still knows what it
//! is comparing against. It also means editing a marked number rebuilds the
//! importer, which is the right shape: the check and the claim move together or
//! not at all.
//!
//! # What it is not for
//!
//! Numbers a test can recount from the tree are **not** here. `commands`,
//! `crates`, `bins` and the rest stay in
//! `the_numbers_in_the_readme_are_measurements`, which counts them and holds
//! the documents against the count. This module is for the ones that need a
//! corpus, which a checkout cannot provide and a shelf build can.

/// `spec.md`, compiled in. See the module note on why it is not read at runtime.
const SPEC: &str = include_str!("../../../spec.md");

/// One number `spec.md` marks, by the name it is marked with.
///
/// `None` when the document marks no such name — which is a claim that went
/// missing, and the callers treat it as loudly as they treat a failed check.
#[must_use]
pub fn claim(name: &str) -> Option<usize> {
    let marker = format!("<!--={name}-->");
    let at = SPEC.find(&marker)?;
    read_back(&SPEC[..at])
}

/// The digits immediately before a marker, through any markdown emphasis.
///
/// The rule `the_numbers_in_the_readme_are_measurements` uses, and it is
/// deliberately the same one: the number is whatever digits and commas run
/// backwards from the marker, with `*`, `` ` `` and `_` allowed between them.
/// So `**978**<!--=x-->` and `~7,576<!--=y-->` both read, and `**fifty**` reads
/// as nothing — which is the point of marking digits rather than words.
fn read_back(before: &str) -> Option<usize> {
    let mut digits = String::new();
    for c in before.chars().rev() {
        match c {
            '0'..='9' | ',' => digits.insert(0, c),
            '*' | '`' | '_' if digits.is_empty() => {}
            _ => break,
        }
    }
    digits
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marks_spec_carries_are_readable() {
        // Both of the numbers `girsa-import` checks. If one of these goes
        // missing, the importer would compare against nothing — so it is this
        // test that says so, once, rather than a shelf build at the end of an
        // hour saying it by panicking.
        for name in ["union-works", "otzaria-only"] {
            assert!(
                claim(name).is_some(),
                "spec.md marks `{name}`, and it no longer does"
            );
        }
    }

    #[test]
    fn a_number_is_read_through_emphasis_and_a_tilde() {
        assert_eq!(read_back("| **978**"), Some(978));
        assert_eq!(read_back("| ~7,576"), Some(7576));
        assert_eq!(read_back("| `1,024`"), Some(1024));
    }

    #[test]
    fn a_word_is_not_a_number() {
        // The claim that started the whole marking discipline in this
        // repository: *"a window and fifty commands"*, unsearchable, wrong by
        // fifty for weeks.
        assert_eq!(read_back("a window and fifty "), None);
    }

    #[test]
    fn a_mark_that_is_not_there_is_none_rather_than_zero() {
        assert_eq!(claim("nothing-marks-this"), None);
    }
}
