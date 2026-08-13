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

/// One thing the reader added to, or subtracted from, where the search looks.
///
/// # Why the steps are kept rather than folded down
///
/// They used to be folded: `only` was a list of anonymous sets, `without` was
/// one merged set, and `named` was a parallel list of labels with no
/// correspondence to either. Nothing could be **taken back**, which is the
/// second half of what a reader asked for — *"i dont know how to add some and
/// minus some things from the search (some seforim or folders)"* — and the only
/// affordance the window could offer was *back to the whole shelf*, throwing
/// away four clicks to undo the fifth.
///
/// A step knows its own label, its own direction and its own seforim, so a panel
/// can list them and put a `×` on each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// What the reader clicked, for the chip and the panel to show.
    pub label: String,
    /// Subtracting rather than adding.
    pub exclude: bool,
    slugs: BTreeSet<String>,
}

impl Step {
    /// How many seforim this step names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slugs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slugs.is_empty()
    }
}

/// Which seforim, and which kinds of link, a search is confined to.
///
/// The default is the whole shelf, which is what the chip says when nobody has
/// touched it.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// What the reader added and subtracted, in the order they did it.
    ///
    /// Each *only* step is one clause and a hit has to be in **all** of them:
    /// narrowing to `תלמוד` and then to `ראשונים` is the rishonim of Shas, not
    /// everything on either. One step is an *or* within itself — a shelf is the
    /// seforim on it. No *only* steps means every sefer.
    steps: Vec<Step>,
    /// A hit must be touched by a link of one of these kinds.
    linked: BTreeSet<EdgeType>,
    /// A hit must be touched by none of these.
    unlinked: BTreeSet<EdgeType>,
}

