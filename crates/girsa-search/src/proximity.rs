//! Proximity with the per-gap distance the reader asked for.
//!
//! # Why this is not a tantivy phrase query
//!
//! `Together::Near { words }` promises *these words within `words` other words
//! of each other*: for a fixed order of the query words, each consecutive pair
//! may have at most `words` words between it. Tantivy's `PhraseQuery` slop is a
//! different quantity — its own doc says *"the slop can be considered a budget
//! between all terms"*, so `"A B C" with slop 1` matches `A X B C` and
//! `A B X C` but not `A X B X C`, where each gap is 1 and the *sum* is 2.
//!
//! Feeding the per-gap number into that budget was pure recall loss in the
//! wrong direction for the mode whose whole pitch is *asked for exactly*: three
//! words each two apart cost 2+2=4 against a slop of 2 and were silently
//! missed, while widening the budget to the sum lets one pair blow the bound
//! while another sits tight — a false positive. Neither shape is the promise.
//!
//! This module implements the promise. [`OrderedProximity`] is a tantivy
//! `Query` over a fixed sequence of slots (each an exact term or a regex over
//! terms); its scorer keeps the position list of every slot and asks, per
//! document, whether some choice of positions — one per slot, in order,
//! strictly increasing — keeps every consecutive step within `gap` words.
//! Order-free proximity is then the existing union over orderings, each
//! ordering asked through this query.

use tantivy::postings::{Postings, SegmentPostings, TermInfo};
use tantivy::query::{
    AutomatonWeight, EmptyScorer, EnableScoring, Explanation, Query, Scorer, Weight,
};
use tantivy::schema::{Field, IndexRecordOption, Term};
use tantivy::{DocId, DocSet, Score, SegmentReader, TERMINATED};

/// One position of the sequence, and what may answer it.
#[derive(Debug, Clone)]
pub enum Slot {
    /// One exact indexed term.
    Term(Term),
    /// Every indexed term this pattern matches, over whole terms.
    Regex { pattern: String },
}

/// The per-gap proximity query over a fixed sequence of slots.
///
/// `gap` is the number of **other words** allowed between two consecutive
/// query words, so two positions `p < q` of consecutive slots agree when
/// `q - p <= gap + 1`.
#[derive(Debug, Clone)]
pub struct OrderedProximity {
    field: Field,
    slots: Vec<Slot>,
    gap: u32,
    /// The ceiling on how many indexed terms a regex slot may expand to.
    /// Enforced at weight build, like tantivy's own regex-phrase ceiling, so a
    /// `contains`/`letters` pattern that would match too much is refused with
    /// a name rather than half-expanded.
    max_expansions: u32,
}

impl OrderedProximity {
    /// A proximity query over `slots`, in the order they must appear, with at
    /// most `gap` other words between consecutive slots.
    #[must_use]
    pub fn new(field: Field, slots: Vec<Slot>, gap: u32, max_expansions: u32) -> Self {
        Self {
            field,
            slots,
            gap,
            max_expansions,
        }
    }
}

impl Query for OrderedProximity {
    fn weight(&self, _enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        Ok(Box::new(ProximityWeight {
            field: self.field,
            slots: self.slots.clone(),
            gap: self.gap,
            max_expansions: self.max_expansions,
        }))
    }

    fn query_terms<'a>(&'a self, visitor: &mut dyn FnMut(&'a Term, bool)) {
        // Exact slots name their term; a regex slot names nothing until it is
        // expanded against a segment. Positions are required either way.
        for slot in &self.slots {
            if let Slot::Term(term) = slot {
                visitor(term, true);
            }
        }
    }
}

/// The weight of an [`OrderedProximity`] against one index.
struct ProximityWeight {
    field: Field,
    slots: Vec<Slot>,
    gap: u32,
    max_expansions: u32,
}

