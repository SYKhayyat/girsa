//! Keeping a segment's permanent name across a **second** import.
//!
//! # The hole this closes
//!
//! spec.md §3 calls permanent ids *"the single most important decision in this
//! document"* and *"close to impossible to retrofit"*, and the reason is T1: an
//! id derived from position means that inserting one line silently re-points
//! every anchor below it. [`crate::segment`] takes that seriously *within* a
//! run — the ordinal is minted once, in reading order, and a split extends it
//! rather than renumbering siblings.
//!
//! It was not taken seriously **across** runs. `SegmentStore::import` handed out
//! `Ordinal::root(i + 1)` from enumeration position and `import::write` was an
//! unconditional overwrite, so re-running `girsa-import` after Sefaria added one
//! se'if to siman 1 of Orach Chayim renumbered 4,170 segments by one. That is T1
//! verbatim, at import granularity instead of line granularity, and the tool it
//! happens in is called `girsa-import`. The redirect table three doc comments
//! promised had no slot on disk to live in.
//!
//! # What a name is matched on, and why it is the text
//!
//! An anchor names **words**. So the evidence that two records are the same
//! place is that they say the same thing, not that they sit at the same address:
//!
//! - Upstream inserts a se'if at 1:3. Every later se'if's *address* shifts by
//!   one and its *text* does not. Matching on the address would hand old `#3`'s
//!   name to the new se'if's words — the defect, one level up. Matching on the
//!   text keeps all 4,170 names and mints one.
//! - Upstream re-sections a whole work. Every address changes and no text does.
//!   Matching on the text keeps every name, which is precisely the case §3 was
//!   written about.
//! - Upstream fixes a typo. That one segment's text changes and its neighbours
//!   do not, so the neighbours anchor it and the **address** decides it inside
//!   that gap. A corrected word must not cost a segment its name.
//!
//! **And the evidence is compared in the same normal form the search index is
//! keyed on.** Every matcher here runs through [`girsa_hebrew::normalize`]:
//! nikud and te'amim are stripped, final letters folded, maqaf and punctuation
//! made into spaces. Upstream adding nikud to a se'if, or re-spelling its first
//! word with a maqaf, changes the printed bytes and leaves the words alone — so
//! a raw-byte comparison would orphan every citation to it to [`super::Why::Gone`],
//! the exact event this machinery exists to survive. The sibling search crate
//! has always normalized every token on the way in; the matcher that guards the
//! permanent names used to compare bytes, which was one answer for the two
//! halves of the same question.
//!
//! So: unique texts anchor the alignment, the longest increasing run of those
//! anchors is kept so the matching cannot cross itself, and addresses settle
//! what is left inside each gap. What still has no partner is reported rather
//! than guessed at — a place upstream no longer has becomes a redirect row, and
//! a place upstream has newly gained gets a name minted **between** its
//! neighbours, which the dotted ordinal can always express.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::segment::Ordinal;

/// One place as a previous run left it, before the oversized cutter touched it.
///
/// Reassembled from disk: a segment that was cut into `#32.1 #32.2 #32.3` is
/// one place here, with the three pieces concatenated back into the text the
/// importer actually judged. Matching against the pieces instead would compare
/// a whole se'if to a third of one and match nothing.
#[derive(Debug, Clone)]
pub struct Place {
    pub ordinal: Ordinal,
    pub path: Vec<String>,
    pub kind: super::SegmentKind,
    pub text: String,
    /// The live ids this place is on disk as — itself, or its cut children.
    /// What a redirect away from it has to point at.
    pub ids: Vec<crate::segment::SegmentId>,
}

/// A place in the run being assembled now, mined and ready to be named.
///
/// Borrowed, not owned. The importer runs sixteen threads over works whose text
/// runs to tens of megabytes, and a third copy of a work's words to hold while
/// deciding what to call them is a third copy per thread.
#[derive(Debug, Clone, Copy)]
pub struct Fresh<'a> {
    pub path: &'a [String],
    pub kind: super::SegmentKind,
    pub text: &'a str,
}

/// How many of a work's places kept the name they had.
///
/// Reported the way [`crate::oversized::Tally`] is reported, and for the same
/// reason: a number nobody prints is a number nobody knows, and this one decides
/// whether every Ksav document written against the shelf still opens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Continuity {
    /// Places that were on the shelf before and kept their permanent id.
    pub kept: usize,
    /// Places upstream has now and did not have before. A fresh id, minted
    /// between its neighbours so nothing around it moves.
    pub minted: usize,
    /// Places upstream no longer has, whose words were found somewhere else in
    /// the new text. A redirect row; anchors on them still resolve.
    pub resegmented: usize,
    /// Places upstream no longer has at all. A redirect row with nowhere to
    /// point, which is the honest answer and is **not** the same as an id that
    /// was never minted — see [`super::Why::Gone`].
    pub gone: usize,
    /// Works that were re-imported over an existing shelf at all.
    works: BTreeSet<String>,
}

impl Continuity {
    /// Note that this work had a previous run to keep faith with.
    pub fn over(&mut self, work: &str) {
        self.works.insert(work.to_string());
    }

