//! One index, normalized once (BUILDER.md W11).
//!
//! Every segment on the shelf goes in through [`tokenizer::Normalized`], which
//! is `girsa-hebrew` — so the terms on disk are exactly the normal forms the
//! query bar produces, and **nothing else**. Nikud and te'amim come off here and
//! in every mode, with no toggle (spec.md §9.1), because nobody searches with
//! them on and one index is simpler and faster than two.
//!
//! # What is deliberately *not* in the index
//!
//! No peeled prefixes, no expanded abbreviations, no roots. Those are
//! [`girsa_hebrew::variants`] and they are applied at query time, by a reader
//! who asked. If they were baked in here:
//!
//! - the literal default of spec.md §9.3 would be unimplementable — there would
//!   be no un-widened index left to search;
//! - §9.6's *"[try other forms — 7]"* could not show a count before the click,
//!   because the widened and unwidened result sets would be the same set.
//!
//! So the widening lives one layer up (W12, W13) and this layer stays honest.
//!
//! # The stale-index problem, and why it gets a file of its own
//!
//! The terms on disk were written by one version of the normalizer. A query
//! normalized by a *different* version and run against them does not error —
//! it returns nothing, or returns less, and the reader is told the sefer does
//! not contain a line that is printed in front of them. That is the worst
//! failure mode in this system, because it looks like an answer.
//!
//! So every index carries a [`Stamp`], and [`SearchIndex::open`] refuses one it
//! did not write. The index is a rebuildable cache (spec.md §4.1); refusing it
//! costs a rebuild, and trusting it costs the reader's trust in the search box.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::CacheProvenance;
use girsa_link::EdgeType;
use girsa_scan::reading::Reader;
use serde::{Deserialize, Serialize};
use tantivy::collector::{Collector, Count, SegmentCollector, TopDocs};
use tantivy::columnar::StrColumn;
use tantivy::query::{
    BooleanQuery, Occur, PhraseQuery, Query, RegexPhraseQuery, RegexQuery, TermQuery,
};
use tantivy::schema::{
    IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, FAST, STORED, STRING,
};
use tantivy::{
    DocId, IndexReader, IndexWriter, ReloadPolicy, Score, SegmentOrdinal, SegmentReader,
    TantivyDocument, Term,
};

use crate::ladder::{
    Alternative, Form, Offer, Offers, Position, Refusal, Rung, Standing, Widened, Widening,
    MOST_EXACT_QUERIES,
};
use crate::scope::Scope;
use crate::tokenizer;
use crate::torat_emet::{self, Match, Plan, Together, MOST_WORDS_UNORDERED};

/// The file that says what rules this index was built under.
pub const CACHE_STAMP: &str = "girsa-cache.json";

/// The file that says what went **into** this index, as against under what
/// rules.
///
/// A different question from [`CACHE_STAMP`] and it has to be asked separately.
/// The stamp says the normalizer agrees; this says whether the link-type cache
/// existed when the index was built — and without it the link facet has to
/// report *not built* rather than a column of zeros (spec.md §9.7's rule, one
/// facet over).
pub const BUILD_REPORT: &str = "girsa-build.json";

/// Whether a directory holds an index, and so may be thrown away and rebuilt.
///
/// Deliberately generous about *which* index: tantivy writes `meta.json` and
/// Girsa writes [`CACHE_STAMP`] beside it, and either one is enough. An index
/// half-written by a run that was killed has the first and not the second, and
/// refusing to rebuild that would be refusing the exact case rebuilding is for.
///
/// An empty directory is fine — there is nothing there to lose. Anything else
/// is somebody's data. See [`SearchIndex::rebuild`] for what this cost before
/// it existed.
#[must_use]
pub fn looks_like_an_index(dir: &Path) -> bool {
    if dir.join(CACHE_STAMP).is_file() || dir.join("meta.json").is_file() {
        return true;
    }
    std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none())
}

/// Bumped when the *shape* of the index changes — a field added, a field
/// indexed differently.
///
/// Separate from the normalizer's version because they go stale for different
/// reasons and both have to be checked. spec.md §9.7 will add PDF pages as a
/// second location type in this same index, and that is a schema change: an
/// index written before it is not wrong, it is *incomplete*, and reading it as
/// though it were complete would produce exactly the silent gap §9.7 forbids.
///
/// **2** — W14 added the `link` column, which is what makes §9.8's link-type
/// facet a count rather than a guess. An index built at 1 has every other field
/// right and would show that facet as empty, so it is refused and rebuilt.
///
/// **3** — W26 added the `by` column and the words of a scan's pages, which is
/// §9.7's second location type. This is exactly the case the paragraph above
/// anticipated: an index built at 2 is not *wrong*, it is **incomplete** — it
/// has every scan in the library as a row of blank pages — and reading it as
/// though it were complete is the silent gap §9.7 forbids. So it is refused.
pub const SCHEMA_VERSION: u32 = 3;

/// How many hits the index's own probes return.
///
/// These are not the search UI. Paging, facets and the relaxation ladder are
/// W13 and W14; this is enough to check that what went in can be found.
pub const PROBE_LIMIT: usize = 100;

/// The default writer budget. Overridden by the indexer, which has a corpus to
/// get through and a machine to use.
const DEFAULT_HEAP_BYTES: usize = 64 * 1024 * 1024;

/// What an index was built under.
///
/// Written on create, checked on open. Compared as a whole: a field added here
/// later is a field an older stamp does not have, and `serde` failing to
/// deserialize it is the same refusal by another route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub schema_version: u32,
    pub normalizer_version: u32,
    pub ref_scheme: String,
}

impl Stamp {
    /// What an index built by *this* binary is stamped with.
    #[must_use]
    pub fn current() -> Self {
        let provenance = CacheProvenance::current();
        Self {
            schema_version: SCHEMA_VERSION,
            normalizer_version: provenance.normalizer_version,
            ref_scheme: provenance.ref_scheme.to_string(),
        }
    }

    /// Why this stamp is not the current one, in words a person can act on.
    #[must_use]
    pub fn disagreement(&self) -> Option<String> {
        let current = Self::current();
        if self == &current {
            return None;
        }
        Some(format!(
            "built under schema {} / normalizer {} / refs {}; this build wants schema {} / \
             normalizer {} / refs {}",
            self.schema_version,
            self.normalizer_version,
            self.ref_scheme,
            current.schema_version,
            current.normalizer_version,
            current.ref_scheme,
        ))
    }
}

/// Why an index could not be built, opened or asked.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// The index on disk was written under rules that no longer hold, or was
    /// not written by Girsa at all. **Rebuild it** — never read it anyway.
    #[error("the index at {path} cannot be trusted: {reason}")]
    Stale { path: String, reason: String },
    /// [`SearchIndex::rebuild`] was pointed at a directory holding something
    /// that is not an index.
    ///
    /// Rebuilding deletes the directory first, so this is the difference
    /// between a mistyped argument and a corpus. See [`SearchIndex::rebuild`].
    #[error(
        "{path} is not an index and will not be deleted — `girsa-index build` takes the \
         index directory first and the corpus roots after it"
    )]
    NotAnIndex { path: String },
    #[error("the index is missing the field {0}, which this build requires")]
    Field(&'static str),
    /// A `contains` or `letters` pattern matched more distinct words than a
    /// phrase search can hold at once.
    ///
    /// **Refused, not trimmed.** Running the phrase over the first few thousand
    /// of the matching words would return a subset of the truth wearing the
    /// face of the whole of it, and the reader would have no way to tell.
    #[error(
        "those letters match more than {limit} different words — narrow them, or drop the \
         proximity and search for the letters alone"
    )]
    TooBroad { limit: u32 },
    /// Order-free proximity over more words than there are orderings for.
    #[error(
        "{words} words is too many to check in any order (the limit is {limit}) — ask for them \
         in order instead"
    )]
    TooManyWords { words: usize, limit: usize },
    /// Widening a proximity query into every form of every word would take more
    /// exact searches than the ceiling allows.
    #[error(
        "widening those words into all their forms would take {queries} exact searches (the \
         limit is {limit}) — narrow the query, or take one rung at a time"
    )]
    TooManyForms { queries: usize, limit: usize },
    /// Asked of the index, and not a question an index can answer.
    ///
    /// Named rather than answered with the nearest thing an inverted index can
    /// do. A dilug over an index of words would be a different instrument
    /// wearing this one's name.
    #[error("{what} is not something the index can answer — {instead}")]
    NotAnIndexQuestion {
        what: &'static str,
        instead: &'static str,
    },
    #[error(transparent)]
    Tantivy(#[from] tantivy::TantivyError),
}

