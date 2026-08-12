//! The transmission chain — forward, backward, between, and where it forks.
//!
//! spec.md §8.6, BUILDER.md W28: *trace forward from a Gemara to how it became
//! halacha; trace backward from a ruling to where the posek got it; find the
//! path between two texts, or report honestly that none exists; and break
//! analysis — where two rishonim read one Gemara into incompatible halachos.*
//!
//! # Direction is time, not the arrow
//!
//! Every sentence above has a direction in it and **the graph does not have
//! one**. An edge is stored once, in the shard of the work it points from
//! (§8.2), and which end that was is an accident of whoever wrote the row:
//! Berakhot points *at* its commentaries, the Mishnah Berurah points *back at*
//! the Shulchan Arukh, and the Shulchan Arukh does both. Following arrows would
//! walk one chain forwards and the next one backwards and call them the same
//! thing.
//!
//! So a hop is forward when the sefer at the far end was **written later**, and
//! [`girsa_corpus::era`] is the only thing that answers that. Where it cannot
//! answer — 11.3% of the graph's edges point at a work with neither a date nor
//! an era — the hop is **not taken and is counted** in [`Refused`]. A chain that
//! quietly skipped what it could not date would look shorter and surer than it
//! is.
//!
//! # What the corpus can and cannot say
//!
//! Every edge type present in the 4,182,337 edges on this machine, counted:
//!
//! ```text
//! 2,123,215  comments-on     50.8%
//! 2,048,326  references      49.0%    ← "connected somehow", and nothing more
//!     7,812  paraphrases      0.2%
//!     2,984  quotes           0.1%
//! ```
//!
//! There are **no `disputes` edges at all**, and there is no `codifies`. So a
//! machlokes cannot be read off this graph, and a chain built out of
//! `references` hops is a coincidence of the corpus rather than a transmission.
//! Both facts are carried on the answer rather than hidden by it:
//! [`Trace::is_transmission`] is false the moment one hop in a chain is
//! unasserted, and [`Fork`] reports the *shape* of a disagreement — two later
//! readings of one source, brought back together by a third text that cites
//! both — while never claiming the two disagree. Rule 6: a wrong claim about a
//! link is worse than no claim.
//!
//! # Cost
//!
//! A hop needs every edge touching an anchor, which is that work's outgoing
//! shard **and** the inbound cache W28 added beside it ([`crate::inbound`]).
//! Two file reads per work, cached for the life of a [`Graph`], so a trace that
//! passes through Berakhot three times reads Berakhot once.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};

use girsa_corpus::era::{Order, Timeline, When};
use girsa_corpus::segment::SegmentId;

use crate::repair::{Repaired, Repairs};
use crate::{inbound, Anchor, EdgeType};

/// Which way along the axis of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Later seforim: how this became halacha.
    Forward,
    /// Earlier seforim: where this was got from.
    Back,
}

impl Direction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Back => "back",
        }
    }

    /// The [`Order`] a hop must have for this direction, reading
    /// `order(here, there)`.
    const fn wanted(self) -> Order {
        match self {
            Self::Forward => Order::Before,
            Self::Back => Order::After,
        }
    }
}

/// How far and how wide a walk is allowed to go.
///
/// Both are real limits and both are **reported when they bite** — a list that
/// silently stopped at the twelfth commentary reads as *these are all of them*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// How many hops from the start.
    pub depth: usize,
    /// How many neighbours are followed from any one place.
    pub width: usize,
    /// How many places a [`path`] search may open before it gives up. Reaching
    /// it is [`Found::NotWithin`], which is **not** "there is no path".
    pub budget: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            depth: 3,
            width: 8,
            budget: 40_000,
        }
    }
}

/// What a walk did not follow, and why.
///
/// Not diagnostics — part of the answer. *Nine of the eleven seforim that read
/// this line could not be dated* changes what the eleven-line chain above it
/// means.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Refused {
    /// The far sefer has neither a date nor an era, so which way the hop goes
    /// is not knowable.
    pub undated: usize,
    /// The far sefer is on the other side of the axis from the way we are
    /// walking. Expected, and counted because it is the bulk of them.
    pub wrong_way: usize,
    /// The two seforim were being written at the same time, so neither came
    /// from the other.
    pub contemporary: usize,
    /// Dropped by [`Limits::width`], best-first, after everything above.
    pub over_budget: usize,
    /// Your layer says this link is wrong.
    pub rejected: usize,
    /// Works whose incoming half could not be read because
    /// [`crate::inbound::built`] is false. Every one of these is a place the
    /// walk may have missed a hop, and the caller must say so.
    pub incoming_unknown: BTreeSet<String>,
}

impl Refused {
    /// Whether anything was left out at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    fn absorb(&mut self, other: &Self) {
        self.undated += other.undated;
        self.wrong_way += other.wrong_way;
        self.contemporary += other.contemporary;
        self.over_budget += other.over_budget;
        self.rejected += other.rejected;
        self.incoming_unknown
            .extend(other.incoming_unknown.iter().cloned());
    }
}

