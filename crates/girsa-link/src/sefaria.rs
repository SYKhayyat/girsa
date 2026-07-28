//! Reading Sefaria's `links*.csv` onto permanent segment ids.
//!
//! spec.md §8.1 and §2.2: Sefaria's link graph is Otzaria's, **before** it was
//! converted down to line numbers. It addresses both ends by canonical citation
//! —
//!
//! ```text
//! Citation 1,Citation 2,Conection Type,Text 1,Text 2,Category 1,Category 2
//! "A Dictionary of the Talmud, אֱגוֹד 1",Mishnah Peah 6:6,quotation,…
//! ```
//!
//! — so importing it is a resolver problem rather than a repair problem, and
//! the result anchors to segment ids that survive an edit. That is why W8
//! imports these and uses Otzaria's JSON only for the 978 works Sefaria does
//! not have.
//!
//! # Three traps, all of them in this file
//!
//! **T2** — the column is spelled `Conection Type`, in Sefaria's export and in
//! Otzaria's copy of it. Read correctly spelled, every link in the corpus types
//! as the catch-all and nothing looks wrong.
//!
//! **T5** — 74% of the types are blank, and it originates upstream, so
//! re-importing does not fix it. A blank is data, not a parse failure.
//!
//! And the one that is not in BUILDER's list: **a citation is usually coarser
//! than a segment.** `Exodus 1:1-6:1` covers a parsha. Resolving that to its
//! first verse would be silently wrong for a large fraction of the graph, so an
//! endpoint is a run (see [`Anchor`]).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use girsa_corpus::csv::{fields, link_columns};
use girsa_corpus::index::SegmentIndex;
use girsa_ref::{Lexicon, Ref, Resolution};

use crate::{Anchor, Edge, EdgeType, Method};

/// What a pass over the link files did, in enough detail to be honest about.
///
/// BUILDER.md W8: *report resolution rate and the count of links dropped as
/// unresolvable — a silent drop is a defect.*
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub rows: usize,
    pub imported: usize,
    /// The citation did not name a work the lexicon knows.
    pub unresolved_citation: usize,
    /// The citation named two or more works and Sefaria's own `Text N` column
    /// did not say which. Never picked at random — see [`resolve_citation`].
    pub ambiguous: usize,
    /// The citation named a work that is not on the shelf at all.
    ///
    /// The resolver's lexicon is built from **every** Sefaria schema — 6,595 of
    /// them — and the shelf holds the 6,211 that have Hebrew text plus the 978
    /// Otzaria-only. So a link into a work Sefaria catalogues and has no Hebrew
    /// for resolves perfectly and lands nowhere, and that is a different fact
    /// from a bad address.
    pub work_not_on_shelf: usize,
    /// The work is on the shelf and the address is not in it — a siman that
    /// does not exist, or a citation one level deeper than the text goes.
    pub address_not_found: usize,
    /// Rows with a blank `Conection Type`. Expected — T5 — and counted so the
    /// 74% stays visible rather than becoming folklore.
    pub untyped: usize,
}

impl Tally {
    pub fn absorb(&mut self, other: Self) {
        self.rows += other.rows;
        self.imported += other.imported;
        self.unresolved_citation += other.unresolved_citation;
        self.ambiguous += other.ambiguous;
        self.work_not_on_shelf += other.work_not_on_shelf;
        self.address_not_found += other.address_not_found;
        self.untyped += other.untyped;
    }

    /// The fraction of rows that became an edge.
    #[must_use]
    pub fn rate(&self) -> f64 {
        if self.rows == 0 {
            return 0.0;
        }
        self.imported as f64 / self.rows as f64
    }
}

/// A resolver with a memory.
///
/// The link files hold millions of rows and a few hundred thousand distinct
/// citations — every commentary on a daf cites that daf. Resolving each string
/// once is the difference between an import that finishes and one that does
/// not.
pub struct Resolver<'a> {
    lexicon: &'a Lexicon,
    cache: HashMap<String, Option<Ref>>,
    /// Citations that resolved to several works, kept so the tally can tell an
    /// ambiguity from a miss.
    ambiguous: HashMap<String, Vec<Ref>>,
    /// Bare work titles from the `Text N` column. Kept apart from the citation
    /// cache: the same string can be both, and one map would let a title that
    /// resolved to nothing overwrite the record of a citation that resolved to
    /// several — turning an ambiguity into a miss in the tally.
    works: HashMap<String, Option<String>>,
}

impl<'a> Resolver<'a> {
    #[must_use]
    pub fn new(lexicon: &'a Lexicon) -> Self {
        Self {
            lexicon,
            cache: HashMap::new(),
            ambiguous: HashMap::new(),
            works: HashMap::new(),
        }
    }

