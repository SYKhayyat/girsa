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

use std::collections::{BTreeMap, HashMap};
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
    /// The citation named two or more works, and neither the row's own `Text N`
    /// column nor the shelf said which. Never picked — see
    /// [`Resolver::resolve_citation`] — and never only counted either: each one
    /// is written out with its candidates ([`Unsettled`]), because rule 6 says
    /// ambiguity is surfaced as a **choice**, and a choice nobody can see is a
    /// drop with better manners.
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
    works: HashMap<String, Vec<String>>,
    /// What nothing settled, kept so it can be written down.
    unsettled: BTreeMap<String, Unsettled>,
    /// Endpoints an ambiguity was taken off by the row's own `Text N`.
    settled_by_column: usize,
    /// Endpoints an ambiguity was taken off because every other candidate names
    /// no place on the shelf. Counted apart from the column because it is the
    /// weaker evidence of the two and its size should be visible.
    settled_by_shelf: usize,
}

/// One citation the row's own columns and the shelf between them could not
/// narrow to a single work.
///
/// A count is not enough. BUILDER.md rule 6 says ambiguity is surfaced to the
/// reader **as a choice**, and a choice nobody can see is a drop with better
/// manners — so every one of these is written out with its candidates, ready
/// for the repair queue W23 builds. The import cannot ask anybody anything;
/// what it can do is not throw the question away.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Unsettled {
    /// The citation exactly as the CSV wrote it.
    pub citation: String,
    /// Every work it could equally be, as refs.
    pub candidates: Vec<String>,
    /// How many times this citation turned up as one end of a link. Not the
    /// number of edges it would unlock — the other end may have failed too.
    pub occurrences: usize,
    /// Whether every candidate is a place that exists on the shelf. When it is
    /// false, some candidate is a work Sefaria catalogues and has no Hebrew
    /// text for, and the choice cannot be made from the corpus alone.
    pub all_on_shelf: bool,
}

impl<'a> Resolver<'a> {
    #[must_use]
    pub fn new(lexicon: &'a Lexicon) -> Self {
        Self {
            lexicon,
            cache: HashMap::new(),
            ambiguous: HashMap::new(),
            works: HashMap::new(),
            unsettled: BTreeMap::new(),
            settled_by_column: 0,
            settled_by_shelf: 0,
        }
    }

    /// How many ambiguous endpoints each kind of evidence took off, so the
    /// narrowing is a reported number rather than an invisible improvement to
    /// the import rate.
    #[must_use]
    pub fn settled(&self) -> (usize, usize) {
        (self.settled_by_column, self.settled_by_shelf)
    }

    /// The slug a bare work title names, if exactly one work goes by it.
    ///
    /// Public because W8's Otzaria half needs the same answer for a different
    /// reason: T4 says to resolve an Otzaria link target **by filename**, and a
    /// filename is a bare title.
    pub fn work_slug_of(&mut self, title: &str) -> Option<String> {
        match self.work_slugs(title) {
            slugs if slugs.len() == 1 => slugs.into_iter().next(),
            _ => None,
        }
    }

    /// Record an ambiguity that did not come from a citation.
    ///
    /// The Otzaria half asks the same question from the other direction — T4
    /// resolves a link target **by filename**, and a filename two seforim
    /// answer to settles nothing either. One queue, because it is one question:
    /// *which sefer did this mean?*
    pub fn record_unsettled(&mut self, key: &str, candidates: Vec<String>, all_on_shelf: bool) {
        let entry = self
            .unsettled
            .entry(key.to_string())
            .or_insert_with(|| Unsettled {
                citation: key.to_string(),
                candidates,
                occurrences: 0,
                all_on_shelf,
            });
        entry.occurrences += 1;
    }

    /// Every ambiguity that survived, worst first.
    pub fn unsettled(&self) -> Vec<&Unsettled> {
        let mut out: Vec<&Unsettled> = self.unsettled.values().collect();
        out.sort_by(|a, b| {
            b.occurrences
                .cmp(&a.occurrences)
                .then(a.citation.cmp(&b.citation))
        });
        out
    }

