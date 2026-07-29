//! Placing a sefer in time — the axis a transmission chain runs along.
//!
//! spec.md §8.6 asks to *trace forward from a Gemara to how it became halacha*
//! and *backward from a ruling to where the posek got it*. Both sentences have
//! a direction in them, and **the graph does not**. An edge is stored once, in
//! the shard of the work it points from (§8.2), and which end that is was
//! decided by whoever wrote the row:
//!
//! ```text
//! bavli/berakhot          → its commentaries     51,927 edges   earlier → later
//! mishnah-berurah         → shulchan-arukh        18,806 edges   later → earlier
//! shulchan-arukh/or-ch    → turei-zahav            3,315 edges   earlier → later
//! shulchan-arukh/or-ch    → tur                      719 edges   later → earlier
//! ```
//!
//! Following edge direction would walk one of those chains forwards and the
//! next one backwards, and call both a chain. So direction comes from **when
//! the seforim were written**, and this module is the only place that answers
//! it.
//!
//! # The era code is not enough, and the corpus says so
//!
//! Sefaria stamps `era` on 4,812 of the 7,189 works here — `T`, `A`, `GN`,
//! `RI`, `AH`, `CO`. It is too coarse for the question this is for: the
//! **Shulchan Arukh (1563) and the Mishnah Berurah (1905) are both `AH`**, and
//! that pair is the single most-asked hop in the whole feature. On era codes
//! alone they are contemporaries and the chain stops.
//!
//! `comp_date` is on 5,294 works and it is a real year. It carries the ordering
//! era loses, and it reaches Tanach, which has dates and no era code at all.
//! So: **years first, era only where there are no years**, and where there is
//! neither the answer is [`Order::Unknown`] and is shown as such.
//!
//! Measured over the graph on this machine: **88.7% of the 4,182,344 edges
//! point at a work that can be placed in time** (78.2% on era codes alone). The
//! other 11.3% are not walked and are counted where they were refused, because
//! a chain that quietly skipped the seforim it could not date would look
//! shorter and surer than it is.
//!
//! # Six shapes, and fifty of them are in Hebrew
//!
//! Every distinct shape of `comp_date` in the corpus, counted:
//!
//! ```text
//! 4,820  c.1065  – c.1115 CE      (two spaces before the en-dash)
//!   317  1563 CE
//!    47  c.550  – c.500 BCE       descending, because BCE counts down
//!    43  c.50 BCE  – c.100 CE     straddles the epoch
//!    16  1815  – 1870 CE
//!     1  c.1200 CE
//!    50  ה' תרלז - ה' תרלז (בקירוב)   Hebrew, anno mundi
//! ```
//!
//! The last group is small enough to have skipped and is parsed anyway: they
//! are Otzaria-side acharonim, which is the layer a halachic chain **ends** at,
//! so dropping them would shorten exactly the traces this exists for.

use std::collections::HashMap;
use std::path::Path;

/// The six eras Sefaria labels a work with, in the order they happened.
///
/// `Ord` is chronological and is the point of the type. A code this table does
/// not know is **not** mapped to the nearest one — [`Era::from_code`] returns
/// `None` and the work is treated as undated, because a guess at which century
/// an unknown code means is a claim a reader cannot check (BUILDER.md rule 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Era {
    Tannaim,
    Amoraim,
    Geonim,
    Rishonim,
    Acharonim,
    Contemporary,
}

