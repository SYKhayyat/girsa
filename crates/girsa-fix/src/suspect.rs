//! The ranked queue: finding the scanning errors nobody tripped over.
//!
//! spec.md §7.3, BUILDER.md W21. *A word appearing exactly once in the corpus,
//! one edit-distance from a word appearing ten thousand times, is almost
//! certainly an OCR error. That is a cheap batch job over the whole library,
//! and it produces a ranked, reviewable queue.* — and the spec adds that this
//! is worth more than the editor, which it is: fixing the typos you trip over
//! is a hobby, and being handed four thousand ranked candidates is a tool.
//!
//! # What makes it usable is what it refuses
//!
//! The naive version of this is twenty lines and produces a queue that is
//! mostly grammar, because **Hebrew attaches its function words to the front of
//! the next one**. `ובשבת` is `בשבת` with a vav; on edit distance that is a
//! dropped letter and looks exactly like a scanner. So:
//!
//! | refused | why |
//! |---|---|
//! | a letter added or dropped at the **front**, where it is ו ה ב כ ל מ ש ד | that is a prefix (W2's table), not a scanner |
//! | a letter added or dropped at the **end**, where it is ו י ה כ מ נ ת | that is a pronoun or a plural |
//! | words shorter than four letters | every short Hebrew word is one edit from a dozen others, and all of them are coincidences |
//!
//! What is left is ranked, and a **known confusion of shapes** — ד/ר, ב/כ, ה/ח,
//! the pairs spec.md §7.2 names — outranks a coincidence of the same size. That
//! ordering is the whole product: the queue is read from the top, and what is
//! at the top has to be worth the reader's attention.
//!
//! # Nothing here corrects anything
//!
//! A suspect is a **question**, not a patch. It says which word, which word it
//! looks like, how often each was seen, and where to go and look — and it makes
//! no correction. BUILDER.md rule 6: ambiguity resolves to a choice, and a
//! machine that is *almost certain* about four thousand words is a machine
//! about to rewrite forty of them wrongly.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use girsa_corpus::segment::SegmentId;
use girsa_personal::{fingerprint, Log, Store as _};
use serde::{Deserialize, Serialize};

use crate::FixError;

/// The 22 letters, in their non-final forms.
///
/// Final forms are absent on purpose: the index folds ך ם ן ף ץ onto their
/// letters (W11), so `אדם` and `אדמ` are one word here, and a candidate spelt
/// with a final would never be found.
const LETTERS: [char; 22] = [
    'א', 'ב', 'ג', 'ד', 'ה', 'ו', 'ז', 'ח', 'ט', 'י', 'כ', 'ל', 'מ', 'נ', 'ס', 'ע', 'פ', 'צ', 'ק',
    'ר', 'ש', 'ת',
];

/// The letters that attach to the front of a word (W2's prefix table).
const PREFIXES: [char; 8] = ['ו', 'ה', 'ב', 'כ', 'ל', 'מ', 'ש', 'ד'];

/// The letters a word ends in for grammatical reasons — possessives, plurals,
/// the perfect. `דברו`, `דברי`, `דברה`, `דברת`.
const SUFFIXES: [char; 7] = ['ו', 'י', 'ה', 'כ', 'מ', 'נ', 'ת'];

/// Pairs of letters that a scanner confuses, because they look alike in print.
///
/// The first three are the ones spec.md §7.2 names. The rest are the other
/// shape-neighbours of Hebrew type: a vav with a mark reads as a yod or a
/// zayin, a gimel with a broken foot as a nun, a samekh with a gap as a mem.
///
/// This table only **ranks** — a pair that is not here is still offered, lower
/// down. It is not a filter, because a scanner having a bad day confuses
/// anything.
const CONFUSIONS: [(char, char); 8] = [
    ('ד', 'ר'),
    ('ב', 'כ'),
    ('ה', 'ח'),
    ('ו', 'י'),
    ('ו', 'ז'),
    ('ג', 'נ'),
    ('מ', 'ס'),
    ('ת', 'ח'),
];