impl IndexError {
    fn io(path: &Path) -> impl Fn(std::io::Error) -> Self + '_ {
        move |source| Self::Io {
            path: path.display().to_string(),
            source,
        }
    }

    fn stale(path: &Path, reason: impl Into<String>) -> Self {
        Self::Stale {
            path: path.display().to_string(),
            reason: reason.into(),
        }
    }

    /// Give tantivy's expansion ceiling a name of ours.
    ///
    /// It arrives as a formatted string, so this reads one. The test
    /// `a_pattern_that_matches_too_much_is_refused_rather_than_quietly_cut`
    /// pins it: if a tantivy upgrade rewords the message, that test fails
    /// rather than the refusal silently becoming a generic error.
    fn from_tantivy(error: tantivy::TantivyError) -> Self {
        match &error {
            tantivy::TantivyError::InvalidArgument(message)
                if message.contains("max expansions") =>
            {
                Self::TooBroad {
                    limit: crate::torat_emet::MOST_EXPANSIONS,
                }
            }
            _ => Self::Tantivy(error),
        }
    }
}

/// What a search found, and what it was asked.
#[derive(Debug, Clone)]
pub struct Found {
    /// The first page of them, best first.
    pub hits: Vec<Hit>,
    /// How many there were altogether. Counted, not estimated — a result
    /// header that cannot say *"1 of 4,190"* is a header nobody can act on,
    /// and W13's ladder is counts computed before the click.
    pub total: usize,
    /// Exactly what was searched for. The literal mode's promise, in a struct.
    pub asked: Plan,
    /// What was done to the query beyond taking its marks off, if anything.
    /// `None` in the literal mode, always.
    pub widening: Option<Widening>,
}

impl Found {
    /// The rule that says what to highlight in these hits.
    ///
    /// Routed through the widening when there is one, so a hit found by peeling
    /// a prefix is marked on `וכשהמלך` — the word that actually answered the
    /// question — rather than on the three letters of it the reader typed.
    ///
    /// **One description of that rule.** `Found::marks` and
    /// [`crate::bar::Marker`] were two, character-identical, and each had its
    /// own caller: `girsa-index find` marked its results through one and the
    /// window through the other. `torat_emet.rs:196` states the hazard for a
    /// different pair — *"two descriptions of one rule drift"* — and it applies
    /// here exactly.
    #[must_use]
    pub fn marker(&self) -> crate::bar::Marker {
        self.widening.as_ref().map_or_else(
            || crate::bar::Marker::Literal(self.asked.clone()),
            |widening| crate::bar::Marker::Widened(Box::new(widening.clone())),
        )
    }

    /// Where in a hit's printed text the words that answered the query sit.
    #[must_use]
    pub fn marks(&self, hit: &Hit) -> Vec<(usize, usize)> {
        self.marker().marks(hit)
    }
}

/// An instrument, built into a query.
///
/// Instruments have no [`Plan`], because no words were typed to plan for. What
/// they have instead is [`Sounded::words`]: the words of the corpus the
/// instrument actually reached, which is half the finding when the question was
/// *which words come to 613* — and which is also what a highlight marks.
#[derive(Debug)]
pub struct Sounded {
    pub prepared: Prepared,
    /// The words this reached, where naming them is part of the answer.
    pub words: Vec<String>,
}

/// The clauses of a boolean query, before it is built.
type Clauses = Vec<(Occur, Box<dyn Query>)>;

/// A query, built, scoped, and ready to be asked more than one question.
///
/// Hits, a total and the facet counts are three questions about one query. Two
/// builds of "the same" query is how a result header comes to disagree with the
/// facet column beside it, so it is built once and asked three times.
pub struct Prepared {
    query: Box<dyn Query>,
}

impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prepared").finish_non_exhaustive()
    }
}

/// Which page of the results, and how many to a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paging {
    pub from: usize,
    pub size: usize,
}

impl Paging {
    /// The first page, at the default size.
    #[must_use]
    pub const fn first() -> Self {
        Self {
            from: 0,
            size: PROBE_LIMIT,
        }
    }

    /// A page of a size the caller chose.
    #[must_use]
    pub const fn of(size: usize) -> Self {
        Self { from: 0, size }
    }

    /// The next page along.
    #[must_use]
    pub const fn then(self) -> Self {
        Self {
            from: self.from + self.size,
            size: self.size,
        }
    }
}

impl Default for Paging {
    fn default() -> Self {
        Self::first()
    }
}

/// What a result set is made of, counted by what a reader can narrow by.
///
/// The raw material of [`crate::facets`]: this counts the two columns the index
/// carries, and the catalogue turns the works into shelves, eras and authors.
#[derive(Debug, Clone, Default)]
pub struct Counts {
    /// Hits per sefer, by slug.
    pub by_work: BTreeMap<String, usize>,
    /// Hits per kind of link touching them.
    pub by_link: BTreeMap<EdgeType, usize>,
    /// Hits altogether. Equal to the sum of [`Counts::by_work`], and not to the
    /// sum of [`Counts::by_link`] — a segment can be touched by two kinds of
    /// link and is one hit.
    pub total: usize,
    /// Whether the link column was filled in when this index was built. When it
    /// was not, [`Counts::by_link`] is empty because **nobody worked it out**,
    /// which is a different statement from *there are none*.
    pub link_types_built: bool,
}

/// Counts every matching document by the two columns a facet is drawn from.
///
/// A collector rather than one search per facet row: the alternative is
/// thousands of queries per keystroke, and the counts would be taken at
/// slightly different moments.
struct Tallies;

/// One segment's worth: ordinal counts, resolved to strings at the end.
///
/// Counting into a vector indexed by term ordinal keeps the hot loop to an
/// increment. Turning ordinals into slugs is done once per segment of the
/// index, not once per hit.
struct SegmentTallies {
    work: Option<StrColumn>,
    link: Option<StrColumn>,
    work_counts: Vec<usize>,
    link_counts: Vec<usize>,
    total: usize,
}

impl Collector for Tallies {
    type Fruit = (BTreeMap<String, usize>, BTreeMap<String, usize>, usize);
    type Child = SegmentTallies;

    fn for_segment(
        &self,
        _: SegmentOrdinal,
        reader: &SegmentReader,
    ) -> tantivy::Result<SegmentTallies> {
        let work = reader.fast_fields().str(Fields::WORK)?;
        let link = reader.fast_fields().str(Fields::LINK)?;
        let work_counts = vec![0; work.as_ref().map_or(0, |c| c.dictionary().num_terms())];
        let link_counts = vec![0; link.as_ref().map_or(0, |c| c.dictionary().num_terms())];
        Ok(SegmentTallies {
            work,
            link,
            work_counts,
            link_counts,
            total: 0,
        })
    }

    fn requires_scoring(&self) -> bool {
        false
    }

    fn merge_fruits(&self, fruits: Vec<Self::Fruit>) -> tantivy::Result<Self::Fruit> {
        let mut works: BTreeMap<String, usize> = BTreeMap::new();
        let mut links: BTreeMap<String, usize> = BTreeMap::new();
        let mut total = 0;
        for (work, link, n) in fruits {
            for (key, count) in work {
                *works.entry(key).or_default() += count;
            }
            for (key, count) in link {
                *links.entry(key).or_default() += count;
            }
            total += n;
        }
        Ok((works, links, total))
    }
}

impl SegmentCollector for SegmentTallies {
    type Fruit = (BTreeMap<String, usize>, BTreeMap<String, usize>, usize);

    fn collect(&mut self, doc: DocId, _: Score) {
        self.total += 1;
        if let Some(column) = &self.work {
            for ord in column.term_ords(doc) {
                if let Some(slot) = self.work_counts.get_mut(ord as usize) {
                    *slot += 1;
                }
            }
        }
        if let Some(column) = &self.link {
            for ord in column.term_ords(doc) {
                if let Some(slot) = self.link_counts.get_mut(ord as usize) {
                    *slot += 1;
                }
            }
        }
    }

    fn harvest(self) -> Self::Fruit {
        let named = |column: Option<&StrColumn>, counts: &[usize]| {
            let mut out = BTreeMap::new();
            let Some(column) = column else {
                return out;
            };
            let mut name = String::new();
            for (ord, count) in counts.iter().enumerate() {
                if *count == 0 {
                    continue;
                }
                name.clear();
                if column.ord_to_str(ord as u64, &mut name).unwrap_or(false) {
                    *out.entry(name.clone()).or_default() += *count;
                }
            }
            out
        };
        let works = named(self.work.as_ref(), &self.work_counts);
        let links = named(self.link.as_ref(), &self.link_counts);
        (works, links, self.total)
    }
}