    pub fn absorb(&mut self, other: &Self) {
        self.kept += other.kept;
        self.minted += other.minted;
        self.resegmented += other.resegmented;
        self.gone += other.gone;
        self.works.extend(other.works.iter().cloned());
    }

    #[must_use]
    pub fn works(&self) -> usize {
        self.works.len()
    }

    /// True when nothing on the shelf was imported over — a first import.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.works.is_empty()
    }

    /// The lines a report prints, or nothing at all when this was a first
    /// import and there was no name to keep.
    ///
    /// One implementation, so the importer's report and a test's expectation
    /// cannot disagree about a count.
    #[must_use]
    pub fn said(&self) -> Vec<String> {
        if self.is_empty() {
            return Vec::new();
        }
        let mut out = vec![
            format!(
                "  re-imported        {} works already on the shelf",
                self.works()
            ),
            format!("  kept their id      {}", self.kept),
            format!(
                "  newly minted       {} (between their neighbours; nothing moved)",
                self.minted
            ),
        ];
        if self.resegmented > 0 {
            out.push(format!(
                "  re-segmented       {} — redirected to where the words went",
                self.resegmented
            ));
        }
        if self.gone > 0 {
            out.push(format!(
                "  no longer upstream {} — redirected to nothing, which is said rather than guessed",
                self.gone
            ));
        }
        out
    }
}

/// What the alignment decided, place by place, for the run being assembled.
///
/// `for_fresh[i]` is the previous place that `fresh[i]` is a continuation of.
/// `orphaned` is every previous place nothing continued.
#[derive(Debug, Clone)]
pub struct Alignment {
    pub for_fresh: Vec<Option<usize>>,
    pub orphaned: Vec<usize>,
}

