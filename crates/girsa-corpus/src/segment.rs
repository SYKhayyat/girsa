//! Permanent segment IDs (spec.md §3, decision 2).
//!
//! # The defect this exists to prevent
//!
//! Otzaria addresses a link as *file + line number*. Fix a typo in a way that
//! splits or joins a line and every link below it in that file points at the
//! wrong text. Not a broken link that errors — **the wrong text, silently**.
//! Splitting one line of Mishnah Berurah moves the other 18,119.
//!
//! # The shape
//!
//! ```text
//! girsa:mishnah-berurah/1:1#7
//!       └── work ─────┘ └p┘ └ordinal
//! ```
//!
//! - The **work slug** and **section path** say where this is, and are what a
//!   person reads. They come from the schema.
//! - The **ordinal** is assigned once, at import, in reading order, and is never
//!   recomputed from anything. It is the durable part.
//!
//! Splitting `#7` mints `#7.1` and `#7.2`. It does not touch `#8`, and that is
//! the whole trick: children *extend* their parent's ordinal rather than being
//! inserted into the sequence, so nothing after them moves.
//!
//! ```text
//! before                  after
//!   #6                      #6        untouched
//!   #7   ── split ──┐       #7.1      new
//!                   └───→   #7.2      new
//!   #8                      #8        untouched
//!   … 18,000 more           … 18,000 more, all untouched
//! ```
//!
//! Ordinals sort in reading order — `#7 < #7.1 < #7.2 < #8` — which is what
//! makes a span addressable, and a quote is a span rather than a point
//! (spec.md §4.2).
//!
//! # Why this shape and not a number or a hash
//!
//! An opaque counter is smaller and faster to join on, but two people importing
//! the same corpus get different numbers, so a patch file or a Ksav document
//! cannot cross machines. A content hash is identical everywhere by
//! construction but moves when the text is corrected, which unmoors every note
//! attached to it — the exact failure being designed out.
//!
//! # Why the section path is written with `:` and not `/`
//!
//! A segment id and a [`girsa_ref::Ref`] are stored in the same places — inside
//! Ksav documents, in patch files, in link rows — and `girsa-cite` is handed
//! ids and prints refs. So they have to be **one grammar**, and `girsa-ref`'s
//! rule is that *the last `/`-separated component is the address*.
//!
//! Writing the section path with `/` broke that. `girsa:shulchan-arukh/orach-chayim/1/1#7`
//! read back as the work `shulchan-arukh/orach-chayim/1` — a sefer that does
//! not exist — at siman 1. It resolved, it printed, and it was wrong, which is
//! the failure mode this whole crate is arranged against.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Where a segment sits in reading order, permanently.
///
/// A dotted sequence: `7`, then `7.1` and `7.2` after a split, then `7.1.1` if
/// one of those is split again. Depth is unbounded because there is no
/// principled place to stop a reader from correcting a correction.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Ordinal(Vec<u32>);

impl Ordinal {
    /// The `n`th segment of a work, as assigned at import. One-based, because
    /// it is shown to people.
    #[must_use]
    pub fn root(n: u32) -> Self {
        Self(vec![n])
    }

    /// The ordinals a split mints, oldest sibling first.
    ///
    /// `#7` split three ways gives `#7.1 #7.2 #7.3`. The parent keeps existing —
    /// anything anchored to `#7` still resolves, to the whole group.
    #[must_use]
    pub fn children(&self, count: usize) -> Vec<Self> {
        (1..=count)
            .map(|i| {
                let mut v = self.0.clone();
                #[allow(clippy::cast_possible_truncation)]
                v.push(i as u32);
                Self(v)
            })
            .collect()
    }

    /// Whether `self` is this ordinal or one of its descendants.
    ///
    /// This is what makes an old anchor keep working: a link that pointed at
    /// `#7` before the split still covers `#7.1` and `#7.2` after it.
    #[must_use]
    pub fn covers(&self, other: &Self) -> bool {
        other.0.starts_with(&self.0)
    }