/// Every ordering of the positions, for order-free proximity.
///
/// Heap's algorithm, iterative. The caller has already refused anything long
/// enough for this to matter.
fn orderings<T: Clone>(patterns: &[T]) -> Vec<Vec<T>> {
    let mut current: Vec<T> = patterns.to_vec();
    let mut out = vec![current.clone()];
    let n = current.len();
    let mut counters = vec![0usize; n];
    let mut i = 0;
    while i < n {
        if counters[i] < i {
            let j = if i % 2 == 0 { 0 } else { counters[i] };
            current.swap(j, i);
            out.push(current.clone());
            counters[i] += 1;
            i = 0;
        } else {
            counters[i] = 0;
            i += 1;
        }
    }
    out
}

/// How many combinations of alternatives these positions have between them.
///
/// Computed before any of them is built, so a query too wide to ask exactly is
/// refused instead of half-run.
fn combination_count(order: &[&Position]) -> usize {
    order
        .iter()
        .try_fold(1usize, |acc, p| acc.checked_mul(p.alternatives.len()))
        .unwrap_or(usize::MAX)
}

/// Every combination of these positions' alternatives, as flat runs of forms.
///
/// A multi-word alternative — `שו"ע` expanded to `שולחן ערוך` — flattens into
/// consecutive slots, so the expansion stays contiguous while the positions
/// around it keep the distance the reader asked for.
fn flattened(order: &[&Position]) -> Vec<Vec<Form>> {
    let mut out: Vec<Vec<Form>> = vec![Vec::new()];
    for position in order {
        let mut next: Vec<Vec<Form>> = Vec::with_capacity(out.len() * position.alternatives.len());
        for so_far in &out {
            for alternative in &position.alternatives {
                let mut sequence = so_far.clone();
                sequence.extend(alternative.forms.iter().cloned());
                next.push(sequence);
            }
        }
        out = next;
    }
    out
}

/// The five things every segment is indexed by.
#[derive(Debug, Clone, Copy)]
struct Fields {
    /// The permanent name, stored so a hit can be opened.
    id: tantivy::schema::Field,
    /// The work slug — the unit a re-import replaces, and the first facet of
    /// spec.md §9.8. Shelf, era and author are read off the work, so this one
    /// column answers four of the five.
    work: tantivy::schema::Field,
    /// `text` · `heading` · `page`. A hit inside a heading is a different kind
    /// of result, and a `page` with no words is spec.md §9.7's *"not
    /// searchable yet"* rather than an absence.
    kind: tantivy::schema::Field,
    /// Every kind of link touching this segment, in or out — the fifth facet.
    ///
    /// Multi-valued: a se'if the Mishnah Berurah comments on and the Beur
    /// Halacha quotes carries both. Written from
    /// [`girsa_link::touching`], which is the graph read from the segment's
    /// side; without that cache this column is empty and the facet says so
    /// rather than showing zeros (see [`BuildReport`]).
    link: tantivy::schema::Field,
    /// The words, as printed. Indexed through the normalizer, stored as they
    /// stand — the reader is looking at the page, not at the index.
    text: tantivy::schema::Field,
    /// Who worked out the words of this segment, where anybody had to
    /// (W26) — `embedded` for a PDF that carries its own text, the engine's
    /// name and version for a page somebody OCR'd, and **empty for the corpus**,
    /// which was not read off anything.
    ///
    /// spec.md §9.7's badge, and a facet in its own right: *only what was
    /// typeset* is one click. Stored so a row can say it, fast so it can be
    /// counted and filtered without touching the document store.
    by: tantivy::schema::Field,
}

impl Fields {
    const ID: &'static str = "id";
    const WORK: &'static str = "work";
    const KIND: &'static str = "kind";
    const LINK: &'static str = "link";
    const TEXT: &'static str = "text";
    const BY: &'static str = "by";

    fn schema() -> Schema {
        let mut builder = Schema::builder();
        let text = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer(tokenizer::NAME)
                    // Positions, because spec.md §9.3's operators include
                    // *these words within X words of each other* and a phrase
                    // is the X=1 case. Deciding this after five million
                    // segments are indexed means indexing them again.
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();
        builder.add_text_field(Self::ID, STRING | STORED);
        builder.add_text_field(Self::WORK, STRING | STORED | FAST);
        builder.add_text_field(Self::KIND, STRING | STORED | FAST);
        // Indexed *and* fast, because a facet is two things at once: a count
        // (the column) and a filter the reader clicks (the term). §9.8's
        // *"one click to narrow or exclude"* is the second half.
        builder.add_text_field(Self::LINK, STRING | FAST);
        builder.add_text_field(Self::BY, STRING | STORED | FAST);
        builder.add_text_field(Self::TEXT, text);
        builder.build()
    }

    fn of(schema: &Schema) -> Result<Self, IndexError> {
        Ok(Self {
            id: schema
                .get_field(Self::ID)
                .map_err(|_| IndexError::Field(Self::ID))?,
            work: schema
                .get_field(Self::WORK)
                .map_err(|_| IndexError::Field(Self::WORK))?,
            kind: schema
                .get_field(Self::KIND)
                .map_err(|_| IndexError::Field(Self::KIND))?,
            link: schema
                .get_field(Self::LINK)
                .map_err(|_| IndexError::Field(Self::LINK))?,
            text: schema
                .get_field(Self::TEXT)
                .map_err(|_| IndexError::Field(Self::TEXT))?,
            by: schema
                .get_field(Self::BY)
                .map_err(|_| IndexError::Field(Self::BY))?,
        })
    }
}

/// What went into an index, as against under what rules it was built.
///
/// Written on build, read by the facets. The field that matters is
/// [`BuildReport::link_types`]: an index built while the link-type cache was
/// missing has an empty `link` column, and *nothing comments on any of these
/// segments* is a different claim from *nobody worked out the link types*. One
/// of those is an answer and the other is a gap, and a facet showing zeros
/// cannot tell a reader which it is looking at.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildReport {
    pub works: usize,
    pub segments: usize,
    /// Whether `girsa-link-types` had run when this index was built.
    pub link_types: bool,
}

/// One segment, found.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// The permanent name. This is what a result row opens, what a note
    /// anchors to, and what a citation in a Ksav document carries.
    pub id: SegmentId,
    pub kind: SegmentKind,
    /// The text **as printed** — nikud, punctuation and inline markup all where
    /// the corpus has them.
    pub text: String,
    pub score: f32,
    /// Who read the words, where anybody had to (W26).
    ///
    /// `None` for the corpus, which is a text file and was not read off
    /// anything. `Some(Reader::Ocr { .. })` is spec.md §9.7's badge: the row
    /// ranks where it ranks and **says where it came from**, because OCR text
    /// is dirtier and a reader is entitled to know which kind of result is in
    /// front of them.
    pub by: Option<Reader>,
}

impl Hit {
    /// Whether this row is a machine's opinion about a photograph.
    #[must_use]
    pub fn is_scanned(&self) -> bool {
        self.by.as_ref().is_some_and(Reader::is_ocr)
    }

    /// Whether this row's id names a volume rather than a place (B12).
    ///
    /// 5,733 segments in the corpus are over 10,000 characters and the largest is
    /// 1,275,307. A hit inside one is honest about the words being *in there* and
    /// dishonest about the citation being a mareh makom, so the row says so — the
    /// same rule as the OCR badge one field up, and for the same reason: a reader is
    /// entitled to know which kind of result is in front of them.
    ///
    /// A corpus imported since B12 has none of these, because the importer cuts
    /// them. This is what a row looks like against a corpus imported before it.
    #[must_use]
    pub fn is_a_volume(&self) -> bool {
        self.text.chars().count() > girsa_corpus::oversized::NAMES_A_PLACE
    }

    /// How long the segment is, for a row that wants to say.
    #[must_use]
    pub fn characters(&self) -> usize {
        self.text.chars().count()
    }

    /// Whether this row is a page of a scan at all — OCR'd or read off the
    /// file's own text.
    #[must_use]
    pub fn is_a_page(&self) -> bool {
        self.kind == SegmentKind::Page
    }
}

/// The spans of every word in `text` that `keep` says yes to.
///
/// Every marker in the search bar is this function with a different `keep`, and
/// each of them used to be its own `tokenize().filter().map().collect()` — four
/// copies of a walk whose only difference was one predicate.
///
/// It walks rather than collects, because it keeps the spans and throws every
/// string away. A hit can be a whole oversized segment — the largest in the
/// corpus is 1,275,307 characters — and this runs once per row of a result page,
/// so `tokenize`'s `String`-per-word was hundreds of thousands of allocations to
/// answer a question about integers.
pub(crate) fn spans_where(text: &str, keep: impl Fn(&str) -> bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    girsa_hebrew::for_each_token(text, |word, start, end| {
        if keep(word) {
            out.push((start, end));
        }
    });
    out
}