/// One place a walk reached, and the link it got there by.
#[derive(Debug, Clone)]
pub struct Step {
    /// Which step this was reached from. `None` for a hop straight off the
    /// start.
    pub parent: Option<usize>,
    /// Hops from the start; the first is 1.
    pub depth: usize,
    pub at: Anchor,
    /// When the sefer at this step was written, as much as anything knows.
    pub when: When,
    /// The type of the link that got here, **after** your repair layer.
    pub edge_type: EdgeType,
    /// What the corpus called that link, verbatim — a blank being three
    /// quarters of them (T5), and the difference between *the corpus said
    /// nothing* and *the corpus said `related`*.
    pub label: String,
    pub confidence: f32,
    /// You drew this link, or confirmed it.
    pub mine: bool,
}

impl Step {
    #[must_use]
    pub fn work(&self) -> &str {
        self.at.from.work()
    }
}

/// A walk out from one place.
#[derive(Debug, Clone)]
pub struct Trace {
    pub start: SegmentId,
    pub direction: Direction,
    pub steps: Vec<Step>,
    pub refused: Refused,
    /// Works read off disk. Reported so a slow trace can be seen to be slow for
    /// a reason.
    pub works_read: usize,
}

impl Trace {
    /// The chain from the start to one step, oldest hop first.
    ///
    /// Indices into [`Trace::steps`]. Empty for an index that is not a step.
    #[must_use]
    pub fn chain(&self, step: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut at = Some(step);
        while let Some(i) = at {
            let Some(hop) = self.steps.get(i) else {
                return Vec::new();
            };
            out.push(i);
            at = hop.parent;
        }
        out.reverse();
        out
    }

    /// Whether every hop from the start to this step is a link that claims
    /// something.
    ///
    /// False the moment one hop is [`EdgeType::References`], which is 49% of
    /// the graph. A chain of *these two are connected somehow* is not a
    /// transmission and this is the flag that stops it being shown as one.
    #[must_use]
    pub fn is_transmission(&self, step: usize) -> bool {
        let chain = self.chain(step);
        !chain.is_empty()
            && chain
                .iter()
                .filter_map(|i| self.steps.get(*i))
                .all(|hop| hop.edge_type.is_asserted())
    }

    /// The weakest claim anywhere on the chain to this step — what the whole
    /// chain is worth.
    #[must_use]
    pub fn weakest(&self, step: usize) -> Option<EdgeType> {
        self.chain(step)
            .iter()
            .filter_map(|i| self.steps.get(*i))
            .map(|hop| hop.edge_type)
            .max()
    }

    /// The steps at the far end of the walk — the ones nothing was reached
    /// from.
    #[must_use]
    pub fn ends(&self) -> Vec<usize> {
        let parents: BTreeSet<usize> = self.steps.iter().filter_map(|s| s.parent).collect();
        (0..self.steps.len())
            .filter(|i| !parents.contains(i))
            .collect()
    }
}

/// The graph, read one work at a time and kept.
///
/// Holds every edge touching a work — its own shard and the inbound cache — put
/// through your repair layer once. A trace asks the same work about several
/// anchors, and re-reading an 8 MB shard per anchor is the difference between a
/// panel and a wait.
pub struct Graph<'a> {
    root: PathBuf,
    timeline: &'a Timeline,
    repairs: &'a Repairs,
    /// Every link you drew by hand, assembled once.
    ///
    /// `beside` — the BFS inner loop — ended with `for repaired in
    /// self.repairs.drawn()`, and `drawn()` walks the **whole** personal repair
    /// log and rebuilds a `Repaired` for each drawn record it finds. So a trace
    /// paid for a full scan of everything you have ever confirmed, denied or
    /// retyped, per anchor visited, to collect the handful of edges you drew.
    ///
    /// The same class as `repair.rs:269`'s guard, hoisted the same way as
    /// `beside(a)` in `forks` below: `repairs` is borrowed immutably for this
    /// graph's whole life, so the answer cannot change between the first anchor
    /// and the last. For a reader who has drawn nothing it is an empty `Vec`,
    /// which is the common case and now costs a length check.
    drawn: Vec<Repaired>,
    by_work: HashMap<String, Held>,
    incoming_unknown: BTreeSet<String>,
}

/// One work's edges, and a way into them that is not a full scan.
///
/// # Why an index and not a sort
///
/// `beside` asks *which edges have an end overlapping this anchor*, and
/// `Anchor::overlaps` is a range test. Sorting the edges by their `from` gives
/// an upper bound and no lower one, because a span's far end is not ordered by
/// its near end.
///
/// So the ends are split. **A point end goes into a sorted list keyed by its
/// segment**, where a query — itself a point or a run — is a binary search for
/// a contiguous range. **A span end goes into a list that is still scanned**,
/// because a set of intervals cannot be searched by one key and the honest
/// answer is to look at all of them.
///
/// What that buys: Shulchan Arukh Orach Chayim is 156,076 edges, and a
/// depth-3/width-8 trace re-walked the whole vector up to 73 times. The point
/// ends — the great majority — now cost a binary search each.
struct Held {
    edges: Vec<Repaired>,
    /// `(the segment a point end names, which edge)`, sorted by the segment.
    /// An edge with two point ends appears twice.
    points: Vec<(SegmentId, usize)>,
    /// Edges with at least one **span** end, which have to be looked at.
    spans: Vec<usize>,
}

