//! The typed link graph, repair overrides, and later link mining.
//!
//! Filled in by W8 and W23–W24. This is the W1 scaffold.

/// Edge types (spec.md §8.2). Directed; the inverse is derived, never stored
/// twice.
///
/// The type field is stored from day one and populated with whatever we have.
/// Sefaria's four labels map onto these; the 74% of links that arrive with a
/// blank `"Conection Type"` land on [`References`](EdgeType::References) — the
/// weak catch-all — and stay there until something better assigns them. A
/// schema change is expensive; filling in values is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catch_all_type_is_not_an_assertion() {
        assert!(!EdgeType::References.is_asserted());
        assert!(EdgeType::CommentsOn.is_asserted());
    }
}