impl Hit {
    /// Where in [`Hit::text`] the words this plan asked for sit, as byte spans.
    ///
    /// Computed from the printed text through the same normalizer the index was
    /// built with, so a mark lands on `קוֹרִין` and not on the bare spelling that
    /// matched. A caller that highlighted by searching the printed string for
    /// the query would find nothing at all on a menukad page.
    ///
    /// It marks by the **plan's own rule**: a `contains` search highlights the
    /// longer word it found the letters inside, because that is the word that
    /// answered the question. Highlighting the typed letters instead would
    /// point at a word the reader did not search for.
    #[must_use]
    pub fn marks(&self, plan: &Plan) -> Vec<(usize, usize)> {
        spans_where(&self.text, |found| {
            plan.words.iter().any(|word| plan.matches(word, found))
        })
    }
}

/// The index, open.
pub struct SearchIndex {
    index: tantivy::Index,
    fields: Fields,
    reader: IndexReader,
    /// `None` for an in-memory index, which cannot go stale because it does not
    /// outlive the process that built it.
    path: Option<PathBuf>,
    /// What went in, if the builder said. `None` is *nobody wrote it down*,
    /// which the facets report as such.
    report: Option<BuildReport>,
}

impl std::fmt::Debug for SearchIndex {
    /// Where it is and how much is in it. Tantivy's own types are not `Debug`
    /// and would say nothing useful if they were.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchIndex")
            .field(
                "path",
                &self
                    .path
                    .as_ref()
                    .map_or("in memory".into(), |p| p.display().to_string()),
            )
            .field("segments", &self.count())
            .finish()
    }
}

impl SearchIndex {
    /// A fresh index in a directory, stamped with the rules it was built under.
    ///
    /// # Errors
    ///
    /// If the directory cannot be made, if tantivy will not create an index in
    /// it, or if the stamp cannot be written.
    pub fn create(dir: &Path) -> Result<Self, IndexError> {
        std::fs::create_dir_all(dir).map_err(IndexError::io(dir))?;
        let index = tantivy::Index::create_in_dir(dir, Fields::schema())?;
        let stamp = dir.join(CACHE_STAMP);
        let body = serde_json::to_string(&Stamp::current())
            .map_err(|e| IndexError::stale(dir, e.to_string()))?;
        std::fs::write(&stamp, body).map_err(IndexError::io(&stamp))?;
        Self::wrap(index, Some(dir.to_path_buf()))
    }

    /// Throw away whatever is in `dir` and create an index there.
    ///
    /// What the indexer does when the corpus has been re-imported: the segments
    /// were rewritten wholesale, so the index built from them is not worth
    /// patching.
    ///
    /// # It will only delete an index
    ///
    /// This function recursively deletes the path it is given, and the path
    /// comes off a command line. `girsa-index build` takes the index directory
    /// **first** and the corpus roots after it, so one transposition —
    /// `build corpus index` — pointed this at the corpus and destroyed it. That
    /// happened here, on this machine, and cost a 2.2 GB refetch and the whole
    /// of Tier 2 again.
    ///
    /// So it refuses anything that is not already an index or an empty
    /// directory. The check is not clever and does not need to be: a tantivy
    /// index has a `meta.json` and Girsa's has a [`CACHE_STAMP`] beside it, and
    /// a directory with neither is somebody's data. The cost of the check is a
    /// `stat`; the cost of not having it was measured.
    ///
    /// # Errors
    ///
    /// [`IndexError::NotAnIndex`] if `dir` holds something that is not an
    /// index. Otherwise, if the directory cannot be removed or the index
    /// cannot be created.
    pub fn rebuild(dir: &Path) -> Result<Self, IndexError> {
        if dir.exists() {
            if !looks_like_an_index(dir) {
                return Err(IndexError::NotAnIndex {
                    path: dir.display().to_string(),
                });
            }
            std::fs::remove_dir_all(dir).map_err(IndexError::io(dir))?;
        }
        Self::create(dir)
    }

    /// Open an index, refusing one written under rules that no longer hold.
    ///
    /// # Errors
    ///
    /// [`IndexError::Stale`] if the stamp is missing, unreadable or does not
    /// match this build — see the module docs for why that is not a warning.
    pub fn open(dir: &Path) -> Result<Self, IndexError> {
        let stamp_path = dir.join(CACHE_STAMP);
        let body = std::fs::read_to_string(&stamp_path)
            .map_err(|e| IndexError::stale(dir, format!("no readable {CACHE_STAMP}: {e}")))?;
        let stamp: Stamp = serde_json::from_str(&body)
            .map_err(|e| IndexError::stale(dir, format!("{CACHE_STAMP} does not parse: {e}")))?;
        if let Some(why) = stamp.disagreement() {
            return Err(IndexError::stale(dir, why));
        }
        let index = tantivy::Index::open_in_dir(dir)?;
        Self::wrap(index, Some(dir.to_path_buf()))
    }

    /// An index that lives as long as the process. For tests and for probing.
    ///
    /// # Errors
    ///
    /// If the reader cannot be built.
    pub fn in_memory() -> Result<Self, IndexError> {
        Self::wrap(tantivy::Index::create_in_ram(Fields::schema()), None)
    }

    /// The one place the tokenizer is registered.
    ///
    /// Every constructor comes through here. Registering it only where the
    /// index is *built* gives an index that writes perfectly and cannot be
    /// queried after a restart — the schema names a tokenizer the reopened
    /// index has never heard of.
    fn wrap(index: tantivy::Index, path: Option<PathBuf>) -> Result<Self, IndexError> {
        index
            .tokenizers()
            .register(tokenizer::NAME, tokenizer::Normalized);
        let fields = Fields::of(&index.schema())?;
        let reader = index
            .reader_builder()
            // Manual, so a caller that has just committed and wants to read
            // says so. The alternative is a test that passes on a fast machine.
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let report = path
            .as_ref()
            .and_then(|dir| std::fs::read_to_string(dir.join(BUILD_REPORT)).ok())
            .and_then(|body| serde_json::from_str(&body).ok());
        Ok(Self {
            index,
            fields,
            reader,
            path,
            report,
        })
    }

    /// Where this index lives, if it lives anywhere.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Say what went into this index, and write it down beside it.
    ///
    /// # Errors
    ///
    /// If the report cannot be written. It is not optional: an index whose
    /// build was never described reports every facet it cannot compute as
    /// *not built*, which is honest but useless, and silently skipping the
    /// write would make that the normal state.
    pub fn declare(&mut self, report: BuildReport) -> Result<(), IndexError> {
        if let Some(dir) = &self.path {
            let path = dir.join(BUILD_REPORT);
            let body = serde_json::to_string(&report)
                .map_err(|e| IndexError::stale(dir, e.to_string()))?;
            std::fs::write(&path, body).map_err(IndexError::io(&path))?;
        }
        self.report = Some(report);
        Ok(())
    }

    /// What the builder said went in, if it said.
    #[must_use]
    pub fn report(&self) -> Option<&BuildReport> {
        self.report.as_ref()
    }

    /// A writer with the default budget.
    ///
    /// # Errors
    ///
    /// If another writer holds the lock, or tantivy will not start its threads.
    pub fn writer(&self) -> Result<Writer, IndexError> {
        self.writer_with_heap(DEFAULT_HEAP_BYTES)
    }

    /// A writer with a budget you chose. More memory, fewer segment merges.
    ///
    /// # Errors
    ///
    /// If another writer holds the lock, or the budget is outside what tantivy
    /// will accept.
    pub fn writer_with_heap(&self, heap_bytes: usize) -> Result<Writer, IndexError> {
        Ok(Writer {
            writer: self.index.writer::<TantivyDocument>(heap_bytes)?,
            fields: self.fields,
            replaced: HashSet::new(),
        })
    }

    /// Pick up what has been committed since this index was opened.
    ///
    /// # Errors
    ///
    /// If tantivy cannot read the new segments.
    pub fn reload(&self) -> Result<(), IndexError> {
        self.reader.reload()?;
        Ok(())
    }

    /// How many segments are in the index.
    #[must_use]
    pub fn count(&self) -> usize {
        usize::try_from(self.reader.searcher().num_docs()).unwrap_or(usize::MAX)
    }

    /// Run a Torat Emet query over the whole shelf (W12).
    ///
    /// The literal mode, and the default. What comes back is what was asked
    /// for, and [`Found::asked`] says what that was — see
    /// [`crate::torat_emet`].
    ///
    /// # Errors
    ///
    /// [`IndexError::TooBroad`] or [`IndexError::TooManyWords`] when the query
    /// cannot be run exactly; a partial answer is never returned in its place.
    /// Otherwise if the search fails or a stored document cannot be read back.
    pub fn search(&self, query: &torat_emet::Query) -> Result<Found, IndexError> {
        self.search_in(query, &Scope::everything(), Paging::first())
    }

