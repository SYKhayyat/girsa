//! The relaxation ladder (BUILDER.md W13, spec.md §9.6).
//!
//! Zero results is a bug in the interface, not an answer. But the default mode
//! is the literal one, so the engine **offers** the next step instead of taking
//! it:
//!
//! > *No results for `טרף`.*
//! > *`[try other forms — 7]` `[expand abbreviations — 2]` `[widen to same passage — 19]`*
//!
//! The counts are computed **before** the reader clicks, which is what makes
//! the offer informative on its own: you learn there are seven other forms
//! without leaving the literal mode. One click applies it, and the header then
//! says what changed, reversibly.
//!
//! # The rungs are data, not four call sites that agree today
//!
//! [`Rung::ALL`] is the ladder of §9.6 in order — drop nikud, other forms,
//! root, expand abbreviations, widen proximity. Smart mode climbs it in that
//! order and the offers come back in it, from the one list, so the two cannot
//! drift apart.
//!
//! Two rungs are on the ladder without being offers, and both would be a
//! silent gap if they were simply missing:
//!
//! - **Nikud** is [`Standing::Climbed`] — every mode strips it at index time,
//!   with no toggle (§9.1). Offering it would be offering to do something that
//!   is already done.
//! - **Root** is [`Standing::Deferred`] — §9.4 investigated the candidates and
//!   rejected all of them, because there is no rabbinic-Hebrew-and-Aramaic
//!   analyser to build it on. It is named so a reader is told the rung exists
//!   and is not built, rather than reading a missing chip as *nothing down that
//!   road*.
//!
//! # Which direction a rung widens in
//!
//! This is the part that looks done and is not. §9.2's table reads *you type
//! `שבת`, the page says `וּבַשַּׁבָּת`* — the corpus word is the **longer** one, and
//! the index holds it unwidened on purpose (W11). So a rung cannot simply swap
//! the typed word for another word; it has to reach words the typed one is
//! inside of. Each rung therefore offers both directions where they differ:
//!
//! | Rung | Typed long, page short | Typed short, page long |
//! |---|---|---|
//! | prefixes | `ובשבת` → peel to `שבת` | `שבת` → any term `[והבכלמשד]{1,4}שבת` |
//! | ktiv | `כוהן` → `כהן` | `כהן` → `כוהן` (both from the same table) |
//! | gershayim | `שו"ע` → `שוע` | `רמבם` → any term spelling it with them |
//! | abbreviations | `שו"ע` → `שולחן ערוך` | and back, the table is symmetric |
//!
//! # An alternative can be more than one word
//!
//! `שו"ע` expands to `שולחן ערוך`, which is two words. So a position in a query
//! is not *a word or a word* but *a word or a run of words*, and the widened
//! query is built from the runs rather than from a rewritten query string. In a
//! proximity search that means a cross product, and a cross product means a
//! ceiling — see [`MOST_EXACT_QUERIES`].

use girsa_hebrew::{variants_with, VariantKind, PREFIX_LETTERS};

use crate::torat_emet::{escape, matches_under, pattern_for, Match, Query, Together};

/// How many prefix letters may be stacked in front of a typed word.
///
/// `וכשהמלך` is four, and four is what the corpus actually stacks. The bound
/// exists because an unbounded `[והבכלמשד]*` in front of a two-letter word is a
/// wildcard, and a wildcard is the opposite of a rung a reader can predict.
pub const MOST_STACKED_PREFIXES: usize = 4;

/// How many exact searches one widened query may be worth.
///
/// Order-free proximity is already the union over orderings (W12); widening
/// multiplies that by the combinations of the words' forms. Past this the query
/// is **refused**, not sampled — running some of the combinations and calling
/// the result an answer is precisely what this mode exists not to do.
pub const MOST_EXACT_QUERIES: usize = 1024;

/// One rung of the relaxation ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rung {
    /// Nikud and te'amim off. Climbed at index time, in every mode (§9.1).
    Nikud,
    /// Other surface forms of the same word — one rung per transformation, so
    /// the reader can take the one they meant.
    Forms(VariantKind),
    /// Every form of a root. Named by §9.6, deferred by §9.4.
    Root,
    /// From a phrase or a proximity to the whole passage.
    Proximity,
}

