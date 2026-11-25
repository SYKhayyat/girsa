//! Who the commentary is by — the register the catalogue has and never showed.
//!
//! spec.md §5 keeps `author`, `era` and `comp_date` on every work, and a reader
//! opening a mefaresh they have never met has one question: *who is this, when,
//! and why is he here?* The fields answer it per **work**, which is the wrong
//! unit — `רמב"ם` authored 195 works in this corpus, and the shelf showed him 195
//! times.
//!
//! This module groups them into people. Nothing here is inferred beyond what the
//! corpus wrote down:
//!
//! - **the join key is the normalized author string** (`girsa_hebrew::normalize`,
//!   the same rule the index is built under). `רמב״ם` and `רמב"ם` are one man;
//!   `רש"י` and `רשב"א` are not, and no fuzzy match is attempted — a wrong
//!   merging of two mefarshim is the same class of error as a wrong ref
//!   (BUILDER.md rule 6), and it is worse than showing two rows for one man.
//! - **dates come from [`crate::era`]** and nowhere else: `comp_date` first, then
//!   the era code, and where there is neither the person is *undated*, which is a
//!   state and not a year 0. Ordering puts them at the end rather than in the
//!   middle of a sequence nobody wrote down.
//! - **attribution is a field, not a qualifier in a title.** `תשובות הרשב"א
//!    מיוחס להרמב"ן` is filed under the Rashba and *claimed* for the Ramban, and
//!   the register carries both halves: who the catalogue says, and what the title
//!   says. It does not decide between them — that is a question for a person who
//!   has read the responsa.
//!
//! # What this is not
//!
//! Not a register of *people* — it is a register of the authors the corpus
//! names, so a mefaresh whose work was catalogued anonymously has no entry, and
//! 990 works in this corpus carry no `author` at all. An absence is counted where
//! it happens ([`Register::anonymous`]) rather than hidden in an empty row.

use std::collections::BTreeMap;

use crate::era::{self, Era, When};
use crate::work::Work;

/// The ways a title says *this is not straightforwardly his*, and whether the
/// phrase names anybody.
///
/// Hebrew first because that is what the corpus writes: 3 of the 6,509 works in
/// this corpus carry one of these, and all three are Otzaria-side. Matching is on
/// the phrase, and the name after it is kept **as written** — resolving
/// `הרמב"ן` here to a register entry would be this module inventing an identity
/// the title did not state.
///
/// `מיוחס לו` is a **pronoun** — *attributed to him* — and it names nobody. It is
/// listed first and matched by length rather than by order, because `מיוחס ל` is a
/// prefix of it: the shorter phrase used to win and turned *attributed to him*
/// into a man called `ו`.
const DOUBT: [(&str, bool); 3] = [
    ("מיוחס לו", false),
    ("מיוחס ל", true),
    ("attributed to", true),
];

/// Whether the catalogue's author is the author.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "how", rename_all = "snake_case")]
pub enum Attribution {
    /// The title asserts it, with nothing qualifying the claim.
    Stated,
    /// The title qualifies it. `said` is the qualification **as written** and
    /// `to` is the name after it, where the phrase named one at all — `מיוחס לו`
    /// ("attributed to him") points at the work's own title and names nobody.
    Doubted { said: String, to: Option<String> },
}

impl Attribution {
    /// Whether anybody has said this attribution is uncertain.
    #[must_use]
    pub const fn is_doubted(&self) -> bool {
        matches!(self, Self::Doubted { .. })
    }
}

/// One sefer this person wrote, and what it is a commentary on.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Written {
    pub slug: String,
    pub he_title: String,
    /// The base seforim, as the corpus declares them — read from the schema, not
    /// guessed from the title (see [`Work::commentary_on`]).
    pub on: Vec<String>,
    pub attribution: Attribution,
}