/// How rare is rare, how common is common, and how short is too short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// A word seen in this many segments or fewer is a candidate. spec.md §7.3
    /// says *exactly once*, which is the default.
    pub rare_at: u64,
    /// …one edit from a word seen in this many or more. The spec's ten
    /// thousand.
    pub common_at: u64,
    /// Shortest word worth suspecting, in letters.
    pub shortest: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rare_at: 1,
            common_at: 10_000,
            shortest: 4,
        }
    }
}

/// Every word in the corpus, and how many segments each was seen in.
///
/// Built from the search index's term dictionary, which already holds exactly
/// this table — see `girsa_search::SearchIndex::vocabulary`. The words are as
/// the index has them: nikud off, final letters folded (W11), which is what
/// makes a comparison of two spellings mean anything.
#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    counts: HashMap<String, u64>,
}

impl Vocabulary {
    /// The word is copied only the first time it is seen.
    ///
    /// `entry(word.to_string())` allocates on **every** call, including the
    /// millionth sighting of `את` — and a vocabulary read off the corpus is
    /// almost entirely repeat sightings. A `get_mut` first turns that into an
    /// allocation per *distinct* word, which is the number of rows in the table
    /// rather than the number of words in Shas.
    pub fn add(&mut self, word: &str, count: u64) {
        if let Some(seen) = self.counts.get_mut(word) {
            *seen += count;
        } else {
            self.counts.insert(word.to_string(), count);
        }
    }

    /// Count the words of one segment, for a caller with no index. Splitting is
    /// [`girsa_hebrew::for_each_token`]'s, because what a word is belongs to one
    /// crate (W2) — and walking rather than collecting, because this is called
    /// once per segment of a whole corpus and keeps none of the words it is
    /// shown.
    pub fn read(&mut self, text: &str) {
        girsa_hebrew::for_each_token(text, |word, _, _| {
            if let Some(seen) = self.counts.get_mut(word) {
                *seen += 1;
            } else {
                self.counts.insert(word.to_string(), 1);
            }
        });
    }

    #[must_use]
    pub fn count(&self, word: &str) -> u64 {
        self.counts.get(word).copied().unwrap_or_default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.counts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    pub fn words(&self) -> impl Iterator<Item = (&str, u64)> {
        self.counts
            .iter()
            .map(|(word, count)| (word.as_str(), *count))
    }
}

/// What the scanner did, as far as one edit can say.
///
/// Kept because it is most of how much a candidate is worth. A letter read as
/// another letter is a scanner doing the thing scanners do; a letter that
/// appeared out of nowhere beside a very common short word is a much weaker
/// claim, and on the real corpus there are thousands of those — see the
/// weights below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    /// One letter read as another.
    Letter,
    /// The rare word has a letter the common one does not.
    Added,
    /// The rare word is missing one.
    Dropped,
    /// Two letters the wrong way round.
    Swapped,
}

girsa_corpus::spelled!(Edit {
    Letter => "letter",
    Added => "added",
    Dropped => "dropped",
    Swapped => "swapped",
});

impl Edit {
    /// The same edit, said from the other word's point of view.
    ///
    /// [`Edit`] describes what happened to the **rare** word, and `hunt` walks
    /// the common ones — so an edit that removed a letter from the common word
    /// to reach the rare one means the rare word is *missing* one.
    /// Substitution and transposition read the same from either side.
    #[must_use]
    pub const fn mirrored(self) -> Self {
        match self {
            Self::Letter => Self::Letter,
            Self::Added => Self::Dropped,
            Self::Dropped => Self::Added,
            Self::Swapped => Self::Swapped,
        }
    }
}

impl Edit {
    /// How much a finding of this shape is worth, in tenths.
    ///
    /// Measured against the real corpus rather than chosen. Ranking by
    /// frequency alone put ten misspellings of `הוא` (1,305,264 segments) at
    /// the top of a queue of 28,124, ahead of every ד/ר in the library: a
    /// letter added beside a three-letter word is the weakest evidence there
    /// is, because every very common short word has hundreds of them.
    const fn weight(self) -> u64 {
        match self {
            Self::Letter => 10,
            Self::Swapped => 9,
            Self::Added | Self::Dropped => 5,
        }
    }
}

