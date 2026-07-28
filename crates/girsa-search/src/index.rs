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

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use girsa_corpus::import::{Segment, SegmentKind};
use girsa_corpus::segment::SegmentId;
use girsa_corpus::CacheProvenance;
use serde::{Deserialize, Serialize};
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{
    BooleanQuery, Occur, PhraseQuery, Query, RegexPhraseQuery, RegexQuery, TermQuery,
};
use tantivy::schema::{
    IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, FAST, STORED, STRING,
};
use tantivy::{IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};

use crate::tokenizer;
use crate::torat_emet::{self, Match, Plan, Together, MOST_WORDS_UNORDERED};

/// The file that says what rules this index was built under.
pub const CACHE_STAMP: &str = "girsa-cache.json";

/// Bumped when the *shape* of the index changes — a field added, a field
/// indexed differently.
///
/// Separate from the normalizer's version because they go stale for different
/// reasons and both have to be checked. spec.md §9.7 will add PDF pages as a
/// second location type in this same index, and that is a schema change: an
/// index written before it is not wrong, it is *incomplete*, and reading it as
/// though it were complete would produce exactly the silent gap §9.7 forbids.
pub const SCHEMA_VERSION: u32 = 1;

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
}

/// The clauses of a boolean query, before it is built.
type Clauses = Vec<(Occur, Box<dyn Query>)>;