/// A person the corpus names, and what it says about them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Commentator {
    /// The normalized name — the join key, and what [`Register::find`] matches.
    pub key: String,
    /// The name as the corpus writes it. When the corpus spells one man's name
    /// two ways, this is the **most frequent** spelling, and the others are in
    /// [`Commentator::also_called`]: a register that picked the longest or the
    /// first would be the register showing one of the spellings at random.
    pub name: String,
    /// The other spellings of the same normalized name, in the corpus's order.
    pub also_called: Vec<String>,
    /// The English name, where every one of this person's works agrees on one.
    ///
    /// Read off `en_title` — `Rashi on Berakhot` → `Rashi`. Only used when the
    /// stems all agree; two works disagreeing means the catalogue does not name
    /// this person in English and a guess would be a name nobody wrote.
    pub en_name: Option<String>,
    /// The era, where the corpus gives one. The **most specific** of this
    /// person's works — see [`best_when`].
    pub era: Option<String>,
    /// The years, where `comp_date` gave them.
    pub years: Option<(i32, i32)>,
    /// The base seforim this person comments on, deduplicated, in the order the
    /// corpus lists them.
    pub on: Vec<String>,
    /// The works, in the order a shelf holds them.
    pub works: Vec<Written>,
}

impl Commentator {
    /// When this person is placed on the axis, in the shape [`crate::era`] uses.
    #[must_use]
    pub fn when(&self) -> When {
        When {
            era: self.era.as_deref().and_then(Era::from_code),
            years: self.years,
        }
    }

    /// The years as a column reads them — `1065–1115`, or `None`.
    #[must_use]
    pub fn written(&self) -> Option<String> {
        self.when().written()
    }

    /// The era in the words a reader uses, or `None`.
    #[must_use]
    pub fn era_spoken(&self) -> Option<&'static str> {
        self.when().era.map(Era::he)
    }

    /// Whether anything about this person is doubted.
    #[must_use]
    pub fn is_doubted(&self) -> bool {
        self.works.iter().any(|work| work.attribution.is_doubted())
    }
}

/// The authors the corpus names, grouped.
#[derive(Debug, Clone, Default)]
pub struct Register {
    /// Sorted by key, so a caller that forgets to sort still gets one order twice.
    people: Vec<Commentator>,
    /// The index into [`Register::people`] for a normalized name.
    by_key: BTreeMap<String, usize>,
    /// The index into [`Register::people`] for a work's slug.
    ///
    /// The other direction of the same table, because the question *who is this
    /// class* starts from a work and not from a name — `mefarshim` hands out
    /// slugs, and looking the person up by re-parsing his name out of the title
    /// would be a second naming rule.
    by_work: BTreeMap<String, usize>,
    /// How many commentary works had no author to file them under, in total and
    /// per base sefer.
    ///
    /// Counted because a register that quietly dropped them would look complete —
    /// *nobody is named here* and *nobody comments here* are different answers, and
    /// the second one would be wrong.
    anonymous: usize,
    anonymous_on: BTreeMap<String, usize>,
}