impl Weight for ProximityWeight {
    fn scorer(&self, reader: &SegmentReader, _boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        let inverted = reader.inverted_index(self.field)?;
        let mut per_slot = Vec::with_capacity(self.slots.len());
        let mut expansions = 0u32;
        for slot in &self.slots {
            match slot {
                Slot::Term(term) => {
                    // Positions are the whole question here, so the postings
                    // have to carry them — `WithFreqs` alone returns none and
                    // the distance walk would see every document as empty.
                    let Some(postings) =
                        inverted.read_postings(term, IndexRecordOption::WithFreqsAndPositions)?
                    else {
                        // The term is absent from this segment; the sequence
                        // can never be found here.
                        return Ok(Box::new(EmptyScorer));
                    };
                    per_slot.push(SlotPostings::one(postings));
                }
                Slot::Regex { pattern } => {
                    let automaton: AutomatonWeight<tantivy_fst::Regex> = AutomatonWeight::new(
                        self.field,
                        tantivy_fst::Regex::new(pattern).map_err(|e| {
                            tantivy::TantivyError::InvalidArgument(format!("Invalid regex: {e}"))
                        })?,
                    );
                    let infos: Vec<TermInfo> = automaton.get_match_term_infos(reader)?;
                    // The same refusal tantivy's regex-phrase ceiling makes,
                    // with the same words in it, so the caller's mapper turns
                    // it into the same named `TooBroad` it always has.
                    expansions = expansions.saturating_add(infos.len() as u32);
                    if expansions > self.max_expansions {
                        return Err(tantivy::TantivyError::InvalidArgument(format!(
                            "Phrase query exceeded max expansions {expansions}"
                        )));
                    }
                    if infos.is_empty() {
                        // No term matches the pattern in this segment; the
                        // sequence can never be found here.
                        return Ok(Box::new(EmptyScorer));
                    }
                    let mut postings = Vec::with_capacity(infos.len());
                    for info in &infos {
                        let p = inverted.read_postings_from_terminfo(
                            info,
                            IndexRecordOption::WithFreqsAndPositions,
                        )?;
                        postings.push(p);
                    }
                    per_slot.push(SlotPostings::many(postings));
                }
            }
        }
        Ok(Box::new(ProximityScorer::new(per_slot, self.gap)))
    }

    fn explain(&self, _reader: &SegmentReader, _doc: DocId) -> tantivy::Result<Explanation> {
        // Existence over a distance bound: there are no term weights to break
        // the score down with, because the score is whether the positions fit.
        Ok(Explanation::new("ordered proximity, per-gap", 1.0))
    }
}

/// One slot's postings: a single term, or the union of every term a regex
/// matched. The union is kept as a list rather than as a merged object so
/// positions can be collected per term and merged at the document — the same
/// construction tantivy's regex-phrase weight uses.
enum SlotPostings {
    // `SegmentPostings` itself is the largest thing here (the postings reader
    // holds its buffer inline), so it is boxed — the enum is stored per slot
    // per scorer, and the box is what keeps every variant the same size.
    One(Box<SegmentPostings>),
    // A boxed slice: a regex slot's union can hold many term postings.
    Many(Box<[SegmentPostings]>),
}

impl SlotPostings {
    fn one(postings: SegmentPostings) -> Self {
        Self::One(Box::new(postings))
    }

    fn many(postings: Vec<SegmentPostings>) -> Self {
        Self::Many(postings.into_boxed_slice())
    }

    fn doc(&self) -> DocId {
        match self {
            Self::One(p) => p.doc(),
            // Union: the smallest document any sub-list is standing on.
            Self::Many(ps) => ps
                .iter()
                .map(SegmentPostings::doc)
                .min()
                .unwrap_or(TERMINATED),
        }
    }

    fn seek(&mut self, target: DocId) -> DocId {
        match self {
            Self::One(p) => p.seek(target),
            Self::Many(ps) => {
                for p in ps.iter_mut() {
                    if p.doc() < target {
                        p.seek(target);
                    }
                }
                self.doc()
            }
        }
    }

    /// Advance past the current document.
    fn advance(&mut self) -> DocId {
        match self {
            Self::One(p) => p.advance(),
            Self::Many(ps) => {
                let at = ps
                    .iter()
                    .map(SegmentPostings::doc)
                    .min()
                    .unwrap_or(TERMINATED);
                if at != TERMINATED {
                    for p in ps.iter_mut() {
                        if p.doc() == at {
                            p.advance();
                        }
                    }
                }
                ps.iter()
                    .map(SegmentPostings::doc)
                    .min()
                    .unwrap_or(TERMINATED)
            }
        }
    }