    /// Resolve a citation, narrowing it by everything the row and the corpus
    /// already say — and by nothing else.
    ///
    /// `או"ח` is Orach Chayim in the Shulchan Arukh *and* in the Tur, and
    /// `girsa-ref` returns both rather than choosing — BUILDER.md rule 6. Two
    /// things narrow that without anybody guessing:
    ///
    /// 1. **The row's own `Text N` column**, which names the work the citation
    ///    came from separately from the citation itself. Using it is not a
    ///    guess, it is reading the other column. It narrows even when it is
    ///    *itself* ambiguous: a column meaning either of two seforim, against a
    ///    citation meaning either of two others, can still leave exactly one in
    ///    common.
    /// 2. **The shelf.** A candidate whose work is here and whose address is
    ///    not in it is not a place — the citation cannot have meant it. This is
    ///    elimination, not selection, and it is the same rule
    ///    [`girsa_corpus::index`] already applies to a section name that might
    ///    be two levels: *accepted only if exactly one of them is a real
    ///    address in that work*.
    ///
    ///    A candidate whose work is **not** on the shelf cannot be eliminated
    ///    this way — nothing here knows what is inside a sefer it does not
    ///    have — so one of those surviving keeps the whole thing a choice. That
    ///    asymmetry is the point: refuting a candidate needs evidence, and
    ///    absence of a sefer is not evidence about its contents.
    ///
    ///    It inherits the address lookup's own limits, and honestly: where the
    ///    lookup cannot find a real address, this reads it as a refutation. That
    ///    is why rows settled this way are counted separately in the [`Tally`]
    ///    rather than folded into the import rate.
    ///
    /// What neither settles is counted, **dropped, and written down** — see
    /// [`Unsettled`]. An edge is followed by a reader who is not asked
    /// anything, so an ambiguous link is a wrong link half the time.
    pub fn resolve_citation(
        &mut self,
        citation: &str,
        work_column: &str,
        index: &SegmentIndex,
    ) -> Resolved {
        let citation = citation.trim();
        if citation.is_empty() {
            return Resolved::Unresolved;
        }
        if let Some(cached) = self.cache.get(citation) {
            return match cached {
                Some(r) => Resolved::Exact(r.clone()),
                None => match self.ambiguous.get(citation) {
                    Some(candidates) => {
                        self.settle(citation, candidates.clone(), work_column, index)
                    }
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
                self.settle(citation, candidates, work_column, index)
            }
            Resolution::Unresolved => {
                self.cache.insert(citation.to_string(), None);
                Resolved::Unresolved
            }
        }
    }

    /// Narrow candidates by the row's own `Text N` and then by the shelf.
    fn settle(
        &mut self,
        citation: &str,
        candidates: Vec<Ref>,
        work_column: &str,
        index: &SegmentIndex,
    ) -> Resolved {
        let mut candidates = self.narrow_by_column(candidates, work_column);
        if candidates.len() == 1 {
            self.settled_by_column += 1;
            return Resolved::Exact(candidates.remove(0));
        }

        // Elimination by the shelf. Three outcomes per candidate, and the
        // difference between the last two is the whole safety of this step.
        let mut real: Vec<Ref> = Vec::new();
        let mut unknown: Vec<Ref> = Vec::new();
        for candidate in candidates {
            if index.resolve(&candidate).is_some() {
                real.push(candidate);
            } else if !index.has_work(&candidate.work_slug()) {
                unknown.push(candidate);
            }
            // Otherwise: on the shelf, and this address is not in it. Refuted.
        }

        if unknown.is_empty() {
            match real.len() {
                // Every candidate refuted. Not an ambiguity — there is no place
                // here at all, and saying "ambiguous" would blame the reader
                // for a citation that names nothing.
                0 => return Resolved::NoPlace,
                1 => {
                    self.settled_by_shelf += 1;
                    return Resolved::Exact(real.remove(0));
                }
                _ => {}
            }
        }

        real.extend(unknown);
        let all_on_shelf = real.iter().all(|r| index.has_work(&r.work_slug()));
        self.record_unsettled(
            citation,
            real.iter().map(ToString::to_string).collect(),
            all_on_shelf,
        );
        Resolved::Ambiguous(real)
    }

    /// Keep only the candidates the row's own work column also allows.
    ///
    /// An intersection rather than a match, so a column that is itself
    /// ambiguous still narrows. A column naming nothing in the candidate set
    /// says nothing about it, and the set comes back untouched.
    fn narrow_by_column(&mut self, candidates: Vec<Ref>, work_column: &str) -> Vec<Ref> {
        let work_column = work_column.trim();
        if work_column.is_empty() {
            return candidates;
        }
        let allowed = self.work_slugs(work_column);
        if allowed.is_empty() {
            return candidates;
        }
        let narrowed: Vec<Ref> = candidates
            .iter()
            .filter(|r| allowed.contains(&r.work_slug()))
            .cloned()
            .collect();
        if narrowed.is_empty() {
            candidates
        } else {
            narrowed
        }
    }

    /// Every work a bare title could name, cached.
    fn work_slugs(&mut self, title: &str) -> Vec<String> {
        if let Some(cached) = self.works.get(title) {
            return cached.clone();
        }
        let mut slugs: Vec<String> = girsa_ref::resolve(self.lexicon, title)
            .candidates()
            .iter()
            .map(Ref::work_slug)
            .collect();
        slugs.sort();
        slugs.dedup();
        self.works.insert(title.to_string(), slugs.clone());
        slugs
    }
}

/// What a citation turned out to be, once the row's own columns and the shelf
/// were used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Exact(Ref),
    /// Still several works. Counted, dropped, and written down as a choice
    /// nobody was here to make — see [`Unsettled`].
    Ambiguous(Vec<Ref>),
    /// Every candidate was a work on the shelf that does not contain this
    /// address. A different fact from an ambiguity, and reported as one.
    NoPlace,
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

