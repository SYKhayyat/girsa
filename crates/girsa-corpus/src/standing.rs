//! The place a reader is standing on, under every name those words have carried.
//!
//! # The question, and the two answers it had
//!
//! Everything anchored — a link, a note, a highlight, a folder membership — is
//! written down as a [`SegmentId`] and read back by asking *does this anchor
//! name the words I am looking at*. That question had two implementations:
//!
//! - [`SegmentId::covers`], a prefix test on the ordinal, used by the links
//!   panel, notes, marks, folders and the repair layer;
//! - `Open::covered_by` in `girsa-app`, which reads `redirects.jsonl`.
//!
//! Two answers to one question is one answer too many, and the cheap one was
//! wrong in both directions.
//!
//! # A dotted name means two opposite things
//!
//! [`Ordinal::child`] has two callers. The oversized cutter (B12) carves `#7`
//! into `#7.1` and `#7.2`; [`crate::import::continuity::mint_between`] names a
//! se'if upstream **inserted** after `#7`, and the only name that sorts between
//! `#7` and `#8` is also `#7.1`. One is `#7`'s own words subdivided. The other
//! is words `#7` has never contained. A prefix test says yes to both, so every
//! comment ever written on se'if 7 would appear on a se'if that did not exist
//! when they were written — a connection Girsa asserts and nobody made, which is
//! BUILDER.md rule 6 with the sign flipped.
//!
//! **A cut deletes its parent and an insertion does not.** `import::assemble`
//! says so where it does it — *"The parent id is not written to disk: it is not
//! a segment any more"* — and [`crate::store::SegmentStore::split`] removes it
//! too. `mint_between` is handed a `low` that kept its name and is still on the
//! shelf, and `Note::insert_after` leaves the paragraph above it exactly where
//! it was. So the shelf already knows which event minted a name, and it needs no
//! new file to say it:
//!
//! > An ancestor names a descendant's words only if the ancestor is **not itself
//! > live**. Walk up, and stop at the first name still on the shelf.
//!
//! Stopping matters as much as walking. `#7` cut into `#7.1` and `#7.2`, then a
//! se'if inserted after `#7.2`, gives `#7.2.1`: its parent `#7.2` is live, so the
//! walk stops there and `#7` does not reach it either — correct, because those
//! words were never in `#7`.
//!
//! # And the half ancestry cannot express
//!
//! Upstream merges se'if 3 into se'if 2. The importer records `#3 → #2` in
//! `redirects.jsonl`, and no amount of care about descent will find it: `#3` is
//! not an ancestor of `#2` and never was. That half is what the redirect table
//! is for, walked here in reverse — *which dead names lead to where I am* —
//! rather than forward the way `Open::covered_by` walks it to find text.
//!
//! Both halves land in one set of names, and one membership test over it.

use std::collections::BTreeSet;

use crate::segment::SegmentId;

/// A redirect chain longer than this is a cycle somebody built by hand.
///
/// The same cap as [`crate::store::SegmentStore`] and `Open::redirected`, for
/// the same reason: a hand-edited overlay that built a loop should cost the
/// reader a short answer, not a frozen window.
const MAX_DEPTH: usize = 32;

/// A place, and every name whose anchors name its words.
///
/// Built once per question and asked many times — the links panel tests it
/// against every edge in a shard, of which the Shulchan Arukh's holds 156,076 —
/// so the walking happens here and the asking is a set lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    at: SegmentId,
    /// `at`, its inherited ancestors, and the dead names redirected here.
    ///
    /// A `BTreeSet` and deliberately not a `HashSet`: [`SegmentId`]'s `Ord` is
    /// work then **ordinal**, ignoring the section path, and its `Hash` is not.
    /// An anchor written down before upstream re-sectioned a work carries the
    /// old address with the same durable ordinal, and matching it is the point —
    /// *"the ordinal is the durable name; the ref is the human address, and the
    /// two are deliberately different things"* ([`SegmentId::path`]). A hashed
    /// set would miss it.
    names: BTreeSet<SegmentId>,
}

impl Standing {
    /// A place that names only itself.
    ///
    /// Honest when there is no shelf to ask — nothing has been said about this
    /// id's history, so nothing is assumed about it. Not the same as a place
    /// whose history is known to be empty, but indistinguishable to a caller,
    /// which is why [`Standing::of`] exists and takes the evidence.
    #[must_use]
    pub fn just(at: SegmentId) -> Self {
        let names = [at.clone()].into();
        Self { at, names }
    }

