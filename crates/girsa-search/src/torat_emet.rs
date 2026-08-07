//! Torat Emet — the literal mode, and the default (BUILDER.md W12).
//!
//! **What you typed is what was searched for.** Nothing is stemmed, expanded or
//! guessed (spec.md §9.3). The one thing that happens to the words is that
//! their nikud comes off, and that is not a widening: it removes marks nobody
//! types and it cannot cause a match the reader would not want (§9.1).
//!
//! # Why literal is the default and not a limitation
//!
//! Predictability is the feature. A search you can predict is one you can
//! *aim*: you know why you got a result and why you did not, so a bad result
//! tells you how to fix the query. The moment the engine helps without saying
//! so, every empty result becomes ambiguous — did the text not say this, or did
//! the engine mangle what I asked? Sefaria's analyzer over-stems and does not
//! report it, and spec.md §9 names that as a failure mode not to reproduce.
//!
//! # The operators are the ones that get used
//!
//! Not a query language. [`Match`] says how one written word is matched — as
//! itself, by the letters it contains, or by letters in order with others
//! between — and [`Together`] says how the words relate: anywhere in the
//! segment, adjacent, or within so many words of each other. Both are chips in
//! the search bar (spec.md §9.5); W14 draws them.
//!
//! # Every query shows its work
//!
//! [`Query::plan`] returns exactly what will be asked of the index. A mode
//! whose whole promise is *no surprises* has to be able to prove it, and the
//! same [`Plan`] is what a result header will say the search was.
//!
//! Widening — prefixes, ktiv male, abbreviations, roots — is W13's ladder, is
//! **offered** with a count computed up front, and is applied only when the
//! reader clicks. A zero here stays a zero.

use girsa_hebrew::normalize;

/// How one written word is matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Match {
    /// This word. Its normal form, and nothing else.
    #[default]
    Word,
    /// The word **contains** these letters, adjacent and in this order:
    /// `קדש` → `המקדש`, `ויקדשהו`.
    Contains,
    /// These letters **in this order**, with any others between them:
    /// `קדש` → `קידוש`.
    Letters,
}

girsa_corpus::spelled!(Match {
    Word => "Word",
    Contains => "Contains",
    Letters => "Letters",
});

/// How the words of a query relate to each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Together {
    /// All of them, anywhere in the segment. An **and**, never an or.
    #[default]
    Anywhere,
    /// Adjacent, in the order they were typed.
    Phrase,
    /// With at most `words` other words between them, **in any order** —
    /// which is what *"within X words of each other"* says.
    Near { words: u32 },
}

impl Together {
    /// What this is called on the wire.
    ///
    /// `Near` carries a number, so it cannot be a `spelled!` table: the
    /// spelling *is* the number. `Near5` is one chip choice and `Near12` is
    /// another, and a reader who has set one is not offered the other.
    #[must_use]
    pub fn key(self) -> String {
        match self {
            Self::Anywhere => "Anywhere".to_string(),
            Self::Phrase => "Phrase".to_string(),
            Self::Near { words } => format!("{NEAR}{words}"),
        }
    }

    /// Read back what [`Self::key`] wrote.
    ///
    /// `None` for anything else — including `Near` with no number and `Near`
    /// with something that is not one. It used to fall through to `Anywhere`,
    /// so `Nearbanana` was a search of the whole segment presented as a
    /// proximity search, with the chip showing what the reader asked for.
    #[must_use]
    pub fn named(key: &str) -> Option<Self> {
        match key {
            "Anywhere" => Some(Self::Anywhere),
            "Phrase" => Some(Self::Phrase),
            other => other
                .strip_prefix(NEAR)
                .and_then(|n| n.parse().ok())
                .map(|words| Self::Near { words }),
        }
    }
}

/// The prefix on the proximity chip's key, before the number.
const NEAR: &str = "Near";

