//! Which end of a `comments-on` edge is the commentary.
//!
//! # The defect this exists to close
//!
//! Sefaria's `links*.csv` gives two citations per row and a label. It does not
//! say which citation is the commentary and which is the sefer being commented
//! on, and for `commentary` rows it is genuinely both ways round in the data.
//! [`crate::sefaria::read_file`] recorded them in the order it read them, so
//! 1,044,350 of the corpus's 2,123,215 `comments-on` edges — 49% — ended up
//! stored as *base → commentary*:
//!
//! ```text
//! girsa:bavli/berakhot/10a:1#418  --comments-on-->  girsa:bavli/rashi-on-berakhot/10a:1:1#367
//! ```
//!
//! which asserts that the gemara is a commentary on Rashi. Every internal
//! check passed anyway — both ends resolve to real segments, the type is
//! right, the count is right — while `girsa-app`'s panel, which asks
//! `inbound.jsonl` *what lands on this daf*, was told: Ben Yehoyada and
//! Benayahu, and not Rashi. See `tests/the_meforshim_are_on_the_daf.rs`.
//!
//! # Why `commentary_on` and not the title
//!
//! The corpus already states the answer. Sefaria's schemas carry `dependence`
//! and `base_text_titles`, the importer records them as
//! [`girsa_corpus::work::Work::commentary_on`], and 5,701 of them resolve to a
//! work on the shelf — every single one. So orientation is read, not guessed.
//!
//! Guessing it from the slug is forbidden, and for a concrete reason
//! (BUILDER.md rule 6): `X-on-Y` would attach `Rashi on Berakhot` to the
//! Yerushalmi masechta of the same name.
//!
//! # Why an undeclared pair is left alone
//!
//! Two works can be joined by a `commentary` row while the corpus never says
//! either is a commentary on the other — 2,039 works declare nothing. Flipping
//! those on a hunch would trade a knowably-wrong direction for an unknowably
//! wrong one, so they are counted and left as they are. A count that is
//! reported is a question somebody can answer later; a silent guess is not.

use std::collections::HashMap;

use girsa_corpus::work::Work;

use crate::{Edge, EdgeType};

/// What the corpus declares about which works comment on which.
#[derive(Debug, Default, Clone)]
pub struct Bases {
    by_work: HashMap<String, Vec<String>>,
}

/// What orienting one edge did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Not a `comments-on` edge. Nothing to orient, and nothing this module
    /// has an opinion about.
    NotCommentary,
    /// Already commentary → base. Untouched.
    Kept,
    /// Was base → commentary. The ends were swapped.
    Flipped,
    /// Neither end declares the other as its base. Left as it was.
    Undeclared,
}

/// How many of each, over a run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    pub not_commentary: usize,
    pub kept: usize,
    pub flipped: usize,
    pub undeclared: usize,
}

impl Tally {
    pub fn count(&mut self, what: Orientation) {
        match what {
            Orientation::NotCommentary => self.not_commentary += 1,
            Orientation::Kept => self.kept += 1,
            Orientation::Flipped => self.flipped += 1,
            Orientation::Undeclared => self.undeclared += 1,
        }
    }

    /// One line for the import log.
    ///
    /// Every count is named separately on purpose. An earlier version folded
    /// the three million `references` into "already right", which made the
    /// orientation look far healthier than it was — the exact shape of lying
    /// message this codebase spent a week removing.
    #[must_use]
    pub fn said(&self) -> String {
        format!(
            "comments-on orientation: {} already right, {} flipped, {} left alone \
             (neither end declares the other); {} edges are not commentary",
            self.kept, self.flipped, self.undeclared, self.not_commentary
        )
    }
}

/// The declarations and the running count, together.
///
/// One argument instead of two at every import site — and the pair is only ever
/// useful together, since orienting without counting is how the defect stayed
/// quiet for as long as it did.
#[derive(Debug)]
pub struct Orienting<'a> {
    bases: &'a Bases,
    tally: Tally,
}

impl<'a> Orienting<'a> {
    #[must_use]
    pub fn new(bases: &'a Bases) -> Self {
        Self {
            bases,
            tally: Tally::default(),
        }
    }

    /// Orient one edge and count what that did.
    pub fn apply(&mut self, edge: &mut Edge) {
        let what = self.bases.orient(edge);
        self.tally.count(what);
    }

    #[must_use]
    pub const fn tally(&self) -> Tally {
        self.tally
    }
}

impl Bases {
    /// Read the declarations off the work index.
    #[must_use]
    pub fn of(works: &[Work]) -> Self {
        Self {
            by_work: works
                .iter()
                .filter(|w| !w.commentary_on.is_empty())
                .map(|w| {
                    let bases = w.commentary_on.iter().map(|b| b.slug.clone()).collect();
                    (w.slug.clone(), bases)
                })
                .collect(),
        }
    }

    /// Whether `work` declares itself a commentary on `base`.
    #[must_use]
    pub fn says(&self, work: &str, base: &str) -> bool {
        self.by_work
            .get(work)
            .is_some_and(|bases| bases.iter().any(|b| b == base))
    }

    /// How many works declared a base. For the import log.
    #[must_use]
    pub fn declaring(&self) -> usize {
        self.by_work.len()
    }