impl Register {
    /// Build the register from a catalogue.
    ///
    /// Only works the corpus says are **commentaries** are read: `author` on a
    /// base text is the sefer's author, which is a person the register would then
    /// claim comments on nothing. A person who wrote both appears through the
    /// commentary — the Rambam's פירוש המשנה is a work, and the Mishneh Torah is
    /// not a commentary on anything.
    #[must_use]
    pub fn of(works: &[Work]) -> Self {
        struct Building {
            spellings: BTreeMap<String, usize>,
            en_stems: BTreeMap<String, usize>,
            wrotes: Vec<Written>,
            when: Vec<When>,
        }
        let mut by_key: BTreeMap<String, Building> = BTreeMap::new();
        let mut anonymous = 0;
        let mut anonymous_on: BTreeMap<String, usize> = BTreeMap::new();
        for work in works {
            if work.commentary_on.is_empty() {
                continue;
            }
            let written_name = work.author.as_deref().map(str::trim).unwrap_or_default();
            let named = (!written_name.is_empty())
                .then(|| girsa_hebrew::normalize(written_name))
                .filter(|key| !key.is_empty());
            let Some(key) = named else {
                anonymous += 1;
                for base in &work.commentary_on {
                    *anonymous_on.entry(base.slug.clone()).or_default() += 1;
                }
                continue;
            };
            let building = by_key.entry(key).or_insert_with(|| Building {
                spellings: BTreeMap::new(),
                en_stems: BTreeMap::new(),
                wrotes: Vec::new(),
                when: Vec::new(),
            });
            *building
                .spellings
                .entry(written_name.to_string())
                .or_default() += 1;
            if let Some(stem) = en_stem(&work.en_title) {
                *building.en_stems.entry(stem).or_default() += 1;
            }
            building.wrotes.push(Written {
                slug: work.slug.clone(),
                he_title: work.he_title.clone(),
                on: work
                    .commentary_on
                    .iter()
                    .map(|base| base.slug.clone())
                    .collect(),
                attribution: attribution_of(&work.he_title),
            });
            building.when.push(When {
                era: work.era.as_deref().and_then(Era::from_code),
                years: work.comp_date.as_deref().and_then(era::parse_comp_date),
            });
        }

        let mut people: Vec<Commentator> = Vec::with_capacity(by_key.len());
        let mut index: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_work: BTreeMap<String, usize> = BTreeMap::new();
        for (key, building) in by_key {
            let (name, also_called) = spellings(&building.spellings);
            let en_name = only_one(&building.en_stems);
            let when = best_when(&building.when);
            let mut on: Vec<String> = Vec::new();
            for written in &building.wrotes {
                for base in &written.on {
                    if !on.contains(base) {
                        on.push(base.clone());
                    }
                }
            }
            index.insert(key.clone(), people.len());
            for written in &building.wrotes {
                by_work.insert(written.slug.clone(), people.len());
            }
            people.push(Commentator {
                key,
                name,
                also_called,
                en_name,
                era: when.era.map(|era| era.code().to_string()),
                years: when.years,
                on,
                works: building.wrotes,
            });
        }

        Self {
            people,
            by_key: index,
            by_work,
            anonymous,
            anonymous_on,
        }
    }

    /// The people, in key order.
    #[must_use]
    pub fn people(&self) -> &[Commentator] {
        &self.people
    }

    /// How many commentary works carry no author, and so are in no row here.
    #[must_use]
    pub const fn anonymous(&self) -> usize {
        self.anonymous
    }

    /// How many commentaries on **this** sefer carry no author.
    ///
    /// The count a *who comments here* answer needs: zero means every commentary
    /// on this sefer is named above, and a number means the list is that many
    /// short of the shelf.
    #[must_use]
    pub fn anonymous_on(&self, slug: &str) -> usize {
        self.anonymous_on.get(slug).copied().unwrap_or(0)
    }

    /// A person by any spelling of their name, or by their key.
    ///
    /// The query goes through the same normalizer the keys did, so the reader may
    /// type `רמב״ם` or `רמב"ם` and both find him; nothing else is matched. `None`
    /// is a real answer — the corpus does not name this person — and the caller
    /// says so rather than showing an empty profile.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Commentator> {
        let key = girsa_hebrew::normalize(name);
        self.by_key.get(&key).map(|at| &self.people[*at])
    }

    /// The person behind a work, where the corpus names one.
    ///
    /// `None` is a commentary with no author — the corpus has the work and not
    /// the man — and every caller says that rather than showing a nameless row.
    #[must_use]
    pub fn of_work(&self, slug: &str) -> Option<&Commentator> {
        self.by_work.get(slug).map(|at| &self.people[*at])
    }

    /// Who comments on this sefer.
    ///
    /// By slug, which is the id the corpus uses for the base text in
    /// `commentary_on`. A slug no commentary names answers empty, and *nobody
    /// comments here in this corpus* is a true answer.
    #[must_use]
    pub fn on(&self, slug: &str) -> Vec<&Commentator> {
        self.people
            .iter()
            .filter(|person| person.on.iter().any(|base| base == slug))
            .collect()
    }

    /// The people in the order time puts them, undated last.
    ///
    /// Three groups, because there are three kinds of answer: those with years
    /// (ordered by the year they **began**, which is when a person appears in the
    /// world rather than when their last sefer was finished), those with only an
    /// era (ordered by [`Era`], which is the one chronological thing an era code
    /// is), and those with neither — sorted by name, because putting them
    /// anywhere else would be a claim that they belong there.
    #[must_use]
    pub fn chronological(&self) -> Vec<&Commentator> {
        let mut people: Vec<&Commentator> = self.people.iter().collect();
        people.sort_by(|a, b| {
            let rank = |person: &&Commentator| {
                if person.years.is_some() {
                    0u8
                } else if person.era.is_some() {
                    1
                } else {
                    2
                }
            };
            rank(a)
                .cmp(&rank(b))
                .then_with(|| match (a.years, b.years) {
                    (Some((a_from, a_to)), Some((b_from, b_to))) => {
                        a_from.cmp(&b_from).then_with(|| a_to.cmp(&b_to))
                    }
                    _ => {
                        let era_rank = |person: &&Commentator| {
                            person.when().era.map_or(usize::MAX, |era| {
                                Era::all()
                                    .iter()
                                    .position(|known| *known == era)
                                    .unwrap_or(usize::MAX)
                            })
                        };
                        era_rank(a).cmp(&era_rank(b))
                    }
                })
                // Name last, so the same register sorts the same way twice and two
                // people the corpus dates the same year are not ordered by hash.
                .then_with(|| a.name.cmp(&b.name))
        });
        people
    }
}