impl Era {
    /// Sefaria's code, as written on the schema.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Tannaim => "T",
            Self::Amoraim => "A",
            Self::Geonim => "GN",
            Self::Rishonim => "RI",
            Self::Acharonim => "AH",
            Self::Contemporary => "CO",
        }
    }

    /// The era in the words a reader uses.
    #[must_use]
    pub const fn he(self) -> &'static str {
        match self {
            Self::Tannaim => "תנאים",
            Self::Amoraim => "אמוראים",
            Self::Geonim => "גאונים",
            Self::Rishonim => "ראשונים",
            Self::Acharonim => "אחרונים",
            Self::Contemporary => "מחברי זמננו",
        }
    }

    /// Read a code. `None` for a code this project does not know.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().into_iter().find(|e| e.code() == code)
    }

    /// Every era, earliest first. The order a facet lists them in.
    #[must_use]
    pub const fn all() -> [Self; 6] {
        [
            Self::Tannaim,
            Self::Amoraim,
            Self::Geonim,
            Self::Rishonim,
            Self::Acharonim,
            Self::Contemporary,
        ]
    }
}

/// When a sefer was written, as much as the corpus knows.
///
/// Both fields optional and both kept: the years order finely and the era is
/// what a reader recognises, so a hop is *labelled* with the era and *ordered*
/// by the years.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct When {
    pub era: Option<Era>,
    /// Years CE, BCE negative, inclusive. `None` where `comp_date` was absent
    /// or in a shape this does not read.
    pub years: Option<(i32, i32)>,
}

impl When {
    /// Whether this can be placed on the axis at all.
    #[must_use]
    pub const fn is_placed(&self) -> bool {
        self.era.is_some() || self.years.is_some()
    }

    /// The year a chain sorts by, where there is one — the **later** end.
    ///
    /// A range is when the sefer was being written, and what matters for
    /// "did this exist yet" is when it was finished.
    #[must_use]
    pub const fn latest_year(&self) -> Option<i32> {
        match self.years {
            Some((_, to)) => Some(to),
            None => None,
        }
    }
}

/// Which of two seforim came first.
///
/// [`Order::Unknown`] is a fourth answer and not a synonym for
/// [`Order::Contemporary`]. *These two were written at the same time* and *I
/// cannot date one of them* send a chain in different directions, and telling
/// them apart is the whole reason this is an enum and not an `Option<Ordering>`
/// collapsed at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// The first was finished before the second was begun.
    Before,
    /// The first was begun after the second was finished.
    After,
    /// Their years overlap, or their eras are the same.
    Contemporary,
    /// One of them has no date and no era.
    Unknown,
}

impl Order {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
            Self::Contemporary => "contemporary",
            Self::Unknown => "unknown",
        }
    }
}

/// Which of two came first, on years where both have years and on eras
/// otherwise.
///
/// A **mixed** pair — one dated, one only in an era — is [`Order::Unknown`]
/// rather than resolved by pretending the era is a year range. The conventional
/// span of "ראשונים" differs by a century between authorities, and a chain that
/// ordered a hop on that would be asserting something nobody wrote down. It
/// costs little: of the 5,375 placeable works, 5,294 carry years.
#[must_use]
pub fn order(a: &When, b: &When) -> Order {
    if let (Some((a_from, a_to)), Some((b_from, b_to))) = (a.years, b.years) {
        if a_to < b_from {
            return Order::Before;
        }
        if b_to < a_from {
            return Order::After;
        }
        return Order::Contemporary;
    }
    match (a.era, b.era) {
        (Some(a), Some(b)) => match a.cmp(&b) {
            std::cmp::Ordering::Less => Order::Before,
            std::cmp::Ordering::Greater => Order::After,
            std::cmp::Ordering::Equal => Order::Contemporary,
        },
        _ => Order::Unknown,
    }
}

/// Every work's place in time, read once from the catalogue.
///
/// `corpus/works/index.jsonl` is 5 MB and holds the `era` and `comp_date` the
/// importer copied off Sefaria's schemas. Reading it is milliseconds; deriving
/// it per hop would be the whole file per hop.
#[derive(Debug, Clone, Default)]
pub struct Timeline {
    when: HashMap<String, When>,
    /// Works in the catalogue with neither a date nor an era.
    undated: usize,
}