impl Rung {
    /// Every rung, in the order spec.md §9.6 sets out.
    ///
    /// Note where abbreviations sit: **after** the root rung, not with the
    /// other form rungs. That is the spec's order and it is deliberate —
    /// expanding `שו"ע` into `שולחן ערוך` changes what the query is *about*
    /// far more than respelling a word does.
    pub const ALL: [Self; 7] = [
        Self::Nikud,
        Self::Forms(VariantKind::PrefixPeeled),
        Self::Forms(VariantKind::KtivSwapped),
        Self::Forms(VariantKind::GershayimDropped),
        Self::Root,
        Self::Forms(VariantKind::AbbreviationExpanded),
        Self::Proximity,
    ];

    /// Whether this rung can be offered, and if not, why not.
    #[must_use]
    pub const fn standing(self) -> Standing {
        match self {
            Self::Nikud => Standing::Climbed,
            Self::Root => Standing::Deferred(
                "there is no rabbinic-Hebrew-and-Aramaic morphological analyser to build it on \
                 (spec.md §9.4)",
            ),
            Self::Forms(_) | Self::Proximity => Standing::Ready,
        }
    }

    /// How this rung is named on its chip.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nikud => "drop nikud",
            Self::Forms(kind) => kind.label(),
            Self::Root => "match the root",
            Self::Proximity => "widen to the same passage",
        }
    }

    /// The name this rung travels under — on a command line, and between the
    /// window and the engine.
    ///
    /// One naming, in one place. A chip that said `prefixes` while the flag
    /// that applied it was called something else would be two vocabularies for
    /// one ladder, and the second one to be edited would quietly stop matching.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nikud => "nikud",
            Self::Forms(VariantKind::PrefixPeeled) => "prefixes",
            Self::Forms(VariantKind::KtivSwapped) => "spellings",
            Self::Forms(VariantKind::GershayimDropped) => "gershayim",
            Self::Forms(VariantKind::AbbreviationExpanded) => "abbreviations",
            Self::Root => "root",
            Self::Proximity => "proximity",
        }
    }

    /// A rung by that name, or nothing. Never a nearest match.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rung| rung.name() == name)
    }

    /// Where this rung sits on the ladder. Used to keep offers in order.
    #[must_use]
    pub fn height(self) -> usize {
        Self::ALL
            .iter()
            .position(|r| *r == self)
            .unwrap_or(usize::MAX)
    }
}

/// Whether a rung is available, and if not, why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Already climbed, in every mode, before any search runs.
    Climbed,
    /// A rung that can be offered and applied.
    Ready,
    /// On the ladder by name and not built. The reason is shown, not hidden.
    Deferred(&'static str),
}

/// What one alternative word is matched by.
///
/// A rule rather than a bare pattern, because a highlight has to agree with the
/// search exactly and a regex cannot be asked *did you match this word*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    /// This word, under the query's own [`Match`] rule.
    Word(String),
    /// Any term that is this word behind up to [`MOST_STACKED_PREFIXES`]
    /// prefix letters.
    Prefixed(String),
    /// Any term that is this word with geresh or gershayim written into it.
    Punctuated(String),
}

/// One word the index may be asked for, and the rule it is asked under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    /// What the index is asked: a term, or a regex over whole terms.
    pub pattern: String,
    /// Whether [`Form::pattern`] is a regex.
    pub regex: bool,
    /// The rule, for highlighting — see [`Rule`].
    pub rule: Rule,
    /// What a reader is shown this form as.
    pub shown: String,
}

impl Form {
    /// A plain word, matched under the query's own rule.
    fn word(matching: Match, word: &str) -> Self {
        Self {
            pattern: pattern_for(matching, word),
            regex: matching != Match::Word,
            rule: Rule::Word(word.to_string()),
            shown: word.to_string(),
        }
    }

    /// Any term that is `word` behind stacked prefixes.
    fn prefixed(word: &str) -> Self {
        let letters: String = PREFIX_LETTERS
            .iter()
            .map(|c| escape(&c.to_string()))
            .collect();
        Self {
            pattern: format!("[{letters}]{{1,{MOST_STACKED_PREFIXES}}}{}", escape(word)),
            regex: true,
            rule: Rule::Prefixed(word.to_string()),
            // The ellipsis leads, so in right-to-left it renders on the side a
            // prefix is actually written on.
            shown: format!("…{word}"),
        }
    }

    /// Any term that is `word` with gershayim written into it.
    fn punctuated(word: &str) -> Self {
        let mut pattern = String::new();
        for (i, c) in word.chars().enumerate() {
            if i > 0 {
                pattern.push_str("[\"']?");
            }
            pattern.push_str(&escape(&c.to_string()));
        }
        Self {
            pattern,
            regex: true,
            rule: Rule::Punctuated(word.to_string()),
            shown: format!("{word} with gershayim"),
        }
    }