    /// A place and the names it is known to have inherited.
    ///
    /// For a caller holding the history already. The general derivation is
    /// `Open::standing` in `girsa-app`, which is the one place that has both the
    /// live set and the redirect table.
    #[must_use]
    pub fn of(at: SegmentId, inherited: impl IntoIterator<Item = SegmentId>) -> Self {
        let mut names: BTreeSet<SegmentId> = inherited.into_iter().collect();
        names.insert(at.clone());
        Self { at, names }
    }

    /// Derive the names from the shelf itself.
    ///
    /// `live` answers *is this id a segment on disk right now*, and
    /// `redirected_here` answers *which dead names point at this one* — the
    /// reverse of `redirects.jsonl`. The two walks interleave: a name reached by
    /// a redirect has ancestors of its own, and an inherited ancestor may have
    /// been redirected at from somewhere else again.
    #[must_use]
    pub fn derived(
        at: &SegmentId,
        live: impl Fn(&SegmentId) -> bool,
        redirected_here: impl Fn(&SegmentId) -> Vec<SegmentId>,
    ) -> Self {
        let mut names = BTreeSet::new();
        names.insert(at.clone());
        let mut queue = vec![(at.clone(), 0usize)];

        while let Some((name, depth)) = queue.pop() {
            if depth >= MAX_DEPTH {
                continue;
            }
            // Upward, stopping at the first name still on the shelf: a cut
            // deletes its parent, so an ancestor that is still a segment is a
            // *neighbour* this one was inserted beside, not a forebear it was
            // carved out of. See the module note.
            let mut rung = name.parent();
            while let Some(up) = rung {
                if live(&up) {
                    break;
                }
                rung = up.parent();
                if names.insert(up.clone()) {
                    queue.push((up, depth + 1));
                }
            }
            // …and backwards along the redirects, for the names that were moved
            // here rather than carved out of something.
            for source in redirected_here(&name) {
                if names.insert(source.clone()) {
                    queue.push((source, depth + 1));
                }
            }
        }

        Self {
            at: at.clone(),
            names,
        }
    }

    /// The live id — where the reader actually is.
    #[must_use]
    pub fn at(&self) -> &SegmentId {
        &self.at
    }

    /// Whether an anchor written down at any time names these words.
    ///
    /// The whole point of the type: a set lookup, with every judgement about
    /// what descends from what already made.
    #[must_use]
    pub fn named_by(&self, anchor: &SegmentId) -> bool {
        self.names.contains(anchor)
    }

    /// Every name these words have carried, in ordinal order.
    ///
    /// For a caller that has to test a *range* rather than a point — a run
    /// anchor covers what sorts between its ends, and any of these names may be
    /// the one that falls inside it.
    pub fn names(&self) -> impl Iterator<Item = &SegmentId> {
        self.names.iter()
    }

    /// How many names this place answers to. One, for a corpus nothing has
    /// re-segmented — which is every work on the shelf today.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::segment::Ordinal;

    fn id(ordinal: Ordinal) -> SegmentId {
        SegmentId::new("shulchan-arukh/orach-chayim", vec!["1".into()], ordinal)
    }

    fn seif(n: u32) -> SegmentId {
        id(Ordinal::root(n))
    }

    /// The shelf as a list of what is live, and no redirects at all — which is
    /// the whole corpus as it stands.
    fn shelf(live: &[SegmentId], at: &SegmentId) -> Standing {
        let live: BTreeSet<SegmentId> = live.iter().cloned().collect();
        Standing::derived(at, |id| live.contains(id), |_| Vec::new())
    }

    #[test]
    fn a_cut_childs_parent_still_names_it() {
        // `#7` was too long to be a place and was carved into two. The parent is
        // gone from disk, so an anchor on it is an anchor on these words.
        let pieces = seif(7).split(2);
        let standing = shelf(&[seif(6), pieces[0].clone(), pieces[1].clone()], &pieces[1]);
        assert!(
            standing.named_by(&seif(7)),
            "the cut parent names its piece"
        );
        assert!(standing.named_by(&pieces[1]));
        assert!(
            !standing.named_by(&pieces[0]),
            "a sibling is not this place"
        );
        assert!(!standing.named_by(&seif(6)));
    }