    /// The same, confined to a scope and one page deep (W14).
    ///
    /// # Errors
    ///
    /// As [`SearchIndex::search`].
    pub fn search_in(
        &self,
        query: &torat_emet::Query,
        scope: &Scope,
        paging: Paging,
    ) -> Result<Found, IndexError> {
        let asked = query.plan();
        if asked.is_empty() {
            return Ok(Self::nothing(asked));
        }
        let prepared = self.prepare(query, scope)?;
        // A literal search reports no widening because there was none. A zero
        // here stays a zero: the ladder is offered by [`SearchIndex::offers`]
        // and climbed only when the reader clicks.
        self.found_with(&prepared, asked, None, paging)
    }

    /// The hits, the total and the plan, for a query **already prepared**.
    ///
    /// The seam [`Prepared`]'s own note asks for and did not have. Its doc says
    /// it is *"built once and asked three times, because a facet computed from
    /// a differently-built copy of it would be a column of numbers that did not
    /// add up to the header"* — and every caller that wanted hits **and** facets
    /// had to build one for the facets and let [`SearchIndex::search_in`] build
    /// a second, private one for the hits. Two builds, two chances to differ,
    /// and the second one invisible from the call site.
    ///
    /// # Errors
    ///
    /// If the search fails.
    pub fn found_with(
        &self,
        prepared: &Prepared,
        asked: Plan,
        widening: Option<Widening>,
        paging: Paging,
    ) -> Result<Found, IndexError> {
        let (hits, total) = self.page(prepared, paging)?;
        Ok(Found {
            hits,
            total,
            asked,
            widening,
        })
    }

    /// Build a literal query, ready to be asked more than one question.
    ///
    /// Hits, a total and the facet counts are three questions about **one**
    /// query, and a facet computed from a differently-built copy of it would be
    /// a column of numbers that did not add up to the header.
    ///
    /// # Errors
    ///
    /// As [`SearchIndex::search`].
    pub fn prepare(
        &self,
        query: &torat_emet::Query,
        scope: &Scope,
    ) -> Result<Prepared, IndexError> {
        // Through the same builder as a widened search, with no rungs applied.
        // Two builders would be two chances for the literal mode to stop being
        // literal without anyone noticing.
        let widening = Widened::new(query.clone(), []).widening();
        let built = self.build(&widening, query.max_expansions())?;
        Ok(self.confined(built, scope))
    }

    /// Build a widened query the same way.
    ///
    /// # Errors
    ///
    /// As [`SearchIndex::search_widened`].
    pub fn prepare_widened(
        &self,
        widened: &Widened,
        scope: &Scope,
    ) -> Result<Prepared, IndexError> {
        let built = self.build(&widened.widening(), widened.literal().max_expansions())?;
        Ok(self.confined(built, scope))
    }

    /// Confine a built query to a scope.
    ///
    /// Every clause added here is a `Must` or a `MustNot`, so a scope can only
    /// ever take hits away. Nothing in this path can widen a query, which is
    /// the property that lets the chip change the number in the header without
    /// changing what was searched for.
    fn confined(&self, query: Box<dyn Query>, scope: &Scope) -> Prepared {
        if scope.is_everything() {
            return Prepared { query };
        }
        let mut clauses: Clauses = vec![(Occur::Must, query)];
        // One `Must` per click, so two narrowings are an *and*. Folding them
        // into one set of slugs would make a second click widen the first.
        for clause in scope.clauses() {
            clauses.push((
                Occur::Must,
                self.any_of(self.fields.work, clause.iter().map(String::as_str)),
            ));
        }
        for slug in scope.excluded_works() {
            clauses.push((Occur::MustNot, self.one_term(self.fields.work, &slug)));
        }
        if !scope.link_types().is_empty() {
            clauses.push((
                Occur::Must,
                self.any_of(
                    self.fields.link,
                    scope.link_types().iter().map(|t| t.as_str()),
                ),
            ));
        }
        for kind in scope.excluded_link_types() {
            clauses.push((
                Occur::MustNot,
                self.one_term(self.fields.link, kind.as_str()),
            ));
        }
        Prepared {
            query: Box::new(BooleanQuery::new(clauses)),
        }
    }

    fn one_term(&self, field: tantivy::schema::Field, value: &str) -> Box<dyn Query> {
        Box::new(TermQuery::new(
            Term::from_field_text(field, value),
            IndexRecordOption::Basic,
        ))
    }