    /// How deep the ordinal is. A root segment is 1; a child of a split is 2.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for Ordinal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, n) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            write!(f, "{n}")?;
        }
        Ok(())
    }
}

/// Why a segment ID would not parse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SegmentIdError {
    #[error("a segment id must start with `girsa:`")]
    NotAGirsaId,
    #[error("a segment id must carry a `#ordinal`; `{0}` is a citation, not an anchor")]
    NoOrdinal(String),
    #[error("`{0}` is not a dotted ordinal")]
    BadOrdinal(String),
    #[error("a segment id must name a work")]
    NoWork,
}

/// A permanent anchor for one segment of one work.
///
/// Every correction, note, highlight, link and citation inside a Ksav document
/// points at one of these. Editing text cannot move one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SegmentId {
    work: String,
    path: Vec<String>,
    ordinal: Ordinal,
}

impl SegmentId {
    /// Mint an ID. Done by the importer, once per segment, and by a split.
    #[must_use]
    pub fn new(work: impl Into<String>, path: Vec<String>, ordinal: Ordinal) -> Self {
        Self {
            work: work.into(),
            path,
            ordinal,
        }
    }

    /// The work slug — `mishnah-berurah`.
    #[must_use]
    pub fn work(&self) -> &str {
        &self.work
    }

    /// The section path — `["1", "1"]` for siman 1, se'if 1.
    ///
    /// Descriptive, not load-bearing. It is what a reader sees and what a
    /// citation is built from; if upstream re-sections a work, this changes and
    /// the redirect table absorbs it. Reading order comes from the ordinal.
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// The part that never changes.
    #[must_use]
    pub fn ordinal(&self) -> &Ordinal {
        &self.ordinal
    }

    /// Split into `count` children. `#7` → `#7.1`, `#7.2`.
    ///
    /// Siblings are untouched, by construction: there is no way to express
    /// "renumber everything after this" with this return type.
    #[must_use]
    pub fn split(&self, count: usize) -> Vec<Self> {
        self.ordinal
            .children(count)
            .into_iter()
            .map(|ordinal| Self {
                work: self.work.clone(),
                path: self.path.clone(),
                ordinal,
            })
            .collect()
    }

    /// Whether an anchor on `self` still covers `other` — true for `other`
    /// itself and for anything a split of `self` produced.
    #[must_use]
    pub fn covers(&self, other: &Self) -> bool {
        self.work == other.work && self.ordinal.covers(&other.ordinal)
    }

    /// The citation this segment sits at, dropping the ordinal.
    ///
    /// What `girsa-cite` prints and what a link row is matched against. The
    /// ordinal is the durable name; the ref is the human address, and the two
    /// are deliberately different things.
    #[must_use]
    pub fn to_ref(&self) -> girsa_ref::Ref {
        let work: Vec<String> = self.work.split('/').map(str::to_string).collect();
        if self.path.is_empty() {
            return girsa_ref::Ref::whole_work(work);
        }
        let levels = self
            .path
            .iter()
            .map(|p| girsa_ref::Level::canonical(p.clone()))
            .collect();
        girsa_ref::Ref::point(work, girsa_ref::Address::new(levels))
    }

    /// Whether this id survives being written down and read back.
    ///
    /// It does not, if a section name carries one of the grammar's own
    /// separators — a Sefaria named section really can be
    /// `שער חמישי - שער ייחוד המעשה`, and one containing a `/` or a `:` would
    /// re-read as a different work at a different place. The importer asserts
    /// this on every id it mints, because that is the one moment it can be
    /// caught: after the ids are on disk and inside documents, they are
    /// permanent by definition.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        !self.work.is_empty()
            && !self.work.starts_with('/')
            && !self.work.ends_with('/')
            && !self.work.contains(':')
            && !self.work.contains('#')
            && !self.path.is_empty()
            && !self
                .path
                .iter()
                .any(|p| p.is_empty() || p.contains(['/', ':', '#']))
    }
}

