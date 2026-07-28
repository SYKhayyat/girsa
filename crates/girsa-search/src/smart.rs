//! Smart mode (BUILDER.md W13, spec.md §9.3, §9.6).
//!
//! The opt-in mode: *type words, and prefixes, male/chaser and abbreviations
//! are handled for you*. It is the one place in this engine where a query is
//! widened without being asked each time — and that is allowed for exactly one
//! reason, which is that **widening is the mode's declared purpose and it
//! always reports itself**. Take away the report and Smart becomes Sefaria's
//! analyzer, which over-stems and does not say so; spec.md §9 names that as a
//! failure mode not to reproduce.
//!
//! # Why the widening happens on the first search and not after a zero
//!
//! spec.md §9.6's table says Smart *auto-relaxes on zero results*, and §9.4
//! shows Smart reporting *"43 results — 12 match other forms of כתב"* — a
//! result set that is **not** zero and has variant matches in it. Both are true
//! at once because the ladder has two kinds of rung:
//!
//! - the **form** rungs are what the mode *is*, so they are applied from the
//!   start. That is what "handled for you" means: the reader never has to
//!   notice that the page spells it `כוהן`.
//! - the **proximity** rung changes the shape of the question rather than the
//!   spelling of a word, so it is held back and climbed only when everything
//!   else came to nothing.
//!
//! [`Answered`] carries both numbers — what the literal query alone would have
//! found, and what the widened one did — so the difference between them can be
//! shown, and [`Answered::literal`] is the query the *[exact form only]* button
//! re-runs. The undo is a query, not a flag.
//!
//! # Where Smart refuses
//!
//! A widening that would take more exact searches than the ceiling allows is an
//! error, not a quietly narrowed search (see [`crate::ladder::MOST_EXACT_QUERIES`]).
//! Smart could fall back to fewer rungs and say nothing; it does not, because a
//! mode whose contract is *it tells you what it did* cannot have a path where
//! it does less than it said.

use girsa_hebrew::VariantKind;

use crate::index::{Found, IndexError, SearchIndex};
use crate::ladder::{Rung, Widened};
use crate::torat_emet::Query;

/// A query in Smart mode.
#[derive(Debug, Clone)]
pub struct Smart {
    base: Query,
}

impl Smart {
    /// Take a literal query into Smart mode.
    #[must_use]
    pub fn new(base: Query) -> Self {
        Self { base }
    }

    /// The rungs Smart applies before it runs at all — the mode's own meaning.
    ///
    /// Every form rung of the ladder, in ladder order. Deliberately *not*
    /// [`Rung::Proximity`]: widening from a phrase to a whole passage is a
    /// different question, not a different spelling, and it is climbed only
    /// after the rest found nothing.
    #[must_use]
    pub fn baseline() -> Vec<Rung> {
        vec![
            Rung::Forms(VariantKind::PrefixPeeled),
            Rung::Forms(VariantKind::KtivSwapped),
            Rung::Forms(VariantKind::GershayimDropped),
            Rung::Forms(VariantKind::AbbreviationExpanded),
        ]
    }

    /// The literal query underneath.
    #[must_use]
    pub fn literal(&self) -> &Query {
        &self.base
    }

    /// Run it: widen, and if that still found nothing, climb the last rung.
    ///
    /// # Errors
    ///
    /// As [`SearchIndex::search_widened`] — including
    /// [`IndexError::TooManyForms`], which is raised rather than worked around.
    pub fn run(&self, index: &SearchIndex) -> Result<Answered, IndexError> {
        let exact = index.search(&self.base)?;
        let exact_total = exact.total;

        let mut climbed: Vec<Rung> = Vec::new();
        let mut widened = Widened::new(self.base.clone(), Self::baseline());
        if !widened.widening().changes_anything() {
            // Nothing on the shelf of transformations applies to these words.
            // Saying "other forms" of a query that has none would be a report
            // of work that did not happen.
            return Ok(Answered {
                found: exact,
                applied: Vec::new(),
                exact_total,
                literal: self.base.clone(),
                climbed: Self::baseline(),
            });
        }
        climbed.extend(widened.rungs().iter().copied());
        let mut found = index.search_widened(&widened)?;

        if found.total == 0 {
            let with_proximity = Widened::new(
                self.base.clone(),
                Self::baseline().into_iter().chain([Rung::Proximity]),
            );
            // Only if it is a rung at all: a query that is already *all these
            // words, anywhere in the segment* has nothing left to widen, and
            // claiming otherwise would be announcing a change that was not
            // made.
            if with_proximity.widening().together != widened.widening().together {
                climbed.push(Rung::Proximity);
                widened = with_proximity;
                found = index.search_widened(&widened)?;
            }
        }

        Ok(Answered {
            applied: widened.rungs().to_vec(),
            found,
            exact_total,
            literal: self.base.clone(),
            climbed,
        })
    }
}

/// What Smart mode found, and what it did to the query to find it.
#[derive(Debug)]
pub struct Answered {
    /// The results, with [`Found::widening`] saying what was run.
    pub found: Found,
    /// The rungs that were applied, in ladder order.
    pub applied: Vec<Rung>,
    /// How many the literal query alone would have found.
    pub exact_total: usize,
    /// The query the *[exact form only]* button re-runs — the one-click undo.
    pub literal: Query,
    /// Every rung reached for, in order, including ones that changed nothing.
    /// What the mode *tried*, as against what it managed.
    pub climbed: Vec<Rung>,
}

impl Answered {
    /// How many hits are there only because the query was widened.
    ///
    /// Cannot go negative: the typed form is always the first alternative at
    /// every position and the proximity rung only ever loosens, so the widened
    /// result set contains the literal one. The saturating subtraction is
    /// belt-and-braces against a future rung that forgets it.
    #[must_use]
    pub fn from_other_forms(&self) -> usize {
        self.found.total.saturating_sub(self.exact_total)
    }

    /// The line the result header shows — what the mode did, in words.
    ///
    /// spec.md §9.4's shape: *"43 results — 12 match other forms of כתב"*.
    #[must_use]
    pub fn announcement(&self) -> String {
        if self.found.total == 0 {
            let tried: Vec<&str> = self.climbed.iter().map(|r| r.label()).collect();
            return if tried.is_empty() {
                "no results".to_string()
            } else {
                format!("no results, and none after {}", tried.join(", "))
            };
        }
        let extra = self.from_other_forms();
        if extra == 0 {
            return format!("{} results", self.found.total);
        }
        let how: Vec<&str> = self.applied.iter().map(|r| r.label()).collect();
        format!(
            "{} results — {extra} match other forms ({})",
            self.found.total,
            how.join(", ")
        )
    }
}
