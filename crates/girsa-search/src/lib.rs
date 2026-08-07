//! tantivy indices, the five search modes, and the relaxation ladder.
//!
//! The governing constraint is not "make it powerful" — it is that **the engine
//! never changes your query without you knowing** (spec.md §9). Torat Emet, the
//! literal mode, is the default; widening is offered with counts, and only
//! auto-applied in Smart mode where widening is the declared purpose.
//!
//! W11 built the index and the tokenizer under it, W12 the literal mode and
//! W13 the ladder and Smart mode. The remaining modes, the chips and the facets
//! are W14.

pub mod bar;
pub mod chips;
pub mod citation;
pub mod facets;
pub mod index;
pub mod instruments;
pub mod ladder;
pub mod mekoros;
pub mod regex_mode;
pub mod scope;
// One snippet renderer, windowed on the match (B16). There were two, and the
// wrong one answered *where is this from*.
pub mod smart;
pub mod snippet;
pub mod tokenizer;
pub mod torat_emet;

/// The five modes of spec.md §9.3. The selector is always visible, and the
/// default is the literal one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Completely literal. Nothing stemmed, expanded or guessed.
    #[default]
    ToratEmet,
    /// Opt-in widening: prefixes, ktiv male/chaser, abbreviations.
    Smart,
    /// Full power, no hand-holding.
    Regex,
    /// Type a mareh makom, jump.
    Citation,
    /// Gematria, notarikon, atbash, dilug.
    Instruments,
}

// The words are Rust's own `Debug` spellings, because that is what the chip
// row has always sent — `format!("{mode:?}")` — and what the window sends back.
// Written down here instead, so the wire format is a list rather than a
// derive nobody thinks of as a wire format.
girsa_corpus::spelled!(Mode {
    ToratEmet => "ToratEmet",
    Smart => "Smart",
    Regex => "Regex",
    Citation => "Citation",
    Instruments => "Instruments",
});

impl Mode {
    /// Whether this mode may widen a query on its own initiative.
    ///
    /// Only Smart may, and even there it announces the change and offers a
    /// one-click undo. Everywhere else a widening is *offered* with a count
    /// computed up front, and applied only when clicked (spec.md §9.6).
    #[must_use]
    pub const fn may_auto_relax(self) -> bool {
        matches!(self, Self::Smart)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_mode_is_literal() {
        assert_eq!(Mode::default(), Mode::ToratEmet);
    }

    #[test]
    fn only_smart_mode_may_widen_a_query_by_itself() {
        for mode in [
            Mode::ToratEmet,
            Mode::Regex,
            Mode::Citation,
            Mode::Instruments,
        ] {
            assert!(!mode.may_auto_relax(), "{mode:?} must not auto-relax");
        }
        assert!(Mode::Smart.may_auto_relax());
    }
}