/// Two scopes are the same when they let the same segments through.
///
/// A step's **label** is deliberately not compared: it is what the reader
/// clicked, and `תלמוד/בבלי` and `the Bavli` naming one set of seforim are one
/// scope. Comparing it would let a caller conclude that a scope had changed
/// when nothing about the search had.
impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        let sets = |scope: &Self| -> Vec<(bool, BTreeSet<String>)> {
            scope
                .steps
                .iter()
                .map(|step| (step.exclude, step.slugs.clone()))
                .collect()
        };
        sets(self) == sets(other) && self.linked == other.linked && self.unlinked == other.unlinked
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
    ///
    /// Clicking the same thing twice is one step, not two: a reader who adds the
    /// Bavli, wanders off and adds it again has said one thing.
    #[must_use]
    pub fn only(mut self, slugs: impl IntoIterator<Item = String>, named: &str) -> Self {
        self.add(Step {
            label: named.to_string(),
            exclude: false,
            slugs: slugs.into_iter().collect(),
        });
        self
    }

    /// Rule these seforim out.
    #[must_use]
    pub fn without(mut self, slugs: impl IntoIterator<Item = String>, named: &str) -> Self {
        self.add(Step {
            label: named.to_string(),
            exclude: true,
            slugs: slugs.into_iter().collect(),
        });
        self
    }

    fn add(&mut self, step: Step) {
        if step.slugs.is_empty() {
            return;
        }
        if self
            .steps
            .iter()
            .any(|held| held.exclude == step.exclude && held.slugs == step.slugs)
        {
            return;
        }
        self.steps.push(step);
    }

    /// Take one step back — the `×` on a row of the scope panel.
    ///
    /// Out of range does nothing: the window and the engine can disagree for a
    /// frame, and a second impatient click is not a reason to refuse.
    pub fn drop_step(&mut self, at: usize) {
        if at < self.steps.len() {
            self.steps.remove(at);
        }
    }

    /// What the reader added and subtracted, in the order they did it.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Only segments a link of this kind touches.
    #[must_use]
    pub fn linked(mut self, kind: EdgeType) -> Self {
        self.linked.insert(kind);
        self
    }

    /// Only segments no link of this kind touches.
    #[must_use]
    pub fn unlinked(mut self, kind: EdgeType) -> Self {
        self.unlinked.insert(kind);
        self
    }

    /// Whether this scope lets everything through.
    #[must_use]
    pub fn is_everything(&self) -> bool {
        self.steps.is_empty() && self.linked.is_empty() && self.unlinked.is_empty()
    }

    /// The clauses, in the order they were clicked. A hit is in every one.
    #[must_use]
    pub fn clauses(&self) -> Vec<&BTreeSet<String>> {
        self.steps
            .iter()
            .filter(|step| !step.exclude)
            .map(|step| &step.slugs)
            .collect()
    }

    /// The seforim this scope actually admits — the clauses, intersected.
    ///
    /// What a caller needs when it has to *read* the seforim rather than search
    /// them (a dilug scans text). Empty when nothing has been narrowed, which
    /// means every sefer and not none: [`Scope::is_everything`] is the question
    /// to ask first.
    #[must_use]
    pub fn works(&self) -> BTreeSet<String> {
        let mut clauses = self.clauses().into_iter();
        let Some(first) = clauses.next() else {
            return BTreeSet::new();
        };
        clauses.fold(first.clone(), |so_far, clause| {
            so_far.intersection(clause).cloned().collect()
        })
    }

    #[must_use]
    pub fn excluded_works(&self) -> BTreeSet<String> {
        self.steps
            .iter()
            .filter(|step| step.exclude)
            .flat_map(|step| step.slugs.iter().cloned())
            .collect()
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
    /// The names of what was clicked, in the order it was clicked, and **an
    /// empty string** when nothing has been. A scope whose narrowing a reader
    /// cannot read off the chip is a result count nobody can account for.
    ///
    /// # Why an exclusion is `−` and not `not`
    ///
    /// The names in here are seforim and shelves — the corpus's words, in
    /// whatever language the corpus wrote them. The only thing this function
    /// contributed of its own was the English `not`, and `whole shelf` for the
    /// empty case, which put two English words in a Hebrew panel that has no
    /// other. `−` is the glyph the scope panel's own *take out* button already
    /// carries, it needs no language, and the empty case is left to the window
    /// to name — which is where every other word on that panel comes from.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut named: Vec<String> = self
            .steps
            .iter()
            .map(|step| {
                if step.exclude {
                    format!("−{}", step.label)
                } else {
                    step.label.clone()
                }
            })
            .collect();
        named.extend(self.linked.iter().map(|k| k.as_str().to_string()));
        named.extend(self.unlinked.iter().map(|k| format!("−{}", k.as_str())));
        named.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_chip_says_nothing_until_something_narrows_it() {
        // Empty, and the **window** says `כל המדף` over it. This used to return
        // the English words `whole shelf`, which is two of the four English
        // strings a Hebrew search panel opened on.
        assert_eq!(Scope::everything().describe(), "");
        assert!(Scope::everything().is_everything());
    }

    #[test]
    fn narrowing_and_excluding_both_show_on_the_chip() {
        let scope = Scope::everything()
            .only(["bavli/berakhot".to_string()], "תלמוד/בבלי")
            .without(["mishnah-berurah".to_string()], "משנה ברורה");
        // `−`, not `not`: the names are the corpus's words and the glyph needs
        // no language. It is the one the scope panel's *take out* button carries.
        assert_eq!(scope.describe(), "תלמוד/בבלי · −משנה ברורה");
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
    fn a_step_can_be_taken_back_without_losing_the_others() {
        // *"i dont know how to add some and minus some things from the search
        // (some seforim or folders)."* Adding was a facet click; subtracting was
        // a facet click; **un**-adding was *back to the whole shelf*, which threw
        // away every other click as well.
        let mut scope = Scope::everything()
            .only(["a".to_string()], "תלמוד")
            .only(["b".to_string()], "ראשונים")
            .without(["c".to_string()], "משנה ברורה");
        assert_eq!(scope.steps().len(), 3);
        assert_eq!(scope.describe(), "תלמוד · ראשונים · −משנה ברורה");

        scope.drop_step(1);
        assert_eq!(scope.describe(), "תלמוד · −משנה ברורה");
        assert_eq!(
            scope.works().into_iter().collect::<Vec<_>>(),
            ["a"],
            "and the clause that is left is the one that was kept"
        );
        // Out of range is a window one frame behind, not a reason to panic.
        scope.drop_step(9);
        assert_eq!(scope.steps().len(), 2);
    }

    #[test]
    fn adding_the_same_shelf_twice_is_one_step() {
        // Two identical clauses intersect to themselves, so nothing about the
        // search changes — but the chip would read `תלמוד · תלמוד` and the panel
        // would grow a row that does nothing.
        let scope = Scope::everything()
            .only(["a".to_string()], "תלמוד")
            .only(["a".to_string()], "תלמוד");
        assert_eq!(scope.steps().len(), 1);
    }

    #[test]
    fn a_step_that_names_no_sefer_is_not_a_step() {
        // Clicking a shelf nothing is filed on would otherwise add a clause that
        // admits nothing, and every search after it would come back empty with a
        // chip that looked reasonable.
        let scope = Scope::everything().only(Vec::new(), "מדף ריק");
        assert!(scope.is_everything());
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
