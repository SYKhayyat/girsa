//! The transmission chain, as rows a panel can draw (spec.md §8, W28).
//!
//! `girsa-link::chain` walks the graph and `girsa-chain` prints the walk on a
//! terminal. Both have existed since W28 and **nothing in the window drew
//! either of them** — which made the whole of spec.md §8 a feature you could
//! only see by leaving the application, and `BUILDER.md` §0.3's *a work order
//! is not done until a reader can reach it* says that is not done.
//!
//! What was missing is this file: the walk turned into rows. Not the walk
//! itself, which is `girsa-link`'s and is shared — a panel and a terminal that
//! disagreed about which hops are real would be two answers to *how did this
//! become halacha*, and the shape of the answer is the whole claim.
//!
//! # Three things every row carries, and why
//!
//! - **What the hop claims.** `edge_type` after your repair layer, not as the
//!   corpus stored it. 49% of this graph is `references`, which says only that
//!   two places are connected somehow — so a chain is marked as a
//!   *transmission* only when every hop along it asserts something, and the
//!   weakest link on the way is named. A panel that drew all chains alike would
//!   be presenting half a graph of shrugs as a mesorah.
//! - **When.** The era a reader recognises and the years that order the hops,
//!   both, because they answer different questions — and *no date* is a state
//!   with a name rather than a blank cell, since a blank reads as *earlier than
//!   the row above*.
//! - **Whether it is yours.** A link you drew or confirmed is not the corpus's
//!   claim, and a panel that hid that would be presenting your own guess back
//!   to you as evidence.
//!
//! # And what the walk refused
//!
//! Carried on the answer, not logged. *Nine of the eleven seforim that read
//! this line could not be dated* changes what the chain above it means, and a
//! reader who cannot see that number is reading a chain that looks complete.
//! `girsa-chain` ends every command with this paragraph; so does the panel.

use girsa_corpus::segment::SegmentId;
use girsa_corpus::standing::Standing;
use girsa_link::chain::{self, Direction, Limits, Refused, Trace};
use girsa_link::Anchor;
use serde::Serialize;

use crate::naming::Names;

/// One place the walk reached, and the link it got there by.
#[derive(Debug, Clone, Serialize)]
pub struct Hop {
    /// Which hop this was reached from, as an index into [`Chain::hops`].
    /// `None` for a step straight off the start, so a view can draw the tree
    /// the terminal draws.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<usize>,
    pub depth: usize,
    /// The segment id, for opening it.
    pub at: String,
    /// The whole anchor as text, which is what a run link needs when the hop is
    /// onto a span of words rather than a whole segment.
    pub anchor: String,
    pub work: String,
    pub title: String,
    pub address: String,
    /// `1565`, or `1488–1575`. Absent where the corpus cannot date the sefer,
    /// and then `era` may still say something.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub era: Option<String>,
    /// What the link claims, after your repairs — `quotes`, `explains`, and so
    /// on down to `references`.
    pub edge_type: &'static str,
    /// What the corpus called it, verbatim. Blank for three quarters of them
    /// (T5), and the difference between *the corpus said nothing* and *the
    /// corpus said `related`*.
    pub label: String,
    pub confidence: f32,
    /// You drew this link, or confirmed it.
    pub mine: bool,
    /// Every hop from the start to here asserts something.
    pub transmission: bool,
    /// The weakest claim on the chain to here — what the whole chain is worth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weakest: Option<&'static str>,
    /// Nothing was reached from this hop: it is the far end of a chain.
    pub end: bool,
}

/// What a walk would not follow, in numbers a reader can read.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LeftOut {
    pub undated: usize,
    pub wrong_way: usize,
    pub contemporary: usize,
    pub over_budget: usize,
    pub rejected: usize,
    /// Seforim whose incoming half could not be read, because the inbound cache
    /// has not been built. Every one is a place the walk may have missed a hop.
    pub incoming_unknown: usize,
    /// Nothing at all was left out.
    pub nothing: bool,
}

impl LeftOut {
    fn of(refused: &Refused) -> Self {
        Self {
            undated: refused.undated,
            wrong_way: refused.wrong_way,
            contemporary: refused.contemporary,
            over_budget: refused.over_budget,
            rejected: refused.rejected,
            incoming_unknown: refused.incoming_unknown.len(),
            nothing: refused.is_empty(),
        }
    }
}

/// A walk out from one place, ready to draw.
#[derive(Debug, Clone, Serialize)]
pub struct Chain {
    pub start: String,
    pub title: String,
    pub address: String,
    /// `forward` or `back`.
    pub direction: &'static str,
    pub hops: Vec<Hop>,
    /// How many chains reached an end, and how many of those assert something
    /// at every hop. The second number is the honest one.
    pub chains: usize,
    pub transmissions: usize,
    pub left_out: LeftOut,
    pub works_read: usize,
}

