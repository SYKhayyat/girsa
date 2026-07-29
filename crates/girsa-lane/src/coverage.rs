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
//! aren't searchable yet"* over the literal index. One sentence,
//! [`Coverage::said`], composed here so the window, the command line, the MCP
//! surface and the test cannot drift apart.
//!
//! # Counted in segments, not in seforim
//!
//! A sefer half-embedded is neither in nor out. Reporting it as *in* would be
//! the lie above; reporting it as *out* would send a reader to start a job that
//! is nearly finished. Both numbers, then — the same call W26 made about a scan
//! stopped at page 40 of 302, for the same reason.

use std::collections::BTreeMap;

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
    /// Never `None` — unlike `Gap`, which has nothing to say when the shelf
    /// holds no scans. A lane always has something to say about its own
    /// coverage, because *this is complete* is exactly the claim a reader is
    /// entitled to have checked rather than assumed.
    #[must_use]
    pub fn said(&self) -> String {
        let mut said = if self.chosen.is_empty() {
            "nothing is in the semantic lane yet".to_string()
        } else if self.everything {
            format!(
                "this lane covers the whole library — {} of {} segments so far",
                thousands(self.embedded()),
                thousands(self.wanted())
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
        if !self.everything && !self.outside.is_empty() {
            said.push_str(&format!(
                "; {} other {} on this shelf {} in it",
                thousands(self.outside.len()),
                if self.outside.len() == 1 {
                    "sefer"
                } else {
                    "seforim"
                },
                if self.outside.len() == 1 {
                    "isn't"
                } else {
                    "aren't"
                },
            ));
        }
        if !self.other_model.is_empty() {
            said.push_str(&format!(
                "; {} {} vectors made by another model and are not being read",
                thousands(self.other_model.len()),
                if self.other_model.len() == 1 {
                    "sefer has"
                } else {
                    "seforim have"
                },
            ));
        }
        said
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

/// `12904` as `12,904`. A five-figure number with no separator in a sentence
/// about how much of a library is covered is a number nobody reads.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
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
}