/// Work out which of the new places are which of the old ones.
///
/// Three passes, strongest evidence first — see the module docs. The result is
/// **monotonic** by construction: if `fresh[i]` matches `previous[a]` and
/// `fresh[j]` matches `previous[b]` with `i < j`, then `a < b`. A matching that
/// crossed itself would hand two names to one set of words.
#[must_use]
pub fn align(previous: &[Place], fresh: &[Fresh<'_>]) -> Alignment {
    let mut for_fresh: Vec<Option<usize>> = vec![None; fresh.len()];
    if previous.is_empty() || fresh.is_empty() {
        return Alignment {
            for_fresh,
            orphaned: (0..previous.len()).collect(),
        };
    }

    // Pass 1 — texts that appear exactly once on each side. A text that appears
    // twice says nothing about which of the two this is, so it does not get to
    // anchor anything; it is left for pass 3, which has the surrounding
    // agreement to lean on.
    let old_once = once_by_text(previous.iter().map(|p| p.text.as_str()));
    let new_once = once_by_text(fresh.iter().map(|f| f.text));
    let mut anchors: Vec<(usize, usize)> = Vec::new();
    for (text, at) in &new_once {
        if let Some(was) = old_once.get(text) {
            anchors.push((*at, *was));
        }
    }
    anchors.sort_unstable();

    // Pass 2 — keep the longest run of those anchors that goes forward on both
    // sides. Upstream moving one paragraph to the end of a sefer should cost
    // that paragraph its match, not cost every paragraph after it one.
    for (at, was) in longest_increasing(&anchors) {
        for_fresh[at] = Some(was);
    }

    // Pass 3 — inside each gap between two kept anchors, the addresses decide.
    // A gap is short by construction, and both sides of it are pinned, so an
    // address here is evidence about a place rather than a position in a file.
    let mut gaps: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut last = (0usize, 0usize);
    for (at, was) in for_fresh
        .iter()
        .enumerate()
        .filter_map(|(at, was)| was.map(|w| (at, w)))
        .collect::<Vec<_>>()
    {
        gaps.push((last.0, at, last.1, was));
        last = (at + 1, was + 1);
    }
    gaps.push((last.0, fresh.len(), last.1, previous.len()));

    for (fresh_from, fresh_to, old_from, old_to) in gaps {
        if fresh_from >= fresh_to || old_from >= old_to {
            continue;
        }
        let mut by_address: HashMap<(&[String], super::SegmentKind), Vec<usize>> = HashMap::new();
        for (was, place) in previous.iter().enumerate().take(old_to).skip(old_from) {
            by_address
                .entry((place.path.as_slice(), place.kind))
                .or_default()
                .push(was);
        }
        let mut taken: BTreeSet<usize> = BTreeSet::new();
        for at in fresh_from..fresh_to {
            let key = (fresh[at].path, fresh[at].kind);
            let Some(candidates) = by_address.get(&key) else {
                continue;
            };
            // Only when the address names exactly one place in this gap. Two
            // would be a choice, and BUILDER rule 6 says a choice is not a
            // guess to be made silently.
            if candidates.len() != 1 {
                continue;
            }
            let was = candidates[0];
            // And only when the words agree that this is the same place. An
            // address on its own is not enough and the failure is not
            // hypothetical: upstream merging se'if 2 into se'if 1 shifts every
            // address in that siman up by one, so `1:3` is a *different se'if*
            // wearing the old one's address — and handing it the old one's name
            // is T1 again, one level up from the line numbers this whole design
            // exists to escape.
            if !same_opening(&previous[was].text, fresh[at].text) {
                continue;
            }
            if taken.insert(was) {
                for_fresh[at] = Some(was);
            }
        }
    }

    // The gap fill above is address-keyed and each gap is bounded by kept
    // anchors, so it cannot cross them; within a gap it can, when upstream both
    // reordered and re-addressed. Drop anything that came out backwards rather
    // than let two places share a name.
    let mut highest: Option<usize> = None;
    for slot in &mut for_fresh {
        match (*slot, highest) {
            (Some(was), Some(top)) if was <= top => *slot = None,
            (Some(was), _) => highest = Some(was),
            (None, _) => {}
        }
    }

    let matched: BTreeSet<usize> = for_fresh.iter().flatten().copied().collect();
    Alignment {
        orphaned: (0..previous.len())
            .filter(|w| !matched.contains(w))
            .collect(),
        for_fresh,
    }
}

/// Whether two texts begin with the same word.
///
/// The corroboration an address needs before it may hand over a permanent name.
/// Text is read in order, so everything upstream actually does to a se'if — fix
/// a typo in it, absorb the next one into it, cut it in two — leaves its
/// opening alone; two different se'ifim at one address do not share one.
///
/// A whole word rather than a character count, because a character count is a
/// threshold somebody has to defend and the first word is a fact about the
/// sentence. The conservative failure is a se'if whose *first* word upstream
/// corrected: it gets a new name and a redirect row, rather than an old name
/// over new words. Loud beats silent.
///
/// **Two texts with no words in them agree.** `tur` has 18 segments whose
/// entire content is a single `<i data-commentator="Mystery"></i>`, so after
/// mining they are empty — Sefaria's own `push` checks the *raw* text for
/// emptiness and these are not raw-empty. Refusing to match nothing against
/// nothing cost those 18 their names on every re-import, and it bought nothing:
/// the failure being guarded against is an old name landing on new *words*, and
/// a segment with no words cannot be wrong about which ones it has.
///
/// **The word is compared after [`girsa_hebrew::normalize`], not byte for byte.**
/// The whole text is normalized rather than the raw first token, because the
/// first *normalized* word is not always the first whitespace run of the print:
/// a maqaf splits a token in two (`אֶת־הַשָּׁמַיִם` → `את השמים`) and a leading
/// punctuation mark vanishes. Upstream adding nikud to a se'if's opening, or
/// re-spelling it, leaves the word itself alone — and that is exactly the edit
/// this corroboration exists to absorb. The conservative failure is unchanged:
/// an opening corrected to a *different* word still refuses the handover, loudly
/// (a new name and a redirect row) rather than silently.
fn same_opening(was: &str, now: &str) -> bool {
    fn opening(text: &str) -> &str {
        text.split_whitespace().next().unwrap_or_default()
    }
    opening(&girsa_hebrew::normalize(was)) == opening(&girsa_hebrew::normalize(now))
}

/// Position of every text that appears exactly once in the sequence, after
/// [`girsa_hebrew::normalize`].
///
/// The anchor pass compares what the words *are*, not how they were printed, so
/// an upstream edit that adds nikud or fixes a spelling without changing the
/// words still anchors — and a text whose normalized form collides with a
/// sibling's says nothing about which of the two this is, exactly as a repeated
/// raw text already did. The keys are owned because the normal form is a new
/// string, not a borrow of the input.
fn once_by_text<'a>(texts: impl Iterator<Item = &'a str>) -> HashMap<String, usize> {
    let mut seen: HashMap<String, Option<usize>> = HashMap::new();
    for (at, text) in texts.enumerate() {
        let key = girsa_hebrew::normalize(text);
        seen.entry(key)
            .and_modify(|slot| *slot = None)
            .or_insert(Some(at));
    }
    seen.into_iter()
        .filter_map(|(text, at)| at.map(|at| (text, at)))
        .collect()
}

/// The longest subsequence of `pairs` (sorted by the first element) whose
/// second elements also increase.
///
/// Patience sorting: `tails[k]` is the smallest second-element a run of length
/// `k + 1` can end on, so a binary search places each pair in O(log n).
fn longest_increasing(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if pairs.is_empty() {
        return Vec::new();
    }
    let mut tails: Vec<usize> = Vec::new(); // index into `pairs`
    let mut came_from: Vec<Option<usize>> = vec![None; pairs.len()];
    for (i, (_, was)) in pairs.iter().enumerate() {
        let slot = tails.partition_point(|t| pairs[*t].1 < *was);
        came_from[i] = slot.checked_sub(1).map(|p| tails[p]);
        if slot == tails.len() {
            tails.push(i);
        } else {
            tails[slot] = i;
        }
    }
    let mut out = Vec::with_capacity(tails.len());
    let mut at = tails.last().copied();
    while let Some(i) = at {
        out.push(pairs[i]);
        at = came_from[i];
    }
    out.reverse();
    out
}