    /// Put a `comments-on` edge the right way round.
    ///
    /// Idempotent: orienting an already-oriented edge returns
    /// [`Orientation::Kept`] and changes nothing, so this is safe to run over
    /// a store that has been through it before.
    pub fn orient(&self, edge: &mut Edge) -> Orientation {
        // Only the type whose name states a direction. `references` and
        // `quotes` are directional too, but the row itself is the evidence for
        // which way they point and there is nothing to check them against.
        if edge.edge_type != EdgeType::CommentsOn {
            return Orientation::NotCommentary;
        }
        let (from, to) = (edge.from.from.work(), edge.to.from.work());
        // A work that comments on itself is not a thing to reason about, and a
        // self-edge would flip forever.
        if from == to {
            return Orientation::Kept;
        }
        if self.says(from, to) {
            return Orientation::Kept;
        }
        if self.says(to, from) {
            std::mem::swap(&mut edge.from, &mut edge.to);
            return Orientation::Flipped;
        }
        Orientation::Undeclared
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::{Anchor, Method};
    use girsa_corpus::segment::SegmentId;
    use girsa_corpus::work::{BaseText, Mapping, Source};

    fn work(slug: &str, on: &[&str]) -> Work {
        Work {
            slug: slug.to_string(),
            he_title: slug.to_string(),
            en_title: slug.to_string(),
            categories: Vec::new(),
            source: Source::Sefaria,
            origin: std::path::PathBuf::new(),
            schema: None,
            he_sections: Vec::new(),
            author: None,
            era: None,
            comp_date: None,
            version: None,
            commentary_on: on
                .iter()
                .map(|b| BaseText {
                    slug: (*b).to_string(),
                    mapping: Mapping::ManyToOne,
                })
                .collect(),
        }
    }

    fn id(work: &str) -> SegmentId {
        SegmentId::new(
            work.to_string(),
            vec!["1".to_string()],
            girsa_corpus::segment::Ordinal::root(1),
        )
    }

    fn edge(from: &str, to: &str, edge_type: EdgeType) -> Edge {
        Edge {
            from: Anchor::point(id(from)),
            to: Anchor::point(id(to)),
            edge_type,
            method: Method::SefariaSeed,
            source_label: "commentary".to_string(),
        }
    }

    fn bases() -> Bases {
        Bases::of(&[
            work("bavli/berakhot", &[]),
            work("bavli/rashi-on-berakhot", &["bavli/berakhot"]),
            work("some-sefer", &[]),
            work("another-sefer", &[]),
        ])
    }

    #[test]
    fn an_edge_written_from_the_daf_to_rashi_is_turned_round() {
        let mut e = edge(
            "bavli/berakhot",
            "bavli/rashi-on-berakhot",
            EdgeType::CommentsOn,
        );
        assert_eq!(bases().orient(&mut e), Orientation::Flipped);
        assert_eq!(e.from.from.work(), "bavli/rashi-on-berakhot");
        assert_eq!(e.to.from.work(), "bavli/berakhot");
    }

    #[test]
    fn an_edge_already_the_right_way_round_is_left_exactly_as_it_was() {
        let before = edge(
            "bavli/rashi-on-berakhot",
            "bavli/berakhot",
            EdgeType::CommentsOn,
        );
        let mut after = before.clone();
        assert_eq!(bases().orient(&mut after), Orientation::Kept);
        assert_eq!(before, after);
    }

    #[test]
    fn orienting_twice_is_orienting_once() {
        // The repair pass runs over a store that may already have been
        // repaired. If this were not idempotent it would swing the graph back
        // and forth on every run.
        let bases = bases();
        let mut once = edge(
            "bavli/berakhot",
            "bavli/rashi-on-berakhot",
            EdgeType::CommentsOn,
        );
        bases.orient(&mut once);
        let mut twice = once.clone();
        assert_eq!(bases.orient(&mut twice), Orientation::Kept);
        assert_eq!(once, twice);
    }

    #[test]
    fn a_pair_the_corpus_says_nothing_about_is_not_guessed_at() {
        let before = edge("some-sefer", "another-sefer", EdgeType::CommentsOn);
        let mut after = before.clone();
        assert_eq!(bases().orient(&mut after), Orientation::Undeclared);
        assert_eq!(before, after, "an undeclared pair must be left alone");
    }

    #[test]
    fn a_reference_is_not_reoriented_even_between_a_commentary_and_its_base() {
        // `references` between these two works is an ordinary citation and its
        // direction is the row's own claim. Only `comments-on` has an outside
        // fact to be checked against.
        let before = edge(
            "bavli/berakhot",
            "bavli/rashi-on-berakhot",
            EdgeType::References,
        );
        let mut after = before.clone();
        assert_eq!(bases().orient(&mut after), Orientation::NotCommentary);
        assert_eq!(before, after);
    }

    #[test]
    fn the_tally_never_counts_a_reference_as_an_edge_it_checked() {
        // The first version of this reported `references` as "already right",
        // so a graph where a quarter of the commentary was backwards printed a
        // 96% success rate. The counts are separate so that cannot recur.
        let mut t = Tally::default();
        for _ in 0..97 {
            t.count(Orientation::NotCommentary);
        }
        t.count(Orientation::Kept);
        t.count(Orientation::Flipped);
        t.count(Orientation::Undeclared);
        assert_eq!(
            t.kept, 1,
            "a reference is not a correctly oriented commentary"
        );
        assert_eq!(t.not_commentary, 97);
        let said = t.said();
        assert!(said.contains("not commentary"), "{said}");
        assert!(
            said.contains("left alone"),
            "the undeclared edges must be reported, not folded into kept: {said}"
        );
    }
}
