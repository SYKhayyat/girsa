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
//! It does not, because W28's `inbound.jsonl` holds every edge that lands on a
//! work, filed under that work. So the incoming half is **one more file**, and
//! that cache is the difference between a sidebar and a spinner.
//!
//! Until W28 the incoming half read the shards of every work the companions
//! cache listed as joined to this one — which was a handful to a few dozen
//! files, and was capped: `girsa-companions` keeps the top 200 works per sefer,
//! so a line in Berakhot, which is joined to 1,600 works, could have its rarer
//! commentaries silently missing. One file has no cap.
//!
//! When the cache is not there, the incoming half is **empty and says so**
//! rather than being quietly short: a link sidebar missing half its links is
//! indistinguishable, from the reader's chair, from a sefer nobody comments on.

use std::path::Path;

use girsa_corpus::segment::SegmentId;
use girsa_corpus::standing::Standing;
use girsa_link::repair::{Repaired, Repairs};
use girsa_link::{inbound, store, Anchor};

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
    /// The same sefer's other name, so the window can print whichever the reader
    /// asked for (W41) without going back to the shelf per row.
    pub en_title: String,
    pub address: String,
    /// Which words of **the segment you are standing on** this link is about
    /// (spec.md §8.4, W24), as characters of the text the pane drew.
    ///
    /// `None` when nothing says: the link is on the whole segment, which is
    /// what the shipped data addresses and what most links will say forever.
    pub span: Option<std::ops::Range<usize>>,
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
///
/// `at` is a [`Standing`] rather than an id, and that is the whole of W6's
/// promise reaching the graph: an edge is stored under the name its endpoint had
/// when the row was written, and a corpus update can cut that segment into
/// pieces, fold it into its neighbour, or put a new se'if beside it whose name
/// is spelled like one of its pieces. `Open::standing` is what knows which.
#[must_use]
pub fn touching(shelf: &Shelf, repairs: &Repairs, at: &Standing) -> Touching {
    let root = shelf.root();
    let mut links = Vec::new();

    // Which rows are worth understanding. Both files are read whole either way —
    // the cost that mattered was never the disk, it was building an `Edge` out
    // of every row in a 27 MB shard to keep sixty of them. The gate is text and
    // is generous; `names` below is still the only thing that decides.
    //
    // A repair that re-anchored an edge can have moved it onto this line from
    // somewhere its stored ends do not mention, so those rows are let through by
    // name and re-tested after the layer is applied. Skipping that would drop
    // exactly the links a reader moved by hand.
    let mut wanted = store::Landing::naming(at);
    for moved in repairs.moved_from() {
        wanted.also_moved(moved);
    }

    // Outgoing: one file, the shard of the sefer you are reading.
    let mine = read_shard(root, at.at().work(), &wanted);
    for repaired in repairs.apply(mine) {
        if !repaired.edge.from.names(at) {
            continue;
        }
        links.push(link(shelf, repaired, true));
    }

    // Incoming: the edges that land on this sefer, from the cache that holds
    // them under it.
    //
    // Whether the cache exists at all is a different question from whether it
    // lists anything for this sefer, and answering the first with the second
    // would tell a reader "nothing links here" when the truth is "I have not
    // been told what does".
    //
    // The index knows where rows land and can read a few kilobytes instead of
    // twenty-seven megabytes — but it knows it from the rows as *stored*, and a
    // re-anchored edge is precisely one that is not where its row says. So a
    // reader who has moved a link by hand takes the gate over the whole file,
    // which finds it. Slower for them, and the same answers for everyone: both
    // paths hand what they find to the same `names` test below.
    let incoming_unknown = !inbound::built(root);
    let onto = if repairs.moves_anything() {
        None
    } else {
        inbound::read_at(root, at.at().work(), at)
    };
    let onto = onto.unwrap_or_else(|| {
        inbound::read_landing(root, at.at().work(), &wanted).unwrap_or_default()
    });
    for repaired in repairs.apply(onto) {
        if !repaired.edge.to.names(at) {
            continue;
        }
        links.push(link(shelf, repaired, false));
    }

    // …the ones you drew, which are in no shard at all…
    for repaired in repairs.drawn_touching(at) {
        let outgoing = repaired.edge.from.names(at);
        links.push(link(shelf, repaired, outgoing));
    }

    // …and what you have written about this line (W27).
    //
    // This is the whole of spec.md §11's claim, and it is four lines because it
    // has to be: a note's edge is a `girsa_link::Edge` like the four million
    // Sefaria seeded, so *what have I written that touches this sugya* is
    // answered here, by this function, in the same list and the same sort —
    // not by a second panel with a second idea of what a connection is.
    //
    // Through `repairs.apply` like everything else, so a note's edge can be
    // retyped or rejected by W23's layer; and `mine`, because you wrote it.
    for repaired in repairs.apply(shelf.notes().edges_touching(at)) {
        let outgoing = repaired.edge.from.names(at);
        let mut repaired = repaired;
        repaired.mine = true;
        links.push(link(shelf, repaired, outgoing));
    }

    // Commentary first, then everything else (W37).
    //
    // > *"kishuri i cant tell what is going on. it is hard to read."*
    //
    // A line of Gemara can have four commentaries on it and forty seforim that
    // cite it, and the list was sorted by confidence — so a `references` edge
    // somebody scored at 0.9 stood above Rashi. *Somebody wrote about this line*
    // and *somebody mentioned this line* are different enough that ranking them
    // together is what made the panel unreadable.
    links.sort_by(|a, b| {
        commentary_first(&a.repaired.edge.edge_type)
            .cmp(&commentary_first(&b.repaired.edge.edge_type))
            .then_with(|| {
                b.repaired
                    .confidence()
                    .partial_cmp(&a.repaired.confidence())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.repaired.edge.edge_type.cmp(&b.repaired.edge.edge_type))
            .then_with(|| a.he_title.cmp(&b.he_title))
            .then_with(|| a.other.from.cmp(&b.other.from))
    });
    Touching {
        links,
        incoming_unknown,
    }
}

/// Which band a link sorts into: what somebody *wrote about* this line, then what
/// merely *mentions* it.
///
/// Two bands and not a full ordering of the eight edge types, because that is the
/// distinction a reader is actually making when they open the panel. Within a band
/// the old order stands.
fn commentary_first(kind: &girsa_link::EdgeType) -> u8 {
    match kind {
        girsa_link::EdgeType::CommentsOn | girsa_link::EdgeType::Translates => 0,
        _ => 1,
    }
}

fn read_shard(root: &Path, slug: &str, wanted: &store::Landing) -> Vec<girsa_link::Edge> {
    store::read_edges_landing(&store::edges_path(root, slug), wanted).unwrap_or_default()
}

/// Which words of the segment you are standing on a link is about, where
/// anything says (spec.md §8.4, W24).
///
/// Two sources and no third: **you pinned it**, or the commentary at the far end
/// declares a dibur hamatchil that is in this line exactly once. A link with
/// neither is on the whole segment, which is what the shipped data addresses.
///
/// The far end's words are only looked at when that sefer is **already open** —
/// the panel is not entitled to read forty seforim off the disk to decorate a
/// list, and the case where this matters is the one where the commentary is in
/// the column beside you anyway.
#[must_use]
pub fn span_on(
    link: &Link,
    at: &SegmentId,
    base: &str,
    far: Option<&crate::shelf::Open>,
    nikud: bool,
) -> Option<std::ops::Range<usize>> {
    if let Some((pinned_at, span)) = &link.repaired.pinned {
        if pinned_at == at {
            return Some(span.clone());
        }
    }
    let far = far?;
    let commentary = far
        .position_of(&link.other.from)
        .and_then(|nth| far.segments.get(nth))?;
    crate::spans::dibur_span(base, &commentary.text, nikud)
}

/// Keep the links that touch a highlight.
///
/// A link whose words are known and are **not** these words goes; a link with no
/// span stays, because it is on the whole segment and the whole segment includes
/// what was highlighted. Dropping those would be answering "which links are on
/// these words" with "the ones I happen to know the words of".
#[must_use]
pub fn touching_words(links: Vec<Link>, span: std::ops::Range<usize>) -> Vec<Link> {
    links
        .into_iter()
        .filter(|link| {
            link.span
                .as_ref()
                .is_none_or(|on| on.start < span.end && span.start < on.end)
        })
        .collect()
}

fn link(shelf: &Shelf, repaired: Repaired, outgoing: bool) -> Link {
    let other = if outgoing {
        repaired.edge.to.clone()
    } else {
        repaired.edge.from.clone()
    };
    let work = other.from.work().to_string();
    let named = shelf.work(&work);
    let he_title = named.map_or_else(|| work.clone(), |w| w.he_title.clone());
    let en_title = named.map_or_else(|| work.clone(), |w| w.en_title.clone());
    Link {
        address: other.from.path().join(":"),
        other,
        work,
        he_title,
        en_title,
        outgoing,
        repaired,
        span: None,
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
    fn what_was_written_about_this_line_comes_before_what_merely_mentions_it() {
        // W37. The panel used to sort by confidence alone, so a `references` edge
        // scored at 0.9 stood above Rashi at 0.8 — and *somebody wrote about this
        // line* and *somebody mentioned this line* are not the same claim.
        use girsa_link::EdgeType;
        assert!(commentary_first(&EdgeType::CommentsOn) < commentary_first(&EdgeType::References));
        assert!(commentary_first(&EdgeType::Translates) < commentary_first(&EdgeType::References));
        // A targum is what somebody wrote about the pasuk; a parallel sugya is
        // not, however strong the link.
        assert_eq!(
            commentary_first(&EdgeType::CommentsOn),
            commentary_first(&EdgeType::Translates)
        );
        assert_eq!(
            commentary_first(&EdgeType::ParallelTo),
            commentary_first(&EdgeType::References)
        );
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