    #[test]
    fn an_inserted_seifs_neighbour_does_not_name_it() {
        // Upstream added a se'if after 7. `mint_between` calls it `#7.1` because
        // that is the only name that sorts between `#7` and `#8` — and `#7` kept
        // its own name and its own words. Prefix says yes; the shelf says no.
        let inserted = seif(7).split(2).remove(0);
        assert!(
            seif(7).covers(&inserted),
            "descent holds — which is exactly why it is not the test"
        );
        let standing = shelf(&[seif(7), inserted.clone(), seif(8)], &inserted);
        assert!(
            !standing.named_by(&seif(7)),
            "se'if 7 is still on the shelf, so it was not carved into this"
        );
        assert!(standing.named_by(&inserted));
    }

    #[test]
    fn the_walk_stops_at_the_first_live_ancestor() {
        // `#7` cut into `#7.1` and `#7.2`; then upstream inserted a se'if after
        // `#7.2`, which is named `#7.2.1`. Its words were never in `#7`, so
        // neither `#7.2` (live, so not a cut) nor `#7` (unreachable past it)
        // names them.
        let pieces = seif(7).split(2);
        let inserted = pieces[1].split(2).remove(0);
        let standing = shelf(
            &[pieces[0].clone(), pieces[1].clone(), inserted.clone()],
            &inserted,
        );
        assert!(!standing.named_by(&pieces[1]), "its parent is still live");
        assert!(
            !standing.named_by(&seif(7)),
            "and the walk does not step over a live name to reach a dead one"
        );
        assert_eq!(standing.len(), 1);
    }

    #[test]
    fn a_cut_of_a_cut_inherits_both_names() {
        // `#7` → `#7.1`, `#7.2`; then `#7.1` → `#7.1.1`, `#7.1.2`. Neither
        // intermediate name is on disk, so both name these words.
        let pieces = seif(7).split(2);
        let deeper = pieces[0].split(2);
        let standing = shelf(
            &[deeper[0].clone(), deeper[1].clone(), pieces[1].clone()],
            &deeper[1],
        );
        assert!(standing.named_by(&seif(7)));
        assert!(standing.named_by(&pieces[0]));
        assert!(standing.named_by(&deeper[1]));
        assert!(!standing.named_by(&pieces[1]));
    }

    #[test]
    fn a_merged_seif_is_reached_backwards_along_the_redirect() {
        // Upstream folded se'if 3 into se'if 2. `#3` is not an ancestor of `#2`
        // and no amount of care about descent will find it — this is the half
        // the table exists for.
        let live: BTreeSet<SegmentId> = [seif(1), seif(2)].into();
        let standing = Standing::derived(
            &seif(2),
            |id| live.contains(id),
            |id| {
                if *id == seif(2) {
                    vec![seif(3)]
                } else {
                    Vec::new()
                }
            },
        );
        assert!(standing.named_by(&seif(3)), "an anchor on 3 reaches 2");
        assert!(standing.named_by(&seif(2)));
        assert!(!standing.named_by(&seif(1)));
    }

    #[test]
    fn a_redirect_chain_is_followed_and_a_cycle_does_not_hang() {
        // Two corpus updates moved the same words twice, so an anchor from
        // before both still resolves. The cycle is what a hand-edited file does.
        let live: BTreeSet<SegmentId> = [seif(1)].into();
        let standing = Standing::derived(
            &seif(1),
            |id| live.contains(id),
            |id| match id.ordinal().at(0) {
                Some(1) => vec![seif(2)],
                Some(2) => vec![seif(3)],
                Some(3) => vec![seif(1)],
                _ => Vec::new(),
            },
        );
        assert!(standing.named_by(&seif(2)));
        assert!(standing.named_by(&seif(3)));
        assert_eq!(standing.len(), 3, "and it stopped rather than spinning");
    }

    #[test]
    fn a_name_from_another_sefer_never_names_this_place() {
        let standing = shelf(&[seif(7)], &seif(7));
        let elsewhere = SegmentId::new("bavli/berakhot", vec!["1".into()], Ordinal::root(7));
        assert!(!standing.named_by(&elsewhere));
    }
}
