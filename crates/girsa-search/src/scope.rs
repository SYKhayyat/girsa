//! Where a search looks — the `[whole shelf ▾]` chip (BUILDER.md W14).
//!
//! spec.md §9.5 puts three chips under the query bar and this is the middle
//! one. It is also what §9.8's facets *do*: a facet row is a count, and
//! clicking it narrows or excludes, which means writing one more line into a
//! scope and asking again.
//!
//! # It narrows by things the index knows
//!
//! A scope is sets of **work slugs** and sets of **link types**, because those
//! are the two columns every segment carries. The other three facets — shelf,
//! era, author — are properties of a *work*, so narrowing by one of them is
//! resolved to the seforim it means before it gets here (see
//! [`crate::facets`]). That keeps one rule in one place: the index answers
//! questions about segments, and the catalogue answers questions about seforim.
//!
//! # Narrow *and* exclude, because they are different questions
//!
//! §9.8 asks for both. *Only the Bavli* and *anything but the Bavli* are not
//! each other's opposite in a result list of fifteen shelves, and a reader
//! chasing a phrase usually wants the second — everything except the sefer they
//! already know says it.
//!
//! # What it is not
//!
//! It is not a widening and it cannot become one. Every clause here is a
//! `Must` or a `MustNot` over the same result set; nothing in this module can
//! add a hit the unscoped query did not have. That is worth stating because a
//! scope is the one control that changes the number in the header without
//! changing what was searched for.

use std::collections::BTreeSet;

use girsa_link::EdgeType;

/// Which seforim, and which kinds of link, a search is confined to.
///
/// The default is the whole shelf, which is what the chip says when nobody has
/// touched it.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// One set of seforim per click, and a hit has to be in **all** of them.
    ///
    /// A list rather than a set, because two clicks are an *and*: narrowing to
    /// `תלמוד` and then to `ראשונים` is the rishonim of Shas, not everything on
    /// either. One click is an *or* within itself — a shelf is the seforim on
    /// it — which is exactly a clause. Empty means every sefer.
    only: Vec<BTreeSet<String>>,
    /// Seforim ruled out.
    without: BTreeSet<String>,
    /// A hit must be touched by a link of one of these kinds.
    linked: BTreeSet<EdgeType>,
    /// A hit must be touched by none of these.
    unlinked: BTreeSet<EdgeType>,
    /// What the reader clicked to get here, for the chip to show.
    ///
    /// Display only. Two scopes that hold the same seforim are the same scope
    /// however they were arrived at, which is why this is not in the equality.
    named: Vec<String>,
}

/// Two scopes are the same when they let the same segments through.
///
/// [`Scope::named`] is deliberately not compared: it is what the reader
/// clicked, and `תלמוד/בבלי` and `the Bavli` naming one set of seforim are one
/// scope. Comparing it would let a caller conclude that a scope had changed
/// when nothing about the search had.
impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        self.only == other.only
            && self.without == other.without
            && self.linked == other.linked
            && self.unlinked == other.unlinked
    }
}

impl Eq for Scope {}

impl Scope {
    /// Everything on the shelf.
    #[must_use]
    pub fn everything() -> Self {
        Self::default()
    }

    /// Narrow to these seforim, under a name to show on the chip.
    ///
    /// Each call is one more clause, and every clause has to be satisfied. A
    /// second click that merged into the first would **widen** — the reader
    /// would narrow twice and get more — and it would do it silently, since the
    /// chip would read as though both had been applied.
    #[must_use]
    pub fn only(mut self, slugs: impl IntoIterator<Item = String>, named: &str) -> Self {
        self.only.push(slugs.into_iter().collect());
        self.name(named);
        self
    }

    /// Rule these seforim out.
    #[must_use]
    pub fn without(mut self, slugs: impl IntoIterator<Item = String>, named: &str) -> Self {
        self.without.extend(slugs);
        self.name(&format!("not {named}"));
        self
    }

