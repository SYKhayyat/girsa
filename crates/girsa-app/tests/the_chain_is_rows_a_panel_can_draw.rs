//! spec.md §8, reachable from the window at last (W28).
//!
//! The walk has existed since W28 and so has `girsa-chain`, which prints it on
//! a terminal. **Nothing in the window drew either**, so the whole of §8 was a
//! tier a reader could only see by leaving the application — and `BUILDER.md`
//! §0.3 says a work order is not done until a reader can reach it.
//!
//! What is asserted here is that turning the walk into rows did not turn it
//! into a second opinion. The hops are `girsa-link`'s; what this crate adds is
//! naming, the tree's shape, and the three judgements a panel must not make for
//! itself:
//!
//! 1. **whether a chain is a transmission** — 49% of this graph is
//!    `references`, which says only that two places are connected somehow, and
//!    a panel that drew those like `quotes` would be presenting a shrug as a
//!    mesorah;
//! 2. **what the weakest hop on the way claims**, which is what the whole chain
//!    is worth;
//! 3. **what the walk refused**, which is part of the answer and not a
//!    footnote: *nine of the eleven seforim that read this line could not be
//!    dated* changes what the chain above it means.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use girsa_app::chaining;
use girsa_app::naming::Names;
use girsa_app::session::Language;
use girsa_app::shelf::Shelf;
use girsa_corpus::era::Timeline;
use girsa_corpus::segment::SegmentId;
use girsa_link::chain::{Direction, Graph, Limits};

const MISHNAH: &str = "mishnah-berakhot";

fn corpus() -> &'static Path {
    girsa_fixture::linked().root()
}

fn first_mishnah(shelf: &Shelf) -> SegmentId {
    let sefer = shelf.read(MISHNAH).expect("Berakhot is on the shelf");
    sefer
        .segments
        .iter()
        .find(|s| {
            s.id.path().first().map(String::as_str) == Some("1")
                && s.id.path().get(1).map(String::as_str) == Some("1")
        })
        .map(|s| s.id.clone())
        .expect("Mishnah Berakhot 1:1 is on the shelf")
}

/// The window's own three objects, in the order the shell builds them.
struct Panel {
    shelf: Shelf,
    timeline: Timeline,
}

impl Panel {
    /// Named per test: these run in parallel, and two of them removing and
    /// creating one directory is a race that fails whichever loses.
    fn open(name: &str) -> Self {
        let personal = std::env::temp_dir().join(format!("girsa-chain-{name}"));
        let _ = std::fs::remove_dir_all(&personal);
        std::fs::create_dir_all(&personal).expect("a personal layer");
        let shelf = Shelf::open(corpus(), &personal).expect("the shelf opens");
        // Both roots, so a note of yours can be a hop rather than being
        // `Unknown` against every sefer on the shelf.
        let timeline = Timeline::across(corpus(), &personal).expect("the catalogue reads");
        Self { shelf, timeline }
    }

    fn names(&self) -> Names<'_> {
        Names::new(
            &self.shelf,
            Some(&self.timeline),
            Language::Hebrew,
            girsa_cite::CiteStyle::HebrewFull,
        )
    }

    fn graph(&self) -> Graph<'_> {
        Graph::new(corpus(), &self.timeline, self.shelf.repairs())
    }

    /// Where the reader stands, under every name those words have carried.
    fn standing(&self, at: &SegmentId) -> girsa_corpus::standing::Standing {
        let sefer = self.shelf.read(at.work()).expect("the sefer opens");
        sefer.standing(at)
    }
}

#[test]
fn walking_forward_from_a_mishnah_names_the_sefer_that_read_it() {
    let panel = Panel::open("walking_forward_from_a_mishnah_names_the_sefer_that_read_it");
    let at = first_mishnah(&panel.shelf);
    let names = panel.names();
    let mut graph = panel.graph();

    let chain = chaining::walk(
        &mut graph,
        &names,
        &at,
        Direction::Forward,
        Limits::default(),
        &panel.standing(&at),
    );

    assert_eq!(chain.direction, "forward");
    assert_eq!(chain.start, at.to_string());
    assert!(!chain.title.is_empty(), "the start is named, never a slug");
    assert!(
        !chain.hops.is_empty(),
        "the Rambam read this mishnah, and the graph says so"
    );
    // Every row carries what a reader needs to act on it: a name, a place, and
    // an id to open. A row with a slug in it is a row nobody can read.
    for hop in &chain.hops {
        assert!(!hop.title.is_empty(), "{} has no title", hop.at);
        assert!(!hop.at.is_empty());
        assert!(!hop.edge_type.is_empty());
    }
    assert_eq!(
        chain.chains,
        chain.hops.iter().filter(|hop| hop.end).count(),
        "a chain is a hop nothing was reached from, counted the same way twice"
    );
}