    fn any_of<'a>(
        &self,
        field: tantivy::schema::Field,
        values: impl Iterator<Item = &'a str>,
    ) -> Box<dyn Query> {
        let clauses: Clauses = values
            .map(|value| (Occur::Should, self.one_term(field, value)))
            .collect();
        Box::new(BooleanQuery::new(clauses))
    }

    /// One page of hits, and how many there were altogether.
    ///
    /// # Errors
    ///
    /// If the search fails or a stored document cannot be read back.
    pub fn page(
        &self,
        prepared: &Prepared,
        paging: Paging,
    ) -> Result<(Vec<Hit>, usize), IndexError> {
        self.run(&*prepared.query, paging)
    }

    /// How many a prepared query matches, without fetching any of them.
    ///
    /// # Errors
    ///
    /// If the search fails.
    pub fn count_of(&self, prepared: &Prepared) -> Result<usize, IndexError> {
        self.reader
            .searcher()
            .search(&*prepared.query, &Count)
            .map_err(IndexError::from_tantivy)
    }

    /// The counts behind the facets (spec.md §9.8), over the **whole** result
    /// set.
    ///
    /// Not over the page. A facet row that counted only what fits on screen
    /// would tell a reader that a shelf holds three of their hits when it holds
    /// three hundred, and the number would change as they scrolled.
    ///
    /// # Errors
    ///
    /// If the search fails or a column cannot be read.
    pub fn tally(&self, prepared: &Prepared) -> Result<Counts, IndexError> {
        let (by_work, by_link, total) = self
            .reader
            .searcher()
            .search(&*prepared.query, &Tallies)
            .map_err(IndexError::from_tantivy)?;
        Ok(Counts {
            by_work,
            by_link: by_link
                .into_iter()
                .filter_map(|(name, n)| girsa_link::touching::type_named(&name).map(|t| (t, n)))
                .collect(),
            total,
            link_types_built: self.report.as_ref().is_some_and(|r| r.link_types),
        })
    }

    /// Run a query with rungs of the relaxation ladder applied (W13).
    ///
    /// What comes back carries [`Found::widening`], which says which rungs were
    /// applied and what each word was allowed to be — so a header can report
    /// the change out of the thing that ran rather than out of a description of
    /// it.
    ///
    /// # Errors
    ///
    /// [`IndexError::TooManyForms`] when the widening would take more exact
    /// searches than the ceiling allows, plus everything
    /// [`SearchIndex::search`] can fail with.
    pub fn search_widened(&self, widened: &Widened) -> Result<Found, IndexError> {
        self.search_widened_in(widened, &Scope::everything(), Paging::first())
    }

    /// The same, confined to a scope and one page deep.
    ///
    /// # Errors
    ///
    /// As [`SearchIndex::search_widened`].
    pub fn search_widened_in(
        &self,
        widened: &Widened,
        scope: &Scope,
        paging: Paging,
    ) -> Result<Found, IndexError> {
        let asked = widened.literal().plan();
        if asked.is_empty() {
            return Ok(Self::nothing(asked));
        }
        let widening = widened.widening();
        let prepared = self.prepare_widened(widened, scope)?;
        self.found_with(&prepared, asked, Some(widening), paging)
    }

    /// How many results a widened query would return, without fetching any.
    ///
    /// This is what makes `[try other forms — 7]` possible: the number beside
    /// the offer comes from the query the click would run, so the promise and
    /// the result cannot disagree.
    ///
    /// # Errors
    ///
    /// As [`SearchIndex::search_widened`].
    pub fn count_widened(&self, widened: &Widened) -> Result<usize, IndexError> {
        self.count_widened_in(widened, &Scope::everything())
    }

    /// The same, in a scope — so an offer made inside a narrowed search
    /// promises the number that search will show.
    ///
    /// # Errors
    ///
    /// As [`SearchIndex::search_widened`].
    pub fn count_widened_in(&self, widened: &Widened, scope: &Scope) -> Result<usize, IndexError> {
        if widened.literal().plan().is_empty() {
            return Ok(0);
        }
        let prepared = self.prepare_widened(widened, scope)?;
        self.count_of(&prepared)
    }

    /// What the ladder has to say about a literal query (spec.md §9.6).
    ///
    /// Every rung is priced by running it, and **nothing is applied**: the
    /// reader's result set is whatever [`SearchIndex::search`] returned, until
    /// they click. Rungs that would change nothing are left out, rungs that
    /// would find nothing are left out — there is no such thing as
    /// `[try other forms — 0]` — and rungs that could not be priced are named
    /// in [`Offers::refused`] rather than dropped, because a missing chip reads
    /// as *there is nothing down that road*.
    #[must_use]
    pub fn offers(&self, query: &torat_emet::Query) -> Offers {
        self.offers_in(query, &Scope::everything())
    }

    /// The same, priced inside the scope the reader is searching in.
    ///
    /// An offer counted over the whole shelf and applied inside a narrowed one
    /// would promise a number the click cannot produce.
    #[must_use]
    pub fn offers_in(&self, query: &torat_emet::Query, scope: &Scope) -> Offers {
        let mut offers = Offers::default();
        if query.plan().is_empty() {
            return offers;
        }
        for rung in Rung::ALL {
            match rung.standing() {
                Standing::Climbed => {}
                Standing::Deferred(_) => offers.deferred.push(rung),
                Standing::Ready => {
                    let widened = Widened::new(query.clone(), [rung]);
                    if !widened.widening().changes_anything() {
                        continue;
                    }
                    match self.count_widened_in(&widened, scope) {
                        Ok(0) => {}
                        Ok(count) => offers.offers.push(Offer {
                            rung,
                            label: rung.label(),
                            count,
                            widened,
                        }),
                        Err(why) => offers.refused.push(Refusal {
                            rung,
                            why: why.to_string(),
                        }),
                    }
                }
            }
        }
        offers
    }

    /// Run a Regex query — mode 3 (W14).
    ///
    /// Each pattern is matched against a **whole word** of the index, and the
    /// words relate the way the `together` chip says. No ladder, no offers, no
    /// widening: spec.md §9.6's table says nothing happens on a zero here.
    ///
    /// # Errors
    ///
    /// If a pattern will not compile — tantivy's message, unedited, because a
    /// person writing a regex wants the parser's complaint and not a
    /// paraphrase of it.
    pub fn prepare_regex(
        &self,
        query: &crate::regex_mode::Query,
        scope: &Scope,
    ) -> Result<Prepared, IndexError> {
        let built = self.regex_query(query.patterns(), query.shape())?;
        Ok(self.confined(built, scope))
    }

    /// Patterns over whole terms, related by shape.
    fn regex_query(
        &self,
        patterns: &[String],
        together: Together,
    ) -> Result<Box<dyn Query>, IndexError> {
        if let [only] = patterns {
            return Ok(Box::new(RegexQuery::from_pattern(only, self.fields.text)?));
        }
        match together {
            Together::Anywhere => {
                let mut clauses: Clauses = Vec::with_capacity(patterns.len());
                for pattern in patterns {
                    clauses.push((
                        Occur::Must,
                        Box::new(RegexQuery::from_pattern(pattern, self.fields.text)?),
                    ));
                }
                Ok(Box::new(BooleanQuery::new(clauses)))
            }
            // In order, adjacent. `Near` never arrives: `regex_mode::Query`
            // refuses it rather than answering it with a slop, which would be a
            // window the reader did not ask for (W12's rule, unchanged).
            Together::Phrase | Together::Near { .. } => {
                let mut phrase = RegexPhraseQuery::new(self.fields.text, patterns.to_vec());
                phrase.set_slop(0);
                phrase.set_max_expansions(torat_emet::MOST_EXPANSIONS);
                Ok(Box::new(phrase))
            }
        }
    }

    /// Build an instrument into a query — mode 5 (W14).
    ///
    /// **Two of the four are not index questions.** A dilug runs through the
    /// letters of a sefer and ignores where words end; a notarikon is four
    /// patterns each matching half the vocabulary. Both are refused here, by
    /// name and with what to do instead, rather than approximated with
    /// something an inverted index happens to be able to do.
    ///
    /// # Errors
    ///
    /// [`IndexError::NotAnIndexQuestion`] for those two; otherwise as the
    /// searches it is built from.
    pub fn prepare_instrument(
        &self,
        instrument: &crate::instruments::Instrument,
        scope: &Scope,
    ) -> Result<Sounded, IndexError> {
        use crate::instruments::Instrument;
        match instrument {
            Instrument::Gematria { value, .. } => {
                // Every distinct word in the index, added up once. The words
                // are the finding as much as the segments are: *which* words
                // come to 613 is the question, and the segments follow.
                let words = self.words_worth(*value)?;
                let terms: Vec<Term> = words
                    .iter()
                    .map(|word| Term::from_field_text(self.fields.text, word))
                    .collect();
                let built: Box<dyn Query> = Box::new(tantivy::query::TermSetQuery::new(terms));
                Ok(Sounded {
                    prepared: self.confined(built, scope),
                    words: words.into_iter().collect(),
                })
            }
            // Not an index question, though it looks like one. `מקאש` is four
            // one-letter patterns and each of them matches more distinct words
            // than a phrase query will hold — the index answers it with a
            // refusal about postings lists, which is true and useless. It is
            // read off the text instead, in a scope the reader named.
            Instrument::Notarikon { .. } => Err(IndexError::NotAnIndexQuestion {
                what: "a notarikon",
                instead: "one letter matches half the words in the corpus, so it is read off \
                          the text — narrow the scope to a sefer and ask again",
            }),
            // The transformed word, searched for literally. Atbash is a
            // different **word**, not a different kind of search, so it goes
            // down the same path as anything a reader types.
            Instrument::Atbash { becomes, .. } => {
                let query = torat_emet::Query::new(becomes.clone());
                Ok(Sounded {
                    prepared: self.prepare(&query, scope)?,
                    words: query.plan().words,
                })
            }
            Instrument::Dilug { .. } => Err(IndexError::NotAnIndexQuestion {
                what: "a dilug",
                instead: "it reads the letters of a sefer in order, so it needs the text and a \
                          sefer to read — narrow the scope to one and ask again",
            }),
        }
    }

    /// Every distinct word in the index that comes to a value.
    ///
    /// Walks the term dictionary rather than guessing at candidates: the words
    /// worth 613 are whatever is written in the seforim, and no generated list
    /// of them would be the same list.
    ///
    /// # Errors
    ///
    /// If a term dictionary cannot be read.
    pub fn words_worth(
        &self,
        value: u32,
    ) -> Result<std::collections::BTreeSet<String>, IndexError> {
        let searcher = self.reader.searcher();
        let mut out = std::collections::BTreeSet::new();
        for reader in searcher.segment_readers() {
            let inverted = reader.inverted_index(self.fields.text)?;
            let mut terms = inverted.terms().stream().map_err(|source| IndexError::Io {
                path: "the term dictionary".to_string(),
                source,
            })?;
            while terms.advance() {
                let Ok(word) = std::str::from_utf8(terms.key()) else {
                    continue;
                };
                if crate::instruments::value_of(word) == Some(value) {
                    out.insert(word.to_string());
                }
            }
        }
        Ok(out)
    }

    /// One segment, by its permanent name.
    ///
    /// What a citation opens: the ref resolved to an id, and the id read back
    /// out of the index the reader is already searching.
    ///
    /// # Errors
    ///
    /// If the search fails or the stored document cannot be read.
    pub fn segment(&self, id: &SegmentId) -> Result<Option<Hit>, IndexError> {
        let query = TermQuery::new(
            Term::from_field_text(self.fields.id, &id.to_string()),
            IndexRecordOption::Basic,
        );
        let (hits, _) = self.run(&query, Paging::of(1))?;
        Ok(hits.into_iter().next())
    }

    /// A result that found nothing, for a query that asked for nothing.
    fn nothing(asked: Plan) -> Found {
        Found {
            hits: Vec::new(),
            total: 0,
            asked,
            widening: None,
        }
    }

    /// Segments holding **all** of these words, in any order.
    ///
    /// The index's own probe: [`SearchIndex::search`] with everything left at
    /// its default. Kept because a probe wants to be one line.
    ///
    /// # Errors
    ///
    /// If the search cannot be run or a stored document cannot be read back.
    pub fn words(&self, query: &str) -> Result<Vec<Hit>, IndexError> {
        Ok(self.search(&torat_emet::Query::new(query))?.hits)
    }

    /// Every word in the index, and how many segments each is in.
    ///
    /// The OCR queue's input (W21). It is read straight out of tantivy's term
    /// dictionary, which is this table already — the alternative is a pass over
    /// five million segments to count what the index counted while it was being
    /// built.
    ///
    /// The words are the **indexed** spellings: nikud off, final letters
    /// folded, exactly as W11 wrote them. That is what makes comparing two of
    /// them mean something, and it is why a suspect has to be turned back into
    /// the printed word before anybody is shown it.
    ///
    /// # Errors
    ///
    /// If a segment of the index will not open.
    pub fn vocabulary(&self) -> Result<Vec<(String, u64)>, IndexError> {
        let searcher = self.reader.searcher();
        let mut counts: HashMap<String, u64> = HashMap::new();
        for reader in searcher.segment_readers() {
            let inverted = reader.inverted_index(self.fields.text)?;
            let mut words = inverted.terms().stream().map_err(|source| IndexError::Io {
                path: "the term dictionary".to_string(),
                source,
            })?;
            while let Some((word, info)) = words.next() {
                let Ok(word) = std::str::from_utf8(word) else {
                    // A term that is not text is not a word anybody typed.
                    continue;
                };
                *counts.entry(word.to_string()).or_default() += u64::from(info.doc_freq);
            }
        }
        Ok(counts.into_iter().collect())
    }

    /// Segments holding these words, adjacent and in this order.
    ///
    /// # Errors
    ///
    /// If the search cannot be run or a stored document cannot be read back.
    pub fn phrase(&self, query: &str) -> Result<Vec<Hit>, IndexError> {
        Ok(self
            .search(&torat_emet::Query::new(query).together(torat_emet::Together::Phrase))?
            .hits)
    }

    /// Turn a widening into the query tantivy will run.
    ///
    /// Every branch here matches a sentence in [`Widening::describe`]. If the
    /// two ever disagree, the result header is describing a search that did not
    /// happen — which is the one thing this engine may not do.
    ///
    /// A literal query is the case where every position has exactly one
    /// alternative of exactly one word, so it comes down this same path and
    /// builds the same `TermQuery`/`PhraseQuery` W12 built.
    fn build(&self, wide: &Widening, max_expansions: u32) -> Result<Box<dyn Query>, IndexError> {
        let positions = &wide.positions;
        let single = positions.len() == 1;
        match wide.together {
            // All of them, anywhere in the segment. An *and*: a result that
            // does not have every word you asked for is a result you have to
            // check by eye, which is the other way a search box loses trust.
            //
            // Widening never crosses positions here — each one is still
            // required, it may just be answered more ways — so there is no
            // cross product and no ceiling.
            Together::Anywhere => {
                let mut clauses: Clauses = Vec::with_capacity(positions.len());
                for position in positions {
                    clauses.push((
                        Occur::Must,
                        self.one_position(wide.matching, position, max_expansions)?,
                    ));
                }
                Ok(Box::new(BooleanQuery::new(clauses)))
            }
            // One word is a legal phrase and a legal proximity — it is the
            // word. Tantivy's phrase queries assert on fewer than two terms,
            // and an assert is a window that closes.
            Together::Phrase | Together::Near { .. } if single => {
                self.one_position(wide.matching, &positions[0], max_expansions)
            }
            Together::Phrase => self.exactly(
                wide.matching,
                &positions.iter().collect::<Vec<_>>(),
                0,
                max_expansions,
                1,
            ),
            // Order-free proximity: the union over orderings. Each ordering is
            // asked for exactly, so the union is exactly *"these words with at
            // most `words` between them, in some order"* — as against a slop
            // wide enough to allow a reversal, which would also allow a
            // distance the reader did not ask for.
            Together::Near { words: gap } => {
                if positions.len() > MOST_WORDS_UNORDERED {
                    return Err(IndexError::TooManyWords {
                        words: positions.len(),
                        limit: MOST_WORDS_UNORDERED,
                    });
                }
                let every = orderings(&positions.iter().collect::<Vec<_>>());
                let mut clauses: Clauses = Vec::with_capacity(every.len());
                for ordering in &every {
                    clauses.push((
                        Occur::Should,
                        self.exactly(wide.matching, ordering, gap, max_expansions, every.len())?,
                    ));
                }
                Ok(Box::new(BooleanQuery::new(clauses)))
            }
        }
    }

    /// One position — the typed word, or any of the forms a rung allows for it.
    fn one_position(
        &self,
        matching: Match,
        position: &Position,
        max_expansions: u32,
    ) -> Result<Box<dyn Query>, IndexError> {
        if let [only] = position.alternatives.as_slice() {
            return self.one_alternative(matching, only, max_expansions);
        }
        let mut clauses: Clauses = Vec::with_capacity(position.alternatives.len());
        for alternative in &position.alternatives {
            clauses.push((
                Occur::Should,
                self.one_alternative(matching, alternative, max_expansions)?,
            ));
        }
        Ok(Box::new(BooleanQuery::new(clauses)))
    }

    /// One alternative: a word, or the run of words an abbreviation expands to.
    fn one_alternative(
        &self,
        matching: Match,
        alternative: &Alternative,
        max_expansions: u32,
    ) -> Result<Box<dyn Query>, IndexError> {
        match alternative.forms.as_slice() {
            [only] => self.one_form(only),
            // `שו"ע` becomes `שולחן ערוך`, and those two words have to be found
            // beside each other — an *and* over them would match a page holding
            // both in different sentences.
            forms => self.in_a_row(matching, forms, 0, max_expansions),
        }
    }

    /// One written word, matched the way the reader asked.
    fn one_form(&self, form: &Form) -> Result<Box<dyn Query>, IndexError> {
        Ok(if form.regex {
            Box::new(RegexQuery::from_pattern(&form.pattern, self.fields.text)?)
        } else {
            // The term itself. No automaton, no expansion, no ceiling to hit.
            Box::new(TermQuery::new(
                Term::from_field_text(self.fields.text, &form.pattern),
                IndexRecordOption::WithFreqs,
            ))
        })
    }

    /// These positions in this order, with `slop` other words allowed between.
    ///
    /// Where a position has several alternatives, this is the union over every
    /// combination of them — each combination asked for **exactly**, because a
    /// phrase query cannot hold an alternation at one of its positions without
    /// also loosening what sits either side of it.
    fn exactly(
        &self,
        matching: Match,
        order: &[&Position],
        slop: u32,
        max_expansions: u32,
        orderings: usize,
    ) -> Result<Box<dyn Query>, IndexError> {
        let queries = combination_count(order).saturating_mul(orderings);
        if queries > MOST_EXACT_QUERIES {
            return Err(IndexError::TooManyForms {
                queries,
                limit: MOST_EXACT_QUERIES,
            });
        }
        let sequences = flattened(order);
        if let [only] = sequences.as_slice() {
            return self.in_a_row(matching, only, slop, max_expansions);
        }
        let mut clauses: Clauses = Vec::with_capacity(sequences.len());
        for sequence in &sequences {
            clauses.push((
                Occur::Should,
                self.in_a_row(matching, sequence, slop, max_expansions)?,
            ));
        }
        Ok(Box::new(BooleanQuery::new(clauses)))
    }

    /// Several forms in a row, with `slop` others allowed between them.
    fn in_a_row(
        &self,
        matching: Match,
        forms: &[Form],
        slop: u32,
        max_expansions: u32,
    ) -> Result<Box<dyn Query>, IndexError> {
        if let [only] = forms {
            return self.one_form(only);
        }
        if forms.iter().any(|form| form.regex) {
            let patterns: Vec<String> = forms
                .iter()
                .map(|form| {
                    if form.regex {
                        form.pattern.clone()
                    } else {
                        torat_emet::escape(&form.pattern)
                    }
                })
                .collect();
            let mut phrase = RegexPhraseQuery::new(self.fields.text, patterns);
            phrase.set_slop(slop);
            phrase.set_max_expansions(max_expansions);
            return Ok(Box::new(phrase));
        }
        debug_assert_eq!(matching, Match::Word, "only Match::Word builds plain terms");
        let terms: Vec<Term> = forms
            .iter()
            .map(|form| Term::from_field_text(self.fields.text, &form.pattern))
            .collect();
        let mut phrase = PhraseQuery::new(terms);
        phrase.set_slop(slop);
        Ok(Box::new(phrase))
    }

    /// The hits, and how many there were in total.
    ///
    /// Both, always: a page of results whose total is unknown cannot say
    /// *"1 of 4,190"*, and W13's ladder is counts before clicks.
    fn run(&self, query: &dyn Query, paging: Paging) -> Result<(Vec<Hit>, usize), IndexError> {
        let searcher = self.reader.searcher();
        let (found, total) = searcher
            .search(
                query,
                &(
                    TopDocs::with_limit(paging.size.max(1))
                        .and_offset(paging.from)
                        .order_by_score(),
                    Count,
                ),
            )
            .map_err(IndexError::from_tantivy)?;
        let mut hits = Vec::with_capacity(found.len());
        for (score, address) in found {
            let doc: TantivyDocument = searcher.doc(address)?;
            if let Some(hit) = self.hit(&doc, score) {
                hits.push(hit);
            }
        }
        // Score first, then the permanent name — so a tie is broken by
        // something that does not depend on which thread indexed what.
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
        });
        Ok((hits, total))
    }

    fn hit(&self, doc: &TantivyDocument, score: f32) -> Option<Hit> {
        let id: SegmentId = doc.get_first(self.fields.id)?.as_str()?.parse().ok()?;
        let kind = SegmentKind::parse(doc.get_first(self.fields.kind)?.as_str()?)?;
        let text = doc.get_first(self.fields.text)?.as_str()?.to_string();
        // Empty is *nobody read it* — the corpus, which is a text file — and
        // is a different statement from `embedded`, which is the file itself
        // having said so.
        let by = doc
            .get_first(self.fields.by)
            .and_then(|value| value.as_str())
            .filter(|name| !name.is_empty())
            .map(Reader::named);
        Some(Hit {
            id,
            kind,
            text,
            score,
            by,
        })
    }
}