    fn size_hint(&self) -> u32 {
        match self {
            Self::One(p) => p.size_hint(),
            Self::Many(ps) => ps.iter().map(SegmentPostings::size_hint).sum(),
        }
    }

    /// The positions of this slot in the current document, sorted and
    /// de-duplicated.
    fn positions(&mut self, output: &mut Vec<u32>) {
        match self {
            Self::One(p) => p.positions(output),
            Self::Many(ps) => {
                let mut merged = Vec::new();
                for p in ps.iter_mut() {
                    let mut here = Vec::new();
                    p.positions(&mut here);
                    merged.append(&mut here);
                }
                merged.sort_unstable();
                merged.dedup();
                *output = merged;
            }
        }
    }
}

/// The scorer: documents where every slot is present, checked for a position
/// choice whose consecutive steps respect the per-gap bound.
struct ProximityScorer {
    slots: Vec<SlotPostings>,
    /// Consecutive positions must differ by at most `gap + 1`.
    step: u32,
    doc: DocId,
    /// Per-slot position lists of the current document.
    per_slot: Vec<Vec<u32>>,
}

impl ProximityScorer {
    fn new(slots: Vec<SlotPostings>, gap: u32) -> Self {
        let per_slot = vec![Vec::new(); slots.len()];
        let mut scorer = Self {
            slots,
            step: gap.saturating_add(1),
            doc: TERMINATED,
            per_slot,
        };
        // A scorer is handed out **positioned**: the collector's first call is
        // `seek(0)` or `advance()`, and the default `DocSet::seek` starts from
        // `self.doc` — which is `TERMINATED` for a scorer that never moved,
        // making the first seek return the sentinel and the query answer
        // nothing. Positioning here is what tantivy's own phrase scorer does.
        scorer.advance();
        scorer
    }

    /// Whether some strictly increasing choice of positions, one per slot in
    /// order, keeps every consecutive step within the bound.
    fn matches(&mut self) -> bool {
        for (slot, buffer) in self.slots.iter_mut().zip(self.per_slot.iter_mut()) {
            slot.positions(buffer);
        }
        has_per_gap_choice(&self.per_slot, self.step)
    }
}

/// Whether some strictly increasing choice of positions — one from each list,
/// in list order — keeps every consecutive step within `step` positions.
///
/// The walk keeps, per slot, every position that can end a valid prefix. A
/// position `q` of the next slot joins when some reachable `p` has
/// `p < q <= p + step`. Both lists are sorted, so the test per `q` is one
/// partition point.
///
/// Split out from the scorer so the distance logic can be tested against
/// hand-built position lists, without an index in the way.
fn has_per_gap_choice(per_slot: &[Vec<u32>], step: u32) -> bool {
    let Some(first) = per_slot.first() else {
        return false;
    };
    let mut reachable: Vec<u32> = first.clone();
    reachable.sort_unstable();
    reachable.dedup();
    for positions in per_slot.iter().skip(1) {
        let mut next = Vec::new();
        for &q in positions {
            // `p` qualifies when `q - step <= p < q`.
            let lo = q.saturating_sub(step);
            let start = reachable.partition_point(|&p| p < lo);
            if reachable[start..].first().is_some_and(|&p| p < q) {
                next.push(q);
            }
        }
        if next.is_empty() {
            return false;
        }
        next.sort_unstable();
        next.dedup();
        reachable = next;
    }
    true
}