    /// Whether an indexed word answers this form.
    #[must_use]
    pub fn matches(&self, matching: Match, indexed: &str) -> bool {
        match &self.rule {
            Rule::Word(word) => matches_under(matching, word, indexed),
            Rule::Prefixed(word) => {
                let Some(front) = indexed.strip_suffix(word.as_str()) else {
                    return false;
                };
                let stacked = front.chars().count();
                (1..=MOST_STACKED_PREFIXES).contains(&stacked)
                    && front.chars().all(|c| PREFIX_LETTERS.contains(&c))
            }
            Rule::Punctuated(word) => bare(indexed) == bare(word),
        }
    }
}

/// A word with its geresh and gershayim taken out.
fn bare(word: &str) -> String {
    word.chars().filter(|c| *c != '\'' && *c != '"').collect()
}

/// One thing a position may be answered by: a word, or a run of words an
/// abbreviation expands into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alternative {
    /// One entry for a word, several for an expansion, in order.
    pub forms: Vec<Form>,
    /// What a reader is shown this alternative as.
    pub shown: String,
}

impl Alternative {
    fn of(matching: Match, words: &[String]) -> Self {
        Self {
            forms: words.iter().map(|w| Form::word(matching, w)).collect(),
            shown: words.join(" "),
        }
    }

    fn single(form: Form) -> Self {
        Self {
            shown: form.shown.clone(),
            forms: vec![form],
        }
    }
}

/// One typed word, and everything the applied rungs let it be answered by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// The word as typed, with its marks off.
    pub typed: String,
    /// What may answer it. The first is always the typed word itself, so a
    /// widened search can never lose a literal hit.
    pub alternatives: Vec<Alternative>,
}

impl Position {
    /// Whether any rung actually gave this position something new.
    #[must_use]
    pub fn was_widened(&self) -> bool {
        self.alternatives.len() > 1
    }
}

/// A literal query with rungs of the ladder applied to it.
///
/// Holds the literal query untouched, so the widening is always reversible:
/// [`Widened::literal`] is the one-click undo of spec.md §9.6.
#[derive(Debug, Clone)]
pub struct Widened {
    base: Query,
    rungs: Vec<Rung>,
}

impl Widened {
    /// Apply these rungs to this query.
    ///
    /// Rungs that are not [`Standing::Ready`] are dropped, and the rest are put
    /// into ladder order and deduplicated — so a caller cannot produce a
    /// widening whose reported order differs from the order it was built in.
    #[must_use]
    pub fn new(base: Query, rungs: impl IntoIterator<Item = Rung>) -> Self {
        let mut kept: Vec<Rung> = Vec::new();
        for rung in rungs {
            if rung.standing() == Standing::Ready && !kept.contains(&rung) {
                kept.push(rung);
            }
        }
        kept.sort_by_key(|r| r.height());
        Self { base, rungs: kept }
    }

    /// The literal query this was widened from — the undo.
    #[must_use]
    pub fn literal(&self) -> &Query {
        &self.base
    }

    /// The rungs applied, in ladder order.
    #[must_use]
    pub fn rungs(&self) -> &[Rung] {
        &self.rungs
    }

    /// Exactly what the index will be asked.
    #[must_use]
    pub fn widening(&self) -> Widening {
        let plan = self.base.plan();
        let matching = self.base.matching_kind();
        let positions = plan
            .words
            .iter()
            .map(|word| self.position(matching, word))
            .collect();
        let together = if self.rungs.contains(&Rung::Proximity) {
            // One step, to the whole passage: spec.md §9.6's
            // "[widen to same passage — 19]". Half-steps through intermediate
            // distances would be more rungs than the spec has, each one a chip
            // whose difference from the last a reader cannot predict.
            Together::Anywhere
        } else {
            self.base.shape()
        };
        Widening {
            applied: self.rungs.clone(),
            positions,
            together,
            matching,
            base: self.base.shape(),
        }
    }