/// Two seforim that read one line, and who had to deal with both (spec.md §8.6).
#[derive(Debug, Clone, Serialize)]
pub struct Fork {
    pub a: Side,
    pub b: Side,
    /// Later places that had to deal with both sides, nearest first — the
    /// reason to think the two readings were ever argued out.
    pub witnesses: Vec<Seen>,
    /// A link joins the two sides directly, so one is answering the other
    /// rather than the two passing each other. A different thing to look at,
    /// and marked rather than merged.
    pub joined: bool,
}

/// A witness to a fork, and how far down it is.
///
/// `steps` carries the whole difference between *these two were argued out on
/// one page* and *these two are both somewhere above a sefer six hops down*.
/// A panel that drew those alike would be inventing the first out of the
/// second, so the number travels rather than being flattened into a count.
#[derive(Debug, Clone, Serialize)]
pub struct Seen {
    #[serde(flatten)]
    pub side: Side,
    /// Hops to whichever reading is further. `1` is a sefer that quotes both.
    pub steps: usize,
}

/// One side of a fork, or the place a witness is.
#[derive(Debug, Clone, Serialize)]
pub struct Side {
    pub at: String,
    pub work: String,
    pub title: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub era: Option<String>,
}

/// The forks below a place, and what the walk refused.
#[derive(Debug, Clone, Serialize)]
pub struct Forked {
    pub start: String,
    pub title: String,
    pub address: String,
    pub forks: Vec<Fork>,
    pub left_out: LeftOut,
    pub works_read: usize,
}

fn side(names: &Names<'_>, at: &Anchor) -> Side {
    let named = names.of(&at.from);
    Side {
        at: at.from.to_string(),
        work: named.work,
        title: named.title,
        address: named.address,
        written: named.written,
        era: named.era,
    }
}

/// Walk, and turn the walk into rows.
///
/// The `Graph` is the caller's because it is expensive to build and worth
/// keeping: it holds every edge touching a work, and a panel that rebuilt it
/// per question would re-read an 8 MB shard for every click.
///
/// `standing` is where the reader stands — the one place in the walk that is a
/// live segment with a shelf. The first hop is resolved by it exactly as the
/// links panel resolves its own list, so a re-segmentation between the reader
/// and a known relation cannot drop the hop (Lamdan 1).
pub fn walk(
    graph: &mut chain::Graph<'_>,
    names: &Names<'_>,
    at: &SegmentId,
    direction: Direction,
    limits: Limits,
    standing: &Standing,
) -> Chain {
    let trace = chain::trace(graph, at, direction, limits, Some(standing));
    let start = names.of(at);
    let ends = trace.ends();
    let transmissions = ends.iter().filter(|i| trace.is_transmission(**i)).count();
    Chain {
        start: at.to_string(),
        title: start.title,
        address: start.address,
        direction: match direction {
            Direction::Forward => "forward",
            Direction::Back => "back",
        },
        hops: hops(&trace, names),
        chains: ends.len(),
        transmissions,
        left_out: LeftOut::of(&trace.refused),
        works_read: graph.works_read(),
    }
}

fn hops(trace: &Trace, names: &Names<'_>) -> Vec<Hop> {
    let ends: std::collections::BTreeSet<usize> = trace.ends().into_iter().collect();
    trace
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let named = names.of(&step.at.from);
            Hop {
                parent: step.parent,
                depth: step.depth,
                at: step.at.from.to_string(),
                anchor: step.at.to_string(),
                work: named.work,
                title: named.title,
                address: named.address,
                written: named.written,
                era: named.era,
                edge_type: step.edge_type.as_str(),
                label: step.label.clone(),
                confidence: step.confidence,
                mine: step.mine,
                transmission: trace.is_transmission(i),
                weakest: trace.weakest(i).map(girsa_link::EdgeType::as_str),
                end: ends.contains(&i),
            }
        })
        .collect()
}

/// The forks below a place, as rows.
///
/// `standing` is where the reader stands, and reaches the walk's first hop the
/// same way it does for [`walk`] — a re-segmentation cannot lose a reading of
/// the line the reader is actually on.
pub fn forked(
    graph: &mut chain::Graph<'_>,
    names: &Names<'_>,
    at: &SegmentId,
    limits: Limits,
    standing: &Standing,
) -> Forked {
    let (found, refused) = chain::forks(graph, at, limits, Some(standing));
    let start = names.of(at);
    Forked {
        start: at.to_string(),
        title: start.title,
        address: start.address,
        forks: found
            .iter()
            .map(|fork| Fork {
                a: side(names, &fork.a),
                b: side(names, &fork.b),
                witnesses: fork
                    .witnesses
                    .iter()
                    .map(|w| Seen {
                        side: side(names, &w.at),
                        steps: w.steps,
                    })
                    .collect(),
                joined: fork.joined,
            })
            .collect(),
        left_out: LeftOut::of(&refused),
        works_read: graph.works_read(),
    }
}