impl DocSet for ProximityScorer {
    fn advance(&mut self) -> DocId {
        // A k-way intersection: bring every slot to the greatest current
        // document, and when they agree, ask the distance question.
        //
        // `TERMINATED` is a *sentinel*, not a document — a slot that has run
        // out can never meet the others again, so the whole intersection is
        // over the moment any one of them reports it.
        //
        // First, past the document this scorer last yielded. The slots are
        // still *at* it — they converged there to be checked — so a scorer
        // that returned `d` and is asked to advance again must not find `d`
        // a second time. `TERMINATED` is the "before the first document"
        // state the constructor advances from, so it is skipped.
        if self.doc != TERMINATED {
            for slot in &mut self.slots {
                if slot.doc() == self.doc {
                    slot.advance();
                } else if slot.doc() < self.doc {
                    slot.seek(self.doc + 1);
                }
            }
        }
        loop {
            if self.slots.iter().any(|slot| slot.doc() == TERMINATED) {
                self.doc = TERMINATED;
                return TERMINATED;
            }
            let max = self
                .slots
                .iter()
                .map(SlotPostings::doc)
                .max()
                .unwrap_or(TERMINATED);
            let mut converged = true;
            for slot in &mut self.slots {
                if slot.doc() < max {
                    slot.seek(max);
                }
                if slot.doc() != max {
                    converged = false;
                }
            }
            if converged && self.matches() {
                self.doc = max;
                return max;
            }
            for slot in &mut self.slots {
                if slot.doc() == max {
                    slot.advance();
                }
            }
        }
    }

    fn doc(&self) -> DocId {
        self.doc
    }

    fn size_hint(&self) -> u32 {
        self.slots
            .iter()
            .map(SlotPostings::size_hint)
            .min()
            .unwrap_or(0)
    }
}

impl Scorer for ProximityScorer {
    fn score(&mut self) -> Score {
        // Existence, and the number of orderings that found it is what the
        // Boolean union above this combines.
        1.0
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn slots(lists: &[&[u32]]) -> Vec<Vec<u32>> {
        lists.iter().map(|l| l.to_vec()).collect()
    }

    /// The audit's exact case: three words each two apart, `Near{2}`. The
    /// step bound is 3 positions; each pair is exactly 3 apart, so a choice
    /// exists. Tantivy's summed budget said no.
    #[test]
    fn three_words_two_apart_each_is_within_two() {
        let per_slot = slots(&[&[0], &[3], &[6]]);
        assert!(
            has_per_gap_choice(&per_slot, 3),
            "each pair 2 words between"
        );
    }

    /// The other direction: each pair three apart, `Near{2}` must refuse.
    #[test]
    fn three_words_three_apart_each_is_outside_two() {
        let per_slot = slots(&[&[0], &[4], &[8]]);
        assert!(
            !has_per_gap_choice(&per_slot, 3),
            "each pair 3 words between"
        );
    }

    /// The summed-budget false positive: one tight pair and one blow-out. A
    /// total span of 4 would pass `slop 4`; per-gap, the second pair has 3
    /// words between and must refuse `Near{2}`.
    #[test]
    fn one_tight_pair_does_not_absolve_a_blowout_pair() {
        let per_slot = slots(&[&[0], &[1], &[6]]);
        assert!(!has_per_gap_choice(&per_slot, 3));
    }

    /// Greedy-smallsest is not enough: taking the first feasible position of
    /// the middle word blocks the end, while taking the later one succeeds.
    #[test]
    fn a_later_middle_position_can_be_the_only_way() {
        let per_slot = slots(&[&[0], &[1, 3], &[5]]);
        assert!(has_per_gap_choice(&per_slot, 3));
        assert!(!has_per_gap_choice(&per_slot, 2));
    }

    /// Adjacent words are always within any bound, and a repeated term needs
    /// two distinct positions — one per slot.
    #[test]
    fn a_repeated_word_uses_two_positions() {
        let per_slot = slots(&[&[2], &[2, 5]]);
        assert!(has_per_gap_choice(&per_slot, 3), "2 and 5 are 3 apart");
        let only_one = slots(&[&[2], &[2]]);
        assert!(
            !has_per_gap_choice(&only_one, 3),
            "the same position cannot answer both slots"
        );
    }

    /// Positions do not have to be aligned across slots; the walk finds any
    /// increasing choice, not just the first ones.
    #[test]
    fn an_early_position_can_be_skipped_for_a_later_fit() {
        let per_slot = slots(&[&[0, 7], &[3], &[6]]);
        assert!(has_per_gap_choice(&per_slot, 3), "0, 3, 6");
    }
}