    /// One typed word, widened.
    fn position(&self, matching: Match, word: &str) -> Position {
        let mut alternatives = vec![Alternative::of(
            matching,
            std::slice::from_ref(&word.to_string()),
        )];
        let mut push = |alternative: Alternative| {
            if !alternatives.iter().any(|a| a.forms == alternative.forms) {
                alternatives.push(alternative);
            }
        };

        for rung in &self.rungs {
            let Rung::Forms(kind) = rung else { continue };
            for form in variants_with(word, &[*kind]).forms_of_kind(*kind) {
                let words: Vec<String> = girsa_hebrew::normalize(form)
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                if !words.is_empty() {
                    push(Alternative::of(matching, &words));
                }
            }

            // The other direction — reaching a longer word on the page from a
            // shorter one typed. Only under `Match::Word`: `contains` and
            // `letters` already reach inside longer words, so adding these
            // would be a chip that changes nothing, and an inert chip is a lie
            // about what the engine can do.
            if matching != Match::Word {
                continue;
            }
            match kind {
                VariantKind::PrefixPeeled => push(Alternative::single(Form::prefixed(word))),
                VariantKind::GershayimDropped if !word.contains('"') && !word.contains('\'') => {
                    push(Alternative::single(Form::punctuated(word)));
                }
                _ => {}
            }
        }

        Position {
            typed: word.to_string(),
            alternatives,
        }
    }
}

/// What a widened search actually ran — the header's source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Widening {
    /// The rungs applied, in ladder order.
    pub applied: Vec<Rung>,
    /// One per typed word.
    pub positions: Vec<Position>,
    /// How the words relate, after any proximity rung.
    pub together: Together,
    /// How each word is matched. Unchanged by the ladder.
    pub matching: Match,
    /// How the words related *before* any proximity rung, so the header can say
    /// what changed rather than only what is.
    pub base: Together,
}

impl Widening {
    /// Whether this widening asks for anything the literal query did not.
    ///
    /// A rung that changes nothing must not be shown, applied or counted: the
    /// reader would click it, see the same results, and learn that the chips
    /// are decoration.
    #[must_use]
    pub fn changes_anything(&self) -> bool {
        self.together != self.base || self.positions.iter().any(Position::was_widened)
    }

    /// How many forms were added beyond the words as typed.
    #[must_use]
    pub fn added_forms(&self) -> usize {
        self.positions
            .iter()
            .map(|p| p.alternatives.len().saturating_sub(1))
            .sum()
    }

    /// Whether an indexed word is one of the forms this widening asked for.
    #[must_use]
    pub fn matches_word(&self, indexed: &str) -> bool {
        self.positions.iter().any(|position| {
            position
                .alternatives
                .iter()
                .any(|alt| alt.forms.iter().any(|f| f.matches(self.matching, indexed)))
        })
    }

    /// A line a result header can show: what was searched for, and what was
    /// done to the query to get there.
    #[must_use]
    pub fn describe(&self) -> String {
        let words: Vec<String> = self
            .positions
            .iter()
            .map(|position| {
                let others: Vec<&str> = position
                    .alternatives
                    .iter()
                    .skip(1)
                    .map(|a| a.shown.as_str())
                    .collect();
                if others.is_empty() {
                    position.typed.clone()
                } else {
                    format!("{} (or {})", position.typed, others.join(", "))
                }
            })
            .collect();
        let where_ = match self.together {
            Together::Anywhere => "anywhere in a segment".to_string(),
            Together::Phrase => "one after the other".to_string(),
            Together::Near { words } => format!("within {words} words of each other"),
        };
        let how: Vec<&str> = self.applied.iter().map(|r| r.label()).collect();
        if how.is_empty() {
            format!("{}, {where_}", words.join(" "))
        } else {
            format!("{}, {where_} — {}", words.join(" "), how.join(", "))
        }
    }
}

/// One rung, priced before the click.
#[derive(Debug, Clone)]
pub struct Offer {
    pub rung: Rung,
    /// How the chip is named.
    pub label: &'static str,
    /// How many results clicking would show. Computed up front, from the same
    /// query clicking would run, so the promise and the result cannot disagree.
    pub count: usize,
    /// What clicking runs.
    pub widened: Widened,
}

/// A rung whose count could not be worked out, and why.
///
/// Named rather than dropped: a missing chip reads as *there is nothing down
/// that road*, which is a different statement from *this could not be checked*.
#[derive(Debug, Clone)]
pub struct Refusal {
    pub rung: Rung,
    pub why: String,
}

/// Everything the ladder has to say about one literal query.
#[derive(Debug, Clone, Default)]
pub struct Offers {
    /// The rungs worth showing, in ladder order, each with a count above zero.
    pub offers: Vec<Offer>,
    /// The rungs that could not be priced.
    pub refused: Vec<Refusal>,
    /// The rungs spec.md §9.6 names that are not built (§9.4).
    pub deferred: Vec<Rung>,
}