/// The most frequent spelling, and the rest.
fn spellings(counts: &BTreeMap<String, usize>) -> (String, Vec<String>) {
    let mut ordered: Vec<(&String, &usize)> = counts.iter().collect();
    // Most frequent first, then the corpus's own order (`BTreeMap` is sorted by
    // the string), so a tie is broken the same way every run.
    ordered.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let name = ordered
        .first()
        .map_or_else(String::new, |(name, _)| (*name).clone());
    let also = ordered
        .iter()
        .skip(1)
        .map(|(name, _)| (*name).clone())
        .collect();
    (name, also)
}

/// The one English stem, if there is exactly one.
fn only_one(counts: &BTreeMap<String, usize>) -> Option<String> {
    if counts.len() == 1 {
        counts.keys().next().cloned()
    } else {
        None
    }
}

/// `Rashi on Berakhot` → `Rashi`, and `None` for a title that is not that shape.
///
/// The cataloguer's English for a commentary is `<name> on <base>`; a base text's
/// own title has no ` on ` and yields nothing, which is right — this register only
/// reads commentaries anyway.
fn en_stem(title: &str) -> Option<String> {
    let (stem, _) = title.split_once(" on ")?;
    let stem = stem.trim();
    (!stem.is_empty()).then(|| stem.to_string())
}

/// What a title says about who wrote it.
fn attribution_of(title: &str) -> Attribution {
    // The **longest** phrase that matches, not the first in the list: `מיוחס ל`
    // is a prefix of `מיוחס לו`, and choosing by order made *attributed to him*
    // into a man called `ו`.
    let found = DOUBT
        .iter()
        .filter_map(|(phrase, names)| title.find(phrase).map(|at| (at, phrase, names)))
        .max_by_key(|(_, phrase, _)| phrase.len());
    let Some((at, phrase, names)) = found else {
        return Attribution::Stated;
    };
    let after = title[at + phrase.len()..].trim();
    // The name runs while the characters look like a name. The parenthesis
    // Otzaria writes after one — `חידושי הרן (מיוחס לו) על שבת` — is not part of
    // it.
    let to: String = after
        .chars()
        .take_while(|c| c.is_alphabetic() || matches!(c, '"' | '״' | '\'' | '׳'))
        .collect();
    let to = to.trim().to_string();
    Attribution::Doubted {
        said: format!("{phrase}{to}"),
        to: (*names && !to.is_empty()).then_some(to),
    }
}

/// The most specific thing the corpus says about when this person lived.
///
/// Years beat an era code, because an era is a century-wide bucket and the years
/// are a date. Where several works disagree — a commentary written in one decade
/// and another in the next — the **earliest start and the latest end** are taken:
/// a person's span is their life, and one work's dates are a fact about the work.
fn best_when(whens: &[When]) -> When {
    let years = whens.iter().filter_map(|when| when.years).fold(
        None::<(i32, i32)>,
        |so_far, (from, to)| match so_far {
            None => Some((from, to)),
            Some((old_from, old_to)) => Some((old_from.min(from), old_to.max(to))),
        },
    );
    let era = whens.iter().filter_map(|when| when.era).min();
    When { era, years }
}
