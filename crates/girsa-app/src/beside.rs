//! Where the column beside you goes when you move.
//!
//! This is the whole of BUILDER.md W9's acceptance — *scrolling the Gemara
//! moves the Rashi column to the matching ref* — and it is one question asked
//! over and over: **given a segment of one sefer, which segments of the other
//! one sit against it?**
//!
//! # Three answers, and the third is the important one
//!
//! ```text
//! Place::At(ids)   there, and here is exactly where
//! Place::NoPlace   these two are related and this line has nothing beside it
//! Place::Unrelated nothing joins these two seforim; the column does not move
//! ```
//!
//! Rashi does not comment on every line. A pane that slid to the *nearest*
//! comment would show a reader Rashi on a different line, with the header still
//! naming the line they are on and nothing anywhere saying it had moved. That
//! is BUILDER.md rule 6 — a guess presented as a place — in the one spot a
//! reader would never think to check, so [`Place::NoPlace`] exists and the
//! column stays where it is.
//!
//! # What counts as related
//!
//! Only two things, and neither is a resemblance:
//!
//! 1. **The corpus declares it.** Sefaria's schema for `Rashi on Berakhot`
//!    carries `base_text_titles: [Berakhot]`; 5,436 works say this about
//!    themselves. Once it is declared, the addresses line up by construction —
//!    `Rashi on Berakhot 2a:1:3` is the third comment on `Berakhot 2a:1`, the
//!    base text's address with a level added — and reading that off is reading,
//!    not guessing.
//! 2. **W8 imported an edge** between two of their segments.
//!
//! Two seforim with neither are left alone even though half the corpus is
//! addressed `1:1` and would line up beautifully.

use std::collections::HashMap;
use std::path::Path;

use girsa_corpus::segment::SegmentId;
use girsa_link::store;

use crate::shelf::{address_of, Open};

/// What relates two open seforim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// The corpus declares one a commentary on the other, and the addresses of
    /// the commentary extend the addresses of its base.
    Declared {
        /// Whether the *following* pane is the commentary. The addresses run
        /// the other way when it is the base text.
        follower_is_commentary: bool,
    },
    /// No declaration, but edges join their segments.
    Linked,
    /// Nothing joins them.
    Unrelated,
}

impl Relation {
    #[must_use]
    pub const fn is_declared(self) -> bool {
        matches!(self, Self::Declared { .. })
    }

    #[must_use]
    pub const fn is_linked(self) -> bool {
        matches!(self, Self::Linked)
    }

    /// Whether the two panes can follow each other at all.
    #[must_use]
    pub const fn can_follow(self) -> bool {
        !matches!(self, Self::Unrelated)
    }
}

/// Where the follower pane belongs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "ids")]
pub enum Place {
    /// The segments that sit against the leader's, in reading order.
    At(Vec<SegmentId>),
    /// Related, and nothing here. The column stays where it is and says so.
    NoPlace,
    /// Nothing relates the two seforim.
    Unrelated,
}

impl Place {
    /// The first segment to scroll to, if there is one.
    #[must_use]
    pub fn first(&self) -> Option<&SegmentId> {
        match self {
            Self::At(ids) => ids.first(),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_somewhere(&self) -> bool {
        matches!(self, Self::At(ids) if !ids.is_empty())
    }
}

/// One sefer placed against another, ready to answer the question repeatedly.
///
/// Built once per pair of open panes. The edge lookup is a map built from the
/// two works' shards — a few hundred rows for a masechta — and not from the
/// four million edge corpus, which nobody needs in a window.
#[derive(Debug)]
pub struct Beside<'a> {
    relation: Relation,
    /// The sefer being placed. Borrowed rather than copied: it holds a
    /// masechta's worth of text and this lives exactly as long as the question
    /// being asked of it.
    follower: &'a Open,
    /// Leader segment → follower segments, for whatever edges join the two.
    /// Held even when the relation is declared: an edge is a fact somebody
    /// recorded, and a declared commentary that also has edges should use them
    /// where the addresses have nothing.
    edges: HashMap<SegmentId, Vec<SegmentId>>,
}

impl<'a> Beside<'a> {
    /// Work out how the follower relates to the leader, and load what it takes
    /// to answer [`Beside::place`].
    ///
    /// `root` is the corpus root; the link shards are read from under it. A
    /// shard that will not read costs the edges and not the relation — a
    /// declared commentary still lines up by address.
    #[must_use]
    pub fn between(leader: &Open, follower: &'a Open, root: &Path) -> Self {
        let declared = if follower
            .work
            .commentary_on
            .iter()
            .any(|b| b.slug == leader.slug())
        {
            Some(true)
        } else if leader
            .work
            .commentary_on
            .iter()
            .any(|b| b.slug == follower.slug())
        {
            Some(false)
        } else {
            None
        };

        let edges = edges_between(root, leader, follower);
        let relation = match declared {
            Some(follower_is_commentary) => Relation::Declared {
                follower_is_commentary,
            },
            None if edges.is_empty() => Relation::Unrelated,
            None => Relation::Linked,
        };
        Self {
            relation,
            follower,
            edges,
        }
    }