    /// The slug a bare work title names, if exactly one work goes by it.
    ///
    /// Public because W8's Otzaria half needs the same answer for a different
    /// reason: T4 says to resolve an Otzaria link target **by filename**, and a
    /// filename is a bare title.
    pub fn work_slug_of(&mut self, title: &str) -> Option<String> {
        self.work_slug(title)
    }

    /// Resolve a citation, using Sefaria's own work column to settle a tie.
    ///
    /// `או"ח` is Orach Chayim in the Shulchan Arukh *and* in the Tur, and
    /// `girsa-ref` returns both rather than choosing — BUILDER.md rule 6. Here
    /// there is a third party who knows: the CSV's `Text N` column names the
    /// work the citation came from, separately from the citation itself. Using
    /// it is not a guess, it is reading the other column.
    ///
    /// When that column does not settle it either, the row is counted as
    /// ambiguous and **dropped**. An edge is followed by a reader who is not
    /// asked anything, so an ambiguous link is a wrong link half the time.
    pub fn resolve_citation(&mut self, citation: &str, work_column: &str) -> Resolved {
        let citation = citation.trim();
        if citation.is_empty() {
            return Resolved::Unresolved;
        }
        if let Some(cached) = self.cache.get(citation) {
            return match cached {
                Some(r) => Resolved::Exact(r.clone()),
                None => match self.ambiguous.get(citation) {
                    Some(candidates) => self.settle(candidates.clone(), work_column),
                    None => Resolved::Unresolved,
                },
            };
        }

        let resolution = girsa_ref::resolve(self.lexicon, citation);
        match resolution {
            Resolution::Exact(r) => {
                self.cache.insert(citation.to_string(), Some(r.clone()));
                Resolved::Exact(r)
            }
            Resolution::Ambiguous(candidates) => {
                self.cache.insert(citation.to_string(), None);
                self.ambiguous
                    .insert(citation.to_string(), candidates.clone());
                self.settle(candidates, work_column)
            }
            Resolution::Unresolved => {
                self.cache.insert(citation.to_string(), None);
                Resolved::Unresolved
            }
        }
    }

    /// Narrow candidates using the work named in the row's own `Text N`.
    fn settle(&mut self, candidates: Vec<Ref>, work_column: &str) -> Resolved {
        let work_column = work_column.trim();
        if !work_column.is_empty() {
            if let Some(slug) = self.work_slug(work_column) {
                let mut matching: Vec<Ref> = candidates
                    .iter()
                    .filter(|r| r.work_slug() == slug)
                    .cloned()
                    .collect();
                if matching.len() == 1 {
                    return Resolved::Exact(matching.remove(0));
                }
            }
        }
        Resolved::Ambiguous(candidates.len())
    }

    /// The slug for a bare work title, cached.
    fn work_slug(&mut self, title: &str) -> Option<String> {
        if let Some(cached) = self.works.get(title) {
            return cached.clone();
        }
        let slug = match girsa_ref::resolve(self.lexicon, title) {
            Resolution::Exact(r) => Some(r.work_slug()),
            // A bare title that is itself ambiguous settles nothing.
            _ => None,
        };
        self.works.insert(title.to_string(), slug.clone());
        slug
    }
}

/// What a citation turned out to be, once the row's own columns were used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Exact(Ref),
    /// Still several works after `Text N` was consulted. Counted, dropped.
    Ambiguous(usize),
    Unresolved,
}

/// Read one `links*.csv` into edges.
///
/// # Errors
///
/// If the file cannot be read. A row that cannot be resolved is counted in the
/// [`Tally`], not an error — three quarters of a corpus is not an exception.
pub fn read_file(
    path: &Path,
    resolver: &mut Resolver<'_>,
    index: &SegmentIndex,
    mut emit: impl FnMut(Edge),
) -> Result<Tally, std::io::Error> {
    let body = fs::read_to_string(path)?;
    let mut tally = Tally::default();

    for line in body.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let row = fields(line);
        let Some(citation_1) = row.get(link_columns::CITATION_1) else {
            continue;
        };
        let Some(citation_2) = row.get(link_columns::CITATION_2) else {
            continue;
        };
        tally.rows += 1;

        // T2. The misspelling is the data's, and matching it is not optional:
        // read from a correctly spelled column this is always empty.
        let label = row
            .get(link_columns::CONECTION_TYPE)
            .map(String::as_str)
            .unwrap_or_default();
        if label.trim().is_empty() {
            tally.untyped += 1;
        }

        let text_1 = row
            .get(link_columns::TEXT_1)
            .map(String::as_str)
            .unwrap_or("");
        let text_2 = row
            .get(link_columns::TEXT_2)
            .map(String::as_str)
            .unwrap_or("");

        let from = resolver.resolve_citation(citation_1, text_1);
        let to = resolver.resolve_citation(citation_2, text_2);

        let (Resolved::Exact(from), Resolved::Exact(to)) = (&from, &to) else {
            if matches!(from, Resolved::Ambiguous(_)) || matches!(to, Resolved::Ambiguous(_)) {
                tally.ambiguous += 1;
            } else {
                tally.unresolved_citation += 1;
            }
            continue;
        };

        let (Some(from_run), Some(to_run)) = (index.resolve(from), index.resolve(to)) else {
            // Two different facts, and lumping them together hides which one
            // this is: a work Sefaria catalogues and has no Hebrew text for is
            // not the same defect as an address that is not in a work we have.
            if !index.has_work(&from.work_slug()) || !index.has_work(&to.work_slug()) {
                tally.work_not_on_shelf += 1;
            } else {
                tally.address_not_found += 1;
            }
            continue;
        };

        tally.imported += 1;
        emit(Edge {
            from: to_anchor(from_run),
            to: to_anchor(to_run),
            edge_type: EdgeType::from_sefaria(label),
            method: Method::SefariaSeed,
            source_label: label.trim().to_string(),
        });
    }
    Ok(tally)
}