/// More orderings than this and the search is refused instead of approximated.
///
/// Order-free proximity is checked one ordering at a time and the count is
/// factorial: five words is 120 phrase queries and six is 720. Running some of
/// them and calling the result an answer is the silent-partial-answer failure
/// this whole mode exists to avoid, so the ceiling is stated and the reader is
/// told which chip to reach for instead.
pub const MOST_WORDS_UNORDERED: usize = 5;

/// How many terms a `contains`/`letters` pattern may expand to inside a phrase.
///
/// Tantivy's ceiling, restated here so the number is ours and the error message
/// can be about seforim rather than about postings lists.
pub const MOST_EXPANSIONS: u32 = 1 << 14;

/// One Torat Emet query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    text: String,
    matching: Match,
    together: Together,
    max_expansions: u32,
}

impl Query {
    /// The literal query for what a reader typed.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            matching: Match::default(),
            together: Together::default(),
            max_expansions: MOST_EXPANSIONS,
        }
    }

    #[must_use]
    pub fn matching(mut self, matching: Match) -> Self {
        self.matching = matching;
        self
    }

    #[must_use]
    pub fn together(mut self, together: Together) -> Self {
        self.together = together;
        self
    }

    /// Lower the expansion ceiling. For tests, and for a caller that would
    /// rather be refused early than wait.
    #[must_use]
    pub fn with_max_expansions(mut self, most: u32) -> Self {
        self.max_expansions = most;
        self
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn matching_kind(&self) -> Match {
        self.matching
    }

    #[must_use]
    pub fn shape(&self) -> Together {
        self.together
    }

    #[must_use]
    pub fn max_expansions(&self) -> u32 {
        self.max_expansions
    }

    /// Exactly what will be asked of the index.
    #[must_use]
    pub fn plan(&self) -> Plan {
        let words: Vec<String> = normalize(&self.text)
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let patterns = words.iter().map(|w| self.pattern_for(w)).collect();
        let (slop, orderings) = match self.together {
            Together::Anywhere => (0, 1),
            Together::Phrase => (0, 1),
            Together::Near { words: gap } => (gap, permutation_count(words.len())),
        };
        Plan {
            words,
            patterns,
            slop,
            orderings,
            matching: self.matching,
            together: self.together,
        }
    }

    fn pattern_for(&self, word: &str) -> String {
        pattern_for(self.matching, word)
    }
}

/// One word as the pattern the index will be asked for.
///
/// For [`Match::Word`] the pattern **is** the word: no `.*`, no alternation,
/// nothing that could reach a different word. The other two are regexes over
/// whole terms, so they carry their own anchoring.
///
/// Free rather than a method because W13's ladder builds patterns for words the
/// reader did not type — a peeled stem, an abbreviation's expansion — and those
/// have to be built by the same rule as the typed ones or the widened search
/// stops meaning what the literal one meant.
#[must_use]
pub fn pattern_for(matching: Match, word: &str) -> String {
    match matching {
        Match::Word => word.to_string(),
        Match::Contains => format!(".*{}.*", escape(word)),
        Match::Letters => {
            let mut pattern = String::from(".*");
            for c in word.chars() {
                pattern.push_str(&escape(&c.to_string()));
                pattern.push_str(".*");
            }
            pattern
        }
    }
}

/// Whether an indexed word answers a typed one under a given [`Match`] rule.
///
/// The same three rules the patterns encode, in Rust rather than in a regex —
/// because a highlight has to agree with the search exactly, and two
/// descriptions of one rule drift. Both arguments are normal forms.
#[must_use]
pub fn matches_under(matching: Match, typed: &str, indexed: &str) -> bool {
    match matching {
        Match::Word => typed == indexed,
        Match::Contains => indexed.contains(typed),
        Match::Letters => {
            let mut letters = typed.chars();
            let mut wanted = letters.next();
            for c in indexed.chars() {
                if Some(c) == wanted {
                    wanted = letters.next();
                }
            }
            wanted.is_none()
        }
    }
}