impl Timeline {
    /// Read the catalogue of one root. Call it again per root to merge in your
    /// own layer, whose works have no dates and are `Unknown` against
    /// everything — which is the truthful answer for a note.
    ///
    /// # Errors
    ///
    /// If `works/index.jsonl` is not there. A line that will not parse is
    /// skipped rather than fatal, the way every other reader of this file
    /// treats it.
    pub fn load(&mut self, root: &Path) -> Result<(), std::io::Error> {
        let body = std::fs::read_to_string(root.join("works/index.jsonl"))?;
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(work) = serde_json::from_str::<crate::work::Work>(line) else {
                continue;
            };
            let when = When {
                era: work.era.as_deref().and_then(Era::from_code),
                years: work.comp_date.as_deref().and_then(parse_comp_date),
            };
            if !when.is_placed() {
                self.undated += 1;
            }
            self.when.insert(work.slug, when);
        }
        Ok(())
    }

    /// Read one root, in one call.
    ///
    /// # Errors
    ///
    /// As [`Timeline::load`].
    pub fn of(root: &Path) -> Result<Self, std::io::Error> {
        let mut timeline = Self::default();
        timeline.load(root)?;
        Ok(timeline)
    }

    /// Where a work sits. A work not in the catalogue is undated, not absent —
    /// the caller wants to know it could not be placed, not that it does not
    /// exist.
    #[must_use]
    pub fn when(&self, slug: &str) -> When {
        self.when.get(slug).copied().unwrap_or_default()
    }

    /// Which of two works came first.
    #[must_use]
    pub fn order(&self, a: &str, b: &str) -> Order {
        order(&self.when(a), &self.when(b))
    }

    #[must_use]
    pub fn works(&self) -> usize {
        self.when.len()
    }

    /// How many works in the catalogue could not be placed at all.
    ///
    /// Reported by every tool that walks the graph, because it is the size of
    /// the hole in every answer they give.
    #[must_use]
    pub const fn undated(&self) -> usize {
        self.undated
    }
}

/// `c.1065  – c.1115 CE` → `(1065, 1115)`. BCE years are negative.
///
/// Returns `None` on a shape this does not read, and never a half-answer: a
/// range whose second half will not parse is not silently turned into a point.
#[must_use]
pub fn parse_comp_date(text: &str) -> Option<(i32, i32)> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(years) = parse_hebrew_range(text) {
        return Some(years);
    }

    // The corpus writes the separator as an en-dash and as a hyphen, and pads
    // it with two spaces on the left. Splitting on the bare character would cut
    // `c.1065` in half at the hyphen in a date like `1815-1870`, which is why
    // the padded form is tried first and the bare one only as a whole token.
    let (left, right) = split_range(text)?;

    // `CE`/`BCE` is written once, at the end, and governs whichever halves do
    // not say for themselves: `c.50 BCE  – c.100 CE` says both, `c.1065  –
    // c.1115 CE` says it once and means it for both.
    let trailing_bce = right.trim_end().ends_with("BCE");
    let from = parse_year(left, trailing_bce)?;
    let to = parse_year(right, trailing_bce)?;
    // BCE counts down, so `c.1400 – c.400 BCE` is already in order once both
    // are negative. A pair that is still the wrong way round is put in order
    // rather than rejected: the corpus means a span either way.
    Some((from.min(to), from.max(to)))
}

/// A range into its two halves, or the whole string twice for a single year.
fn split_range(text: &str) -> Option<(&str, &str)> {
    for sep in ["  – ", "  - ", " – ", " - ", "–"] {
        if let Some(cut) = text.split_once(sep) {
            return Some(cut);
        }
    }
    Some((text, text))
}

/// `c.1115 CE` → `1115`; `c.400 BCE` → `-400`.
fn parse_year(text: &str, default_bce: bool) -> Option<i32> {
    let text = text.trim();
    let bce = if text.ends_with("BCE") {
        true
    } else if text.ends_with("CE") {
        false
    } else {
        default_bce
    };
    let digits: String = text
        .trim_start_matches("c.")
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        return None;
    }
    let year: i32 = digits.parse().ok()?;
    Some(if bce { -year } else { year })
}