/// Adding segments to the index.
///
/// A work is the unit of replacement: the first time a writer is given a
/// segment of some work, every segment of that work already in the index is
/// deleted. `girsa-import` rewrites a work's `segments.jsonl` wholesale, so a
/// re-index that appended would give every hit twice and a corpus count that
/// drifted upward on every run.
pub struct Writer {
    writer: IndexWriter<TantivyDocument>,
    fields: Fields,
    replaced: HashSet<String>,
}

impl Writer {
    /// Index one segment, replacing its work the first time this writer sees
    /// it.
    ///
    /// `touching` is every kind of link that lands on this segment, from either
    /// direction — [`girsa_link::touching`] — and it is a parameter rather than
    /// something looked up here because the graph is read per work and this is
    /// called per segment. An empty slice means *no links touch it*; the
    /// difference between that and *nobody worked the links out* is recorded
    /// once, in [`BuildReport::link_types`], rather than five million times.
    ///
    /// # Errors
    ///
    /// If tantivy will not take the document.
    pub fn add(&mut self, segment: &Segment, touching: &[EdgeType]) -> Result<(), IndexError> {
        self.add_saying(segment, touching, &segment.text)
    }

    /// Delete every segment of a work, and add nothing back.
    ///
    /// [`Writer::add`] already does the delete half the first time it sees a
    /// work, which covers every work that still exists. This is for the one
    /// that does not: a note you threw away has no `segments.jsonl` to re-read,
    /// so nothing is ever added under its name and the delete never fires. It
    /// would stay findable until the next full build, and a hit on it opens a
    /// sefer that is not on the shelf.
    ///
    /// Marked as replaced, so a later `add` for the same work does not delete
    /// it a second time.
    pub fn forget(&mut self, slug: &str) {
        self.writer
            .delete_term(Term::from_field_text(self.fields.work, slug));
        self.replaced.insert(slug.to_string());
    }