fn to_anchor(run: girsa_corpus::index::Run) -> Anchor {
    match run.last {
        Some(last) => Anchor::span(run.first, last),
        None => Anchor::point(run.first),
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_ref::Work;

    fn lexicon() -> Lexicon {
        let mut lex = Lexicon::default();
        lex.add(
            Work {
                slug: "shulchan-arukh/orach-chayim".into(),
                he_title: "שולחן ערוך, אורח חיים".into(),
                en_title: "Shulchan Arukh, Orach Chayim".into(),
            },
            &["Shulchan Arukh, Orach Chayim", "או\"ח"],
        );
        lex.add(
            Work {
                slug: "tur/orach-chayim".into(),
                he_title: "טור, אורח חיים".into(),
                en_title: "Tur, Orach Chayim".into(),
            },
            &["Tur, Orach Chayim", "או\"ח"],
        );
        lex
    }

    #[test]
    fn sefarias_own_work_column_settles_a_tie_the_citation_cannot() {
        // `או"ח` is Orach Chayim in the Shulchan Arukh and in the Tur, and the
        // resolver is right to refuse to choose. The row itself says which,
        // in a different column, and reading it is not a guess.
        let lexicon = lexicon();
        let mut resolver = Resolver::new(&lexicon);
        let settled = resolver.resolve_citation("או\"ח א'", "Tur, Orach Chayim");
        assert_eq!(
            settled,
            Resolved::Exact("girsa:tur/orach-chayim/1".parse().expect("parses"))
        );
    }

    #[test]
    fn an_ambiguity_the_row_does_not_settle_is_dropped_and_not_picked() {
        // BUILDER.md rule 6. An edge is followed by a reader who is never asked
        // anything, so an ambiguous link is a wrong link half the time — and a
        // wrong link does not look wrong.
        let lexicon = lexicon();
        let mut resolver = Resolver::new(&lexicon);
        assert_eq!(
            resolver.resolve_citation("או\"ח א'", ""),
            Resolved::Ambiguous(2)
        );
        assert_eq!(
            resolver.resolve_citation("או\"ח א'", "Something Else Entirely"),
            Resolved::Ambiguous(2)
        );
    }

    #[test]
    fn an_unknown_sefer_is_unresolved_rather_than_the_nearest_match() {
        let lexicon = lexicon();
        let mut resolver = Resolver::new(&lexicon);
        assert_eq!(
            resolver.resolve_citation("Keren Orah on Nedarim 2a", "Keren Orah"),
            Resolved::Unresolved
        );
    }

    #[test]
    fn the_cache_answers_the_same_citation_the_same_way() {
        let lexicon = lexicon();
        let mut resolver = Resolver::new(&lexicon);
        let first = resolver.resolve_citation("Shulchan Arukh, Orach Chayim 1:1", "");
        let again = resolver.resolve_citation("Shulchan Arukh, Orach Chayim 1:1", "");
        assert_eq!(first, again);
        assert!(matches!(first, Resolved::Exact(_)));

        // And an ambiguous one stays ambiguous, rather than the cache turning
        // it into a miss on the second visit.
        let a = resolver.resolve_citation("או\"ח א'", "");
        let b = resolver.resolve_citation("או\"ח א'", "");
        assert_eq!(a, b);
        assert_eq!(a, Resolved::Ambiguous(2));

        // …and still settles when the row says which.
        assert!(matches!(
            resolver.resolve_citation("או\"ח א'", "Tur, Orach Chayim"),
            Resolved::Exact(_)
        ));
    }
}