/// Every ordering of the patterns, for order-free proximity.
///
/// Heap's algorithm, iterative. The caller has already refused anything long
/// enough for this to matter.
fn orderings(patterns: &[String]) -> Vec<Vec<String>> {
    let mut current: Vec<String> = patterns.to_vec();
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

/// The four things every segment is indexed by.
#[derive(Debug, Clone, Copy)]
struct Fields {
    /// The permanent name, stored so a hit can be opened.
    id: tantivy::schema::Field,
    /// The work slug — the unit a re-import replaces, and W14's first facet.
    work: tantivy::schema::Field,
    /// `text` · `heading` · `page`. A hit inside a heading is a different kind
    /// of result, and a `page` with no words is spec.md §9.7's *"not
    /// searchable yet"* rather than an absence.
    kind: tantivy::schema::Field,
    /// The words, as printed. Indexed through the normalizer, stored as they
    /// stand — the reader is looking at the page, not at the index.
    text: tantivy::schema::Field,
}

impl Fields {
    const ID: &'static str = "id";
    const WORK: &'static str = "work";
    const KIND: &'static str = "kind";
    const TEXT: &'static str = "text";

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
            text: schema
                .get_field(Self::TEXT)
                .map_err(|_| IndexError::Field(Self::TEXT))?,
        })
    }
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
        girsa_hebrew::tokenize(&self.text)
            .into_iter()
            .filter(|token| {
                plan.words
                    .iter()
                    .any(|word| plan.matches(word, &token.text))
            })
            .map(|token| (token.start, token.end))
            .collect()
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
    /// # Errors
    ///
    /// If the directory cannot be removed or the index cannot be created.
    pub fn rebuild(dir: &Path) -> Result<Self, IndexError> {
        if dir.exists() {
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
        Ok(Self {
            index,
            fields,
            reader,
            path,
        })
    }

    /// Where this index lives, if it lives anywhere.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
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

    /// Run a Torat Emet query (W12).
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
        let asked = query.plan();
        if asked.is_empty() {
            return Ok(Found {
                hits: Vec::new(),
                total: 0,
                asked,
            });
        }
        let built = self.build(&asked, query.max_expansions())?;
        let (hits, total) = self.run(&*built)?;
        Ok(Found { hits, total, asked })
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

    /// Turn a plan into the query tantivy will run.
    ///
    /// Every branch here matches a sentence in [`Plan::describe`]. If the two
    /// ever disagree, the result header is describing a search that did not
    /// happen — which is the one thing this mode may not do.
    fn build(&self, plan: &Plan, max_expansions: u32) -> Result<Box<dyn Query>, IndexError> {
        let single = plan.words.len() == 1;
        match plan.together {
            // All of them, anywhere in the segment. An *and*: a result that
            // does not have every word you asked for is a result you have to
            // check by eye, which is the other way a search box loses trust.
            Together::Anywhere => {
                let mut clauses: Clauses = Vec::with_capacity(plan.patterns.len());
                for pattern in &plan.patterns {
                    clauses.push((Occur::Must, self.one_word(plan, pattern)?));
                }
                Ok(Box::new(BooleanQuery::new(clauses)))
            }
            // One word is a legal phrase and a legal proximity — it is the
            // word. Tantivy's phrase queries assert on fewer than two terms,
            // and an assert is a window that closes.
            Together::Phrase if single => self.one_word(plan, &plan.patterns[0]),
            Together::Near { .. } if single => self.one_word(plan, &plan.patterns[0]),
            Together::Phrase => self.in_a_row(plan, &plan.patterns, 0, max_expansions),
            // Order-free proximity: the union over orderings. Each ordering is
            // asked for exactly, so the union is exactly *"these words with at
            // most `words` between them, in some order"* — as against a slop
            // wide enough to allow a reversal, which would also allow a
            // distance the reader did not ask for.
            Together::Near { words: gap } => {
                if plan.words.len() > MOST_WORDS_UNORDERED {
                    return Err(IndexError::TooManyWords {
                        words: plan.words.len(),
                        limit: MOST_WORDS_UNORDERED,
                    });
                }
                let mut clauses: Clauses = Vec::new();
                for ordering in orderings(&plan.patterns) {
                    clauses.push((
                        Occur::Should,
                        self.in_a_row(plan, &ordering, gap, max_expansions)?,
                    ));
                }
                Ok(Box::new(BooleanQuery::new(clauses)))
            }
        }
    }

    /// One written word, matched the way the reader asked.
    fn one_word(&self, plan: &Plan, pattern: &str) -> Result<Box<dyn Query>, IndexError> {
        Ok(match plan.matching {
            // The term itself. No automaton, no expansion, no ceiling to hit.
            Match::Word => Box::new(TermQuery::new(
                Term::from_field_text(self.fields.text, pattern),
                IndexRecordOption::WithFreqs,
            )),
            Match::Contains | Match::Letters => {
                Box::new(RegexQuery::from_pattern(pattern, self.fields.text)?)
            }
        })
    }

    /// Several words in a row, with `slop` others allowed between them.
    fn in_a_row(
        &self,
        plan: &Plan,
        patterns: &[String],
        slop: u32,
        max_expansions: u32,
    ) -> Result<Box<dyn Query>, IndexError> {
        match plan.matching {
            Match::Word => {
                let terms: Vec<Term> = patterns
                    .iter()
                    .map(|word| Term::from_field_text(self.fields.text, word))
                    .collect();
                let mut phrase = PhraseQuery::new(terms);
                phrase.set_slop(slop);
                Ok(Box::new(phrase))
            }
            Match::Contains | Match::Letters => {
                let mut phrase = RegexPhraseQuery::new(self.fields.text, patterns.to_vec());
                phrase.set_slop(slop);
                phrase.set_max_expansions(max_expansions);
                Ok(Box::new(phrase))
            }
        }
    }

    /// The hits, and how many there were in total.
    ///
    /// Both, always: a page of results whose total is unknown cannot say
    /// *"1 of 4,190"*, and W13's ladder is counts before clicks.
    fn run(&self, query: &dyn Query) -> Result<(Vec<Hit>, usize), IndexError> {
        let searcher = self.reader.searcher();
        let (found, total) = searcher
            .search(
                query,
                &(TopDocs::with_limit(PROBE_LIMIT).order_by_score(), Count),
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
        Some(Hit {
            id,
            kind,
            text,
            score,
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
    /// # Errors
    ///
    /// If tantivy will not take the document.
    pub fn add(&mut self, segment: &Segment) -> Result<(), IndexError> {
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
        doc.add_text(self.fields.text, &segment.text);
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
}
