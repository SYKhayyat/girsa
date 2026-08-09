//! What the lane covers, and what it does not — said out loud, every time.
//!
//! This is the module spec.md §9.9 is really about. *A partial lane is fine. A
//! partial lane that looks complete is the §9 defect this whole section exists
//! to avoid.*
//!
//! The failure it prevents is specific and it is invisible from the results. A
//! reader asks *I remember a Rishon who says something like this*, the lane
//! answers with three Rishonim, and the one they were thinking of is in a sefer
//! nobody embedded. Nothing is wrong with the three; the answer is still a lie,
//! because it was read as **the Rishonim who say something like this** and it
//! was *the Rishonim who say something like this, among the eleven per cent of
//! the shelf that is in the index*.
//!
//! So every answer carries one of these, and every surface that draws an answer
//! draws its sentence. It is the same argument — and deliberately the same
//! shape — as `girsa_app::reading::Gap`, which says *"4 PDFs on this shelf
//! aren't searchable yet"* over the literal index.
//!
//! # It was the same shape and it was not the same sentence
//!
//! That paragraph used to end *"one sentence, [`Coverage::said`], composed here
//! so the window, the command line, the MCP surface and the test cannot drift
//! apart"* — and `girsa_app::reading::Gap` and `girsa_note::since::Unindexed`
//! each carried the same claim about their own clause. Three composers, each
//! correct about its surfaces, none able to see the other two. What drifted was
//! everything between them: this one joined with a semicolon and the other two
//! with a middle dot, this one alone knew a five-figure number wants a comma in
//! it, and `Gap` joined an already-joined string into its own join.
//!
//! Worse than the punctuation: an `adjacent` answer carried this sentence and
//! said nothing about the chaburah written this morning that no lane has
//! embedded, while a `search` answer said exactly that and nothing about the
//! lane. **Three subsets of one truth, each wearing a sentence that claimed to
//! be complete.**
//!
//! So the clause stays here, where the fact is, and [`Coverage::clauses`] hands
//! it over. `girsa_plain::said::Clauses` does the joining and
//! `girsa_nearby::Unseen` decides which clauses are one answer — the decision none
//! of the three was in a position to make.
//!
//! # Counted in segments, not in seforim
//!
//! A sefer half-embedded is neither in nor out. Reporting it as *in* would be
//! the lie above; reporting it as *out* would send a reader to start a job that
//! is nearly finished. Both numbers, then — the same call W26 made about a scan
//! stopped at page 40 of 302, for the same reason.

use std::collections::BTreeMap;

use girsa_plain::said::{plural, thousands, Clauses};

/// What a lane with nothing in it says.
///
/// A constant because `app/src/api.ts` hard-codes this exact sentence twice, in
/// the browser build's stub, where there is no Rust to ask — a fourth copy of a
/// string whose whole point is that there is one of it. It is now spelled once
/// here and checked against the TypeScript by
/// `crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs`.
pub const NOTHING_YET: &str = "nothing is in the semantic lane yet";

/// One sefer's standing in the lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Covered {
    pub slug: String,
    /// What the reader calls it.
    pub title: String,
    /// Segments with words in them that the selection asks for.
    pub wanted: usize,
    /// Of those, how many have a vector.
    pub embedded: usize,
}

impl Covered {
    #[must_use]
    pub const fn is_whole(&self) -> bool {
        self.embedded >= self.wanted
    }

    #[must_use]
    pub const fn is_started(&self) -> bool {
        self.embedded > 0
    }
}

/// What is in the lane and what is outside it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Coverage {
    /// The seforim the selection asks for, whether or not they are finished.
    pub chosen: Vec<Covered>,
    /// Seforim on the shelf that nobody chose. Named, up to [`NAMED`], and
    /// counted in full.
    pub outside: Vec<String>,
    /// Set when the selection is *the whole library and whatever joins it*, in
    /// which case nothing is outside by construction.
    pub everything: bool,
    /// Seforim whose vectors were made by another model and are not being read.
    /// Not the same as *not embedded* — the work is there and unreachable —
    /// so it is counted separately and said separately.
    pub other_model: Vec<String>,
}

/// How many seforim a coverage line names before it starts counting them.
///
/// A sentence with forty titles in it is a sentence nobody reads, and a list
/// that silently stops reads as all of them. So: name a few, count the rest.
pub const NAMED: usize = 3;

impl Coverage {
    /// Add a sefer the selection asks for.
    pub fn add(&mut self, covered: Covered) {
        self.chosen.push(covered);
    }

    #[must_use]
    pub fn wanted(&self) -> usize {
        self.chosen.iter().map(|c| c.wanted).sum()
    }