/// Reading order, which is ordinal order within a work.
///
/// Not path order: a section path is strings (`"9"`, `"10"`, `"2a"`) and
/// sorting those lexicographically puts siman 10 before siman 9. The ordinal is
/// assigned in reading order at import and is the only thing that has to be
/// right here.
impl Ord for SegmentId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.work
            .cmp(&other.work)
            .then_with(|| self.ordinal.cmp(&other.ordinal))
    }
}

impl PartialOrd for SegmentId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for SegmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "girsa:{}", self.work)?;
        for (i, part) in self.path.iter().enumerate() {
            f.write_str(if i == 0 { "/" } else { ":" })?;
            f.write_str(part)?;
        }
        write!(f, "#{}", self.ordinal)
    }
}

impl FromStr for SegmentId {
    type Err = SegmentIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let body = s
            .strip_prefix("girsa:")
            .ok_or(SegmentIdError::NotAGirsaId)?;
        let (address, ordinal) = body
            .split_once('#')
            .ok_or_else(|| SegmentIdError::NoOrdinal(s.to_string()))?;

        // The same split `girsa_ref::Ref` makes, for the same reason: the two
        // are one grammar because they are stored in the same places. The last
        // `/`-separated component is the address; everything before it names
        // the work, `/` and all.
        let (work, path) = match address.rsplit_once('/') {
            Some((work, tail)) => (
                work,
                tail.split(':')
                    .filter(|p| !p.is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
            None => (address, Vec::new()),
        };
        if work.is_empty() {
            return Err(SegmentIdError::NoWork);
        }

        let mut numbers = Vec::new();
        for piece in ordinal.split('.') {
            let n: u32 = piece
                .parse()
                .map_err(|_| SegmentIdError::BadOrdinal(ordinal.to_string()))?;
            numbers.push(n);
        }
        if numbers.is_empty() {
            return Err(SegmentIdError::BadOrdinal(ordinal.to_string()));
        }

        Ok(Self {
            work: work.to_string(),
            path,
            ordinal: Ordinal(numbers),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> SegmentId {
        SegmentId::new(
            "mishnah-berurah",
            vec!["1".into(), "1".into()],
            Ordinal::root(n),
        )
    }

    #[test]
    fn an_id_reads_the_way_it_was_promised() {
        assert_eq!(id(7).to_string(), "girsa:mishnah-berurah/1:1#7");
    }

    #[test]
    fn an_id_survives_a_round_trip_through_text() {
        // It has to: these are stored inside Ksav documents as text.
        for s in [
            "girsa:mishnah-berurah/1:1#7",
            "girsa:mishnah-berurah/1:1#7.2",
            "girsa:bavli/berakhot/2a#1",
            "girsa:shulchan-arukh/orach-chayim/121:3#8",
            "girsa:user/reb-shmuel-handout-2024#12",
        ] {
            let parsed: SegmentId = match s.parse() {
                Ok(p) => p,
                Err(e) => panic!("{s} did not parse: {e}"),
            };
            assert_eq!(parsed.to_string(), s);
        }
    }

    #[test]
    fn a_citation_without_an_ordinal_is_refused_rather_than_guessed_at() {
        // `girsa:shulchan-arukh/orach-chayim/1/1` is a citation — it says where
        // to look, not which segment. Accepting it here and inventing an
        // ordinal would silently anchor a note to whatever happened to be
        // first, which is the class of guess BUILDER rule 6 forbids.
        let e = "girsa:shulchan-arukh/orach-chayim/1/1".parse::<SegmentId>();
        assert!(matches!(e, Err(SegmentIdError::NoOrdinal(_))), "{e:?}");
    }

    #[test]
    fn splitting_mints_children_and_leaves_the_next_segment_alone() {
        let seven = id(7);
        let eight = id(8);
        let children = seven.split(2);

        assert_eq!(children[0].to_string(), "girsa:mishnah-berurah/1:1#7.1");
        assert_eq!(children[1].to_string(), "girsa:mishnah-berurah/1:1#7.2");
        assert_eq!(eight, id(8), "the following segment must not move");
    }

    #[test]
    fn children_sort_between_their_parent_and_the_next_segment() {
        // This is what makes a span still work after a split: a range from #6
        // to #8 has to keep containing everything #7 became.
        let seven = id(7);
        let children = seven.split(2);
        assert!(id(6) < children[0]);
        assert!(children[0] < children[1]);
        assert!(children[1] < id(8));
        assert!(seven < children[0]);
    }

    #[test]
    fn an_old_anchor_still_covers_what_the_segment_became() {
        let seven = id(7);
        for child in seven.split(3) {
            assert!(seven.covers(&child), "{seven} stopped covering {child}");
        }
        assert!(!seven.covers(&id(8)));
    }

    #[test]
    fn a_split_of_a_split_still_nests() {
        let seven = id(7);
        let first_child = seven.split(2).remove(0);
        let grandchild = first_child.split(2).remove(1);
        assert_eq!(grandchild.to_string(), "girsa:mishnah-berurah/1:1#7.1.2");
        assert!(seven.covers(&grandchild));
        assert!(first_child.covers(&grandchild));
        assert!(grandchild < id(8));
    }

    #[test]
    fn a_segment_id_reads_as_a_ref_naming_the_same_place() {
        // Segment ids are stored inside Ksav documents and are handed to
        // `girsa-cite` to print. Both go through `girsa_ref::Ref`, which reads
        // *the last `/`-separated component as the address* — so a work path
        // and a section path cannot both be written with `/`, or the last
        // section silently becomes the address and everything before it
        // becomes the work.
        let id = SegmentId::new(
            "shulchan-arukh/orach-chayim",
            vec!["1".into(), "1".into()],
            Ordinal::root(7),
        );
        let printed = id.to_string();
        let as_ref: girsa_ref::Ref = match printed.parse() {
            Ok(r) => r,
            Err(e) => panic!("{printed} did not read as a ref: {e}"),
        };
        assert_eq!(as_ref.work_slug(), "shulchan-arukh/orach-chayim");
        assert_eq!(as_ref.from().to_string(), "1:1");
        // And the same place when the id hands over its ref directly, rather
        // than the two agreeing only by way of a string.
        assert_eq!(id.to_ref(), as_ref);
    }

    #[test]
    fn a_section_name_carrying_a_separator_is_caught_at_import_and_not_after() {
        // Sefaria really does name a section `שער חמישי - שער ייחוד המעשה`. A
        // `/` or a `:` in one would make the id re-read as a different work at
        // a different place, and an id is permanent from the moment it is
        // written — so the only useful time to notice is while minting it.
        let good = SegmentId::new(
            "chovot-halevavot",
            vec!["5".into(), "1".into()],
            Ordinal::root(1),
        );
        assert!(good.is_well_formed());

        for bad_path in [vec!["5/1".to_string()], vec!["2a:1".to_string()], vec![]] {
            let bad = SegmentId::new("chovot-halevavot", bad_path.clone(), Ordinal::root(1));
            assert!(
                !bad.is_well_formed(),
                "{bad_path:?} should not have passed as a section path"
            );
        }
    }

    #[test]
    fn reading_order_does_not_come_from_the_section_path() {
        // Siman 10 comes after siman 9. Sorted as text it comes before it, and
        // a system that ordered by path would show them the wrong way round.
        let nine = SegmentId::new("sa", vec!["9".into()], Ordinal::root(9));
        let ten = SegmentId::new("sa", vec!["10".into()], Ordinal::root(10));
        assert!(nine < ten, "the ordinal puts them in reading order");
        assert!(
            ten.path() < nine.path(),
            "and the path really does sort them backwards: \"10\" < \"9\" as text"
        );
    }
}
