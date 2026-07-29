//! The typed link graph, repair overrides, and later link mining.
//!
//! spec.md §8.1: import Sefaria's links — **citation-addressed, not
//! line-addressed** — and resolve them onto permanent segment ids. That is
//! strictly better than repairing Otzaria's degraded copies of the same graph,
//! because a citation survives an edit and a line number does not (§3).
//!
//! # An edge names segments, not lines and not citations
//!
//! Otzaria stores `file + line_index`, so fixing a typo re-points every link
//! below it. Sefaria stores `Sanhedrin 74b:9`, which survives an edit but has
//! to be resolved every time it is followed and stops meaning anything if
//! upstream re-sections the work. An edge here holds [`SegmentId`]s, resolved
//! once at import: durable under editing *and* under re-segmentation, because
//! the redirect table absorbs the second.
//!
//! # Why an endpoint can be a span
//!
//! `Exodus 1:1-6:1` is one row in `links0.csv` and it covers a whole parsha;
//! `Rashi on Berakhot 2a` covers a daf. A quote is a range (spec.md §4.2), so
//! an endpoint is a range — a point being the case where it happens to be one
//! segment long, not a different type.

pub mod chain;
pub mod inbound;
pub mod otzaria;
pub mod repair;
pub mod sefaria;
pub mod store;
pub mod touching;

use girsa_corpus::segment::SegmentId;

/// Edge types (spec.md §8.2). Directed; the inverse is derived, never stored
/// twice.
///
/// The type field is stored from day one and populated with whatever we have.
/// Sefaria's four labels map onto these; the 74% of links that arrive with a
/// blank `"Conection Type"` land on [`References`](EdgeType::References) — the
/// weak catch-all — and stay there until something better assigns them. A
/// schema change is expensive; filling in values is not.
/// Ordered so that a set of them has one printing order — the order they are
/// written below, strongest claim first, which is the order a facet lists them
/// in and the order the repair UI will offer them in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeType {
    CommentsOn,
    Quotes,
    Paraphrases,
    Codifies,
    Disputes,
    Emends,
    ParallelTo,
    Translates,
    /// The fallback, not the default. A link carrying this type is never
    /// presented as curated fact.
    References,
}

impl EdgeType {
    /// Whether this type represents a real claim about the relationship, as
    /// opposed to "these two texts are connected somehow".
    ///
    /// The repair UI (W23) uses this to tell an asserted edge from an
    /// unclassified one, and the sidebar uses it to avoid dressing an untyped
    /// link up as scholarship.
    #[must_use]
    pub const fn is_asserted(self) -> bool {
        !matches!(self, Self::References)
    }

    /// Read whatever was in the `"Conection Type"` column.
    ///
    /// **The column name is misspelled in the data** — in Sefaria's
    /// `links*.csv` and in Otzaria's JSON both (T2). Reading it correctly
    /// spelled silently types every link in the corpus as `references`.
    ///
    /// A blank is not a parse failure. T5: 74% of them are blank and it
    /// originates upstream, so re-importing does not fix it and treating a
    /// blank as an error would reject three quarters of the graph.
    #[must_use]
    pub fn from_sefaria(label: &str) -> Self {
        match label.trim().to_ascii_lowercase().as_str() {
            "commentary" | "targum" => Self::CommentsOn,
            "quotation" => Self::Quotes,
            "midrash" | "allusion" => Self::Paraphrases,
            // `reference`, `related`, and the blank three quarters. All three
            // say "connected somehow" and none of them says how.
            _ => Self::References,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CommentsOn => "comments-on",
            Self::Quotes => "quotes",
            Self::Paraphrases => "paraphrases",
            Self::Codifies => "codifies",
            Self::Disputes => "disputes",
            Self::Emends => "emends",
            Self::ParallelTo => "parallel-to",
            Self::Translates => "translates",
            Self::References => "references",
        }
    }
}

/// Where an edge lands: one segment, or a run of them in reading order.
///
/// The end is inclusive, and `None` means the run is one segment long. Reading
/// order is ordinal order (see [`SegmentId`]), so a run is expressible as its
/// two ends and does not have to be listed out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub from: SegmentId,
    pub to: Option<SegmentId>,
}

impl Anchor {
    #[must_use]
    pub fn point(id: SegmentId) -> Self {
        Self { from: id, to: None }
    }

    /// A run. Collapses to a point when both ends are the same segment, so one
    /// place never has two spellings.
    #[must_use]
    pub fn span(from: SegmentId, to: SegmentId) -> Self {
        if from == to {
            return Self::point(from);
        }
        Self { from, to: Some(to) }
    }

    #[must_use]
    pub fn is_span(&self) -> bool {
        self.to.is_some()
    }

    /// Whether this anchor lands on a segment.
    ///
    /// A run covers everything between its ends in **reading order**, which is
    /// ordinal order and so includes the children a split minted (§3) — an edge
    /// onto `#7` still lands after `#7` becomes `#7.1` and `#7.2`.
    #[must_use]
    pub fn covers(&self, id: &SegmentId) -> bool {
        if self.from.work() != id.work() {
            return false;
        }
        match &self.to {
            Some(to) => self.from <= *id && *id <= *to,
            None => self.from.covers(id),
        }
    }

