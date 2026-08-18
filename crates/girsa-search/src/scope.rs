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

use std::collections::{BTreeMap, BTreeSet};

use girsa_link::EdgeType;

/// Which question a step answers.
///
/// # Why a step has to know this
///
/// Every *only* step used to be its own `Must`, so two of them intersected.
/// That is right for two steps that answer **different** questions — the Bavli
/// and then the rishonim is the rishonim of the Bavli, and folding those into
/// one set would let a second narrowing widen the first. It is catastrophic for
/// two steps that answer the **same** one: a work is filed on one shelf and
/// written in one era, so *the Bavli* and then *the Yerushalmi* intersected to
/// the empty set and every search after it came back `0 found` under a chip
/// that read as though both had been added.
///
/// That is not a hypothetical. A reader ticked the masechtos of Shas one at a
/// time in the scope panel, searched `חייב`, and was told it appears nowhere in
/// Shas — while the same query over *some* of Shas found it. Twelve clicks that
/// each looked like they were adding produced a scope that admitted nothing.
///
/// So steps are grouped: same question, *or*; different questions, *and*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Asked {
    /// *Which seforim* — a shelf or a single sefer. Both name seforim directly,
    /// and a reader who names two of them wants both.
    Which,
    /// *From when* — the era a work was written in.
    When,
    /// *By whom* — its author.
    Who,
    /// *Filed under what* — a tag the reader put on it themselves.
    Tagged,
}

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
    /// Which question this step answers. Steps that answer the same one are an
    /// *or*; steps that answer different ones are an *and*. See [`Asked`].
    pub asked: Asked,
    /// The row that was clicked, so the panel can find its own step again and
    /// draw the row as ticked. Two rows can carry the same label — every shelf
    /// has a `ראשונים` — and the key is what tells them apart.
    pub key: String,
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

    /// The seforim this step names.
    #[must_use]
    pub fn slugs(&self) -> &BTreeSet<String> {
        &self.slugs
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
    /// Steps that answer the same question are one clause, *unioned*; steps
    /// that answer different ones are separate clauses, and a hit has to be in
    /// all of them. So `תלמוד/בבלי` and then `תלמוד/ירושלמי` is both talmuds,
    /// while `תלמוד` and then `ראשונים` is the rishonim of Shas. No *only*
    /// steps means every sefer. See [`Asked`] and [`Scope::clauses`].
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
        let sets = |scope: &Self| -> Vec<(bool, Asked, BTreeSet<String>)> {
            scope
                .steps
                .iter()
                .map(|step| (step.exclude, step.asked, step.slugs.clone()))
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
    /// Answers [`Asked::Which`] — the question the scope panel's rows ask. Two
    /// calls are an *or*: a reader who adds Berakhos and then Shabbos wants
    /// both, and before [`Asked`] existed they got neither.
    ///
    /// Clicking the same thing twice is one step, not two: a reader who adds the
    /// Bavli, wanders off and adds it again has said one thing.
    #[must_use]
    pub fn only(self, slugs: impl IntoIterator<Item = String>, named: &str) -> Self {
        self.only_by(Asked::Which, named, slugs, named)
    }

    /// The same, saying which question the step answers and which row asked it.
    #[must_use]
    pub fn only_by(
        mut self,
        asked: Asked,
        key: &str,
        slugs: impl IntoIterator<Item = String>,
        named: &str,
    ) -> Self {
        self.add(Step {
            label: named.to_string(),
            exclude: false,
            asked,
            key: key.to_string(),
            slugs: slugs.into_iter().collect(),
        });
        self
    }

    /// Rule these seforim out.
    #[must_use]
    pub fn without(self, slugs: impl IntoIterator<Item = String>, named: &str) -> Self {
        self.without_by(Asked::Which, named, slugs, named)
    }

    /// The same, saying which question the step answers and which row asked it.
    ///
    /// An exclusion is a `MustNot` whichever question it answers, so [`Asked`]
    /// changes nothing about what it admits — it is carried so the panel can
    /// find the step belonging to a row and untick it.
    #[must_use]
    pub fn without_by(
        mut self,
        asked: Asked,
        key: &str,
        slugs: impl IntoIterator<Item = String>,
        named: &str,
    ) -> Self {
        self.add(Step {
            label: named.to_string(),
            exclude: true,
            asked,
            key: key.to_string(),
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

    /// Take back every step this row put in, whichever direction it went.
    ///
    /// What unticking a checkbox does. Matched on the **key**, so unticking
    /// `תלמוד/בבלי` does not also take back `נביאים`'s `ראשונים`.
    pub fn drop_key(&mut self, asked: Asked, key: &str) {
        self.steps
            .retain(|step| !(step.asked == asked && step.key == key));
    }

    /// Whether a row is ticked — the scope holds an *only* step that is exactly
    /// it.
    #[must_use]
    pub fn holds(&self, asked: Asked, key: &str) -> bool {
        self.steps
            .iter()
            .any(|step| !step.exclude && step.asked == asked && step.key == key)
    }

    /// Whether anything at all has been picked under this question.
    ///
    /// The difference between *the whole shelf minus one* and *these three* —
    /// which is what decides whether unticking a row drops a pick or rules the
    /// row out.
    #[must_use]
    pub fn any_picked(&self, asked: Asked) -> bool {
        self.steps
            .iter()
            .any(|step| !step.exclude && step.asked == asked)
    }

    /// Whether a row was ticked **off** — the scope holds a *without* step for
    /// it.
    #[must_use]
    pub fn refuses(&self, asked: Asked, key: &str) -> bool {
        self.steps
            .iter()
            .any(|step| step.exclude && step.asked == asked && step.key == key)
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

    /// The clauses. A hit is in every one — and one clause is every step that
    /// asked the same question, *unioned*.
    ///
    /// # The `0 found` bug, in one function
    ///
    /// This used to return one clause per step. Every step being its own `Must`
    /// meant that ticking Berakhos and then Shabbos asked for a segment in both
    /// masechtos at once, and there is no such segment. The panel offered a `+`
    /// on every row, so the obvious way to search Shas — tick the masechtos —
    /// was the way to search nothing, and the answer came back `0 found` with a
    /// chip listing all thirty-seven of them.
    ///
    /// Grouping by [`Asked`] keeps what the `Must` was for: era and shelf are
    /// different questions, so *rishonim* after *Bavli* still intersects, and a
    /// second narrowing still cannot widen the first.
    #[must_use]
    pub fn clauses(&self) -> Vec<BTreeSet<String>> {
        let mut grouped: BTreeMap<Asked, BTreeSet<String>> = BTreeMap::new();
        for step in self.steps.iter().filter(|step| !step.exclude) {
            grouped
                .entry(step.asked)
                .or_default()
                .extend(step.slugs.iter().cloned());
        }
        grouped.into_values().collect()
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
        let kept = clauses.fold(first, |so_far, clause| {
            so_far.intersection(&clause).cloned().collect()
        });
        let out = self.excluded_works();
        kept.into_iter()
            .filter(|slug| !out.contains(slug))
            .collect()
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
    fn two_questions_narrow_twice_rather_than_adding_up() {
        // The bug this shape exists to prevent: narrowing to the Bavli and then
        // to the rishonim gave *the Bavli or the rishonim*, which is more
        // results than one click — a widening with a narrowing's label on it.
        //
        // Two *different* questions, so still an `and`.
        let scope = Scope::everything()
            .only_by(
                Asked::Which,
                "תלמוד",
                ["a".to_string(), "b".to_string()],
                "תלמוד",
            )
            .only_by(
                Asked::When,
                "rishonim",
                ["b".to_string(), "c".to_string()],
                "ראשונים",
            );
        assert_eq!(scope.clauses().len(), 2);
        assert_eq!(
            scope.works().into_iter().collect::<Vec<_>>(),
            ["b"],
            "what is in both, not what is in either"
        );
    }

    #[test]
    fn two_shelves_are_both_shelves_and_not_neither() {
        // The `0 found` bug. A reader ticked the masechtos of Shas one at a
        // time and was told `חייב` appears nowhere in it, because a work is on
        // one shelf and two `Must` clauses over one column admit nothing.
        //
        // Same question, so an `or`: one clause, holding both.
        let scope = Scope::everything()
            .only(["a".to_string()], "בבלי")
            .only(["b".to_string()], "ירושלמי");
        assert_eq!(scope.steps().len(), 2, "two rows to untick, still");
        assert_eq!(scope.clauses().len(), 1, "and one clause between them");
        assert_eq!(
            scope.works().into_iter().collect::<Vec<_>>(),
            ["a", "b"],
            "both, which is the only reading of two ticks nobody has to be told"
        );
    }

    #[test]
    fn ticking_thirty_seven_masechtos_searches_thirty_seven_masechtos() {
        // The same thing at the size it was reported at: enough clicks that
        // nobody would have gone looking for an intersection.
        let scope = (0..37).fold(Scope::everything(), |scope, n| {
            scope.only([format!("bavli/{n}")], &format!("מסכת {n}"))
        });
        assert_eq!(scope.clauses().len(), 1);
        assert_eq!(scope.works().len(), 37);
    }

    #[test]
    fn what_was_ruled_out_is_not_among_the_seforim_it_admits() {
        // `works()` is what a reader of *text* gets — the dilug scan. It listed
        // the clauses and never looked at the exclusions, so a sefer the reader
        // had explicitly taken out was still scanned.
        let scope = Scope::everything()
            .only(["a".to_string(), "b".to_string()], "תלמוד")
            .without(["b".to_string()], "ברכות");
        assert_eq!(scope.works().into_iter().collect::<Vec<_>>(), ["a"]);
    }

    #[test]
    fn a_row_can_find_its_own_step_and_take_it_back() {
        // What a checkbox needs: *am I ticked*, and *untick me*. Matched on the
        // key, because every shelf has a `ראשונים` and the label alone would
        // untick somebody else's.
        let mut scope = Scope::everything()
            .only_by(Asked::Which, "תלמוד/בבלי", ["a".to_string()], "בבלי")
            .only_by(Asked::Which, "תלמוד/ירושלמי", ["b".to_string()], "ירושלמי");
        assert!(scope.holds(Asked::Which, "תלמוד/בבלי"));
        assert!(!scope.refuses(Asked::Which, "תלמוד/בבלי"));
        scope.drop_key(Asked::Which, "תלמוד/בבלי");
        assert!(!scope.holds(Asked::Which, "תלמוד/בבלי"));
        assert!(
            scope.holds(Asked::Which, "תלמוד/ירושלמי"),
            "and only that one"
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