    /// Only segments a link of this kind touches.
    #[must_use]
    pub fn linked(mut self, kind: EdgeType) -> Self {
        self.linked.insert(kind);
        self.name(kind.as_str());
        self
    }

    /// Only segments no link of this kind touches.
    #[must_use]
    pub fn unlinked(mut self, kind: EdgeType) -> Self {
        self.unlinked.insert(kind);
        self.name(&format!("not {}", kind.as_str()));
        self
    }

    fn name(&mut self, what: &str) {
        if !what.is_empty() && !self.named.iter().any(|n| n == what) {
            self.named.push(what.to_string());
        }
    }

    /// Whether this scope lets everything through.
    #[must_use]
    pub fn is_everything(&self) -> bool {
        self.only.is_empty()
            && self.without.is_empty()
            && self.linked.is_empty()
            && self.unlinked.is_empty()
    }

    /// The clauses, in the order they were clicked. A hit is in every one.
    #[must_use]
    pub fn clauses(&self) -> &[BTreeSet<String>] {
        &self.only
    }

    /// The seforim this scope actually admits — the clauses, intersected.
    ///
    /// What a caller needs when it has to *read* the seforim rather than search
    /// them (a dilug scans text). Empty when nothing has been narrowed, which
    /// means every sefer and not none: [`Scope::is_everything`] is the question
    /// to ask first.
    #[must_use]
    pub fn works(&self) -> BTreeSet<String> {
        let mut clauses = self.only.iter();
        let Some(first) = clauses.next() else {
            return BTreeSet::new();
        };
        clauses.fold(first.clone(), |so_far, clause| {
            so_far.intersection(clause).cloned().collect()
        })
    }

    #[must_use]
    pub fn excluded_works(&self) -> &BTreeSet<String> {
        &self.without
    }

    #[must_use]
    pub fn link_types(&self) -> &BTreeSet<EdgeType> {
        &self.linked
    }

    #[must_use]
    pub fn excluded_link_types(&self) -> &BTreeSet<EdgeType> {
        &self.unlinked
    }

    /// What the chip says.
    ///
    /// The names of what was clicked, in the order it was clicked, and *whole
    /// shelf* when nothing has been. A scope whose narrowing a reader cannot
    /// read off the chip is a result count nobody can account for.
    #[must_use]
    pub fn describe(&self) -> String {
        if self.named.is_empty() {
            return "whole shelf".to_string();
        }
        self.named.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_chip_says_whole_shelf_until_something_narrows_it() {
        assert_eq!(Scope::everything().describe(), "whole shelf");
        assert!(Scope::everything().is_everything());
    }

    #[test]
    fn narrowing_and_excluding_both_show_on_the_chip() {
        let scope = Scope::everything()
            .only(["bavli/berakhot".to_string()], "תלמוד/בבלי")
            .without(["mishnah-berurah".to_string()], "משנה ברורה");
        assert_eq!(scope.describe(), "תלמוד/בבלי · not משנה ברורה");
        assert!(!scope.is_everything());
    }

    #[test]
    fn two_clicks_narrow_twice_rather_than_adding_up() {
        // The bug this shape exists to prevent: narrowing to the Bavli and then
        // to the rishonim gave *the Bavli or the rishonim*, which is more
        // results than one click — a widening with a narrowing's label on it.
        let scope = Scope::everything()
            .only(["a".to_string(), "b".to_string()], "תלמוד")
            .only(["b".to_string(), "c".to_string()], "ראשונים");
        assert_eq!(scope.clauses().len(), 2);
        assert_eq!(
            scope.works().into_iter().collect::<Vec<_>>(),
            ["b"],
            "what is in both, not what is in either"
        );
    }

    #[test]
    fn two_scopes_holding_the_same_seforim_are_the_same_scope() {
        // The name is what the reader clicked; the scope is what it means. A
        // header that said "narrowed" because the label differed would be
        // reporting a difference the index cannot see.
        let one = Scope::everything().only(["a".to_string()], "the Bavli");
        let two = Scope::everything().only(["a".to_string()], "בבלי");
        assert_eq!(one, two);
    }
}
