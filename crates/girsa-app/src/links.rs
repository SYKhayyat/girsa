//! The links on the line you are standing on, and what you have said about
//! them.
//!
//! spec.md §8.3, BUILDER.md W23. Two halves: finding the edges that touch a
//! segment at all, and putting your repair layer over them.
//!
//! # Finding them without reading four million edges
//!
//! An edge is stored **once, in the shard of the work it points from** (§8.2),
//! so the outgoing half is one file: the shard of the sefer you are reading.
//! The incoming half is the hard one — *who comments on this se'if* is the
//! reverse direction, and answering it honestly would mean opening seven
//! thousand shards.
//!
//! It does not, because `girsa-companions` already wrote down which works share
//! edges with which. So the incoming half reads **only the shards of works that
//! are known to link here** — a handful to a few dozen — and that cache is the
//! difference between a sidebar and a spinner.
//!
//! When the cache is not there, the incoming half is **empty and says so**
//! rather than being quietly short: a link sidebar missing half its links is
//! indistinguishable, from the reader's chair, from a sefer nobody comments on.

use std::path::Path;

use girsa_corpus::segment::SegmentId;
use girsa_link::repair::{Repaired, Repairs};
use girsa_link::{store, Anchor};

use crate::shelf::Shelf;

/// One link on a segment, from that segment's point of view.
#[derive(Debug, Clone)]
pub struct Link {
    /// The edge as it stands after your layer, and what it was before.
    pub repaired: Repaired,
    /// Whether this segment is the end the edge points **from**.
    pub outgoing: bool,
    /// The other end.
    pub other: Anchor,
    /// The sefer at the other end, as the shelf names it.
    pub work: String,
    pub he_title: String,
    pub address: String,
}

impl Link {
    /// How the far end is cited, for the row.
    #[must_use]
    pub fn said(&self) -> String {
        if self.address.is_empty() {
            self.he_title.clone()
        } else {
            format!("{} {}", self.he_title, self.address)
        }
    }
}

/// What was asked, and what could not be answered.
#[derive(Debug, Clone)]
pub struct Touching {
    pub links: Vec<Link>,
    /// True when there is no companions cache, so the incoming half of the
    /// answer is missing. Shown, never swallowed.
    pub incoming_unknown: bool,
}

/// The links touching a segment: outgoing, incoming, and the ones you drew.
///
/// Sorted the way a reader wants to see them — the strongest claim first, then
/// by the sefer's name, so the list is stable between two openings of the same
/// line.
#[must_use]
pub fn touching(shelf: &Shelf, repairs: &Repairs, at: &SegmentId) -> Touching {
    let root = shelf.root();
    let mut links = Vec::new();

    // Outgoing: one file, the shard of the sefer you are reading.
    let mine = read_shard(root, at.work());
    for repaired in repairs.apply(mine) {
        if !repaired.edge.from.covers(at) {
            continue;
        }
        links.push(link(shelf, repaired, true));
    }

    // Incoming: only the works the companions cache says link here.
    //
    // Whether the cache exists at all is a different question from whether it
    // lists anything for this sefer, and answering the first with the second
    // would tell a reader "nothing links here" when the truth is "I have not
    // been told what does".
    let incoming_unknown = !shelf.has_companions();
    let companions = shelf.companions(at.work());
    for companion in &companions {
        for repaired in repairs.apply(read_shard(root, &companion.slug)) {
            if !repaired.edge.to.covers(at) {
                continue;
            }
            links.push(link(shelf, repaired, false));
        }
    }

    // …and the ones you drew, which are in no shard at all.
    for repaired in repairs.drawn_touching(at) {
        let outgoing = repaired.edge.from.covers(at);
        links.push(link(shelf, repaired, outgoing));
    }

    links.sort_by(|a, b| {
        b.repaired
            .confidence()
            .partial_cmp(&a.repaired.confidence())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.repaired.edge.edge_type.cmp(&b.repaired.edge.edge_type))
            .then_with(|| a.he_title.cmp(&b.he_title))
            .then_with(|| a.other.from.cmp(&b.other.from))
    });
    Touching {
        links,
        incoming_unknown,
    }
}

fn read_shard(root: &Path, slug: &str) -> Vec<girsa_link::Edge> {
    store::read_back(root, slug).unwrap_or_default()
}

fn link(shelf: &Shelf, repaired: Repaired, outgoing: bool) -> Link {
    let other = if outgoing {
        repaired.edge.to.clone()
    } else {
        repaired.edge.from.clone()
    };
    let work = other.from.work().to_string();
    let he_title = shelf
        .work(&work)
        .map_or_else(|| work.clone(), |w| w.he_title.clone());
    Link {
        address: other.from.path().join(":"),
        other,
        work,
        he_title,
        outgoing,
        repaired,
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::Ordinal;
    use girsa_link::{Edge, EdgeType, Method};

    fn id(work: &str, n: u32) -> SegmentId {
        SegmentId::new(work, vec!["1".into(), n.to_string()], Ordinal::root(n))
    }

    #[test]
    fn a_link_names_the_end_that_is_not_the_one_you_are_standing_on() {
        let repairs = Repairs::nowhere();
        let edge = Edge {
            from: Anchor::point(id("mishnah/berakhot", 1)),
            to: Anchor::point(id("rambam/berakhot", 5)),
            edge_type: EdgeType::CommentsOn,
            method: Method::SefariaSeed,
            source_label: "commentary".into(),
        };
        let shelf = crate::shelf::tests::shelf_of(
            vec![
                crate::shelf::tests::work("mishnah/berakhot"),
                crate::shelf::tests::work("rambam/berakhot"),
            ],
            &crate::shelf::tests::scratch("girsa-links-test"),
        );

        let repaired = repairs.apply(vec![edge.clone()]).remove(0);
        let outgoing = link(&shelf, repaired.clone(), true);
        assert_eq!(outgoing.work, "rambam/berakhot");
        assert_eq!(outgoing.address, "1:5");

        let incoming = link(&shelf, repaired, false);
        assert_eq!(incoming.work, "mishnah/berakhot");
        assert_eq!(incoming.address, "1:1");
    }
}