/// The names the records of one over-long place will be on disk as.
///
/// # The half of the name supply that was never asked for
///
/// [`crate::standing`] opens by saying that `Ordinal::child` has two callers
/// meaning opposite things by it — the oversized cutter carving `#7` into `#7.1`
/// and `#7.2`, and [`mint_between`] naming a se'if upstream inserted after `#7`
/// — and that only *"a cut deletes its parent"* tells the reader which happened.
/// That is about reading a name. This is about handing one out, and only one of
/// the two callers was doing it under any discipline at all: `mint_between`
/// takes a name that is not in `taken`, and the cutter called `id.split(n)` and
/// took `#7.1 … #7.n` whether or not those names were already somebody's.
///
/// They can be. A se'if inserted after `#7` in a previous run **is named** `#7.1`
/// and is live; let `#7` then grow past the threshold and the cutter writes a
/// second record called `#7.1`, in the same file, silently. `name_them`'s own
/// doc comment says *"in particular no name is ever handed out twice"*, and it
/// was true of the names that function minted and of no others.
///
/// # `mine` is what keeps a re-import still
///
/// `taken` is seeded with every name the previous run used, cut children
/// included — which is the fix — and that on its own would rename every cut
/// child on every import, because a place cut into three finds `#7.1 … #7.3`
/// taken by *itself*. `mine` is the names this same place was on disk as last
/// run: its to take again, and nobody else's to take at all.
///
/// A place cut into three and now cut into two therefore keeps `#7.1` and `#7.2`
/// and sheds `#7.3`, which the caller gives a forwarding address. A place that
/// has stopped being over-long sheds all of them.
#[must_use]
pub fn claim_children(
    parent: &Ordinal,
    count: usize,
    high: Option<&Ordinal>,
    mine: &BTreeSet<Ordinal>,
    taken: &mut BTreeSet<Ordinal>,
) -> Vec<Ordinal> {
    // Not cut: the place is one record, under its own name.
    if count <= 1 {
        return Vec::new();
    }
    // The names a cut has always used. Almost always free — and when they are
    // not, the place they clash with is a se'if an earlier run inserted after
    // this one, whose name `#7.1` sorts *inside* the words being cut.
    let natural = parent.children(count);
    let usable = natural.iter().all(|child| {
        (!taken.contains(child) || mine.contains(child))
            && high.is_none_or(|ceiling| child < ceiling)
    });
    if usable {
        for child in &natural {
            taken.insert(child.clone());
        }
        return natural;
    }
    // Out of room under the obvious names, so the pieces go where an insertion
    // would: strictly between this place and whatever comes next, which is what
    // `mint_between` is. The names it returns are odd to read — `#7.0`,
    // `#7.0.1` — and they are in reading order, which is the property that
    // matters. They are `mine` from the next import on, so this costs one run.
    mint_between(Some(parent), high, count, taken)
}

/// Names for a run of consecutive new places sitting between two kept ones.
///
/// `low` is the name of the place before the run and `high` the name of the one
/// after it; either is `None` at the ends of the sefer. Every name returned
/// sorts strictly between the two, so **nothing already on the shelf moves** —
/// which is the property the whole ordinal scheme exists for, now applied to
/// insertion as well as to splitting.
///
/// `taken` is every name already in use in this work, including the ones minted
/// earlier in this same pass.
#[must_use]
pub fn mint_between(
    low: Option<&Ordinal>,
    high: Option<&Ordinal>,
    count: usize,
    taken: &mut BTreeSet<Ordinal>,
) -> Vec<Ordinal> {
    let mut out = Vec::with_capacity(count);

    // Past the end of the sefer: fresh roots above every root in use. Not
    // `len + 1` — a name that was ever minted is never handed to different
    // words, so the counter goes above the highest, not above the count.
    if high.is_none() {
        let mut next = taken
            .iter()
            .filter_map(|o| o.at(0))
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        for _ in 0..count {
            let mut candidate = Ordinal::root(next);
            while taken.contains(&candidate) {
                next = next.saturating_add(1);
                candidate = Ordinal::root(next);
            }
            taken.insert(candidate.clone());
            out.push(candidate);
            next = next.saturating_add(1);
        }
        return out;
    }

    // Before the first place of the sefer: under a head of `0`, which sorts
    // before every root because roots are one-based. `#0.1` is an odd thing to
    // read and it is the honest one — the segment really is before what used to
    // be first, and giving it `#1` would be handing an existing name to new
    // words.
    let head = Ordinal::root(0);
    let mut under = low.cloned().unwrap_or(head);

    for _ in 0..count {
        let next = one_between(&under, high, taken);
        taken.insert(next.clone());
        out.push(next.clone());
        // Siblings while there is room under the same parent, which keeps a run
        // of five inserted se'ifim reading as `#5.1 … #5.5` rather than nesting
        // five deep. `one_between` returns a sibling whenever it can.
        if next.depth() > under.depth() + 1 {
            under = next;
        }
    }
    out
}