    #[must_use]
    pub fn embedded(&self) -> usize {
        self.chosen.iter().map(|c| c.embedded).sum()
    }

    /// Whether everything chosen has been embedded. **Not** whether everything
    /// exists — a whole lane over two seforim is still a lane over two seforim,
    /// which is why [`Coverage::said`] says both halves.
    #[must_use]
    pub fn is_whole(&self) -> bool {
        self.chosen.iter().all(Covered::is_whole)
    }

    #[must_use]
    pub fn is_nothing(&self) -> bool {
        self.embedded() == 0
    }

    /// The sentence. One implementation, drawn by every surface.
    ///
    /// Never empty — unlike `Gap`, which has nothing to say when the shelf
    /// holds no scans. A lane always has something to say about its own
    /// coverage, because *this is complete* is exactly the claim a reader is
    /// entitled to have checked rather than assumed.
    #[must_use]
    pub fn said(&self) -> String {
        self.clauses().joined()
    }

    /// The clauses, worded here and joined nowhere.
    ///
    /// See [`girsa_plain::said`]. This composer joined with `; ` while the
    /// other two joined with `" · "`, and it was the only one of the three that
    /// knew a five-figure number wants a comma in it — both facts now live one
    /// module down, where the sentence that carries clauses from all three is
    /// assembled.
    #[must_use]
    pub fn clauses(&self) -> Clauses {
        let mut clauses = Clauses::new();
        let opening = if self.chosen.is_empty() {
            NOTHING_YET.to_string()
        } else if self.everything {
            // The cost goes in the sentence that makes the offer.
            //
            // `Chosen::everything()` is a first-class standing choice and this
            // branch is *tested*, so the whole shelf is presented to a reader as
            // an equal option to the one the numbers came from — and the numbers
            // are 54 seconds for Hilchos Tefillah against about thirteen days
            // for 5,000,545 segments, measured, at
            // `crate::model::SEGMENTS_A_SECOND`. That figure was written down in
            // a module note and said nowhere a reader looks.
            let left = self.wanted().saturating_sub(self.embedded());
            format!(
                "this lane covers the whole library — {} of {} segments so far{}",
                thousands(self.embedded()),
                thousands(self.wanted()),
                match crate::model::how_long(left) {
                    Some(when) => format!(", {when} of embedding left"),
                    None => String::new(),
                }
            )
        } else {
            let whole = self.embedded() >= self.wanted();
            format!(
                "this lane covers {}{}",
                self.naming(),
                if whole {
                    format!(" ({} segments)", thousands(self.embedded()))
                } else {
                    format!(
                        " — {} of {} segments so far",
                        thousands(self.embedded()),
                        thousands(self.wanted())
                    )
                }
            )
        };
        clauses.say(opening);
        if !self.everything {
            clauses.count(self.outside.len(), |n| {
                format!(
                    "{} other {} on this shelf {} in it",
                    thousands(n),
                    plural(n, "sefer", "seforim"),
                    plural(n, "isn't", "aren't"),
                )
            });
        }
        clauses.count(self.other_model.len(), |n| {
            format!(
                "{} {} vectors made by another model and are not being read",
                thousands(n),
                plural(n, "sefer has", "seforim have"),
            )
        });
        clauses
    }

    /// The chosen seforim, named up to [`NAMED`] of them and counted after.
    fn naming(&self) -> String {
        let names: Vec<&str> = self
            .chosen
            .iter()
            .take(NAMED)
            .map(|c| c.title.as_str())
            .collect();
        let rest = self.chosen.len().saturating_sub(names.len());
        let named = names.join(", ");
        if rest == 0 {
            named
        } else {
            format!("{named} and {} more", thousands(rest))
        }
    }