/// What a query will actually ask for.
///
/// Held separately from the query so that a result header can say what was
/// searched for without re-deriving it — and so that a test can assert the
/// literal mode changed nothing, which is the acceptance of W12.
///
/// W13 moved the index's query builder onto [`crate::ladder::Widening`], which
/// can hold several forms per position where this holds one. That would have
/// left this struct describing a query nobody runs, so
/// `ladder::tests::a_query_with_no_rungs_applied_is_still_the_literal_query`
/// asserts the two agree wherever a plan has an opinion: same words, same
/// patterns, same shape, one form per position. The acceptance of W12 is
/// checked against the thing that runs, not beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The typed words, with their marks off. In order.
    pub words: Vec<String>,
    /// What each word becomes for the index. Equal to `words` in the plain
    /// case, which is the point.
    pub patterns: Vec<String>,
    /// How many other words may sit between them.
    pub slop: u32,
    /// How many orderings will be tried. One, unless the shape is order-free.
    pub orderings: usize,
    pub matching: Match,
    pub together: Together,
}

impl Plan {
    /// Whether this plan asks for anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Whether an indexed word answers a typed one, under this plan's rule.
    #[must_use]
    pub fn matches(&self, typed: &str, indexed: &str) -> bool {
        matches_under(self.matching, typed, indexed)
    }

    /// A line a result header can show: what was searched for, in words.
    #[must_use]
    pub fn describe(&self) -> String {
        let words = self.words.join(" ");
        let how = match self.matching {
            Match::Word => "the words",
            Match::Contains => "words containing",
            Match::Letters => "words with the letters",
        };
        let where_ = match self.together {
            Together::Anywhere => "anywhere in a segment".to_string(),
            Together::Phrase => "one after the other".to_string(),
            Together::Near { words } => format!("within {words} words of each other"),
        };
        format!("{how} {words}, {where_}")
    }
}

/// How many orderings of `n` words there are, saturating rather than wrapping.
fn permutation_count(n: usize) -> usize {
    (1..=n)
        .try_fold(1usize, |acc, k| acc.checked_mul(k))
        .unwrap_or(usize::MAX)
}

/// Escape the handful of characters a regex would read as syntax.
///
/// Normalization already reduces text to Hebrew letters, ASCII alphanumerics,
/// geresh and gershayim, so in practice nothing here needs escaping. It is done
/// anyway: the day the normal form admits one more character is not the day to
/// discover that a query became a pattern.
#[must_use]
pub fn escape(word: &str) -> String {
    const SYNTAX: [char; 14] = [
        '.', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '\\',
    ];
    let mut out = String::with_capacity(word.len());
    for c in word.chars() {
        if SYNTAX.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_plain_pattern_is_the_word_itself() {
        let plan = Query::new("קדש").plan();
        assert_eq!(plan.patterns, ["קדש"]);
    }

    #[test]
    fn contains_and_letters_differ_in_exactly_one_way() {
        assert_eq!(
            Query::new("קדש").matching(Match::Contains).plan().patterns,
            [".*קדש.*"]
        );
        assert_eq!(
            Query::new("קדש").matching(Match::Letters).plan().patterns,
            [".*ק.*ד.*ש.*"]
        );
    }

    #[test]
    fn a_query_of_marks_alone_asks_for_nothing() {
        assert!(Query::new("׃ ־").plan().is_empty());
    }

    #[test]
    fn orderings_are_counted_before_anything_is_run() {
        assert_eq!(
            Query::new("א ב")
                .together(Together::Near { words: 1 })
                .plan()
                .orderings,
            2
        );
        assert_eq!(
            Query::new("א ב ג")
                .together(Together::Near { words: 1 })
                .plan()
                .orderings,
            6
        );
        // A shape that does not reorder never counts more than one.
        assert_eq!(Query::new("א ב ג").plan().orderings, 1);
    }

    #[test]
    fn the_description_says_what_was_searched_for() {
        let plan = Query::new("יתגבר כארי")
            .together(Together::Near { words: 3 })
            .plan();
        assert_eq!(
            plan.describe(),
            "the words יתגבר כארי, within 3 words of each other"
        );
    }
}