        let from = resolver.resolve_citation(citation_1, text_1, index);
        let to = resolver.resolve_citation(citation_2, text_2, index);

        let (Resolved::Exact(from), Resolved::Exact(to)) = (&from, &to) else {
            // Most specific fact first. A citation that named nothing at all is
            // a different problem from one that named two seforim, and one that
            // named seforim none of which contains the address is a third —
            // lumping any two of them together hides which it was.
            if matches!(from, Resolved::Unresolved) || matches!(to, Resolved::Unresolved) {
                tally.unresolved_citation += 1;
            } else if matches!(from, Resolved::NoPlace) || matches!(to, Resolved::NoPlace) {
                tally.address_not_found += 1;
            } else {
                tally.ambiguous += 1;
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
            // Not known here — this reader has two citations and a label, and
            // the label is exactly what does not say which is which. The
            // orienting pass stamps it.
            direction: crate::Direction::NotRecorded,
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
        // A third sefer that also answers to או"ח, so a `Text N` column which is
        // itself ambiguous still has something to narrow.
        lex.add(
            Work {
                slug: "levush/orach-chayim".into(),
                he_title: "לבוש, אורח חיים".into(),
                en_title: "Levush, Orach Chayim".into(),
            },
            &["Levush, Orach Chayim", "או\"ח"],
        );
        // Written out, `אורח חיים` is the Tur's volume and also the Mishnah
        // Berurah's — but not the Shulchan Arukh's or the Levush's, which is
        // what makes it able to narrow without settling on its own.
        lex.add(
            Work {
                slug: "mishnah-berurah".into(),
                he_title: "משנה ברורה".into(),
                en_title: "Mishnah Berurah".into(),
            },
            &["Mishnah Berurah", "אורח חיים"],
        );
        lex.add(
            Work {
                slug: "tur/orach-chayim".into(),
                he_title: "טור, אורח חיים".into(),
                en_title: "Tur, Orach Chayim".into(),
            },
            &["אורח חיים"],
        );
        lex
    }

    /// A shelf holding the two Orach Chayims, each with the simanim it has.
    ///
    /// The Shulchan Arukh's O.C. runs to 697 simanim and the Tur's to 5 here,
    /// which is enough to make the two disagree about whether a given siman is
    /// a place.
    fn shelf(works: &[(&str, u32)]) -> SegmentIndex {
        use girsa_corpus::index::WorkSegments;
        use girsa_corpus::segment::Ordinal;

        let mut index = SegmentIndex::default();
        for (slug, simanim) in works {
            let paths: Vec<(Vec<String>, Ordinal)> = (1..=*simanim)
                .map(|n| (vec![n.to_string()], Ordinal::root(n)))
                .collect();
            index.insert(
                *slug,
                WorkSegments::from_segments(paths.iter().map(|(p, o)| (p.as_slice(), o))),
            );
        }
        index
    }

    /// Both Orach Chayims present, both deep enough to contain siman 1.
    fn both_on_the_shelf() -> SegmentIndex {
        shelf(&[
            ("shulchan-arukh/orach-chayim", 10),
            ("tur/orach-chayim", 10),
            ("levush/orach-chayim", 10),
        ])
    }

    #[test]
    fn sefarias_own_work_column_settles_a_tie_the_citation_cannot() {
        // `או"ח` is Orach Chayim in the Shulchan Arukh and in the Tur, and the
        // resolver is right to refuse to choose. The row itself says which,
        // in a different column, and reading it is not a guess.
        let lexicon = lexicon();
        let index = both_on_the_shelf();
        let mut resolver = Resolver::new(&lexicon);
        let settled = resolver.resolve_citation("או\"ח א'", "Tur, Orach Chayim", &index);
        assert_eq!(
            settled,
            Resolved::Exact("girsa:tur/orach-chayim/1".parse().expect("parses"))
        );
    }

    #[test]
    fn a_work_column_that_is_itself_ambiguous_still_narrows() {
        // Sefaria's `Text N` is a title, and a title can be ambiguous too —
        // which the first version read as "settles nothing" and threw away. A
        // column meaning either of three seforim, against a citation meaning
        // either of two, leaves exactly one in common. Intersecting two sets is
        // not a guess; taking the first of them would be.
        let lexicon = lexicon();
        let index = both_on_the_shelf();
        let mut resolver = Resolver::new(&lexicon);

        // `או"ח` is three seforim. `אורח חיים` is two, and only one of them is
        // in the first set. Neither column nor citation says which alone.
        let settled = resolver.resolve_citation("או\"ח א'", "אורח חיים", &index);
        assert_eq!(
            settled,
            Resolved::Exact("girsa:tur/orach-chayim/1".parse().expect("parses")),
        );
        assert_eq!(
            resolver.settled(),
            (1, 0),
            "the column's doing, not the shelf's"
        );
    }

    #[test]
    fn a_candidate_that_names_no_place_on_the_shelf_is_not_a_candidate() {
        // The Tur's Orach Chayim here stops at siman 5. A citation to siman 9
        // cannot have meant it: that is not a choice between two places, it is
        // one place and one thing that is not a place. Eliminating it is the
        // rule girsa_corpus::index already applies to a section name that might
        // be two levels.
        let lexicon = lexicon();
        let index = shelf(&[
            ("shulchan-arukh/orach-chayim", 20),
            ("tur/orach-chayim", 5),
            ("levush/orach-chayim", 5),
        ]);
        let mut resolver = Resolver::new(&lexicon);
        assert_eq!(
            resolver.resolve_citation("או\"ח ט'", "", &index),
            Resolved::Exact(
                "girsa:shulchan-arukh/orach-chayim/9"
                    .parse()
                    .expect("parses")
            )
        );
        assert_eq!(resolver.settled(), (0, 1), "counted as the shelf's doing");
    }

    #[test]
    fn a_candidate_that_is_not_on_the_shelf_cannot_be_ruled_out() {
        // The asymmetry that makes the step above safe. Nothing here knows what
        // is inside a sefer the shelf does not have — Sefaria catalogues 387
        // works it ships no Hebrew text for — so "we do not have it" is not
        // evidence that the citation did not mean it. One of those surviving
        // keeps the whole thing a choice.
        let lexicon = lexicon();
        let index = shelf(&[("shulchan-arukh/orach-chayim", 20)]);
        let mut resolver = Resolver::new(&lexicon);
        let r = resolver.resolve_citation("או\"ח ט'", "", &index);
        assert!(
            matches!(&r, Resolved::Ambiguous(candidates) if candidates.len() == 3),
            "{r:?}"
        );
        assert_eq!(resolver.settled(), (0, 0));
    }

    #[test]
    fn a_citation_no_candidate_contains_is_a_missing_address_not_an_ambiguity() {
        // Every candidate on the shelf and none of them holding siman 99. There
        // is nothing to choose between, and calling it ambiguous would report a
        // question where there is only an absence.
        let lexicon = lexicon();
        let index = both_on_the_shelf();
        let mut resolver = Resolver::new(&lexicon);
        assert_eq!(
            resolver.resolve_citation("או\"ח צ\"ט", "", &index),
            Resolved::NoPlace
        );
    }

    #[test]
    fn an_ambiguity_nothing_settles_is_dropped_and_written_down() {
        // BUILDER.md rule 6. An edge is followed by a reader who is never asked
        // anything, so an ambiguous link is a wrong link half the time — and a
        // wrong link does not look wrong.
        //
        // But rule 6 says ambiguity is surfaced **as a choice**, and a choice
        // nobody can see is a drop with better manners. So it is also recorded:
        // the citation, every candidate, and how often it came up.
        let lexicon = lexicon();
        let index = both_on_the_shelf();
        let mut resolver = Resolver::new(&lexicon);
        for column in ["", "Something Else Entirely"] {
            let r = resolver.resolve_citation("או\"ח א'", column, &index);
            assert!(
                matches!(&r, Resolved::Ambiguous(candidates) if candidates.len() == 3),
                "{column:?} -> {r:?}"
            );
        }

        let unsettled = resolver.unsettled();
        assert_eq!(unsettled.len(), 1);
        assert_eq!(unsettled[0].citation, "או\"ח א'");
        assert_eq!(unsettled[0].occurrences, 2);
        assert!(unsettled[0].all_on_shelf);
        let mut candidates = unsettled[0].candidates.clone();
        candidates.sort();
        assert_eq!(
            candidates,
            [
                "girsa:levush/orach-chayim/1",
                "girsa:shulchan-arukh/orach-chayim/1",
                "girsa:tur/orach-chayim/1",
            ]
        );
    }

    #[test]
    fn an_unknown_sefer_is_unresolved_rather_than_the_nearest_match() {
        let lexicon = lexicon();
        let index = both_on_the_shelf();
        let mut resolver = Resolver::new(&lexicon);
        assert_eq!(
            resolver.resolve_citation("Keren Orah on Nedarim 2a", "Keren Orah", &index),
            Resolved::Unresolved
        );
    }

    #[test]
    fn the_cache_answers_the_same_citation_the_same_way() {
        let lexicon = lexicon();
        let index = both_on_the_shelf();
        let mut resolver = Resolver::new(&lexicon);
        let first = resolver.resolve_citation("Shulchan Arukh, Orach Chayim 1:1", "", &index);
        let again = resolver.resolve_citation("Shulchan Arukh, Orach Chayim 1:1", "", &index);
        assert_eq!(first, again);
        assert!(matches!(first, Resolved::Exact(_)));

        // And an ambiguous one stays ambiguous, rather than the cache turning
        // it into a miss on the second visit.
        let a = resolver.resolve_citation("או\"ח א'", "", &index);
        let b = resolver.resolve_citation("או\"ח א'", "", &index);
        assert_eq!(a, b);
        assert!(matches!(a, Resolved::Ambiguous(_)));

        // …and still settles when the row says which.
        assert!(matches!(
            resolver.resolve_citation("או\"ח א'", "Tur, Orach Chayim", &index),
            Resolved::Exact(_)
        ));
    }
}