    /// Roll up a set of per-sefer counts into one coverage.
    ///
    /// Takes what it is given rather than going and looking: which seforim are
    /// on the shelf, and what they are called, is `girsa-app`'s question, and
    /// this crate answering it too would be a second opinion about the shelf.
    #[must_use]
    pub fn of(
        chosen: impl IntoIterator<Item = Covered>,
        outside: impl IntoIterator<Item = String>,
        everything: bool,
    ) -> Self {
        let mut by_slug: BTreeMap<String, Covered> = BTreeMap::new();
        for covered in chosen {
            by_slug.insert(covered.slug.clone(), covered);
        }
        Self {
            chosen: by_slug.into_values().collect(),
            outside: outside.into_iter().collect(),
            everything,
            other_model: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn covered(slug: &str, title: &str, embedded: usize, wanted: usize) -> Covered {
        Covered {
            slug: slug.to_string(),
            title: title.to_string(),
            wanted,
            embedded,
        }
    }

    #[test]
    fn a_lane_with_nothing_in_it_says_so_rather_than_saying_nothing() {
        assert_eq!(
            Coverage::default().said(),
            "nothing is in the semantic lane yet"
        );
        assert!(Coverage::default().is_nothing());
    }

    #[test]
    fn a_whole_lane_over_part_of_the_shelf_still_names_what_is_outside() {
        // The defect this module exists to prevent. Everything chosen is
        // embedded — and the answer is still over eleven per cent of a shelf.
        let coverage = Coverage::of(
            [
                covered("rambam", "רמב\"ם", 4000, 4000),
                covered("ramban", "רמב\"ן", 2000, 2000),
            ],
            (0..41).map(|n| format!("sefer-{n}")),
            false,
        );
        assert!(coverage.is_whole());
        let said = coverage.said();
        assert!(said.contains("6,000 segments"), "{said}");
        assert!(said.contains("41 other seforim"), "{said}");
        assert!(said.contains("aren't in it"), "{said}");
    }

    #[test]
    fn a_half_embedded_sefer_is_reported_as_both_numbers() {
        // A scan stopped at page 40 of 302 is not read and is not unread. Same
        // here, and for the same reason: either number alone sends a reader
        // somewhere wrong.
        let coverage = Coverage::of(
            [covered("mishnah-berurah", "משנה ברורה", 8120, 18120)],
            [],
            false,
        );
        let said = coverage.said();
        assert!(said.contains("8,120 of 18,120 segments"), "{said}");
        assert!(!coverage.is_whole());
    }

    #[test]
    fn more_than_three_seforim_are_named_three_and_counted() {
        let coverage = Coverage::of(
            (0..9).map(|n| covered(&format!("s{n}"), &format!("ספר {n}"), 10, 10)),
            [],
            false,
        );
        let said = coverage.said();
        assert!(said.contains("and 6 more"), "{said}");
        assert_eq!(said.matches('ס').count(), 3, "three named: {said}");
    }

    #[test]
    fn the_whole_library_has_nothing_outside_it_and_says_the_progress() {
        let mut coverage = Coverage::of([covered("x", "ספר", 1_200_000, 5_000_545)], [], true);
        let said = coverage.said();
        assert!(said.contains("the whole library"), "{said}");
        assert!(said.contains("1,200,000 of 5,000,545"), "{said}");
        assert!(!said.contains("aren't in it"), "{said}");

        // And a sefer whose vectors came from another model is a third state,
        // said separately: the work exists and cannot be read.
        coverage.other_model.push("ramban".to_string());
        assert!(
            coverage.said().contains("another model"),
            "{}",
            coverage.said()
        );
    }

    #[test]
    fn one_of_each_reads_like_english() {
        let coverage = Coverage::of([covered("x", "ספר", 5, 5)], ["y".to_string()], false);
        let said = coverage.said();
        assert!(
            said.contains("1 other sefer on this shelf isn't in it"),
            "{said}"
        );
    }

    #[test]
    fn thousands_are_grouped_and_small_numbers_are_left_alone() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(5_000_545), "5,000,545");
    }

    #[test]
    fn the_offer_of_the_whole_shelf_carries_what_it_costs() {
        // `Chosen::everything()` is a first-class standing choice with a tested
        // branch in this sentence, so the whole shelf was presented as an equal
        // option to the one the numbers came from — 54 seconds for Hilchos
        // Tefillah against about thirteen days for 5,000,545 segments, measured,
        // and said in a module note the reader never opens.
        let mut coverage = Coverage {
            everything: true,
            ..Coverage::default()
        };
        coverage.chosen.push(Covered {
            slug: "shulchan-arukh/orach-chayim".into(),
            title: "שולחן ערוך".into(),
            wanted: 5_000_545,
            embedded: 1_200_000,
        });
        let said = coverage.said();
        assert!(said.contains("1,200,000 of 5,000,545"), "{said}");
        assert!(
            said.contains("days") && said.contains("embedding left"),
            "the sentence that offers the whole shelf does not say what it costs: {said}"
        );
    }

    #[test]
    fn a_finished_lane_over_everything_is_not_offered_a_wait() {
        let mut coverage = Coverage {
            everything: true,
            ..Coverage::default()
        };
        coverage.chosen.push(Covered {
            slug: "a".into(),
            title: "א".into(),
            wanted: 240,
            embedded: 240,
        });
        let said = coverage.said();
        assert!(
            !said.contains("left"),
            "nothing is left, and it said there was: {said}"
        );
    }
}