#[test]
fn a_hop_off_the_start_has_no_parent_and_the_rest_point_at_one() {
    // The tree's shape, which is the whole difference between a panel and a
    // wall of text: without it the same three seforim are redrawn under every
    // leaf below them, and a walk eight rows wide becomes two hundred rows.
    let panel = Panel::open("a_hop_off_the_start_has_no_parent_and_the_rest_point_at_one");
    let at = first_mishnah(&panel.shelf);
    let names = panel.names();
    let mut graph = panel.graph();

    let chain = chaining::walk(
        &mut graph,
        &names,
        &at,
        Direction::Forward,
        Limits::default(),
        &panel.standing(&at),
    );

    for (i, hop) in chain.hops.iter().enumerate() {
        match hop.parent {
            None => assert_eq!(hop.depth, 1, "a hop straight off the start is one deep"),
            Some(parent) => {
                assert!(parent < i, "a parent is always an earlier row");
                let above = &chain.hops[parent];
                assert_eq!(
                    hop.depth,
                    above.depth + 1,
                    "one step deeper than its parent"
                );
            }
        }
    }
}

#[test]
fn a_chain_is_only_a_transmission_when_every_hop_claims_something() {
    // The judgement a panel must not make for itself. `references` is 49% of
    // this graph and asserts nothing at all, so a chain that passes through one
    // is a chain of *these are connected somehow* — and the weakest hop on the
    // way is what the whole thing is worth.
    let panel = Panel::open("a_chain_is_only_a_transmission_when_every_hop_claims_something");
    let at = first_mishnah(&panel.shelf);
    let names = panel.names();
    let mut graph = panel.graph();

    let chain = chaining::walk(
        &mut graph,
        &names,
        &at,
        Direction::Forward,
        Limits::default(),
        &panel.standing(&at),
    );

    assert!(
        chain.transmissions <= chain.chains,
        "a transmission is a kind of chain, not an extra one"
    );
    for hop in &chain.hops {
        assert!(
            hop.weakest.is_some(),
            "every hop has a chain behind it, so something is the weakest link on it"
        );
        if hop.transmission {
            assert_ne!(
                hop.edge_type, "references",
                "a hop that asserts nothing cannot be part of a transmission"
            );
        }
    }
}

#[test]
fn what_the_walk_would_not_follow_comes_back_with_the_answer() {
    // Not diagnostics. A reader who cannot see this number is reading a chain
    // that looks complete, and `girsa-chain` ends every command with the same
    // paragraph for the same reason.
    let panel = Panel::open("what_the_walk_would_not_follow_comes_back_with_the_answer");
    let at = first_mishnah(&panel.shelf);
    let names = panel.names();
    let mut graph = panel.graph();

    let chain = chaining::walk(
        &mut graph,
        &names,
        &at,
        Direction::Forward,
        Limits::default(),
        &panel.standing(&at),
    );
    let left = &chain.left_out;
    let counted = left.undated
        + left.wrong_way
        + left.contemporary
        + left.over_budget
        + left.rejected
        + left.incoming_unknown;
    assert_eq!(
        left.nothing,
        counted == 0,
        "*nothing was left out* and *no reason was counted* are one fact said twice"
    );
}

#[test]
fn walking_back_is_the_other_direction_and_says_so() {
    let panel = Panel::open("walking_back_is_the_other_direction_and_says_so");
    let at = first_mishnah(&panel.shelf);
    let names = panel.names();
    let mut graph = panel.graph();

    let back = chaining::walk(
        &mut graph,
        &names,
        &at,
        Direction::Back,
        Limits::default(),
        &panel.standing(&at),
    );
    assert_eq!(back.direction, "back");
    // The mishnah is early, so walking back from it finds little or nothing —
    // and *nothing this walk could follow* is an answer, not a failure. What is
    // asserted is that it is the same shape of answer either way.
    assert_eq!(
        back.chains,
        back.hops.iter().filter(|hop| hop.end).count(),
        "the count is the same count in both directions"
    );
}

#[test]
fn a_fork_names_both_readings_and_says_it_is_not_a_machlokes() {
    // spec.md §8.6, and the honest limit on it: the graph has no `disputes`
    // edge anywhere, so nothing in the data says two seforim disagree. Two of
    // them read one line and a later one had to deal with both, which is the
    // shape a machlokes leaves behind. The panel says so above the list; this
    // asserts the rows underneath it are well formed.
    let panel = Panel::open("a_fork_names_both_readings_and_says_it_is_not_a_machlokes");
    let at = first_mishnah(&panel.shelf);
    let names = panel.names();
    let mut graph = panel.graph();

    let forked = chaining::forked(
        &mut graph,
        &names,
        &at,
        Limits::default(),
        &panel.standing(&at),
    );
    assert_eq!(forked.start, at.to_string());
    for fork in &forked.forks {
        assert_ne!(
            fork.a.work, fork.b.work,
            "two lines of one sefer are not two readings of the sugya"
        );
        assert!(!fork.a.title.is_empty());
        assert!(!fork.b.title.is_empty());
    }
}