    /// Whether two anchors have any text in common.
    ///
    /// A chain (W28) stands on an anchor and asks what else touches it, and
    /// most of Sefaria's citations are coarser than a segment — `Rashi on
    /// Berakhot 2a` covers a daf. Asking with [`Anchor::covers`] on the near
    /// end alone would find only the links that happen to start where this one
    /// starts, and a hop would be missed for no reason a reader could see.
    ///
    /// Built out of `covers` rather than beside it: two ranges that overlap
    /// always have one's start inside the other, so this is the same coverage
    /// rule asked four ways and cannot drift from it.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.covers(&other.from)
            || other.covers(&self.from)
            || other.to.as_ref().is_some_and(|end| self.covers(end))
            || self.to.as_ref().is_some_and(|end| other.covers(end))
    }
}

impl std::fmt::Display for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.from)?;
        if let Some(to) = &self.to {
            write!(f, "-{to}")?;
        }
        Ok(())
    }
}

/// How an edge came to exist. Shown in the repair UI, which spec.md §8.3
/// requires to *show its work*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Resolved from Sefaria's `links*.csv`, which are citation-addressed.
    SefariaSeed,
    /// Resolved from an Otzaria `*_links.json`, for one of the 978 works
    /// Sefaria does not have. Line-indexed at the source, so it carries the
    /// weaker confidence of the two.
    OtzariaSeed,
    /// You drew it (W23). It is in your layer and in no shard, and it is the
    /// only method whose confidence is not a guess about somebody else's data.
    ByHand,
}

impl Method {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SefariaSeed => "sefaria-seed",
            Self::OtzariaSeed => "otzaria-seed",
            Self::ByHand => "by-hand",
        }
    }

    /// How much to believe it, before anybody has looked.
    ///
    /// Not a probability — a rank, so that the sidebar can put a citation-
    /// addressed edge above a line-indexed one and a reader can see why.
    #[must_use]
    pub const fn confidence(self) -> f32 {
        match self {
            Self::SefariaSeed => 0.9,
            Self::OtzariaSeed => 0.7,
            Self::ByHand => 1.0,
        }
    }
}

/// One directed, typed edge between two places in the library.
///
/// spec.md §8.2. `from_span`/`to_span` — the character ranges inside a segment
/// — are W24's and are deliberately absent rather than faked: an edge claiming
/// a span it never measured would be presented to a reader as precision that
/// does not exist.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from: Anchor,
    pub to: Anchor,
    pub edge_type: EdgeType,
    pub method: Method,
    /// The label the source used, kept verbatim.
    ///
    /// `reference`, `related` and a blank all map onto
    /// [`EdgeType::References`], and the three are not the same claim. Keeping
    /// the original is what lets W23's retype tell "the corpus said nothing"
    /// from "the corpus said `related`" without a re-import.
    pub source_label: String,
}

impl Edge {
    #[must_use]
    pub fn confidence(&self) -> f32 {
        self.method.confidence()
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::Ordinal;

    fn id(n: u32) -> SegmentId {
        SegmentId::new("bavli/berakhot", vec!["2a".into()], Ordinal::root(n))
    }

    #[test]
    fn the_catch_all_type_is_not_an_assertion() {
        assert!(!EdgeType::References.is_asserted());
        assert!(EdgeType::CommentsOn.is_asserted());
    }

    #[test]
    fn a_blank_type_is_read_as_the_catch_all_and_not_as_a_failure() {
        // T5: 74% of links arrive blank and it originates upstream in Sefaria,
        // so re-importing does not fix it. Treating a blank as a parse error
        // would drop three quarters of the graph.
        assert_eq!(EdgeType::from_sefaria(""), EdgeType::References);
        assert_eq!(EdgeType::from_sefaria("   "), EdgeType::References);
        assert_eq!(EdgeType::from_sefaria("related"), EdgeType::References);
    }

    #[test]
    fn sefarias_labels_map_onto_ours() {
        assert_eq!(EdgeType::from_sefaria("commentary"), EdgeType::CommentsOn);
        assert_eq!(EdgeType::from_sefaria("quotation"), EdgeType::Quotes);
        assert_eq!(EdgeType::from_sefaria("Commentary"), EdgeType::CommentsOn);
    }

    #[test]
    fn a_span_of_one_segment_is_a_point() {
        // Otherwise one place has two spellings and anything keyed on an
        // anchor splits into two buckets.
        assert_eq!(Anchor::span(id(1), id(1)), Anchor::point(id(1)));
        assert!(!Anchor::span(id(1), id(1)).is_span());
        assert!(Anchor::span(id(1), id(4)).is_span());
    }

    #[test]
    fn a_citation_addressed_edge_is_believed_more_than_a_line_indexed_one() {
        assert!(Method::SefariaSeed.confidence() > Method::OtzariaSeed.confidence());
    }
}