/// What was done about a suspect. `None` until somebody looks at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Not an error — a real word, a name, an unusual spelling.
    Dismissed,
    /// Corrected. The patch itself is in the layer; this is only the queue
    /// remembering not to ask again.
    Fixed,
}

girsa_corpus::spelled!(Decision {
    Dismissed => "dismissed",
    Fixed => "fixed",
});

/// One candidate: a rare word, the common word it is one letter from, and
/// everything the reader needs in order to decide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suspect {
    /// Named by the pair, so the same finding keeps its identity across runs of
    /// the batch job — which is what lets a decision survive one.
    pub id: String,
    pub rare: String,
    pub common: String,
    pub rare_count: u64,
    pub common_count: u64,
    /// The pair of letters, where they are ones a scanner confuses — `ד/ר`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confusion: Option<String>,
    /// What the scanner did.
    pub how: Edit,
    pub score: u64,
    /// Where to go and look. Filled in by whoever has the index; a queue of
    /// words nobody can find is a list, not a queue.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<SegmentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided: Option<Decision>,
}

impl Suspect {
    fn new(rare: &str, common: &str, rare_count: u64, common_count: u64, how: Edit) -> Self {
        let confusion = confusion_between(rare, common);
        Self {
            id: fingerprint(&[rare, common]),
            rare: rare.to_string(),
            common: common.to_string(),
            rare_count,
            common_count,
            score: score(common_count, rare.chars().count(), how, confusion.is_some()),
            confusion,
            how,
            places: Vec::new(),
            decided: None,
        }
    }

    /// What to put in the correction box for a word as it is **printed**.
    ///
    /// The queue works in the index's spelling — no nikud, final letters folded
    /// — and a sefer is printed in neither. So:
    ///
    /// - a printed word carrying nikud gets **no suggestion**. Rebuilding the
    ///   points for different letters is inventing text, and the reader is
    ///   right there with the word in front of them.
    /// - otherwise the common spelling, with its last letter put back into its
    ///   final form when the word it replaces ended in one.
    ///
    /// What comes back goes into a box the reader edits. It is a suggestion,
    /// and it is visible before it is anything else.
    #[must_use]
    pub fn suggestion(&self, printed: &str) -> Option<String> {
        if printed.chars().any(girsa_hebrew::is_mark) {
            return None;
        }
        let mut out: Vec<char> = self.common.chars().collect();
        let last = *out.last()?;
        if printed.chars().last().is_some_and(is_final) {
            if let Some(fin) = final_form(last) {
                let n = out.len() - 1;
                out[n] = fin;
            }
        }
        Some(out.into_iter().collect())
    }
}

/// How much a candidate is worth reading first.
///
/// Three things, and **not** raw frequency, which is what the first version
/// used and what the real corpus threw out: ranked by frequency alone the queue
/// opens with ten misspellings of `הוא`.
///
/// - **how common the word it looks like is**, taken as a logarithm. The
///   difference between a word seen 12,000 times and one seen 1,300,000 times
///   is real, and it is not a hundredfold difference in how likely this is to
///   be an error.
/// - **how long the rare word is.** Six letters agreeing but for one is a much
///   stronger coincidence than four.
/// - **what the scanner did** (`Edit::weight`), doubled where the letters are a
///   pair that look alike in print.
fn score(common_count: u64, letters: usize, how: Edit, confusion: bool) -> u64 {
    let common = u64::from(64 - common_count.max(1).leading_zeros());
    let base = common * 10 + letters as u64 * 5;
    let weighted = base * how.weight() / 10;
    if confusion {
        weighted * 2
    } else {
        weighted
    }
}

fn is_final(ch: char) -> bool {
    matches!(ch, 'ך' | 'ם' | 'ן' | 'ף' | 'ץ')
}

fn final_form(ch: char) -> Option<char> {
    match ch {
        'כ' => Some('ך'),
        'מ' => Some('ם'),
        'נ' => Some('ן'),
        'פ' => Some('ף'),
        'צ' => Some('ץ'),
        _ => None,
    }
}