    #[must_use]
    pub const fn relation(&self) -> Relation {
        self.relation
    }

    /// Where the follower goes when the leader is at `at`.
    #[must_use]
    pub fn place(&self, at: &SegmentId) -> Place {
        let follower = self.follower;
        match self.relation {
            Relation::Unrelated => Place::Unrelated,
            Relation::Linked => self.by_edge(at),
            Relation::Declared {
                follower_is_commentary,
            } => {
                let found = if follower_is_commentary {
                    // The commentary's address is the base text's with levels
                    // added, so everything under it is what sits here.
                    follower.at(&address_of(at))
                } else {
                    // Reading the commentary, following the sefer it is on:
                    // drop the levels the commentary added until an address the
                    // base text actually has is reached. `2a:1:3` → `2a:1`.
                    // Dropping stops at the first hit rather than continuing to
                    // `2a`, which would put the base pane on the top of the daf
                    // every time.
                    let mut levels: Vec<_> = address_of(at).levels().to_vec();
                    loop {
                        let hit = follower.at(&girsa_ref::Address::new(levels.clone()));
                        if !hit.is_empty() {
                            break hit;
                        }
                        if levels.pop().is_none() {
                            break Vec::new();
                        }
                    }
                };
                if found.is_empty() {
                    // Declared, and the addresses have nothing here. An edge is
                    // still a fact somebody recorded, so it is used before
                    // giving up.
                    return self.by_edge(at);
                }
                Place::At(found)
            }
        }
    }