impl Held {
    fn of(edges: Vec<Repaired>) -> Self {
        let mut points = Vec::new();
        let mut spans = Vec::new();
        for (at, repaired) in edges.iter().enumerate() {
            let mut spanned = false;
            for end in [&repaired.edge.from, &repaired.edge.to] {
                if end.is_span() {
                    spanned = true;
                } else {
                    points.push((end.from.clone(), at));
                }
            }
            if spanned {
                spans.push(at);
            }
        }
        points.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            edges,
            points,
            spans,
        }
    }

    /// Which edges could possibly have an end overlapping this anchor.
    ///
    /// A superset, deliberately: `far_end` is still the only thing that decides,
    /// and this only says which rows are worth asking it about.
    fn near(&self, at: &Anchor) -> Vec<usize> {
        let last = at.to.as_ref().unwrap_or(&at.from);
        let lower = self.points.partition_point(|(id, _)| id < &at.from);
        let upper = self.points.partition_point(|(id, _)| id <= last);
        let mut out: Vec<usize> = self.points[lower..upper]
            .iter()
            .map(|(_, at)| *at)
            .chain(self.spans.iter().copied())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

impl<'a> Graph<'a> {
    #[must_use]
    pub fn new(root: &Path, timeline: &'a Timeline, repairs: &'a Repairs) -> Self {
        Self {
            root: root.to_path_buf(),
            timeline,
            repairs,
            drawn: repairs.drawn().collect(),
            by_work: HashMap::new(),
            incoming_unknown: BTreeSet::new(),
        }
    }

    /// How many works have been read off disk.
    #[must_use]
    pub fn works_read(&self) -> usize {
        self.by_work.len()
    }

    /// Every edge touching a work, each exactly once, with your layer over it —
    /// and an index into them.
    fn work(&mut self, slug: &str) -> Option<&Held> {
        if !self.by_work.contains_key(slug) {
            let (edges, known) =
                inbound::touching_work(&self.root, slug).unwrap_or_else(|_| (Vec::new(), false));
            if !known {
                self.incoming_unknown.insert(slug.to_string());
            }
            self.by_work
                .insert(slug.to_string(), Held::of(self.repairs.apply(edges)));
        }
        self.by_work.get(slug)
    }

    /// Everywhere a link joins to this anchor, in either stored direction.
    ///
    /// The links you drew are in no shard at all and are added here, so a hand
    /// drawn edge is a hop like any other (W23).
    fn beside(&mut self, at: &Anchor) -> Vec<(Anchor, Repaired)> {
        let slug = at.from.work().to_string();
        let mut out: Vec<(Anchor, Repaired)> = match self.work(&slug) {
            Some(held) => held
                .near(at)
                .into_iter()
                .filter_map(|which| held.edges.get(which))
                .filter_map(|repaired| far_end(at, repaired))
                .collect(),
            None => Vec::new(),
        };
        // Filtered by `far_end` alone, which asks whether the two anchors
        // overlap — a stricter test than a pre-filter on the near end, and the
        // right one here: a chain hops anchor to anchor and nobody is standing
        // on a segment for a [`girsa_corpus::standing::Standing`] to be about.
        for repaired in &self.drawn {
            if let Some(hop) = far_end(at, repaired) {
                out.push(hop);
            }
        }
        out
    }
}

/// The other end of an edge, if this anchor is on one of its ends.
fn far_end(at: &Anchor, repaired: &Repaired) -> Option<(Anchor, Repaired)> {
    if repaired.edge.from.overlaps(at) {
        return Some((repaired.edge.to.clone(), repaired.clone()));
    }
    if repaired.edge.to.overlaps(at) {
        return Some((repaired.edge.from.clone(), repaired.clone()));
    }
    None
}

/// Walk out from a segment along the axis of time.
///
/// Breadth first, so the nearest heirs are found before the distant ones, and
/// bounded by [`Limits`] in both directions with every drop counted.
pub fn trace(graph: &mut Graph<'_>, at: &SegmentId, direction: Direction, limits: Limits) -> Trace {
    let start = Anchor::point(at.clone());
    let mut steps: Vec<Step> = Vec::new();
    let mut refused = Refused::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    seen.insert(start.to_string());

    let mut frontier: VecDeque<(Option<usize>, Anchor, usize)> = VecDeque::new();
    frontier.push_back((None, start, 0));

    while let Some((parent, here, depth)) = frontier.pop_front() {
        if depth >= limits.depth {
            continue;
        }
        let here_work = here.from.work().to_string();
        let here_when = graph.timeline.when(&here_work);
        let mut candidates: Vec<(Anchor, Repaired, When)> = Vec::new();

        for (other, repaired) in graph.beside(&here) {
            if repaired.rejected {
                refused.rejected += 1;
                continue;
            }
            let other_work = other.from.work();
            if other_work == here_work {
                // A link inside one sefer is not a step in its transmission.
                // It is also the one hop the era axis cannot rule on, since a
                // work is contemporary with itself.
                continue;
            }
            let when = graph.timeline.when(other_work);
            match girsa_corpus::era::order(&here_when, &when) {
                order if order == direction.wanted() => {}
                Order::Unknown => {
                    refused.undated += 1;
                    continue;
                }
                Order::Contemporary => {
                    refused.contemporary += 1;
                    continue;
                }
                _ => {
                    refused.wrong_way += 1;
                    continue;
                }
            }
            if !seen.insert(other.to_string()) {
                continue;
            }
            candidates.push((other, repaired, when));
        }

        rank(&mut candidates, &here_when);
        if candidates.len() > limits.width {
            refused.over_budget += candidates.len() - limits.width;
            candidates.truncate(limits.width);
        }
        for (other, repaired, when) in candidates {
            let index = steps.len();
            steps.push(Step {
                parent,
                depth: depth + 1,
                at: other.clone(),
                when,
                edge_type: repaired.edge.edge_type,
                label: repaired.edge.source_label.clone(),
                confidence: repaired.confidence(),
                mine: repaired.mine,
            });
            frontier.push_back((Some(index), other, depth + 1));
        }
    }

    refused
        .incoming_unknown
        .extend(graph.incoming_unknown.iter().cloned());
    Trace {
        start: at.clone(),
        direction,
        steps,
        refused,
        works_read: graph.works_read(),
    }
}

/// Best hop first: a claim before a shrug, a believed link before a guessed
/// one, and the nearest in time before the distant.
///
/// The last of the three is what makes a chain read like a chain — the sefer
/// that took this from the Gemara is the one written next, not the one written
/// eight hundred years later that also cites it.
fn rank(candidates: &mut [(Anchor, Repaired, When)], here: &When) {
    candidates.sort_by(|a, b| {
        a.1.edge
            .edge_type
            .cmp(&b.1.edge.edge_type)
            .then_with(|| {
                b.1.confidence()
                    .partial_cmp(&a.1.confidence())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| gap(here, &a.2).cmp(&gap(here, &b.2)))
            // `Anchor: Ord`, not two `String`s. This is the *final* tiebreak of
            // an `O(n log n)` sort, so it fires on most comparisons — and
            // `to_string` sorts a section path lexicographically, which puts
            // siman 10 before siman 9. `SegmentId`'s own order is the ordinal,
            // which is reading order.
            .then_with(|| a.0.cmp(&b.0))
    });
}

/// Years between two seforim, or [`i64::MAX`] where either is undated — which
/// sorts it last without pretending to a number.
fn gap(here: &When, there: &When) -> i64 {
    match (here.latest_year(), there.latest_year()) {
        (Some(a), Some(b)) => i64::from(a - b).abs(),
        _ => i64::MAX,
    }
}

/// One link on a path: where it lands and what kind of link it was.
#[derive(Debug, Clone)]
pub struct Link {
    pub at: Anchor,
    pub edge_type: EdgeType,
    pub label: String,
    pub confidence: f32,
}

/// What a search between two places found.
#[derive(Debug, Clone)]
pub enum Found {
    /// The links from the first place to the second, in order.
    Path(Vec<Link>),
    /// The search ran out of budget. **This is not "there is no path"** — it is
    /// "I stopped looking", and the two are different answers to a reader
    /// deciding whether to believe there is no connection.
    NotWithin { opened: usize, depth: usize },
    /// Everything reachable from both ends was opened and they never met. This
    /// one *is* "no path exists", and is rare: the graph is one big component
    /// almost everywhere.
    None,
}

/// One side of a two-sided search: every place it has opened, and for each, the
/// place it came from and the link it came by. `None` for the place it started
/// at.
type Side = BTreeMap<String, (Anchor, Option<(String, Link)>)>;

/// The shortest chain of links between two places, in either direction and
/// regardless of when anything was written.
///
/// Time does not constrain this one on purpose: *how are these two connected*
/// is a different question from *how did this become that*, and a connection
/// that runs through a contemporary is still a connection.
///
/// Searched from both ends at once, which is what makes it finishable: a daf of
/// Gemara has tens of thousands of links, so one-sided breadth-first search
/// spends its whole budget on the first two hops.
pub fn path(graph: &mut Graph<'_>, from: &SegmentId, to: &SegmentId, limits: Limits) -> Found {
    let (start, goal) = (Anchor::point(from.clone()), Anchor::point(to.clone()));
    if start.overlaps(&goal) {
        return Found::Path(Vec::new());
    }

    // anchor text → (the anchor, how we got here from this side)
    let mut sides: [Side; 2] = [
        BTreeMap::from([(start.to_string(), (start.clone(), None))]),
        BTreeMap::from([(goal.to_string(), (goal.clone(), None))]),
    ];
    let mut frontiers: [Vec<Anchor>; 2] = [vec![start], vec![goal]];
    let mut opened = 0usize;

    for depth in 0..limits.depth * 2 {
        // Expand whichever side is narrower — the whole point of two-sided
        // search, and on this graph the two sides differ by orders of
        // magnitude.
        let side = usize::from(frontiers[1].len() < frontiers[0].len());
        if frontiers[side].is_empty() {
            return Found::None;
        }
        let mut next = Vec::new();
        for here in std::mem::take(&mut frontiers[side]) {
            for (other, repaired) in graph.beside(&here) {
                if repaired.rejected {
                    continue;
                }
                opened += 1;
                if opened > limits.budget {
                    return Found::NotWithin {
                        opened,
                        depth: depth / 2,
                    };
                }
                let key = other.to_string();
                let link = Link {
                    at: other.clone(),
                    edge_type: repaired.edge.edge_type,
                    label: repaired.edge.source_label.clone(),
                    confidence: repaired.confidence(),
                };
                if sides[side].contains_key(&key) {
                    continue;
                }
                sides[side].insert(key.clone(), (other.clone(), Some((here.to_string(), link))));
                if sides[1 - side].contains_key(&key) {
                    return Found::Path(join(&sides, &key));
                }
                next.push(other);
            }
        }
        frontiers[side] = next;
    }
    Found::NotWithin {
        opened,
        depth: limits.depth,
    }
}

/// Walk both sides' parent chains out from the meeting point and write them as
/// one path, from the caller's start to the caller's goal.
///
/// Always side 0 first and side 1 second, whichever side happened to be the one
/// expanding when they met: the two halves are a *start* half and a *goal*
/// half, and which of them the search was working on at the moment of contact
/// has nothing to do with which way round they are written.
fn join(sides: &[Side; 2], meeting: &str) -> Vec<Link> {
    // The start half. Each entry's link is named by the place it lands on, so
    // walking parents from the meeting point and reversing gives them in
    // reading order.
    let mut out = Vec::new();
    let mut at = meeting.to_string();
    while let Some((_, Some((parent, link)))) = sides[0].get(&at) {
        out.push(link.clone());
        at = parent.clone();
    }
    out.reverse();

    // The goal half runs from the meeting point *towards* the goal, so each of
    // its links lands on the place before it in that map. The edge is the
    // right edge; the anchor is taken from the next hop along.
    let mut at = meeting.to_string();
    while let Some((_, Some((parent, link)))) = sides[1].get(&at) {
        let Some((parent_anchor, _)) = sides[1].get(parent) else {
            break;
        };
        out.push(Link {
            at: parent_anchor.clone(),
            ..link.clone()
        });
        at = parent.clone();
    }
    out
}

/// Two later readings of one place, brought back together by a third text that
/// cites both.
///
/// This is as close to spec.md §8.6's *break analysis* as this corpus can
/// honestly get, and the gap is worth being plain about: **the graph has no
/// `disputes` edge anywhere in it**, so nothing in the data says two seforim
/// disagree. What the data does say is that two of them read the same line and
/// that a later one had to deal with both — which is the shape a machlokes
/// leaves behind, and is offered as a place to look rather than as a finding.
#[derive(Debug, Clone)]
pub struct Fork {
    pub a: Anchor,
    pub b: Anchor,
    pub a_when: When,
    pub b_when: When,
    /// Later places that link to both sides.
    pub witnesses: Vec<Anchor>,
    /// Whether a link joins the two sides directly. When it does they are not
    /// two readings passing each other — one of them is answering the other,
    /// and that is a different thing to look at.
    pub joined: bool,
}

impl Fork {
    #[must_use]
    pub fn a_work(&self) -> &str {
        self.a.from.work()
    }

    #[must_use]
    pub fn b_work(&self) -> &str {
        self.b.from.work()
    }
}

/// Find the forks below a place.
///
/// Returns them best first — most witnesses, then earliest — and everything the
/// walk would not follow.
pub fn forks(graph: &mut Graph<'_>, at: &SegmentId, limits: Limits) -> (Vec<Fork>, Refused) {
    let one_hop = Limits { depth: 1, ..limits };
    let readings = trace(graph, at, Direction::Forward, one_hop);
    let mut refused = readings.refused.clone();

    // What each reading is itself read by, one hop further on.
    let mut readers: Vec<(Anchor, When, BTreeMap<String, Anchor>)> = Vec::new();
    for step in &readings.steps {
        let below = trace(graph, &step.at.from, Direction::Forward, one_hop);
        refused.absorb(&below.refused);
        let mut of_this: BTreeMap<String, Anchor> = BTreeMap::new();
        for reader in below.steps {
            of_this.insert(reader.at.to_string(), reader.at);
        }
        readers.push((step.at.clone(), step.when, of_this));
    }

    // `beside(a)` once per `a`, not once per pair.
    //
    // It was called inside the O(n²) pair loop below, so a place with eight
    // readings asked for the same neighbours **28 times** where 8 suffice —
    // and `beside` is a shard read. The value depends only on `a`, which the
    // outer loop already fixes, so it is hoisted rather than memoised: a cache
    // for a value with one obvious computation point is a cache nobody can
    // reason about.
    let joined_of: Vec<Vec<(Anchor, Repaired)>> =
        readers.iter().map(|(a, _, _)| graph.beside(a)).collect();

    let mut out = Vec::new();
    for (i, (a, a_when, a_readers)) in readers.iter().enumerate() {
        for (b, b_when, b_readers) in readers.iter().skip(i + 1) {
            if a.from.work() == b.from.work() {
                // Two lines of one sefer are not two readings of the sugya.
                continue;
            }
            let witnesses: Vec<Anchor> = a_readers
                .iter()
                .filter(|(key, _)| b_readers.contains_key(*key))
                .map(|(_, anchor)| anchor.clone())
                .collect();
            if witnesses.is_empty() {
                continue;
            }
            let joined = joined_of[i]
                .iter()
                .any(|(other, repaired)| !repaired.rejected && other.overlaps(b));
            out.push(Fork {
                a: a.clone(),
                b: b.clone(),
                a_when: *a_when,
                b_when: *b_when,
                witnesses,
                joined,
            });
        }
    }
    out.sort_by(|x, y| {
        y.witnesses
            .len()
            .cmp(&x.witnesses.len())
            .then_with(|| x.a.to_string().cmp(&y.a.to_string()))
            .then_with(|| x.b.to_string().cmp(&y.b.to_string()))
    });
    refused
        .incoming_unknown
        .extend(graph.incoming_unknown.iter().cloned());
    (out, refused)
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::{store, Edge, Method};
    use girsa_corpus::segment::Ordinal;
    use girsa_corpus::work::{Source, Work};
    use std::path::PathBuf;

    fn id(work: &str, n: u32) -> SegmentId {
        SegmentId::new(work, vec![n.to_string()], Ordinal::root(n))
    }

    fn edge(from: &str, fi: u32, to: &str, ti: u32, edge_type: EdgeType) -> Edge {
        Edge {
            from: Anchor::point(id(from, fi)),
            to: Anchor::point(id(to, ti)),
            edge_type,
            method: Method::SefariaSeed,
            direction: crate::Direction::NotRecorded,
            source_label: match edge_type {
                EdgeType::CommentsOn => "commentary".into(),
                EdgeType::Quotes => "quotation".into(),
                _ => String::new(),
            },
        }
    }

    fn work(slug: &str, comp_date: &str) -> Work {
        Work {
            slug: slug.into(),
            he_title: slug.into(),
            en_title: slug.into(),
            categories: Vec::new(),
            order: Vec::new(),
            source: Source::Sefaria,
            origin: PathBuf::new(),
            schema: None,
            he_sections: Vec::new(),
            author: None,
            era: None,
            comp_date: (!comp_date.is_empty()).then(|| comp_date.to_string()),
            version: None,
            commentary_on: Vec::new(),
        }
    }

    /// A corpus with a real shape: a Gemara, two rishonim on it, a code that
    /// takes from one of them, and a nineteenth-century commentary that cites
    /// both rishonim.
    fn shas(name: &str) -> (PathBuf, Timeline) {
        let root = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("works")).expect("makes the root");

        let works = [
            work("gemara", "c.450  – c.550 CE"),
            work("rashi", "c.1065  – c.1115 CE"),
            work("rambam", "c.1170  – c.1180 CE"),
            work("shulchan-arukh", "1563 CE"),
            work("mishnah-berurah", "c.1875  – c.1905 CE"),
            work("undated", ""),
        ];
        let body: String = works
            .iter()
            .map(|w| format!("{}\n", serde_json::to_string(w).expect("writes")))
            .collect();
        std::fs::write(root.join("works/index.jsonl"), body).expect("writes the catalogue");

        // Stored in whichever direction the corpus happens to store it — which
        // is the point: two of these run against the arrow of time.
        let edges = [
            edge("gemara", 1, "rashi", 1, EdgeType::CommentsOn),
            edge("gemara", 1, "rambam", 1, EdgeType::CommentsOn),
            edge("shulchan-arukh", 1, "rambam", 1, EdgeType::CommentsOn),
            edge(
                "mishnah-berurah",
                1,
                "shulchan-arukh",
                1,
                EdgeType::CommentsOn,
            ),
            edge("mishnah-berurah", 1, "rashi", 1, EdgeType::Quotes),
            edge("mishnah-berurah", 1, "rambam", 1, EdgeType::Quotes),
            edge("gemara", 1, "undated", 1, EdgeType::CommentsOn),
        ];
        let mut shard = store::Writer::default();
        let mut back = inbound::Writer::default();
        for e in &edges {
            shard.push(e);
            back.push(e);
        }
        shard.flush(&root).expect("writes the shards");
        back.flush(&root).expect("writes the inbound cache");

        let timeline = Timeline::of(&root).expect("reads the catalogue");
        (root, timeline)
    }

    #[test]
    fn a_chain_runs_forward_in_time_and_not_along_the_arrows() {
        // Two of the four edges in this path are stored later → earlier. A walk
        // that followed edge direction would go one hop and stop.
        let (root, timeline) = shas("girsa-chain-forward");
        let repairs = Repairs::nowhere();
        let mut graph = Graph::new(&root, &timeline, &repairs);
        let trace = trace(
            &mut graph,
            &id("gemara", 1),
            Direction::Forward,
            Limits::default(),
        );

        let reached: BTreeSet<&str> = trace.steps.iter().map(Step::work).collect();
        assert!(reached.contains("rashi"), "one hop");
        assert!(reached.contains("rambam"), "one hop");
        assert!(
            reached.contains("shulchan-arukh"),
            "two hops, and the second edge points backwards in time"
        );
        assert!(
            reached.contains("mishnah-berurah"),
            "three hops, and the third edge points backwards too"
        );
        assert!(
            !reached.contains("undated"),
            "the undated sefer is not walked through"
        );
        assert_eq!(trace.refused.undated, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_same_walk_backwards_gets_from_the_ruling_to_the_gemara() {
        let (root, timeline) = shas("girsa-chain-back");
        let repairs = Repairs::nowhere();
        let mut graph = Graph::new(&root, &timeline, &repairs);
        let trace = trace(
            &mut graph,
            &id("mishnah-berurah", 1),
            Direction::Back,
            Limits::default(),
        );
        let reached: BTreeSet<&str> = trace.steps.iter().map(Step::work).collect();
        assert!(reached.contains("shulchan-arukh"));
        assert!(reached.contains("rambam"));
        assert!(
            reached.contains("gemara"),
            "three hops back, through the code and the rishon"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_chain_with_one_unasserted_hop_is_not_a_transmission() {
        // 49% of the graph is `references` — *connected somehow*, and nothing
        // more. A chain through one of them is a coincidence of the corpus, and
        // this is the flag that stops it being drawn as scholarship.
        let root = std::env::temp_dir().join("girsa-chain-standing");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("works")).expect("makes the root");
        let body: String = [
            work("gemara", "c.450  – c.550 CE"),
            work("rishon", "c.1065  – c.1115 CE"),
            work("acharon", "1563 CE"),
        ]
        .iter()
        .map(|w| format!("{}\n", serde_json::to_string(w).expect("writes")))
        .collect();
        std::fs::write(root.join("works/index.jsonl"), body).expect("writes");

        let mut shard = store::Writer::default();
        let mut back = inbound::Writer::default();
        for e in [
            edge("gemara", 1, "rishon", 1, EdgeType::CommentsOn),
            edge("rishon", 1, "acharon", 1, EdgeType::References),
        ] {
            shard.push(&e);
            back.push(&e);
        }
        shard.flush(&root).expect("writes");
        back.flush(&root).expect("writes");

        let timeline = Timeline::of(&root).expect("reads");
        let repairs = Repairs::nowhere();
        let mut graph = Graph::new(&root, &timeline, &repairs);
        let trace = trace(
            &mut graph,
            &id("gemara", 1),
            Direction::Forward,
            Limits::default(),
        );

        let rishon = trace
            .steps
            .iter()
            .position(|s| s.work() == "rishon")
            .expect("one hop");
        let acharon = trace
            .steps
            .iter()
            .position(|s| s.work() == "acharon")
            .expect("two hops");
        assert!(trace.is_transmission(rishon), "one asserted hop");
        assert!(
            !trace.is_transmission(acharon),
            "the second hop only says the two are connected somehow"
        );
        assert_eq!(trace.weakest(acharon), Some(EdgeType::References));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_tree_with_no_inbound_cache_says_which_works_it_could_not_read() {
        // Half the graph is stored at the far end. A trace over a tree without
        // the cache is not wrong, it is short — and it has to say so, or a
        // sefer looks like a dead end because a batch job was not run.
        let (root, timeline) = shas("girsa-chain-unbuilt");
        std::fs::remove_file(root.join("links/inbound.built")).expect("unbuilds it");
        let repairs = Repairs::nowhere();
        let mut graph = Graph::new(&root, &timeline, &repairs);
        let trace = trace(
            &mut graph,
            &id("gemara", 1),
            Direction::Forward,
            Limits::default(),
        );
        assert!(
            trace.refused.incoming_unknown.contains("gemara"),
            "the walk says which works it read only half of"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_width_limit_is_counted_rather_than_silent() {
        let (root, timeline) = shas("girsa-chain-width");
        let repairs = Repairs::nowhere();
        let mut graph = Graph::new(&root, &timeline, &repairs);
        let trace = trace(
            &mut graph,
            &id("gemara", 1),
            Direction::Forward,
            Limits {
                depth: 1,
                width: 1,
                ..Limits::default()
            },
        );
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(
            trace.refused.over_budget, 1,
            "one of the two rishonim was dropped, and the answer says so"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_path_between_two_texts_is_found_in_either_direction() {
        let (root, timeline) = shas("girsa-chain-path");
        let repairs = Repairs::nowhere();
        let mut graph = Graph::new(&root, &timeline, &repairs);
        let found = path(
            &mut graph,
            &id("gemara", 1),
            &id("shulchan-arukh", 1),
            Limits::default(),
        );
        let Found::Path(links) = found else {
            panic!("there is a path: gemara → rambam → shulchan arukh");
        };
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].at.from.work(), "rambam");
        assert_eq!(links[1].at.from.work(), "shulchan-arukh");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_reachable_is_no_path_and_a_spent_budget_is_not() {
        // The distinction the whole enum exists for. A reader deciding whether
        // to believe two texts are unconnected is owed the difference between
        // *they are not* and *I stopped looking*.
        let (root, timeline) = shas("girsa-chain-nopath");
        let repairs = Repairs::nowhere();
        let mut graph = Graph::new(&root, &timeline, &repairs);

        let orphan = id("nowhere", 1);
        assert!(
            matches!(
                path(&mut graph, &orphan, &id("gemara", 1), Limits::default()),
                Found::None
            ),
            "nothing links to a sefer that is not in the graph"
        );
        assert!(
            matches!(
                path(
                    &mut graph,
                    &id("gemara", 1),
                    &id("mishnah-berurah", 1),
                    Limits {
                        budget: 1,
                        ..Limits::default()
                    }
                ),
                Found::NotWithin { .. }
            ),
            "a spent budget is not an answer about the graph"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_fork_is_two_readings_and_the_later_sefer_that_cites_both() {
        // Rashi and the Rambam both read the Gemara; the Mishnah Berurah cites
        // both. That is the shape — and nothing here says they disagree,
        // because nothing in the corpus says so.
        let (root, timeline) = shas("girsa-chain-fork");
        let repairs = Repairs::nowhere();
        let mut graph = Graph::new(&root, &timeline, &repairs);
        let (forks, _) = forks(&mut graph, &id("gemara", 1), Limits::default());

        let pair = forks
            .iter()
            .find(|f| {
                let works = [f.a_work(), f.b_work()];
                works.contains(&"rashi") && works.contains(&"rambam")
            })
            .expect("the two rishonim fork");
        assert_eq!(pair.witnesses.len(), 1);
        assert_eq!(pair.witnesses[0].from.work(), "mishnah-berurah");
        assert!(
            !pair.joined,
            "nothing joins Rashi to the Rambam here, so neither is answering the other"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_link_you_rejected_is_not_a_hop() {
        let (root, timeline) = shas("girsa-chain-rejected");
        let personal = root.join("personal");
        std::fs::create_dir_all(&personal).expect("makes your layer");
        let (mut repairs, trouble) = Repairs::open(&personal);
        assert!(trouble.is_empty());
        repairs
            .judge(
                &edge("gemara", 1, "rashi", 1, EdgeType::CommentsOn),
                crate::repair::Verdict::Rejected,
                "test",
            )
            .expect("writes your layer");
        let mut graph = Graph::new(&root, &timeline, &repairs);
        let trace = trace(
            &mut graph,
            &id("gemara", 1),
            Direction::Forward,
            Limits::default(),
        );
        assert!(
            !trace.steps.iter().any(|s| s.work() == "rashi"),
            "your layer said this link is wrong, so a chain does not walk it"
        );
        assert!(trace.refused.rejected > 0, "and the answer says so");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_index_offers_every_edge_a_full_scan_would_have_found() {
        // `Held::near` is a superset filter and `far_end` is still the only
        // thing that decides — so the property is that the index never *hides*
        // an edge the old full scan would have reached.
        //
        // The old scan was 156,076 rows of Orach Chayim per hop, re-walked up
        // to 73 times by a depth-3/width-8 trace.
        let ids: Vec<SegmentId> = (1..=8)
            .map(|n| {
                SegmentId::new(
                    "bavli/berakhot",
                    vec!["2a".into(), n.to_string()],
                    girsa_corpus::segment::Ordinal::root(n),
                )
            })
            .collect();
        let far = SegmentId::new(
            "bavli/rashi-on-berakhot",
            vec!["2a".into()],
            girsa_corpus::segment::Ordinal::root(1),
        );

        // A mix of point ends and span ends, which is what the corpus is.
        let mut edges = Vec::new();
        for (from, to) in [
            (Anchor::point(ids[1].clone()), Anchor::point(far.clone())),
            (
                Anchor::span(ids[2].clone(), ids[5].clone()),
                Anchor::point(far.clone()),
            ),
            (Anchor::point(far.clone()), Anchor::point(ids[6].clone())),
            (Anchor::point(ids[7].clone()), Anchor::point(far.clone())),
        ] {
            edges.push(Repaired::of(crate::Edge {
                from,
                to,
                edge_type: EdgeType::CommentsOn,
                method: crate::Method::SefariaSeed,
                direction: crate::Direction::NotRecorded,
                source_label: "commentary".into(),
            }));
        }
        let held = Held::of(edges);

        // Every anchor a reader could stand on, point or run.
        let mut asked = 0;
        for lower in 0..ids.len() {
            for upper in lower..ids.len() {
                let at = Anchor::span(ids[lower].clone(), ids[upper].clone());
                let scanned: Vec<usize> = held
                    .edges
                    .iter()
                    .enumerate()
                    .filter(|(_, repaired)| far_end(&at, repaired).is_some())
                    .map(|(which, _)| which)
                    .collect();
                let indexed: Vec<usize> = held
                    .near(&at)
                    .into_iter()
                    .filter(|which| {
                        held.edges
                            .get(*which)
                            .is_some_and(|repaired| far_end(&at, repaired).is_some())
                    })
                    .collect();
                assert_eq!(indexed, scanned, "the index hid an edge at {at}");
                asked += 1;
            }
        }
        assert!(asked > 30, "{asked} anchors is not a sweep");
    }
}