/// The pair of letters two words differ by, where it is one a scanner confuses.
fn confusion_between(a: &str, b: &str) -> Option<String> {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len() != b.len() {
        return None;
    }
    let mut differ = a.iter().zip(&b).filter(|(x, y)| x != y);
    let (x, y) = differ.next()?;
    if differ.next().is_some() {
        return None;
    }
    CONFUSIONS
        .iter()
        .find(|(p, q)| (p == x && q == y) || (p == y && q == x))
        .map(|(p, q)| format!("{p}/{q}"))
}

/// Every rare word that is one letter from a common one, ranked.
///
/// # It walks the common words, and it used to walk the rare ones
///
/// Both sides find the same pairs. The difference is how many words each side
/// has: the **rare** side is the hapax legomena, 40–60% of any natural-language
/// term dictionary, and each of them generates roughly `45L + 22` candidates —
/// every one a fresh `Vec<char>` and a `String`. On a multi-million-term Hebrew
/// index that is 10⁸–10⁹ allocations, under a doc comment that said *"a few
/// million lookups."*
///
/// Only a word seen [`Settings::common_at`] times or more — ten thousand, by
/// default — can ever be the common side, and that is a few thousand terms.
/// So the generator runs over those instead, and the lookups ask *is this
/// neighbour a rare word* rather than *is this neighbour a common one*.
///
/// **The pairs are identical, and that is a property of the filters rather than
/// a hope.** Substitution and transposition are symmetric outright; the two
/// grammar filters are each other's mirror — the rare side refuses dropping a
/// prefix letter from position 0, the common side refuses inserting one there,
/// and it is the same letter at the same position. `Edit` is stated from the
/// rare word's point of view either way, which is what [`Edit::mirrored`] is
/// for. Asserted against the old walk in
/// `both_sides_of_the_join_find_the_same_suspects`.
#[must_use]
pub fn hunt(vocabulary: &Vocabulary, settings: Settings) -> Vec<Suspect> {
    // The best *candidate* per rare word, not the most frequent one: a ד/ר in a
    // long word beats a letter dropped beside `הוא`, however often `הוא` is
    // printed. Keyed by the rare word because the walk now arrives at one rare
    // word from several common ones.
    let mut best: BTreeMap<String, Suspect> = BTreeMap::new();
    for (common, common_count) in vocabulary.words() {
        if common_count < settings.common_at {
            continue;
        }
        let letters: Vec<char> = common.chars().collect();
        // Every candidate the rare side ever generated was built out of
        // `LETTERS`, so the common word of any pair it could find is made of
        // them too. Skipping the others changes no answer and saves the walk.
        if !letters.iter().all(|c| LETTERS.contains(c)) {
            continue;
        }
        for (candidate, how) in neighbours(&letters) {
            let rare_count = vocabulary.count(&candidate);
            if rare_count == 0 || rare_count > settings.rare_at {
                continue;
            }
            if candidate.chars().count() < settings.shortest {
                continue;
            }
            let suspect =
                Suspect::new(&candidate, common, rare_count, common_count, how.mirrored());
            match best.get(&candidate) {
                Some(seen) if seen.score >= suspect.score => {}
                _ => {
                    best.insert(candidate, suspect);
                }
            }
        }
    }
    let mut found: Vec<Suspect> = best.into_values().collect();
    // Highest score first, and by the word after that, so two runs of the batch
    // job over the same corpus hand back the same queue in the same order.
    found.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.rare.cmp(&b.rare))
            .then_with(|| a.common.cmp(&b.common))
    });
    found
}