    /// Index one segment saying something other than what is on disk (W20).
    ///
    /// The one thing that says it: **the reader's corrections**. A typo fixed
    /// this morning showed up in the reading pane, in a quote copied to Ksav
    /// and in an export, and not in a search — the index was built from the
    /// corpus files and the corpus files are the sefer as it was scanned, so
    /// the line was findable by its typo and not by its word.
    ///
    /// The correction stays an overlay and the base text stays exactly where it
    /// is on disk (spec.md §4.1 — *never the text*). What crosses this boundary
    /// is one already-corrected string, worked out by `girsa_fix::Layer::apply_at`
    /// in the caller, because which patches apply to which place under which
    /// `Showing` is that crate's question and not this one's.
    ///
    /// # Errors
    ///
    /// If tantivy will not take the document.
    pub fn add_saying(
        &mut self,
        segment: &Segment,
        touching: &[EdgeType],
        text: &str,
    ) -> Result<(), IndexError> {
        let work = segment.id.work();
        if !self.replaced.contains(work) {
            self.writer
                .delete_term(Term::from_field_text(self.fields.work, work));
            self.replaced.insert(work.to_string());
        }

        let mut doc = TantivyDocument::new();
        doc.add_text(self.fields.id, segment.id.to_string());
        doc.add_text(self.fields.work, work);
        doc.add_text(self.fields.kind, segment.kind.as_str());
        for kind in touching {
            doc.add_text(self.fields.link, kind.as_str());
        }
        doc.add_text(self.fields.text, text);
        doc.add_text(self.fields.by, "");
        self.writer.add_document(doc)?;
        Ok(())
    }

    /// Index a page of a scan, with the words somebody read off it (W26).
    ///
    /// The same document as any other segment, with two differences that are
    /// the whole of spec.md §9.7's *one index, two location types*: the text is
    /// the reading rather than the corpus, and the `by` column says who read
    /// it. Where the words came off the page is **not** in here — that is the
    /// [`girsa_scan::Read`] in the personal layer, looked up when a row is
    /// opened, because a rectangle is not something a query can be asked about
    /// and duplicating it into five million documents would buy nothing.
    ///
    /// A page nobody has read goes in through [`Writer::add`] like anything
    /// else, with no words and no reader — which is what makes it *absent from
    /// the results and present in the count*, so [`girsa_scan::Job`] and the
    /// header can tell the reader what they are not seeing.
    ///
    /// # Errors
    ///
    /// If tantivy will not take the document.
    pub fn add_page(
        &mut self,
        segment: &Segment,
        touching: &[EdgeType],
        read: &girsa_scan::Read,
    ) -> Result<(), IndexError> {
        let work = segment.id.work();
        if !self.replaced.contains(work) {
            self.writer
                .delete_term(Term::from_field_text(self.fields.work, work));
            self.replaced.insert(work.to_string());
        }

        let mut doc = TantivyDocument::new();
        doc.add_text(self.fields.id, segment.id.to_string());
        doc.add_text(self.fields.work, work);
        doc.add_text(self.fields.kind, segment.kind.as_str());
        for kind in touching {
            doc.add_text(self.fields.link, kind.as_str());
        }
        doc.add_text(self.fields.text, read.text());
        doc.add_text(self.fields.by, read.by.name());
        self.writer.add_document(doc)?;
        Ok(())
    }

    /// Make everything added so far visible to readers.
    ///
    /// # Errors
    ///
    /// If the commit fails, in which case tantivy has rolled back to the
    /// previous one and nothing is half-written.
    pub fn commit(&mut self) -> Result<(), IndexError> {
        self.writer.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_stamp_written_now_agrees_with_itself() {
        assert_eq!(Stamp::current().disagreement(), None);
    }

    #[test]
    fn a_stamp_from_other_rules_says_which_rules() {
        let older = Stamp {
            normalizer_version: Stamp::current().normalizer_version.wrapping_sub(1),
            ..Stamp::current()
        };
        let why = older.disagreement().expect("a disagreement");
        assert!(why.contains("normalizer"), "{why}");
    }

    #[test]
    fn the_stamp_round_trips_as_json() {
        let body = serde_json::to_string(&Stamp::current()).expect("json");
        let read: Stamp = serde_json::from_str(&body).expect("json");
        assert_eq!(read, Stamp::current());
    }

    #[test]
    fn rebuilding_will_not_delete_a_directory_that_is_not_an_index() {
        // This is not hypothetical. `girsa-index build` takes the index
        // directory first and the corpus roots after it; one transposition —
        // `build corpus index` — pointed `rebuild` at the corpus and its first
        // act was `remove_dir_all`. 2.2 GB of fetched export, 7,189 imported
        // works and a 4.1-million-edge graph, gone, with the exit code of a
        // missing file.
        let corpus = std::env::temp_dir().join("girsa-rebuild-guard/corpus");
        let _ = std::fs::remove_dir_all(corpus.parent().unwrap_or(&corpus));
        std::fs::create_dir_all(corpus.join("works")).expect("makes a corpus");
        std::fs::write(corpus.join("works/index.jsonl"), "{}\n").expect("writes");

        let refused = SearchIndex::rebuild(&corpus);
        assert!(
            matches!(refused, Err(IndexError::NotAnIndex { .. })),
            "a directory holding a corpus is not a cache to throw away"
        );
        assert!(
            corpus.join("works/index.jsonl").is_file(),
            "and it is still there"
        );

        // What it must still do: an empty directory, a directory that is not
        // there at all, and — the case rebuilding exists for — an index left
        // half-written by a run that was killed.
        let empty = corpus.with_file_name("empty");
        std::fs::create_dir_all(&empty).expect("makes it");
        assert!(SearchIndex::rebuild(&empty).is_ok());
        assert!(SearchIndex::rebuild(&corpus.with_file_name("absent")).is_ok());
        let half = corpus.with_file_name("half-written");
        std::fs::create_dir_all(&half).expect("makes it");
        std::fs::write(half.join("meta.json"), "{}").expect("writes");
        std::fs::write(half.join("0.store"), "not really").expect("writes");
        assert!(
            SearchIndex::rebuild(&half).is_ok(),
            "an index with tantivy's meta.json and no stamp of ours is still an index"
        );

        let _ = std::fs::remove_dir_all(corpus.parent().unwrap_or(&corpus));
    }
}