    fn by_edge(&self, at: &SegmentId) -> Place {
        match self.edges.get(at) {
            Some(ids) if !ids.is_empty() => Place::At(ids.clone()),
            _ => Place::NoPlace,
        }
    }
}

/// Every edge joining the two works, in both directions, as leader → follower.
///
/// spec.md §8.2 stores an edge once, in the direction it was written, so both
/// shards are read: `Rashi on X` points at `X`, and a sefer that quotes it
/// points the other way. Neither shard is a superset of the other.
fn edges_between(
    root: &Path,
    leader: &Open,
    follower: &Open,
) -> HashMap<SegmentId, Vec<SegmentId>> {
    let mut out: HashMap<SegmentId, Vec<SegmentId>> = HashMap::new();
    for slug in [leader.slug(), follower.slug()] {
        let Ok(edges) = store::read_back(root, slug) else {
            continue;
        };
        for edge in edges {
            // An endpoint can be a run of segments (`Rashi on Berakhot 2a`
            // covers a daf), so both ends are expanded to the segments they
            // actually name before being joined.
            let (from, to) = (&edge.from, &edge.to);
            for (near, far) in [(from, to), (to, from)] {
                if near.from.work() != leader.slug() || far.from.work() != follower.slug() {
                    continue;
                }
                let far_ids = expand(far, follower);
                if far_ids.is_empty() {
                    continue;
                }
                for id in expand(near, leader) {
                    out.entry(id).or_default().extend(far_ids.iter().cloned());
                }
            }
        }
    }
    for ids in out.values_mut() {
        ids.sort();
        ids.dedup();
    }
    out
}

/// The segments an anchor names, in reading order.
///
/// Through [`store::span_of`], which is the one implementation of *which
/// segments does this endpoint cover* — the indexer asks it too, to work out
/// which kinds of link touch a segment (spec.md §9.8). Two answers that drifted
/// would put a commentary beside a line the graph does not join it to in one
/// place and count it in the other.
fn expand(anchor: &girsa_link::Anchor, work: &Open) -> Vec<SegmentId> {
    let Some(range) = girsa_link::touching::span_of(anchor, |id| work.position_of(id)) else {
        return Vec::new();
    };
    work.segments
        .get(range)
        .map(|run| run.iter().map(|s| s.id.clone()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::shelf::tests::open;
    use girsa_corpus::work::{BaseText, Mapping};

    /// Berakhot 2a, three lines, and a Rashi on two of them.
    fn gemara() -> Open {
        open(
            "bavli/berakhot",
            &[&["2a", "1"], &["2a", "2"], &["2a", "3"], &["2b", "1"]],
        )
    }

    fn rashi() -> Open {
        let mut rashi = open(
            "bavli/rashi-on-berakhot",
            &[
                &["2a", "1", "1"],
                &["2a", "1", "2"],
                &["2a", "3", "1"],
                &["2b", "1", "1"],
            ],
        );
        rashi.work.commentary_on = vec![BaseText {
            slug: "bavli/berakhot".into(),
            mapping: Mapping::ManyToOne,
        }];
        rashi
    }

    fn nowhere() -> &'static Path {
        Path::new("no-such-corpus")
    }

    fn place(beside: &Beside, at: &str) -> Place {
        beside.place(&at.parse().expect("a segment id"))
    }

    #[test]
    fn the_commentary_column_lands_on_every_comment_on_the_line() {
        let (gemara, rashi) = (gemara(), rashi());
        let beside = Beside::between(&gemara, &rashi, nowhere());
        assert_eq!(
            beside.relation(),
            Relation::Declared {
                follower_is_commentary: true
            }
        );

        let at = place(&beside, "girsa:bavli/berakhot/2a:1#1");
        assert_eq!(
            at,
            Place::At(vec![
                "girsa:bavli/rashi-on-berakhot/2a:1:1#1".parse().unwrap(),
                "girsa:bavli/rashi-on-berakhot/2a:1:2#2".parse().unwrap(),
            ]),
            "both comments on the line, in order"
        );
    }

    #[test]
    fn a_line_with_no_comment_on_it_says_so_rather_than_moving_to_the_nearest() {
        // The whole reason `NoPlace` exists. Line 2a:2 has no Rashi; the
        // nearest comment is on 2a:3, and showing it here would be showing a
        // reader the wrong Rashi with the header naming the right line.
        let (gemara, rashi) = (gemara(), rashi());
        let beside = Beside::between(&gemara, &rashi, nowhere());
        assert_eq!(
            place(&beside, "girsa:bavli/berakhot/2a:2#2"),
            Place::NoPlace
        );
    }

    #[test]
    fn reading_the_commentary_moves_the_gemara_to_the_line_it_is_on() {
        // The same pair, followed the other way — which is what happens when
        // you put your cursor in the Rashi column and scroll it.
        let (gemara, rashi) = (gemara(), rashi());
        let beside = Beside::between(&rashi, &gemara, nowhere());
        assert_eq!(
            beside.relation(),
            Relation::Declared {
                follower_is_commentary: false
            }
        );
        assert_eq!(
            place(&beside, "girsa:bavli/rashi-on-berakhot/2a:1:2#2"),
            Place::At(vec!["girsa:bavli/berakhot/2a:1#1".parse().unwrap()]),
        );
    }

    #[test]
    fn the_base_pane_does_not_jump_to_the_top_of_the_daf() {
        // Dropping levels to find the base text's address has to stop at the
        // first address that exists. `2a:3:1` → `2a:3`; carrying on to `2a`
        // would answer every comment on the daf with the first line of it, and
        // the pane would look stuck.
        let (gemara, rashi) = (gemara(), rashi());
        let beside = Beside::between(&rashi, &gemara, nowhere());
        assert_eq!(
            place(&beside, "girsa:bavli/rashi-on-berakhot/2a:3:1#3"),
            Place::At(vec!["girsa:bavli/berakhot/2a:3#3".parse().unwrap()]),
        );
    }

    #[test]
    fn two_seforim_that_merely_share_an_address_shape_are_left_alone() {
        // Both are addressed `2a:1`. Nothing declares them related and no edge
        // joins them, so the panes do not move each other — even though the
        // lookup would succeed and look perfect.
        let gemara = gemara();
        let other = open("bavli/shabbat", &[&["2a", "1"], &["2a", "2"]]);
        let beside = Beside::between(&gemara, &other, nowhere());
        assert_eq!(beside.relation(), Relation::Unrelated);
        assert_eq!(
            place(&beside, "girsa:bavli/berakhot/2a:1#1"),
            Place::Unrelated
        );
    }

    #[test]
    fn an_edge_places_a_pane_the_addresses_cannot() {
        // The 978 Otzaria-only works have no schema and so declare nothing.
        // What relates them to anything is the edges W8 imported, and an
        // address correspondence between them and anything else would be
        // invented.
        let dir = std::env::temp_dir().join("girsa-app-beside-edges");
        let _ = std::fs::remove_dir_all(&dir);

        let gemara = gemara();
        let acharon = open("קרן-אורה-על-ברכות", &[&["1"], &["2"]]);
        let mut writer = store::Writer::default();
        writer.push(&girsa_link::Edge {
            from: girsa_link::Anchor::point("girsa:קרן-אורה-על-ברכות/2#2".parse().unwrap()),
            to: girsa_link::Anchor::point("girsa:bavli/berakhot/2a:3#3".parse().unwrap()),
            edge_type: girsa_link::EdgeType::CommentsOn,
            method: girsa_link::Method::OtzariaSeed,
            source_label: "commentary".into(),
        });
        writer.flush(&dir).expect("writes");

        let beside = Beside::between(&gemara, &acharon, &dir);
        assert_eq!(beside.relation(), Relation::Linked);
        assert_eq!(
            place(&beside, "girsa:bavli/berakhot/2a:3#3"),
            Place::At(vec!["girsa:קרן-אורה-על-ברכות/2#2".parse().unwrap()]),
        );
        // And a line the edge says nothing about stays a `NoPlace`.
        assert_eq!(
            place(&beside, "girsa:bavli/berakhot/2a:1#1"),
            Place::NoPlace
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