/// Every word one edit away — minus the edits that are grammar — and what the
/// edit was.
fn neighbours(letters: &[char]) -> Vec<(String, Edit)> {
    let mut out = Vec::new();
    let word = |v: &[char]| v.iter().collect::<String>();

    for (i, at) in letters.iter().enumerate() {
        // A letter read as another letter. Every position, including the first
        // and the last: a scanner misreads those too, and unlike an inserted
        // letter a substitution can never be a prefix.
        for other in LETTERS {
            if other == *at {
                continue;
            }
            let mut candidate = letters.to_vec();
            candidate[i] = other;
            out.push((word(&candidate), Edit::Letter));
        }

        // A letter the scanner invented. Dropping the first where it is a
        // prefix would offer `בשבת` for `ובשבת`; dropping the last where it is
        // a suffix would offer `דבר` for `דברו`.
        let grammar =
            (i == 0 && PREFIXES.contains(at)) || (i + 1 == letters.len() && SUFFIXES.contains(at));
        if !grammar {
            let mut candidate = letters.to_vec();
            candidate.remove(i);
            out.push((word(&candidate), Edit::Added));
        }

        // Two letters read the wrong way round.
        if i + 1 < letters.len() {
            let mut candidate = letters.to_vec();
            candidate.swap(i, i + 1);
            out.push((word(&candidate), Edit::Swapped));
        }
    }

    // A letter the scanner lost. Adding one at the front or the back is the
    // same grammar in reverse: `ובשבת` is not `בשבת` with a letter missing, it
    // is a different word.
    for i in 0..=letters.len() {
        for other in LETTERS {
            if (i == 0 && PREFIXES.contains(&other))
                || (i == letters.len() && SUFFIXES.contains(&other))
            {
                continue;
            }
            let mut candidate = letters.to_vec();
            candidate.insert(i, other);
            out.push((word(&candidate), Edit::Dropped));
        }
    }
    out
}

/// What a run of the batch job did to the queue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Refreshed {
    /// Candidates this run found.
    pub found: usize,
    /// Of those, ones nobody has seen before.
    pub fresh: usize,
    /// Of those, ones already decided — kept decided.
    pub decided_before: usize,
    /// Findings on the file this run did not produce, because the corpus
    /// changed under them. Kept if they were decided and dropped if they were
    /// not: a question about a word that is no longer there has no answer.
    ///
    /// **Both halves are counted.** The un-decided half used to be dropped in
    /// silence, which is the one thing this crate does not do with an entry —
    /// every other drop it makes is named.
    pub gone: usize,
}

/// The queue on disk: `personal/suspects.jsonl`.
///
/// Yours, beside your corrections, for the same reason (spec.md §11): the batch
/// job rebuilds it whenever the corpus changes, and the one thing it must never
/// rebuild is what you have already looked at.
/// # One line written per decision
///
/// The queue on the real corpus is **28,124 entries**, and the whole motion the
/// feature is for is going down that list saying yes or no. Every decision used
/// to serialize all 28,124 of them; now it appends the one that changed, and the
/// file is rewritten only when it has grown past twice what it holds.
#[derive(Debug, Clone)]
pub struct Queue {
    log: Log,
    entries: Vec<Suspect>,
}

/// Where the queue lives under a personal layer.
#[must_use]
pub fn queue_in(personal: &Path) -> PathBuf {
    personal.join("suspects.jsonl")
}

impl Queue {
    /// The most one page may ask for, whatever was sent.
    ///
    /// A drawer that asked for the whole queue at 28,124 rows would be asking
    /// the window to hold them; past this many, the reader pages.
    pub const PAGE_LARGEST: usize = 500;