/// One name strictly between `under` and `high`, not already taken.
fn one_between(under: &Ordinal, high: Option<&Ordinal>, taken: &BTreeSet<Ordinal>) -> Ordinal {
    // The ceiling only constrains us when it is a *descendant* of `under`:
    // otherwise every child of `under` sorts below it. `#5` against `#6` has
    // room for `#5.1 … #5.4294967295`; `#5` against `#5.2` has room for `#5.0`
    // and `#5.1` and then has to go deeper.
    let ceiling = high
        .filter(|h| under.covers(h) && *h != under)
        .and_then(|h| h.at(under.depth()))
        .unwrap_or(u32::MAX);

    for k in 1..ceiling.max(1) {
        let candidate = under.child(k);
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    // `#5.0` is below `#5.1` and above `#5`. Only reachable when the ceiling is
    // `#5.1` itself, which needs an ancestor and its descendant both live and
    // an insertion between them — three imports deep at the earliest.
    let zero = under.child(0);
    if !taken.contains(&zero) {
        return zero;
    }
    // Out of siblings. Go a level deeper under the last name below the ceiling
    // and try again; the ceiling has a fixed depth, so descending eventually
    // stops being constrained by it at all and this terminates.
    one_between(&zero, high, taken)
}

/// Where a place upstream no longer has should send the anchors on it.
///
/// Text, again, and only inside the gap the alignment already narrowed it to —
/// a text that matches somewhere else entirely in the sefer is a coincidence,
/// not a re-segmentation, and following it would be the silent wrongness this
/// whole design is arranged against.
///
/// **The raw bytes first, then the normal form.** A raw containment that holds
/// is the strongest evidence and is returned immediately. When it does not hold,
/// the words may still have gone there while their *print* changed — upstream
/// added nikud in the same release it re-drew the boundary — so both directions
/// are retried after [`girsa_hebrew::normalize`], still confined to the same
/// gap. An old text whose words normalize to nothing is no evidence at all and
/// is not retried: `""` is contained in everything, and matching it would turn
/// every orphan into a re-segmentation.
///
/// Returns the new places whose words contain the old ones, or the new places
/// whose words the old one contained. Empty when neither holds, which is
/// [`super::Why::Gone`] and is a real answer.
#[must_use]
pub fn went_to(old: &Place, fresh: &[Fresh<'_>], within: std::ops::Range<usize>) -> Vec<usize> {
    let old_text = old.text.trim();
    if old_text.is_empty() {
        return Vec::new();
    }
    // Two se'ifim merged into one: the old words are inside the new ones.
    for at in within.clone() {
        if fresh[at].text.contains(old_text) {
            return vec![at];
        }
    }
    // One se'if split into several upstream, its own way: the new words are
    // inside the old ones. Consecutive only — a scatter is not a split. The
    // range is cloned because the normalized pass below retries it.
    let run: Vec<usize> = within
        .clone()
        .filter(|at| {
            let piece = fresh[*at].text.trim();
            !piece.is_empty() && old_text.contains(piece)
        })
        .collect();
    if run.len() > 1
        && run
            .last()
            .zip(run.first())
            .is_some_and(|(a, b)| a - b + 1 == run.len())
    {
        return run;
    }

    let old_norm = girsa_hebrew::normalize(old_text);
    if old_norm.is_empty() {
        return Vec::new();
    }
    for at in within.clone() {
        let now = girsa_hebrew::normalize(fresh[at].text);
        if now.contains(&old_norm) {
            return vec![at];
        }
    }
    let run: Vec<usize> = within
        .filter(|at| {
            let piece = girsa_hebrew::normalize(fresh[*at].text);
            !piece.is_empty() && old_norm.contains(&piece)
        })
        .collect();
    if run.len() > 1
        && run
            .last()
            .zip(run.first())
            .is_some_and(|(a, b)| a - b + 1 == run.len())
    {
        return run;
    }
    Vec::new()
}

/// Reassemble what a previous run wrote into the places it judged.
///
/// The segments on disk are post-cut: a se'if the oversized cutter split into
/// three is three records. What the next import has to compare against is the
/// se'if, so the `cut` redirect rows are read back and their children
/// concatenated. That is the redirect table earning its keep on the case this
/// repository already handles well, which is what makes it a file with rows in
/// it rather than an empty slot nobody exercises.
///
/// # And the previous run's text is mined again
///
/// Not belt and braces — a corpus on disk was written by whatever build wrote
/// it, and this one was: `tosefta-shabbat-lieberman` and 1,499 works like it
/// still carry `<i data-commentator…></i>` in their `text` field, because they
/// were imported before W34 mined those out and nothing has re-imported them
/// since. Comparing that against a freshly mined text matches nothing, so the
/// works most in need of a re-import would be the ones it renamed. Mining is
/// idempotent and costs a substring scan when there is nothing to find, which
/// is two thirds of the corpus.
#[must_use]
pub fn places_of(segments: &[super::Segment], redirects: &[super::Redirect]) -> Vec<Place> {
    let mut in_order: Vec<&super::Segment> = segments.iter().collect();
    // Reading order is ordinal order, never file order — `write` writes them in
    // reading order and a hand-sorted file must not change what anything is
    // called.
    in_order.sort_by(|a, b| a.id.cmp(&b.id));

    let live: BTreeMap<&crate::segment::SegmentId, &super::Segment> =
        in_order.iter().map(|s| (&s.id, *s)).collect();
    let mut parent_of: HashMap<&crate::segment::SegmentId, &super::Redirect> = HashMap::new();
    for row in redirects.iter().filter(|r| r.why == super::Why::Cut) {
        if live.contains_key(&row.from) {
            continue;
        }
        for child in &row.to {
            parent_of.insert(child, row);
        }
    }

    let mut out: Vec<Place> = Vec::new();
    let mut done: BTreeSet<&crate::segment::SegmentId> = BTreeSet::new();
    for segment in in_order.iter().copied() {
        if done.contains(&segment.id) {
            continue;
        }
        match parent_of.get(&segment.id) {
            Some(row) => {
                let mut text = String::new();
                let mut ids = Vec::new();
                for child in &row.to {
                    let Some(piece) = live.get(child) else {
                        continue;
                    };
                    text.push_str(&piece.text);
                    ids.push(child.clone());
                    done.insert(child);
                }
                out.push(Place {
                    ordinal: row.from.ordinal().clone(),
                    path: segment.id.path().to_vec(),
                    kind: segment.kind,
                    text: crate::anchors::mine(&text).text.into_owned(),
                    ids,
                });
            }
            None => {
                done.insert(&segment.id);
                out.push(Place {
                    ordinal: segment.id.ordinal().clone(),
                    path: segment.id.path().to_vec(),
                    kind: segment.kind,
                    text: crate::anchors::mine(&segment.text).text.into_owned(),
                    ids: vec![segment.id.clone()],
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::import::SegmentKind;

    fn place(n: u32, path: &[&str], text: &str) -> Place {
        let ordinal = Ordinal::root(n);
        let id = crate::segment::SegmentId::new(
            "w",
            path.iter().map(|p| (*p).to_string()).collect(),
            ordinal.clone(),
        );
        Place {
            ordinal,
            path: path.iter().map(|p| (*p).to_string()).collect(),
            kind: SegmentKind::Text,
            text: text.to_string(),
            ids: vec![id],
        }
    }

    /// `Fresh` borrows, so a test's rows have to be owned somewhere that
    /// outlives them. This is that somewhere.
    fn fresh(path: &[&str], text: &str) -> (Vec<String>, String) {
        (
            path.iter().map(|p| (*p).to_string()).collect(),
            text.to_string(),
        )
    }

    fn run(rows: &[(Vec<String>, String)]) -> Vec<Fresh<'_>> {
        rows.iter()
            .map(|(path, text)| Fresh {
                path,
                kind: SegmentKind::Text,
                text,
            })
            .collect()
    }

    #[test]
    fn inserting_one_seif_moves_nobody_elses_name() {
        // The scenario the whole module exists for, in miniature.
        let previous = vec![
            place(1, &["1", "1"], "אחד"),
            place(2, &["1", "2"], "שנים"),
            place(3, &["1", "3"], "שלשה"),
        ];
        let now = vec![
            fresh(&["1", "1"], "אחד"),
            fresh(&["1", "2"], "חדש"),
            // Everything below the insert is re-addressed by upstream.
            fresh(&["1", "3"], "שנים"),
            fresh(&["1", "4"], "שלשה"),
        ];
        let aligned = align(&previous, &run(&now));
        assert_eq!(aligned.for_fresh, vec![Some(0), None, Some(1), Some(2)]);
        assert!(aligned.orphaned.is_empty());
    }

    #[test]
    fn a_typo_fixed_upstream_does_not_cost_a_segment_its_name() {
        // Its text changed, so it cannot anchor itself. Its neighbours pin the
        // gap and its address settles it.
        let previous = vec![
            place(1, &["1"], "אחד"),
            place(2, &["2"], "שנים בטעות"),
            place(3, &["3"], "שלשה"),
        ];
        let now = vec![
            fresh(&["1"], "אחד"),
            fresh(&["2"], "שנים"),
            fresh(&["3"], "שלשה"),
        ];
        assert_eq!(
            align(&previous, &run(&now)).for_fresh,
            vec![Some(0), Some(1), Some(2)]
        );
    }

    #[test]
    fn re_sectioning_a_whole_work_costs_nothing_because_the_words_did_not_move() {
        let previous = vec![place(1, &["1", "1"], "אחד"), place(2, &["1", "2"], "שנים")];
        let now = vec![
            fresh(&["פרק א", "א"], "אחד"),
            fresh(&["פרק א", "ב"], "שנים"),
        ];
        assert_eq!(
            align(&previous, &run(&now)).for_fresh,
            vec![Some(0), Some(1)]
        );
    }

    #[test]
    fn a_place_upstream_dropped_is_orphaned_rather_than_reassigned() {
        let previous = vec![
            place(1, &["1"], "אחד"),
            place(2, &["2"], "שנים"),
            place(3, &["3"], "שלשה"),
        ];
        let now = vec![fresh(&["1"], "אחד"), fresh(&["2"], "שלשה")];
        let aligned = align(&previous, &run(&now));
        assert_eq!(aligned.for_fresh, vec![Some(0), Some(2)]);
        assert_eq!(aligned.orphaned, vec![1]);
    }

    #[test]
    fn a_matching_never_crosses_itself() {
        // Upstream moved the first paragraph to the end. That one paragraph
        // loses its match; the rest must not.
        let previous = vec![
            place(1, &["1"], "אחד"),
            place(2, &["2"], "שנים"),
            place(3, &["3"], "שלשה"),
            place(4, &["4"], "ארבעה"),
        ];
        let now = vec![
            fresh(&["1"], "שנים"),
            fresh(&["2"], "שלשה"),
            fresh(&["3"], "ארבעה"),
            fresh(&["4"], "אחד"),
        ];
        let aligned = align(&previous, &run(&now));
        assert_eq!(aligned.for_fresh, vec![Some(1), Some(2), Some(3), None]);
        let mut names: Vec<usize> = aligned.for_fresh.iter().flatten().copied().collect();
        let sorted = {
            let mut c = names.clone();
            c.sort_unstable();
            c
        };
        names.dedup();
        assert_eq!(names, sorted, "the matching goes forward on both sides");
    }

    #[test]
    fn a_repeated_line_does_not_anchor_anything_on_its_own() {
        // `וכו'` twice says nothing about which is which. The addresses do.
        let previous = vec![
            place(1, &["1"], "וכו'"),
            place(2, &["2"], "וכו'"),
            place(3, &["3"], "ייחודי"),
        ];
        let now = vec![
            fresh(&["1"], "וכו'"),
            fresh(&["2"], "וכו'"),
            fresh(&["3"], "ייחודי"),
        ];
        assert_eq!(
            align(&previous, &run(&now)).for_fresh,
            vec![Some(0), Some(1), Some(2)]
        );
    }

    #[test]
    fn a_segment_whose_whole_text_was_an_anchor_keeps_its_name() {
        // `tur` has 18 of these: the entire content is one
        // `<i data-commentator="Mystery"></i>`, so after W34's mining they are
        // empty. Found by running `--example measure-continuity` over the real
        // shelf, not by thinking about it — nothing but an address can identify
        // a segment with no words, and refusing to match nothing against
        // nothing cost all 18 their names on every re-import.
        let previous = vec![
            place(1, &["28", "10"], "ממש"),
            place(2, &["28", "11"], ""),
            place(3, &["28", "12"], "ועוד"),
        ];
        let now = vec![
            fresh(&["28", "10"], "ממש"),
            fresh(&["28", "11"], ""),
            fresh(&["28", "12"], "ועוד"),
        ];
        let aligned = align(&previous, &run(&now));
        assert_eq!(aligned.for_fresh, vec![Some(0), Some(1), Some(2)]);
        assert!(aligned.orphaned.is_empty());
    }

    #[test]
    fn a_shelf_written_before_the_markup_was_mined_still_matches() {
        // 1,500 works on the real shelf were imported before W34 and still
        // carry `<i data-commentator…></i>` in their `text`. `places_of` mines
        // the previous run's text for exactly this — without it the works most
        // in need of a re-import are the ones a re-import renames.
        let raw = "<i data-commentator=\"Variants\" data-label=\"א\" data-order=\"1\"></i>ארבעה";
        let previous = places_of(
            &[super::super::Segment {
                id: crate::segment::SegmentId::new("w", vec!["1".into()], Ordinal::root(1)),
                kind: SegmentKind::Text,
                text: raw.to_string(),
                anchors: Vec::new(),
            }],
            &[],
        );
        assert_eq!(previous[0].text, "ארבעה", "the previous side is mined too");
        assert_eq!(
            align(&previous, &run(&[fresh(&["1"], "ארבעה")])).for_fresh,
            vec![Some(0)]
        );
    }

    #[test]
    fn a_name_minted_between_two_others_sorts_between_them() {
        let mut taken: BTreeSet<Ordinal> = [Ordinal::root(5), Ordinal::root(6)].into();
        let minted = mint_between(
            Some(&Ordinal::root(5)),
            Some(&Ordinal::root(6)),
            3,
            &mut taken,
        );
        assert_eq!(
            minted.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["5.1", "5.2", "5.3"]
        );
        for name in &minted {
            assert!(Ordinal::root(5) < *name && *name < Ordinal::root(6));
        }
        assert!(minted[0] < minted[1] && minted[1] < minted[2]);
    }

    #[test]
    fn minting_skips_a_name_a_split_already_took() {
        // `#5` was cut into `#5.1 #5.2` by the oversized cutter. An insertion
        // after it may not be handed either of those.
        let mut taken: BTreeSet<Ordinal> = [
            Ordinal::root(5).child(1),
            Ordinal::root(5).child(2),
            Ordinal::root(6),
        ]
        .into();
        let minted = mint_between(
            Some(&Ordinal::root(5)),
            Some(&Ordinal::root(6)),
            1,
            &mut taken,
        );
        assert_eq!(minted[0].to_string(), "5.3");
    }

    #[test]
    fn minting_before_the_first_segment_sorts_before_it() {
        let mut taken: BTreeSet<Ordinal> = [Ordinal::root(1)].into();
        let minted = mint_between(None, Some(&Ordinal::root(1)), 2, &mut taken);
        assert_eq!(
            minted.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["0.1", "0.2"]
        );
        assert!(minted[1] < Ordinal::root(1));
    }

    #[test]
    fn minting_past_the_end_goes_above_the_highest_name_ever_used() {
        // Not above the count. A work that lost half its segments must not hand
        // a name that is still inside a Ksav document to different words.
        let mut taken: BTreeSet<Ordinal> = [Ordinal::root(1), Ordinal::root(900)].into();
        let minted = mint_between(Some(&Ordinal::root(900)), None, 2, &mut taken);
        assert_eq!(
            minted.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["901", "902"]
        );
    }

    #[test]
    fn there_is_always_room_between_an_ancestor_and_its_descendant() {
        // `#5` and `#5.1` are both live and something has to go between them.
        // `#5.0` is the only name there is, and it is a real one.
        let mut taken: BTreeSet<Ordinal> = [Ordinal::root(5), Ordinal::root(5).child(1)].into();
        let low = Ordinal::root(5);
        let high = Ordinal::root(5).child(1);
        let minted = mint_between(Some(&low), Some(&high), 2, &mut taken);
        assert!(low < minted[0], "{} is not after {low}", minted[0]);
        assert!(minted[0] < minted[1]);
        assert!(minted[1] < high, "{} is not before {high}", minted[1]);
    }

    #[test]
    fn a_merged_seifs_words_are_followed_into_the_segment_that_absorbed_them() {
        let old = place(5, &["1", "5"], "שנים");
        let now = vec![fresh(&["1", "4"], "אחד שנים")];
        assert_eq!(went_to(&old, &run(&now), 0..1), vec![0]);
    }

    #[test]
    fn a_place_whose_words_are_nowhere_is_said_to_be_gone_rather_than_guessed_at() {
        let old = place(5, &["1", "5"], "נמחק לגמרי");
        let now = vec![fresh(&["1", "5"], "משהו אחר לגמרי")];
        assert!(went_to(&old, &run(&now), 0..1).is_empty());
    }

    #[test]
    fn adding_nikud_upstream_costs_no_name_at_all() {
        // The whole class of edits this fixes: same words, different print.
        // With raw-byte matching, pass 1 lost all three anchors (every text
        // changed), pass 3 refused the addresses (the openings no longer agreed
        // byte for byte), and all three were orphaned to `Why::Gone`.
        let previous = vec![
            place(1, &["1"], "אחד"),
            place(2, &["2"], "שנים"),
            place(3, &["3"], "שלשה"),
        ];
        let now = vec![
            fresh(&["1"], "אֶחָד"),
            fresh(&["2"], "שְׁנַיִם"),
            fresh(&["3"], "שְׁלֹשָׁה"),
        ];
        let aligned = align(&previous, &run(&now));
        assert_eq!(aligned.for_fresh, vec![Some(0), Some(1), Some(2)]);
        assert!(aligned.orphaned.is_empty());
    }

    #[test]
    fn same_opening_sees_a_nikud_edit_as_the_same_word() {
        assert!(same_opening("שנים", "שְׁנַיִם"));
        assert!(same_opening("מאימתי קורין", "מֵאֵימָתַי קוֹרִין"));
        // A maqaf splits a token, so the first *normalized* word is the first
        // normalized token, not the raw first whitespace run.
        assert!(same_opening("אֶת־הַשָּׁמַיִם", "את השמים"));
        // A leading punctuation mark is not a word and vanishes.
        assert!(same_opening("»שנים", "שְׁנַיִם"));
        // Two texts with no words in them still agree.
        assert!(same_opening("", ""));
        // A corrected first word is still a different word.
        assert!(!same_opening("שנים", "שלשה"));
        assert!(!same_opening("שנים", ""));
    }

    #[test]
    fn a_nikud_edit_and_a_typo_fix_in_the_same_release_keeps_the_name() {
        // Pass 1 cannot anchor it — the text changed in two ways at once — so
        // the neighbours pin the gap and the address hands the name over. The
        // opening agrees only after normalization.
        let previous = vec![
            place(1, &["1"], "אחד"),
            place(2, &["2"], "שנים בטעות"),
            place(3, &["3"], "שלשה"),
        ];
        let now = vec![
            fresh(&["1"], "אחד"),
            fresh(&["2"], "שְׁנַיִם"),
            fresh(&["3"], "שלשה"),
        ];
        let aligned = align(&previous, &run(&now));
        assert_eq!(aligned.for_fresh, vec![Some(0), Some(1), Some(2)]);
        assert!(aligned.orphaned.is_empty());
    }

    #[test]
    fn a_merge_that_also_added_nikud_is_still_followed() {
        // The raw pass fails — the old words are bare and the absorbing se'if
        // is menukad — and the normalized fallback sees the same words.
        let old = place(5, &["1", "5"], "שלשה");
        let now = vec![fresh(&["1", "4"], "שְׁנַיִם שְׁלֹשָׁה")];
        assert_eq!(went_to(&old, &run(&now), 0..1), vec![0]);
    }

    #[test]
    fn a_split_whose_pieces_are_menukad_is_still_a_split() {
        let old = place(5, &["1", "5"], "שלשה אנשים");
        let now = vec![fresh(&["1", "5"], "שְׁלֹשָׁה"), fresh(&["1", "6"], "אֲנָשִׁים")];
        assert_eq!(went_to(&old, &run(&now), 0..2), vec![0, 1]);
    }

    #[test]
    fn a_place_whose_words_normalize_to_nothing_is_no_evidence_at_all() {
        // A bullet is not a word: its raw text is non-empty, so the raw pass
        // runs and fails, and its normal form is empty. Without the guard the
        // fallback would match the first place in the gap — `""` is contained
        // in everything — and turn every orphan into a re-segmentation.
        let old = place(5, &["1", "5"], "•");
        let now = vec![fresh(&["1", "5"], "משהו אחר")];
        assert!(went_to(&old, &run(&now), 0..1).is_empty());
    }
}