/// `ה' תרלז - ה' תרלז (בקירוב)` → `(1877, 1877)`.
///
/// Anno mundi: the letter before the apostrophe is the millennium, the word
/// after it the rest. `ה' תרלז` is 5,637, and 5637 − 3760 = 1877 CE. Fifty
/// works in the corpus are dated this way and no other.
fn parse_hebrew_range(text: &str) -> Option<(i32, i32)> {
    let text = text.split(" (").next().unwrap_or(text);
    let (left, right) = text.split_once(" - ")?;
    let from = parse_hebrew_year(left)?;
    let to = parse_hebrew_year(right)?;
    Some((from.min(to), from.max(to)))
}

/// How many years the Hebrew calendar is ahead of the common one.
const ANNO_MUNDI_OFFSET: i32 = 3760;

fn parse_hebrew_year(text: &str) -> Option<i32> {
    let text = text.trim();
    let (millennium, rest) = text.split_once('\'')?;
    let thousands = gematria(millennium.trim())?;
    let years = gematria(rest.trim())?;
    Some(thousands * 1000 + years - ANNO_MUNDI_OFFSET)
}

/// A Hebrew numeral as a number. `None` on any character that is not one, so a
/// word that merely looks like a year is refused rather than half-read.
fn gematria(text: &str) -> Option<i32> {
    let mut total = 0;
    let mut any = false;
    for ch in text.chars() {
        if ch == '\u{5F3}' || ch == '\'' || ch == '"' || ch == '\u{5F4}' {
            continue;
        }
        let value = match ch {
            'א' => 1,
            'ב' => 2,
            'ג' => 3,
            'ד' => 4,
            'ה' => 5,
            'ו' => 6,
            'ז' => 7,
            'ח' => 8,
            'ט' => 9,
            'י' => 10,
            'כ' | 'ך' => 20,
            'ל' => 30,
            'מ' | 'ם' => 40,
            'נ' | 'ן' => 50,
            'ס' => 60,
            'ע' => 70,
            'פ' | 'ף' => 80,
            'צ' | 'ץ' => 90,
            'ק' => 100,
            'ר' => 200,
            'ש' => 300,
            'ת' => 400,
            _ => return None,
        };
        total += value;
        any = true;
    }
    if any {
        Some(total)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn dated(from: i32, to: i32) -> When {
        When {
            era: None,
            years: Some((from, to)),
        }
    }

    fn in_era(era: Era) -> When {
        When {
            era: Some(era),
            years: None,
        }
    }

    #[test]
    fn era_codes_are_chronological() {
        assert!(Era::Tannaim < Era::Amoraim);
        assert!(Era::Amoraim < Era::Geonim);
        assert!(Era::Geonim < Era::Rishonim);
        assert!(Era::Rishonim < Era::Acharonim);
        assert!(Era::Acharonim < Era::Contemporary);
    }

    #[test]
    fn a_code_nobody_wrote_is_not_guessed_at() {
        assert_eq!(Era::from_code("RI"), Some(Era::Rishonim));
        assert_eq!(Era::from_code("ri"), None, "the codes are as written");
        assert_eq!(Era::from_code(""), None);
        assert_eq!(Era::from_code("MEDIEVAL"), None);
    }

    #[test]
    fn the_shulchan_arukh_comes_before_the_mishnah_berurah() {
        // The hop the whole feature is for, and the one the era code cannot
        // make: both of these are `AH`.
        let sa = When {
            era: Some(Era::Acharonim),
            years: parse_comp_date("1563 CE"),
        };
        let mb = When {
            era: Some(Era::Acharonim),
            years: parse_comp_date("c.1875  – c.1905 CE"),
        };
        assert_eq!(order(&sa, &mb), Order::Before);
        assert_eq!(order(&mb, &sa), Order::After);
        assert_eq!(
            order(&in_era(Era::Acharonim), &in_era(Era::Acharonim)),
            Order::Contemporary,
            "on era codes alone, which is why the years are read"
        );
    }

    #[test]
    fn an_undated_work_orders_against_nothing() {
        // And is not quietly treated as a contemporary, which would let a chain
        // walk through a sefer it cannot place and present it as a step.
        assert_eq!(order(&dated(1000, 1100), &When::default()), Order::Unknown);
        assert_eq!(order(&When::default(), &When::default()), Order::Unknown);
        assert_eq!(
            order(&dated(1000, 1100), &in_era(Era::Acharonim)),
            Order::Unknown,
            "a mixed pair is not resolved by inventing a span for the era"
        );
    }

    #[test]
    fn overlapping_lifetimes_are_contemporaries_not_an_order() {
        assert_eq!(
            order(&dated(1040, 1105), &dated(1100, 1171)),
            Order::Contemporary
        );
        assert_eq!(order(&dated(450, 550), &dated(1065, 1115)), Order::Before);
    }

    #[test]
    fn every_comp_date_shape_in_the_corpus_reads() {
        // The six shapes, with their counts as they stand in
        // corpus/works/index.jsonl.
        assert_eq!(parse_comp_date("c.1065  – c.1115 CE"), Some((1065, 1115)));
        assert_eq!(parse_comp_date("1563 CE"), Some((1563, 1563)));
        assert_eq!(parse_comp_date("c.1400  – c.400 BCE"), Some((-1400, -400)));
        assert_eq!(parse_comp_date("c.50 BCE  – c.100 CE"), Some((-50, 100)));
        assert_eq!(parse_comp_date("1815  – 1870 CE"), Some((1815, 1870)));
        assert_eq!(parse_comp_date("c.1200 CE"), Some((1200, 1200)));
    }

    #[test]
    fn bce_counts_down_and_still_comes_out_in_order() {
        // Genesis is `c.1400  – c.400 BCE`. Read without the sign it would sort
        // after the Bavli, and every trace out of a pasuk would run backwards.
        let genesis = When {
            era: None,
            years: parse_comp_date("c.1400  – c.400 BCE"),
        };
        let bavli = When {
            era: Some(Era::Amoraim),
            years: parse_comp_date("c.450  – c.550 CE"),
        };
        assert_eq!(order(&genesis, &bavli), Order::Before);
    }

    #[test]
    fn the_hebrew_dated_fifty_are_read_too() {
        // ה' תרלז = 5,637 anno mundi = 1877 CE. These are Otzaria-side
        // acharonim — the layer a halachic chain ends at, so dropping them
        // would shorten exactly the traces this is for.
        assert_eq!(
            parse_comp_date("ה' תרלז - ה' תרלז (בקירוב)"),
            Some((1877, 1877))
        );
        assert_eq!(
            parse_comp_date("ד' תתקכ - ד' תתקכ (בקירוב)"),
            Some((1160, 1160))
        );
        // 3,860 anno mundi, which is the only one of the fifty that lands in
        // the first century — and comes out positive, not eight centuries BCE.
        assert_eq!(
            parse_comp_date("ג' תתס - ג' תתס (בקירוב)"),
            Some((100, 100))
        );
    }

    #[test]
    fn a_date_in_a_shape_this_does_not_read_is_none_not_a_guess() {
        assert_eq!(parse_comp_date(""), None);
        assert_eq!(parse_comp_date("sometime in the middle ages"), None);
        assert_eq!(parse_comp_date("c. שנה"), None);
    }

    #[test]
    fn a_range_is_ordered_by_when_it_was_finished() {
        assert_eq!(dated(1875, 1905).latest_year(), Some(1905));
        assert_eq!(When::default().latest_year(), None);
    }
}