impl Offers {
    /// Whether there is anything at all to show the reader.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.offers.is_empty() && self.refused.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_typed_word_is_always_the_first_alternative() {
        // The invariant that makes every rung a widening rather than a swap.
        for typed in ["שבת", "כהנ", "שו\"ע", "מלכ"] {
            let widened = Widened::new(Query::new(typed), Rung::ALL);
            for position in widened.widening().positions {
                assert_eq!(position.alternatives[0].shown, position.typed);
            }
        }
    }

    #[test]
    fn a_query_with_no_rungs_applied_is_still_the_literal_query() {
        // The sibling W13 created and has to close. Before this order, W12's
        // `Plan` *was* what the index was built from, and asserting
        // `plan.patterns == plan.words` proved the literal mode literal. The
        // builder now works from a `Widening`, so that assertion proves nothing
        // unless the two agree — a widening with no rungs has to be the plan,
        // exactly, or W12's promise is being checked against a document rather
        // than against the query that ran.
        for typed in ["מלך", "שו\"ע", "מֵאֵימָתַי קוֹרִין", "יתגבר כארי"]
        {
            for matching in [Match::Word, Match::Contains, Match::Letters] {
                for together in [
                    Together::Anywhere,
                    Together::Phrase,
                    Together::Near { words: 2 },
                ] {
                    let query = Query::new(typed).matching(matching).together(together);
                    let plan = query.plan();
                    let wide = Widened::new(query, []).widening();
                    assert_eq!(wide.together, plan.together);
                    assert_eq!(wide.matching, plan.matching);
                    assert_eq!(wide.positions.len(), plan.words.len());
                    assert!(!wide.changes_anything());
                    for (position, (word, pattern)) in wide
                        .positions
                        .iter()
                        .zip(plan.words.iter().zip(plan.patterns.iter()))
                    {
                        assert_eq!(position.typed, *word);
                        assert_eq!(position.alternatives.len(), 1);
                        assert_eq!(position.alternatives[0].forms.len(), 1);
                        assert_eq!(position.alternatives[0].forms[0].pattern, *pattern);
                    }
                }
            }
        }
    }

    #[test]
    fn a_rung_that_is_not_ready_cannot_be_applied() {
        let widened = Widened::new(Query::new("שבת"), [Rung::Nikud, Rung::Root]);
        assert!(widened.rungs().is_empty());
    }

    #[test]
    fn rungs_come_out_in_ladder_order_however_they_went_in() {
        let widened = Widened::new(
            Query::new("שבת"),
            [
                Rung::Proximity,
                Rung::Forms(VariantKind::AbbreviationExpanded),
                Rung::Forms(VariantKind::PrefixPeeled),
            ],
        );
        assert_eq!(
            widened.rungs(),
            [
                Rung::Forms(VariantKind::PrefixPeeled),
                Rung::Forms(VariantKind::AbbreviationExpanded),
                Rung::Proximity,
            ]
        );
    }

    #[test]
    fn the_prefix_rule_and_the_prefix_pattern_agree() {
        // The regex goes to tantivy and the rule does the highlighting. If they
        // disagree, a hit is marked on a word that did not match it.
        let form = Form::prefixed("מלכ");
        assert!(form.matches(Match::Word, "וכשהמלכ"));
        assert!(form.matches(Match::Word, "המלכ"));
        assert!(!form.matches(Match::Word, "מלכ"), "one prefix at least");
        assert!(!form.matches(Match::Word, "אמלכ"), "א is not a prefix");
        assert!(
            !form.matches(Match::Word, "והוכשהמלכ"),
            "five is past the stack"
        );
    }

    #[test]
    fn the_gershayim_rule_reaches_the_spelling_with_them_in() {
        let form = Form::punctuated("רמבמ");
        assert!(form.matches(Match::Word, "רמב\"מ"));
        assert!(!form.matches(Match::Word, "רמבמא"));
    }

    #[test]
    fn a_widening_that_adds_nothing_says_so() {
        // `אמת` starts with no prefix letter, is three plain letters, and is in
        // no abbreviation table. Every form rung leaves it alone.
        let widened = Widened::new(
            Query::new("אמת").matching(Match::Contains),
            [Rung::Forms(VariantKind::PrefixPeeled)],
        );
        assert!(!widened.widening().changes_anything());
    }
}