    /// Read the queue. A line that will not parse costs that candidate and is
    /// reported.
    #[must_use]
    pub fn open(personal: &Path) -> (Self, Vec<String>) {
        girsa_personal::open(Self {
            log: Log::at(queue_in(personal)),
            entries: Vec::new(),
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.log.path()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// The next `asked` to review, best first, bounded to what one page of the
    /// queue may hold.
    ///
    /// The bound is a decision about the queue and lives beside it: the window
    /// used to clamp to its own private 500, which was policy with no test in
    /// the one place policy is not allowed to live. `0` asks for the smallest
    /// honest page rather than an empty one, because a drawer that opens to
    /// nothing reads as *the queue is done*.
    #[must_use]
    pub fn page(&self, asked: usize) -> Vec<&Suspect> {
        self.ranked(asked.clamp(1, Self::PAGE_LARGEST))
    }

    /// How many are still to look at.
    #[must_use]
    pub fn waiting(&self) -> usize {
        self.entries.iter().filter(|s| s.decided.is_none()).count()
    }

    /// The next `limit` to review, best first.
    ///
    /// Selection, not a full sort: the queue holds tens of thousands of
    /// waiting entries and every caller wants single-digit `limit`s, so
    /// partitioning once and sorting only the top slice turns an O(all)
    /// sort per window call into O(one pass) plus O(`limit`).
    #[must_use]
    pub fn ranked(&self, limit: usize) -> Vec<&Suspect> {
        let mut waiting: Vec<&Suspect> = self
            .entries
            .iter()
            .filter(|s| s.decided.is_none())
            .collect();
        let best_first =
            |a: &&Suspect, b: &&Suspect| b.score.cmp(&a.score).then_with(|| a.rare.cmp(&b.rare));
        if limit < waiting.len() {
            waiting.select_nth_unstable_by(limit, best_first);
            waiting.truncate(limit);
        }
        waiting.sort_by(best_first);
        waiting
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Suspect> {
        self.entries.iter().find(|s| s.id == id)
    }

    /// Take what the batch job found, keeping every decision already made.
    ///
    /// # Errors
    ///
    /// If the queue cannot be written.
    pub fn refresh(&mut self, found: Vec<Suspect>) -> Result<Refreshed, FixError> {
        let decided: HashMap<String, Decision> = self
            .entries
            .iter()
            .filter_map(|s| s.decided.map(|d| (s.id.clone(), d)))
            .collect();
        // Every id already in the queue, for the *fresh* count below.
        //
        // That test was `self.get(&suspect.id).is_none()` — a linear scan of
        // `entries` — inside a loop over everything the batch found. Measured on
        // a real run: **28,124 entries**, so counting how many of them are new
        // was a quarter of a billion comparisons. The `HashMap` two lines up was
        // already being built over the same list, for the same reason, in the
        // same function.
        let known: std::collections::HashSet<&str> =
            self.entries.iter().map(|s| s.id.as_str()).collect();

        let mut report = Refreshed {
            found: found.len(),
            ..Refreshed::default()
        };
        let mut entries = Vec::with_capacity(found.len());
        let mut seen = std::collections::HashSet::new();
        for mut suspect in found {
            if let Some(decision) = decided.get(&suspect.id) {
                suspect.decided = Some(*decision);
                report.decided_before += 1;
            } else if !known.contains(suspect.id.as_str()) {
                report.fresh += 1;
            }
            seen.insert(suspect.id.clone());
            entries.push(suspect);
        }

        // A decided finding the corpus no longer produces is kept, so that a
        // word you dismissed does not come back the day it is re-scanned. An
        // un-decided one goes — and is counted going, because "the queue got
        // shorter and nothing says why" is the silence this crate exists to
        // close.
        for old in &self.entries {
            if !seen.contains(&old.id) && old.decided.is_none() {
                report.gone += 1;
                continue;
            }
            if old.decided.is_some() && !seen.contains(&old.id) {
                entries.push(old.clone());
                report.gone += 1;
            }
        }

        self.entries = entries;
        // A rebuild is a replacement, so this is the one write that is still a
        // whole file — and it is also the compaction, since it lands exactly
        // the entries the queue now holds.
        //
        // **This is the on-demand compaction `Store::compact` warns about.** It
        // writes the entries this process holds and nothing else, so a second
        // Girsa that decided a finding while the scan was running loses that
        // decision. It is bounded — one queue, and a decision is one word — and
        // it is not fixed here, because the fix is a read offset on the `Store`
        // trait rather than a special case in this one caller.
        self.compact()?;
        Ok(report)
    }

    /// Record what was done about a candidate. `false` if there is no such one.
    ///
    /// # Errors
    ///
    /// If the queue cannot be written.
    pub fn decide(&mut self, id: &str, decision: Decision) -> Result<bool, FixError> {
        let Some(at) = self.entries.iter().position(|s| s.id == id) else {
            return Ok(false);
        };
        // Decided on a copy and written down before the queue holds it, so a
        // decision that will not save is not one the reader is shown as made.
        let mut decided = self.entries[at].clone();
        decided.decided = Some(decision);
        self.log.append(&decided)?;
        self.entries[at] = decided;
        Ok(true)
    }
}

/// The replay, the index and the compaction — `girsa_personal::Store`.
///
/// The "index" here is a plain `Vec` in log order, which is what a *queue* is:
/// the order candidates arrived is the order they are looked at.
impl girsa_personal::Store for Queue {
    type Record = Suspect;
    const WHAT: &'static str = "a candidate";

    fn key_of(s: &Suspect) -> String {
        s.id.clone()
    }
    fn log(&self) -> &Log {
        &self.log
    }
    fn hold(&mut self, s: Suspect) {
        self.entries.push(s);
    }
    fn count(&self) -> usize {
        self.entries.len()
    }
    fn records(&self) -> Vec<&Suspect> {
        self.entries.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_words_one_edit_away_do_not_include_a_prefixed_or_suffixed_one() {
        let of = |word: &str| {
            let letters: Vec<char> = word.chars().collect();
            neighbours(&letters)
                .into_iter()
                .map(|(candidate, _)| candidate)
                .collect::<Vec<String>>()
        };
        let bare = of("ובשבת");
        assert!(
            !bare.contains(&"בשבת".to_string()),
            "a vav at the front is a word"
        );
        assert!(
            bare.contains(&"ובשבח".to_string()),
            "and a misread ת still is one"
        );

        let ends = of("דברו");
        assert!(!ends.contains(&"דבר".to_string()));
        // The other direction: `דבר` must not offer `דברו` either.
        assert!(!of("דבר").contains(&"דברו".to_string()));
        // …but a letter in the middle is fair game both ways.
        assert!(of("דבר").contains(&"דבור".to_string()));
    }

    #[test]
    fn a_confusion_is_named_only_when_it_is_one_pair_of_letters() {
        assert_eq!(confusion_between("הרבר", "הדבר").as_deref(), Some("ד/ר"));
        assert_eq!(confusion_between("הדבר", "הרבר").as_deref(), Some("ד/ר"));
        assert_eq!(confusion_between("הדבל", "הדבר"), None, "ר/ל is not a pair");
        assert_eq!(confusion_between("אבגד", "אבג"), None, "and lengths differ");
    }

    #[test]
    fn a_pointed_word_gets_no_suggestion_and_a_bare_one_keeps_its_final_letter() {
        let suspect = Suspect::new("קורינ", "קורימ", 1, 12_000, Edit::Letter);
        // The reader is looking at `קוֹרִין`. Rebuilding the points for different
        // letters is inventing text.
        assert_eq!(suspect.suggestion("קוֹרִין"), None);
        // Bare, and it ended in a final nun, so the suggestion ends in a final
        // mem rather than in the index's folded one.
        assert_eq!(suspect.suggestion("קורין").as_deref(), Some("קורים"));
        assert_eq!(suspect.suggestion("קורינה").as_deref(), Some("קורימ"));
    }

    #[test]
    fn a_word_of_latin_letters_or_digits_is_not_a_hebrew_scanning_error() {
        let mut vocab = Vocabulary::default();
        vocab.add("abcd", 1);
        vocab.add("abce", 40_000);
        vocab.add("1234", 1);
        assert!(hunt(&vocab, Settings::default()).is_empty());
    }

    /// The walk this replaced: over the **rare** words, generating candidates
    /// and asking whether each is common.
    ///
    /// Kept here and nowhere else. It is the oracle for the flip, and a
    /// property this size — *the same pairs, from the other side of the join* —
    /// is worth an oracle rather than a hand-written expectation.
    fn hunt_from_the_rare_side(vocabulary: &Vocabulary, settings: Settings) -> Vec<Suspect> {
        let mut found: Vec<Suspect> = Vec::new();
        for (rare, rare_count) in vocabulary.words() {
            if rare_count > settings.rare_at {
                continue;
            }
            let letters: Vec<char> = rare.chars().collect();
            if letters.len() < settings.shortest || !letters.iter().all(|c| LETTERS.contains(c)) {
                continue;
            }
            let mut best: Option<Suspect> = None;
            for (candidate, how) in neighbours(&letters) {
                let count = vocabulary.count(&candidate);
                if count < settings.common_at {
                    continue;
                }
                let suspect = Suspect::new(rare, &candidate, rare_count, count, how);
                if best.as_ref().is_none_or(|seen| suspect.score > seen.score) {
                    best = Some(suspect);
                }
            }
            if let Some(suspect) = best {
                found.push(suspect);
            }
        }
        found.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.rare.cmp(&b.rare))
                .then_with(|| a.common.cmp(&b.common))
        });
        found
    }

    #[test]
    fn both_sides_of_the_join_find_the_same_suspects() {
        // `hunt` walks the **common** words now. The rare side is 40–60% of any
        // term dictionary and each rare word generates ~45L+22 candidates, each
        // a fresh `Vec<char>` and a `String` — 10⁸–10⁹ allocations on a
        // multi-million-term index, under a doc comment saying *"a few million
        // lookups."* Only a word seen 10,000 times can be the common side, and
        // that is a few thousand terms.
        //
        // The pairs are the same, and this is why: substitution and
        // transposition are symmetric outright, and the two grammar filters are
        // each other's mirror — the rare side refuses *dropping* a prefix
        // letter from position 0, the common side refuses *inserting* one
        // there, and it is the same letter at the same position.
        let mut vocabulary = Vocabulary::default();

        // Common words, and the rare misreadings around them: a ד/ר
        // substitution, a letter added, a letter dropped, and a transposition —
        // including the shapes the grammar filter must refuse.
        let common = [
            "ברכות",
            "הלכה",
            "שבת",
            "אמר",
            "תפילה",
            "מצוה",
            "ישראל",
            "כתב",
        ];
        for word in common {
            vocabulary.add(word, 12_000);
        }
        let rare = [
            "ברכזת",
            "הלנה",
            "שבתת",
            "אמרר",
            "תפלה",
            "מצוח",
            "ישרלא",
            "כחב",
            // Grammar, from both directions: a prefix on a common word and a
            // suffix on one. Neither is a misreading.
            "ובברכות",
            "הלכהו",
            "ושבת",
            "אמרו",
            "בכתב",
            "מצוהו",
            // And words no edit reaches at all.
            "פלפול",
            "סוגיא",
        ];
        for word in rare {
            vocabulary.add(word, 1);
        }

        let settings = Settings::default();
        let flipped = hunt(&vocabulary, settings);
        let original = hunt_from_the_rare_side(&vocabulary, settings);

        assert!(
            !flipped.is_empty(),
            "the fixture found nothing, so this proves nothing"
        );
        // The two asymmetric edits are the whole risk — a substitution reads the
        // same from either side and proves nothing about the mirror. If the
        // fixture stops producing an `added` and a `dropped`, this test has
        // stopped checking what it is for.
        for wanted in [Edit::Letter, Edit::Added, Edit::Dropped] {
            assert!(
                flipped.iter().any(|s| s.how == wanted),
                "the fixture produced no {} — it is not exercising the mirror: {:?}",
                wanted.as_str(),
                flipped.iter().map(|s| s.how.as_str()).collect::<Vec<_>>()
            );
        }
        let said = |found: &[Suspect]| {
            found
                .iter()
                .map(|s| {
                    format!(
                        "{} → {} ({}, {})",
                        s.rare,
                        s.common,
                        s.how.as_str(),
                        s.score
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            said(&flipped),
            said(&original),
            "the two sides of the join disagree"
        );
    }

    #[test]
    fn a_page_is_bounded_whatever_was_asked() {
        // The clamp used to live in the window (`limit.clamp(1, 500)`), which
        // is a decision with no test in the one place decisions are not
        // allowed to live. Here it is, where it can be watched.
        let mut queue = Queue::open(&std::env::temp_dir().join("girsa-queue-page-unused")).0;
        queue.entries = (0..Queue::PAGE_LARGEST * 2)
            .map(|n| Suspect::new(&format!("קורין{n}"), "קורים", 1, 12_000, Edit::Letter))
            .collect();

        assert_eq!(queue.page(10).len(), 10);
        // Zero asks for the smallest honest page, not an empty one: an open
        // drawer that shows nothing reads as *the queue is done*.
        assert_eq!(queue.page(0).len(), 1);
        // …and however large the ask, the page holds at most `PAGE_LARGEST`.
        assert_eq!(queue.page(usize::MAX).len(), Queue::PAGE_LARGEST);
    }
}
