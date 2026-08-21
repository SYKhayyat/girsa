//! What a work's schema calls the parts of an address, for the works whose
//! schema says it somewhere [`crate::work::Work::he_sections`] never looked.
//!
//! # The finding
//!
//! Every line of the Tur carried `orach_chayim א' א'` in its margin. Every line
//! of the Arukh HaShulchan carried `orach_chaim א' ט"ו`. That is a Latin slug,
//! set left-to-right, inside a right-to-left Hebrew margin, on 6,005 lines of
//! one and 25,265 of the other — and it says nothing about סימן or סעיף either,
//! so the two seforim a person reaches for beside a Shulchan Arukh were the two
//! whose addresses could not be read.
//!
//! It is the same defect the reader already reported once about the Gemara —
//! *"a Hebrew daf carried `30b:11` down its side"* — fixed then by routing the
//! margin through `girsa_cite`, and still true here because `girsa_cite` can
//! only print the words it is given.
//!
//! # Why `he_sections` is empty on exactly these works
//!
//! `Work::he_sections` is read from the schema's `heSectionNames`. A **flat**
//! schema carries them at its root and everything works. A **branch** schema —
//! a work that holds its chalakim inside itself — carries no `heSectionNames`
//! at the root at all: they sit on the leaf nodes, because the Tur's הקדמה is
//! addressed by פסקה and its body by סימן and סעיף, and the root cannot answer
//! for both.
//!
//! `girsa-cite`'s own note names the population: *"1,101 of Sefaria's 6,595 are
//! branch schemas that do not carry them."* The importer read the root, found
//! nothing, and wrote an empty list — which is honest about the root and wrong
//! about the work.
//!
//! # And the section's own name was never carried anywhere
//!
//! The second half. A named level in an address is the **slug of the schema
//! node's `title`** — `Orach Chayim` → `orach_chayim` — and the node's `heTitle`
//! beside it says `אורח חיים`. Nothing in the corpus's own files keeps that
//! pairing, so the address had the slug and no way back to the name.
//!
//! Matched on `title` and not on `key`, and that is measured rather than
//! assumed: the Tur's first node is `key: "Orach Chaim"`, `title: "Orach
//! Chayim"`, and its segments are addressed `orach_chayim`. Slugging the key
//! would miss all four chalakim of the sefer this module was written for.
//!
//! # Read here rather than repaired at import
//!
//! Re-running the importer to write these into `work.json` re-cuts segments,
//! which renames permanent ids — the same argument as
//! [`crate::taxonomy`]'s sibling in `girsa_app::display::opens_a_siman`. This is
//! one small JSON file read once per sefer opened, and the corpus does not move.

use std::collections::BTreeMap;
use std::path::Path;

use girsa_ref::{Address, Level};

/// How a person writes the four chalakim, and what they are writing.
///
/// Held with the gershayim already taken out, because [`spelling`] takes them
/// out of what it is given too — `או"ח`, `או״ח` and `אוח` are one query.
///
/// # Why a table and not a rule
///
/// There is no rule. `אורח חיים` → `או"ח` is the first *two* letters of the
/// first word and the first of the second; `אבן העזר` → `אה"ע` is the first of
/// the first and the first *two* of the second. Any generic acronym rule gets
/// one of them wrong, and a rule that is wrong about a chelek of the Tur is
/// worse than four rows.
///
/// Only the four, deliberately. These are the section names that recur across
/// every code on the shelf — the Tur, the Arukh HaShulchan, the Shulchan Arukh
/// HaRav, the Levush — and they are the ones a person types short. A section
/// with a name of its own (`סדר הגט`, `הקדמה`) is written out, and is matched
/// from the schema like everything else.
const ABBREVIATED: &[(&str, &str)] = &[
    ("אוח", "אורח חיים"),
    // `יוד` and `יורד` both, and see [`Sections::section_of`] for the one of them
    // that usually never arrives here: `יו"ד` is also, letter for letter, the
    // number 20, and the resolver reads it as one before this is asked.
    ("יוד", "יורה דעה"),
    ("יורד", "יורה דעה"),
    ("אהע", "אבן העזר"),
    ("אבהעז", "אבן העזר"),
    ("חומ", "חושן משפט"),
];

/// Every number a section written this way could have been read as before it
/// reached us.
///
/// A great many section names are also valid Hebrew numerals, and the resolver
/// reads the numeral first. Measured on the shelf, that is not an edge case —
/// it is most of the long tail:
///
/// | typed | arrives as | is really |
/// |---|---|---|
/// | `טור יו"ד סימן א` | `20:1` | יורה דעה |
/// | `אגרא דכלה בא א` | `3:1` | פרשת בא |
/// | `אדרת אליהו נח א` | `58:1` | פרשת נח |
/// | `אברבנאל על מורה נבוכים חלק א א` | `1:1` | חלק א |
///
/// Three readings are checked, because there are three ways the spelling can be
/// eaten: the name whole (`בא`), its last word alone (`חלק א`, after the
/// resolver has taken `חלק` for a level label), and the abbreviation a person
/// writes for it (`יו"ד` for יורה דעה).
///
/// See [`Sections::read_as_a_number`] for the guards, and for the case this
/// deliberately gets wrong.
fn read_as(title: &str) -> impl Iterator<Item = u32> + '_ {
    let short = ABBREVIATED
        .iter()
        .find(|(_, full)| spelling(full) == spelling(title))
        .map(|(short, _)| *short);
    std::iter::once(title)
        .chain(title.split_whitespace().next_back())
        .chain(short)
        .filter_map(girsa_ref::numerals::parse_hebrew)
}

/// How many levels a section name may have been cut into.
///
/// Five, which covers the longest name on the shelf — `ימים ראשונים של סוכות`,
/// four words of which one is a numeral — with a word to spare.
const SPREAD: usize = 5;

/// A level, as the word it was before the resolver read it as a number.
///
/// The inverse of the reading that broke the name up: a level that is a number
/// is spelled back out in letters, and a level that is a word is itself.
/// `to_hebrew` writes the gershayim, which [`spelling`] takes straight back
/// off — the marks are how a person writes it and not part of the word.
fn spelled_out(level: &str) -> String {
    level
        .parse::<u32>()
        .map_or_else(|_| level.to_string(), girsa_ref::numerals::to_hebrew)
}

/// A spelling with everything a person varies taken out of it.
///
/// Gershayim, geresh, apostrophes and their ASCII stand-ins, every space, the
/// whole nikud-and-te'amim block, and every other mark a title carries and a
/// reader does not retype — a colon, a bracket, a comma, a dash. What is left is
/// letters and digits, which is the only part of `או"ח` / `או׳׳ח` / `אוח` that
/// is the same in all three, and the only part of `חלק א': בית נתיבות` that
/// survives being cut into levels on the way here.
fn spelling(said: &str) -> String {
    said.chars()
        // U+0591..U+05C7 is the cantillation and vowel block, and it carries
        // the geresh and gershayim punctuation at U+05F3/U+05F4 just above it.
        // Rust calls the marks alphanumeric, so they come off before the test
        // below rather than being caught by it.
        .filter(|c| !('\u{0591}'..='\u{05C7}').contains(c))
        .filter(|c| !matches!(c, '\u{05F3}' | '\u{05F4}'))
        // **Letters and digits, and nothing else.** This kept everything that
        // was not a space, a quote or a nikud mark, which was enough while the
        // titles being compared were words. Sefaria's are not always words:
        // `חלק א': בית נתיבות` and `אדרת אליהו (ר' יוסף חיים)` carry a colon and
        // a bracket inside the **name**, and nobody typing that mekor
        // reproduces the punctuation — the resolver least of all, which cuts an
        // address on a colon long before this is asked anything.
        //
        // Widening cannot make a wrong section be chosen. Two titles that
        // collide once their punctuation is gone are an ambiguity, and
        // [`Sections::section_of`] refuses an ambiguity rather than picking
        // from it.
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// The names a schema gives to the parts of an address.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sections {
    /// The slug of a named section → what the schema calls it in Hebrew.
    /// `orach_chayim` → `אורח חיים`.
    titles: BTreeMap<String, String>,
    /// The slug path of a section → the level names beneath it, outermost
    /// first. `["orach_chayim"]` → `["סימן", "סעיף"]`.
    ///
    /// Keyed by the whole path and not by the last level, because two chalakim
    /// of one sefer need not be addressed the same way — the Tur's הקדמה is
    /// counted in פסקאות and its body in סימנים.
    levels: BTreeMap<Vec<String>, Vec<String>>,
}

impl Sections {
    /// Read a work's schema, if it has one and it can be read.
    ///
    /// `None` for a work with no schema — every Otzaria work, and anything a
    /// reader dropped on the window — which is *the corpus does not say* and
    /// leaves the address printed by number, exactly as it is now.
    ///
    /// A schema that will not parse is also `None` rather than an error: this
    /// decorates an address, and a margin that says less is not a reason to
    /// refuse to open a sefer.
    #[must_use]
    pub fn read(schema: &Path) -> Option<Self> {
        Self::of(&std::fs::read_to_string(schema).ok()?)
    }

    /// The schema a catalogue entry names, read from a corpus at `root`.
    ///
    /// **Told where the corpus is**, because `Work::schema` is a relative path
    /// and only `girsa-import` ever read one before — run from the repository
    /// root, where the relative path happens to work. Resolved against the
    /// process's own directory it finds nothing, silently.
    ///
    /// # Two tries, because the path is relative to the corpus's *parent*
    ///
    /// The catalogue records `corpus\sefaria\schemas\Tur.json` — it already
    /// carries the corpus directory's own name. Joined to the corpus root that
    /// is `…/corpus/corpus/sefaria/…`, which is nothing, and the first version
    /// of this shipped that way and looked exactly like no fix at all.
    ///
    /// So the root's parent is tried first, and the root itself after it. Two
    /// tries rather than stripping a leading component by name: a reader whose
    /// corpus directory is called something else would have that stripping
    /// silently do nothing, and *try both and take what exists* cannot be wrong
    /// about a file that is there.
    #[must_use]
    pub fn beside(root: &Path, schema: Option<&Path>) -> Self {
        let Some(schema) = schema else {
            return Self::default();
        };
        // An absolute path is taken as it stands: a reader's own corpus, or a
        // test that wrote a schema in a temporary directory.
        let tries: Vec<std::path::PathBuf> = if schema.is_absolute() {
            vec![schema.to_path_buf()]
        } else {
            root.parent()
                .map(|up| up.join(schema))
                .into_iter()
                .chain(std::iter::once(root.join(schema)))
                .collect()
        };
        tries
            .iter()
            .find_map(|path| Self::read(path))
            .unwrap_or_default()
    }

    /// The same, for a work named by slug on the corpus at `root`.
    ///
    /// For the callers that have a slug and no open sefer — the citation bar
    /// is the one that matters, because it is where a person types a mekor.
    #[must_use]
    pub fn of_work(root: &Path, slug: &str) -> Self {
        let path = crate::import::work_dir(root, slug).join("work.json");
        let Ok(body) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(work) = serde_json::from_str::<crate::work::Work>(&body) else {
            return Self::default();
        };
        Self::beside(root, work.schema.as_deref())
    }

    /// The same, from the schema's text.
    ///
    /// Separated from the file read so that a test can state a schema and get
    /// an answer without a temporary directory — two of them raced over one and
    /// deleted it out from under each other, which is a test failure about
    /// nothing.
    #[must_use]
    pub fn of(body: &str) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_str(body).ok()?;
        let root = json.get("schema").unwrap_or(&json);
        let mut out = Self::default();
        out.walk(root, &[]);
        (!out.titles.is_empty() || !out.levels.is_empty()).then_some(out)
    }

    /// Gather one node and everything under it.
    ///
    /// `at` is the slug path of the section this node stands in — empty at the
    /// root, because the root is the work and the work is not a level of its
    /// own address.
    fn walk(&mut self, node: &serde_json::Value, at: &[String]) {
        if let Some(names) = node.get("heSectionNames").and_then(|v| v.as_array()) {
            let names: Vec<String> = names
                .iter()
                .filter_map(|n| n.as_str().map(str::to_string))
                .collect();
            if !names.is_empty() {
                // A `default` node carries the level names of the section it
                // stands in rather than of a section of its own — that is what
                // makes it the default — so it does not deepen the path, and
                // `walk` has already been given the parent's path for it.
                self.levels.entry(at.to_vec()).or_insert(names);
            }
        }
        let Some(children) = node.get("nodes").and_then(|v| v.as_array()) else {
            return;
        };
        for child in children {
            // The **title**, slugged the way the importer slugs it. See the
            // module note: the key is a different string and misses.
            let Some(title) = child.get("title").and_then(|v| v.as_str()) else {
                continue;
            };
            let slug = crate::work::section_label_of(title);
            // A `default` node is not a place in the address. Sefaria writes it
            // for the unnamed body of a section that also has named siblings —
            // the Tur's simanim beside its הקדמה — and an address goes straight
            // from the chelek to the siman.
            let here: Vec<String> = if slug == "default" || slug.is_empty() {
                at.to_vec()
            } else {
                if let Some(said) = child.get("heTitle").and_then(|v| v.as_str()) {
                    if !said.is_empty() {
                        self.titles.entry(slug.clone()).or_insert(said.to_string());
                    }
                }
                at.iter().cloned().chain(std::iter::once(slug)).collect()
            };
            self.walk(child, &here);
        }
    }

    /// What the schema calls this level, where the level names a section.
    ///
    /// `None` for a level that is a number, and for a name the schema does not
    /// carry — both of which stay exactly as the address spells them, because
    /// a guessed name on a mekor is worse than a slug somebody can look up.
    #[must_use]
    pub fn titled(&self, level: &str) -> Option<&str> {
        self.titles.get(level).map(String::as_str)
    }

    /// The level names for an address, outermost first — `["סימן", "סעיף"]`.
    ///
    /// Given the whole address path. The named sections at the front of it are
    /// walked off one at a time and the deepest section that says anything
    /// answers, so `["orach_chayim", "1", "1"]` gets Orach Chayim's names and a
    /// path into a section the schema is silent about falls back to the work's
    /// own.
    #[must_use]
    pub fn levels(&self, path: &[String]) -> &[String] {
        let named: Vec<String> = path
            .iter()
            .take_while(|level| self.titles.contains_key(*level))
            .cloned()
            .collect();
        for depth in (0..=named.len()).rev() {
            if let Some(names) = self.levels.get(&named[..depth]) {
                return names;
            }
        }
        &[]
    }

    /// How many levels at the front of this path name sections rather than
    /// places — `["orach_chayim", "1", "1"]` is one.
    ///
    /// What a formatter needs in order to print the section by name and count
    /// the rest as an address.
    #[must_use]
    pub fn named(&self, path: &[String]) -> usize {
        path.iter()
            .take_while(|level| self.titles.contains_key(*level))
            .count()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.titles.is_empty() && self.levels.is_empty()
    }

    /// The way back: the slug of the section this schema calls `said`.
    ///
    /// Named `section_of` and not `slug_of` because
    /// [`crate::work::slug_of`] is the one that spells a **work**, and this
    /// repository has a test that there is only ever one of those.
    ///
    /// [`Sections::titled`] is what a margin needs — a slug out of the corpus,
    /// printed in Hebrew. This is what a **person typing a mekor** needs, which
    /// is the same pairing read the other way round, and it is the half that
    /// was missing. `אורח חיים` and `או"ח` and `אוח` all answer
    /// `orach_chayim`; the corpus's own `orach_chayim` answers `None`, because
    /// it is not a name the schema said and it is already where it needs to be.
    ///
    /// # Ambiguity is refused, not narrowed
    ///
    /// BUILDER rule 6. If one schema calls two different sections the same
    /// thing, this answers `None` and the address stays as it was written —
    /// which fails to open, visibly, instead of opening the wrong chelek.
    ///
    /// # The one spelling that does not reach here
    ///
    /// `יו"ד` never arrives as a name: it is also a valid Hebrew numeral and
    /// the resolver hands back `20`. That is caught one level up, in
    /// [`Sections::read_as_a_number`], which is where the guard on it is
    /// written down.
    #[must_use]
    pub fn section_of(&self, said: &str) -> Option<&str> {
        let wanted = spelling(said);
        if wanted.is_empty() {
            return None;
        }
        let wanted = ABBREVIATED
            .iter()
            .find(|(short, _)| *short == wanted)
            .map_or(wanted, |(_, full)| spelling(full));
        let mut found: Option<&str> = None;
        for (slug, title) in &self.titles {
            if spelling(title) != wanted {
                continue;
            }
            match found {
                Some(first) if first != slug => return None,
                _ => found = Some(slug),
            }
        }
        found
    }

    /// The same address, with the sections a person named turned into the slugs
    /// the corpus addresses them by.
    ///
    /// `אורח חיים:1` on the Tur becomes `orach_chayim:1`, which is what its
    /// segment ids actually say. Everything else is returned untouched: a work
    /// whose schema names no sections, an address already written in slugs, and
    /// every level from the first one that is not a section name onward — a
    /// siman called `סימן` is a number, and only the **front** of an address
    /// can name a chelek.
    ///
    /// # One level can hold several sections
    ///
    /// Measured, and it is why this is not a `map` over the levels. The
    /// resolver is given `טור אורח חיים הקדמה ד'` and hands back two levels,
    /// `אורח חיים הקדמה` and `4` — the two section names arrive glued, because
    /// nothing between the title and the first number told it where one ends.
    /// So each named level is split back into as many sections as it holds,
    /// longest match first, and a level that does not split cleanly all the way
    /// through is left exactly as it was rather than half-translated.
    #[must_use]
    /// # A name and its number can arrive as two levels
    ///
    /// The other way the resolver can cut a section name, and the one that
    /// costs the most. `עין זוכר מערכת א` comes back as **three** levels —
    /// `מערכת`, `1`, `1` — because `מערכת` is an ordinary level word and `א` is
    /// an ordinary number, so pairing them is the right reading of that
    /// sentence everywhere except in a sefer whose schema calls a section
    /// `מערכת א`. `front` is given `מערכת`, which names nothing, and the
    /// address stops being translated at its first level.
    ///
    /// So a level that names nothing is offered one more chance with the number
    /// after it, and with the ones after that — see [`Sections::joined`].
    pub fn slugged(&self, address: &Address) -> Address {
        if self.titles.is_empty() {
            return address.clone();
        }
        let levels = address.levels();
        let mut out: Vec<Level> = Vec::with_capacity(address.depth());
        let mut naming = true;
        let mut at = 0;
        while at < levels.len() {
            let level = &levels[at];
            let mut took = 1;
            let sections = if !naming {
                None
            } else if let Some(slugs) = self.front(level.as_str()) {
                Some(slugs)
            } else if let Some((slugs, across)) = self.joined(&levels[at..]) {
                took = across;
                Some(slugs)
            } else if out.is_empty() {
                // Only at the very front, where a chelek stands.
                self.read_as_a_number(level.as_str())
                    .map(|slug| vec![slug.to_string()])
            } else {
                None
            };
            match sections {
                Some(slugs) => out.extend(slugs.into_iter().map(Level::canonical)),
                None => {
                    naming = false;
                    out.push(level.clone());
                }
            }
            at += took;
        }
        Address::new(out)
    }

    /// The sections a **run** of levels names between them, and how many of
    /// them it took.
    ///
    /// # Why a name arrives in pieces
    ///
    /// The resolver has no schema. It reads a Hebrew word it does not know as a
    /// name and a Hebrew word that is a numeral as a number, which is the right
    /// reading of nearly every sentence and the wrong one inside a section's
    /// title. Measured on the shelf, three shapes of the same thing:
    ///
    /// | typed | arrives as | is really |
    /// |---|---|---|
    /// | `עין זוכר מערכת א א` | `מערכת:1:1` | `מערכת א`, se'if א |
    /// | `אהבת יהונתן הפטרת נח א` | `הפטרת:58:1` | `הפטרת נח` — נח is 58 |
    /// | `אהבת יהונתן הפטרת אחרון של פסח א` | `הפטרת אחרון:330:פסח:1` | `של` is 330 |
    ///
    /// So each number in the run is spelled back out with
    /// [`girsa_ref::numerals::to_hebrew`] — the exact inverse of what read it —
    /// the run is joined with spaces, and [`Sections::front`] is asked whether
    /// that names sections. **Longest first**, so a name is not cut short by a
    /// shorter one that also matches, and never fewer than two levels, because
    /// one level is what `front` was already asked.
    ///
    /// `None` when no run names anything, which leaves the address exactly as
    /// it was written — the same answer as before this existed.
    ///
    /// # The cap
    ///
    /// [`SPREAD`] levels. A section title of more than that many words exists
    /// (`ימים ראשונים של סוכות`) and is covered; the cap is there so that a
    /// deep address does not try every suffix of itself against every title,
    /// and because a run long enough to swallow the segment number at the end
    /// of an address is a run that has stopped being a name.
    fn joined(&self, levels: &[Level]) -> Option<(Vec<String>, usize)> {
        let most = levels.len().min(SPREAD);
        (2..=most).rev().find_map(|take| {
            let said = levels[..take]
                .iter()
                .map(|level| spelled_out(level.as_str()))
                .collect::<Vec<_>>()
                .join(" ");
            self.front(&said).map(|slugs| (slugs, take))
        })
    }

    /// Whether this schema uses `said` as the name of a **level** — a פרק, a
    /// סימן, a פסקה — anywhere in the work.
    ///
    /// The other half of the question [`Sections::section_of`] answers, and the
    /// two together are what let a caller choose between the resolver's two
    /// readings of a level word without guessing at either.
    ///
    /// `עטרת זקנים שער א'`: the resolver's ordinary reading takes `שער` for a
    /// label and hands back `1`, which is a real perek of that sefer and not
    /// the place anybody asked for. Its second reading — `girsa_ref::resolve::
    /// resolve_labels_as_names` — keeps the word. Which is right is the
    /// schema's to say, and it says it twice over: `שער` **is** the title of a
    /// section here, and it is **not** a level name here (the levels are פרק
    /// and פסקה). Both facts, or the ordinary reading stands.
    #[must_use]
    pub fn is_level_name(&self, said: &str) -> bool {
        let wanted = spelling(said);
        !wanted.is_empty()
            && self
                .levels
                .values()
                .flatten()
                .any(|name| spelling(name) == wanted)
    }

    /// Whether this schema calls more than one section `said`.
    ///
    /// [`Sections::section_of`] answers `None` for a name it does not know and
    /// for a name it knows twice, which is right for a caller that has to
    /// decide and wrong for one that has to **report**: those are a gap in the
    /// schema and a refusal this repository makes on purpose, and counting them
    /// together makes a measurement say a working guard is a defect.
    ///
    /// `examples/measure-branch-citations.rs` is the caller. The Chafetz
    /// Chaim's schema names two different sections `הקדמה`, so
    /// `חפץ חיים הקדמה א` is an address that names two places and Girsa opens
    /// neither — BUILDER rule 6, working.
    #[must_use]
    pub fn ambiguous(&self, said: &str) -> bool {
        let wanted = spelling(said);
        !wanted.is_empty()
            && self
                .titles
                .values()
                .filter(|title| spelling(title) == wanted)
                .count()
                > 1
    }

    /// The section a number at the front of an address is really the name
    /// of — see [`read_as`].
    ///
    /// # Two guards, and the case this gets wrong
    ///
    /// It fires only when the schema **counts nothing at the work's top
    /// level** — every address into the Tur begins with a chelek, so a bare
    /// number there is not a place in it — and only when exactly one section
    /// reads as that number. Two would be an ambiguity, and BUILDER rule 6
    /// says an ambiguity is shown and never picked from.
    ///
    /// What it gets wrong, stated plainly: `ערוך השולחן כ' א'` now opens Yoreh
    /// De'ah siman א' instead of failing. That citation names no chelek and so
    /// names no place — it fails today too — and the trade is that
    /// `טור יו"ד סימן א'`, which is how the second-most-cited code on the shelf
    /// is actually written, lands. The wrong reading also announces itself: the
    /// margin of every line it opens says `יורה דעה`, which is not what was
    /// typed.
    fn read_as_a_number(&self, said: &str) -> Option<&str> {
        let n: u32 = said.parse().ok()?;
        if !self.levels(&[]).is_empty() {
            return None;
        }
        let mut found: Option<&str> = None;
        for (slug, title) in &self.titles {
            if !read_as(title).any(|reading| reading == n) {
                continue;
            }
            match found {
                Some(first) if first != slug => return None,
                _ => found = Some(slug.as_str()),
            }
        }
        found
    }

    /// One level split into the sections it names, or `None` if any of it
    /// names nothing.
    fn front(&self, said: &str) -> Option<Vec<String>> {
        let words: Vec<&str> = said.split_whitespace().collect();
        if words.is_empty() {
            return None;
        }
        let mut out = Vec::new();
        let mut at = 0;
        while at < words.len() {
            let took = (1..=words.len() - at).rev().find_map(|take| {
                let slug = self.section_of(&words[at..at + take].join(" "))?;
                out.push(slug.to_string());
                Some(take)
            })?;
            at += took;
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn tur() -> Sections {
        // The Tur's schema, in the shape Sefaria writes it: a branch with four
        // chalakim, each holding a named הקדמה and an unnamed body.
        let json = serde_json::json!({
            "schema": {
                "title": "Tur",
                "heTitle": "טור",
                "nodes": [
                    {
                        "key": "Orach Chaim",
                        "title": "Orach Chayim",
                        "heTitle": "אורח חיים",
                        "nodes": [
                            {
                                "title": "Introduction",
                                "heTitle": "הקדמה",
                                "heSectionNames": ["פסקה"],
                            },
                            {
                                "title": "default",
                                "heTitle": "",
                                "heSectionNames": ["סימן", "סעיף"],
                            },
                        ],
                    },
                    {
                        "key": "Yoreh Deah",
                        "title": "Yoreh De'ah",
                        "heTitle": "יורה דעה",
                        "nodes": [{
                            "title": "default",
                            "heTitle": "",
                            "heSectionNames": ["סימן", "סעיף"],
                        }],
                    },
                ],
            }
        });
        Sections::of(&json.to_string()).expect("the schema reads")
    }

    #[test]
    fn a_named_section_is_matched_on_its_title_and_not_on_its_key() {
        // Measured, and it is the whole reason this is not a one-liner: the
        // Tur's first node is `key: "Orach Chaim"` and `title: "Orach Chayim"`,
        // and its segments are addressed `orach_chayim`. Slugging the key gives
        // `orach_chaim`, which matches nothing in the sefer this was written
        // for.
        let tur = tur();
        assert_eq!(tur.titled("orach_chayim"), Some("אורח חיים"));
        assert_eq!(
            tur.titled("orach_chaim"),
            None,
            "that is the key, not the title"
        );
        assert_eq!(tur.titled("yoreh_deah"), Some("יורה דעה"));
        // Apostrophes are dropped by the slugger, which is how `Yoreh De'ah`
        // and `Yoreh Deah` arrive at one address.
        assert_eq!(tur.titled("introduction"), Some("הקדמה"));
    }

    #[test]
    fn a_branch_schema_carries_its_level_names_on_the_leaves() {
        // The defect, stated: `Work::he_sections` reads `heSectionNames` off the
        // root, and a branch schema has none there. `girsa-cite`'s own note
        // counts 1,101 of Sefaria's 6,595 schemas in that shape, and the Tur and
        // the Arukh HaShulchan are two of them.
        let tur = tur();
        let siman = ["orach_chayim".to_string(), "1".to_string(), "1".to_string()];
        assert_eq!(tur.levels(&siman), ["סימן", "סעיף"]);
        // …and a chelek is allowed to disagree with its neighbour about how it
        // is counted, which is why this is keyed by the whole path: the Tur's
        // הקדמה is in פסקאות and its body is in סימנים.
        let hakdama = [
            "orach_chayim".to_string(),
            "introduction".to_string(),
            "4".to_string(),
        ];
        assert_eq!(tur.levels(&hakdama), ["פסקה"]);
    }

    #[test]
    fn the_named_levels_at_the_front_are_counted_and_the_rest_is_the_address() {
        let tur = tur();
        assert_eq!(
            tur.named(&["orach_chayim".to_string(), "1".to_string(), "1".to_string()]),
            1
        );
        assert_eq!(
            tur.named(&[
                "orach_chayim".to_string(),
                "introduction".to_string(),
                "4".to_string()
            ]),
            2
        );
        // A flat work — the Shulchan Arukh is four separate works — names no
        // sections at all, and every level of its address is a number.
        assert_eq!(tur.named(&["1".to_string(), "1".to_string()]), 0);
    }

    #[test]
    fn a_chelek_a_person_names_is_the_slug_the_corpus_addresses_it_by() {
        let tur = tur();
        assert_eq!(tur.section_of("אורח חיים"), Some("orach_chayim"));
        assert_eq!(tur.section_of("יורה דעה"), Some("yoreh_deah"));
        assert_eq!(tur.section_of("הקדמה"), Some("introduction"));
        // Written short, with gershayim, with the ASCII stand-in, and with
        // neither. A reader types all four and means one chelek.
        assert_eq!(tur.section_of("או\"ח"), Some("orach_chayim"));
        assert_eq!(tur.section_of("או״ח"), Some("orach_chayim"));
        assert_eq!(tur.section_of("אוח"), Some("orach_chayim"));
        // Not a section of this sefer, and not a name at all: both stay put
        // rather than being matched to something near them.
        assert_eq!(tur.section_of("חושן משפט"), None);
        assert_eq!(tur.section_of("1"), None);
        // And the corpus's own spelling is not a name the schema said. It is
        // already an address, and translating it again would be a loop.
        assert_eq!(tur.section_of("orach_chayim"), None);
    }

    #[test]
    fn an_address_a_person_typed_is_turned_into_the_one_the_segments_carry() {
        let tur = tur();
        let slugged = |a: &str| tur.slugged(&Address::parse(a).unwrap()).to_string();
        assert_eq!(slugged("אורח חיים:1"), "orach_chayim:1");
        assert_eq!(slugged("או\"ח:1:2"), "orach_chayim:1:2");
        // The two section names arrive glued into one level, because nothing
        // between the title and the first number said where the chelek ends.
        assert_eq!(slugged("אורח חיים הקדמה:4"), "orach_chayim:introduction:4");
        // Only the front of an address names sections. A se'if that happens to
        // spell a chelek is still a se'if.
        assert_eq!(slugged("1:1"), "1:1");
        // Already canonical: untouched, and not translated twice.
        assert_eq!(slugged("orach_chayim:1"), "orach_chayim:1");
        // Half a name is not half an address. `אורח נסתר` names nothing, so
        // the level stays exactly as it was written and fails to open, which
        // is visible — rather than opening Orach Chayim, which is not.
        assert_eq!(slugged("אורח נסתר:1"), "אורח נסתר:1");
    }

    #[test]
    fn the_resolver_no_longer_reads_yod_vav_dalet_as_twenty() {
        // **This test said so.** It was written to assert the premise of
        // `READS_AS_A_NUMBER` — that `יו"ד` arrives as twenty — and its own
        // note ended *"if a later girsa-ref stops doing this, the workaround
        // stops being needed and this is the test that says so."*
        //
        // `girsa-ref` stopped doing it. A numeral is now required to be the
        // canonical spelling of its own value, and twenty is written `כ'`, so
        // `יו"ד` is a name again and reaches the abbreviation table like the
        // other three chalakim. Inverted rather than deleted: the premise is
        // still worth asserting, it is simply the opposite premise now.
        let mut lex = girsa_ref::Lexicon::default();
        lex.add(
            girsa_ref::Work {
                slug: "tur".into(),
                he_title: "טור".into(),
                en_title: "Tur".into(),
            },
            &["טור"],
        );
        let girsa_ref::Resolution::Exact(got) = girsa_ref::resolve(&lex, "טור יו\"ד סימן א")
        else {
            panic!("the title resolves; it is the address that is the problem");
        };
        assert_eq!(got.from().to_string(), "יו\"ד:1");
        // And it now behaves exactly like the other three chalakim, which were
        // never numerals and never needed any of this.
        for short in ["או\"ח", "אה\"ע", "חו\"מ", "יו\"ד"] {
            let girsa_ref::Resolution::Exact(got) =
                girsa_ref::resolve(&lex, &format!("טור {short} סימן א"))
            else {
                panic!("{short} resolves");
            };
            assert_eq!(got.from().levels()[0].as_str(), short);
        }
    }

    #[test]
    fn the_tur_no_longer_needs_a_chelek_put_back_at_all() {
        let tur = tur();
        let slugged = |a: &str| tur.slugged(&Address::parse(a).unwrap()).to_string();
        // **The cost of `read_as_a_number` was paid here, and is not paid
        // anymore.** `טור יו"ד סימן א'` used to arrive as `20:1`, so this
        // asserted `yoreh_deah:1` — and the price of that repair was
        // `ערוך השולחן כ' א'`, where the reader really did mean twenty,
        // opening Yoreh De'ah instead of failing.
        //
        // Nothing reads as twenty now. Not one of the Tur's four chalakim is a
        // canonical numeral, so a bare `20` at the front is a number nobody
        // claims, and it is left exactly as written — which fails to open,
        // visibly, instead of opening the wrong chelek.
        assert_eq!(slugged("20:1"), "20:1");
        // The repair itself is still needed and still tested, on the names
        // that genuinely are spelled the way their own number is spelled —
        // see `a_parashah_whose_name_is_also_a_number_is_put_back_too`.
        //
        // Only at the front. A siman twenty is a siman twenty.
        assert_eq!(slugged("אורח חיים:20:1"), "orach_chayim:20:1");
        // And a number no section reads as is left alone.
        assert_eq!(slugged("15:1"), "15:1");
    }

    #[test]
    fn a_parashah_whose_name_is_also_a_number_is_put_back_too() {
        // The long tail, and the chiluk inside it that the canonical-numeral
        // rule drew.
        //
        // A commentary on the Chumash holds a section per sidra, and a sidra
        // whose letters descend used to be eaten whole: `אגרא דכלה בא א`
        // arrived as `3:1` and landed nowhere, along with נח, צו, שלח and the
        // rest. **Most of them are not eaten anymore.** `בא` is 2 + 1 = 3, and
        // three is written `ג'` — so `בא` is not a numeral and never arrives
        // as one.
        //
        // `נח` is the half that survives, and it survives for a reason no rule
        // can reach: 50 + 8 is 58, and 58 *is* written `נ"ח`. The word and the
        // number are the same string. That is what `read_as_a_number` is still
        // worth 226 chalakim for, measured over the shelf.
        let chumash = Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "Agra DeKala", "heTitle": "אגרא דכלה",
                    "nodes": [
                        {"title": "Bo", "heTitle": "בא", "heSectionNames": ["פסקה"]},
                        {"title": "Noach", "heTitle": "נח", "heSectionNames": ["פסקה"]},
                        {"title": "Chayei Sarah", "heTitle": "חיי שרה",
                         "heSectionNames": ["פסקה"]},
                    ],
                }
            })
            .to_string(),
        )
        .unwrap();
        let slugged = |a: &str| chumash.slugged(&Address::parse(a).unwrap()).to_string();
        assert_eq!(slugged("58:1"), "noach:1");
        // `בא` reaches the schema as a name now, so a bare three is a three
        // and stays one. The repair is not asked to cover what is no longer
        // broken.
        assert_eq!(slugged("בא:1"), "bo:1");
        assert_eq!(slugged("3:1"), "3:1");
        // A sidra whose name is not a numeral arrives as a name and needs none
        // of this.
        assert_eq!(slugged("חיי שרה:1"), "chayei_sarah:1");
        // And a number nothing reads as is a number.
        assert_eq!(slugged("7:1"), "7:1");
    }

    #[test]
    fn a_section_named_by_a_label_and_a_letter_is_matched_on_the_letter() {
        // `אברבנאל על מורה נבוכים חלק א א` — the resolver takes `חלק` for a
        // level label and reads the `א` after it as one, so what arrives is
        // `1:1` and the section is called `חלק א`.
        let three = Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "A", "heTitle": "א",
                    "nodes": [
                        {"title": "Part One", "heTitle": "חלק א", "heSectionNames": ["פרק"]},
                        {"title": "Part Two", "heTitle": "חלק ב", "heSectionNames": ["פרק"]},
                    ],
                }
            })
            .to_string(),
        )
        .unwrap();
        assert_eq!(
            three.slugged(&Address::parse("2:5").unwrap()).to_string(),
            "part_two:5"
        );
    }

    /// A schema in the shape the `מערכת` seforim are written in: a section
    /// whose name is a word the resolver has never heard of, and a number.
    fn maarachot() -> Sections {
        Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "Ayin Zokher", "heTitle": "עין זוכר",
                    "nodes": [
                        {"title": "Maarechet Alef", "heTitle": "מערכת א", "heSectionNames": ["סעיף"]},
                        {"title": "Maarechet Bet", "heTitle": "מערכת ב", "heSectionNames": ["סעיף"]},
                    ],
                }
            })
            .to_string(),
        )
        .unwrap()
    }

    #[test]
    fn a_name_and_the_number_after_it_are_one_section() {
        // `עין זוכר מערכת א א` arrives as three levels, not two: `מערכת` is a
        // word the resolver does not know, so it survives as a name — and then
        // `א` after it is read as the number it also is.
        let sections = maarachot();
        let slugged = |a: &str| sections.slugged(&Address::parse(a).unwrap()).to_string();
        assert_eq!(slugged("מערכת:1:1"), "maarechet_alef:1");
        assert_eq!(slugged("מערכת:2:5"), "maarechet_bet:5");
        // The run consumes both levels and the rest of the address is
        // untouched — a section, then the se'if in it.
        assert_eq!(slugged("מערכת:2"), "maarechet_bet");
        // A number the schema does not have stays exactly as it was written,
        // and so does the word: half a translation is worse than none.
        assert_eq!(slugged("מערכת:9:1"), "מערכת:9:1");
        assert_eq!(slugged("שער:1:1"), "שער:1:1");
    }

    #[test]
    fn a_name_cut_into_four_levels_is_put_back_together() {
        // `אהבת יהונתן הפטרת אחרון של פסח א` arrives as
        // `הפטרת אחרון:330:פסח:1`, because **`של` is 330** — ש is 300 and ל is
        // 30, and the resolver has no way to know it is reading the middle of
        // a name. Two words of the title were on one level, a third had become
        // a number, and the fourth was a level of its own.
        //
        // This is the shape that says the guard has to work on a *run* of
        // levels rather than on a pair: nothing about `הפטרת אחרון` and nothing
        // about `330` is enough on its own.
        let sections = Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "Ahavat Yehonatan", "heTitle": "אהבת יהונתן",
                    "nodes": [
                        {"title": "Haftarah of the Last Day of Pesach",
                         "heTitle": "הפטרת אחרון של פסח", "heSectionNames": ["פסקה"]},
                        {"title": "Haftarah of Noach",
                         "heTitle": "הפטרת נח", "heSectionNames": ["פסקה"]},
                    ],
                }
            })
            .to_string(),
        )
        .unwrap();
        let slugged = |a: &str| sections.slugged(&Address::parse(a).unwrap()).to_string();
        assert_eq!(
            slugged("הפטרת אחרון:330:פסח:1"),
            "haftarah_of_the_last_day_of_pesach:1"
        );
        // And the two-level case of the same sefer: `נח` is how 58 is written,
        // so the parsha's name arrives as a number and is spelled back.
        assert_eq!(slugged("הפטרת:58:1"), "haftarah_of_noach:1");
        // The longest run wins. Stopping at `הפטרת נח` would leave `של פסח`
        // as an address into a section that has no such place.
        assert_eq!(
            slugged("הפטרת אחרון:330:פסח"),
            "haftarah_of_the_last_day_of_pesach"
        );
    }

    #[test]
    fn a_title_with_punctuation_in_it_is_matched_on_its_letters() {
        // Sefaria writes section titles with punctuation inside the name:
        // `חלק א': בית נתיבות` has a colon in it, and the colon is what an
        // address is cut on — so the two halves arrive as two levels and
        // neither is the title. Compared letter by letter they are.
        //
        // 253 chalakim of 7,627, measured, which is what a colon costs.
        let sections = Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "Avodat HaKodesh", "heTitle": "עבודת הקדש",
                    "nodes": [
                        {"title": "Part One", "heTitle": "חלק א': בית נתיבות",
                         "heSectionNames": ["פסקה"]},
                        {"title": "Part Two", "heTitle": "חלק ב': בית מועד",
                         "heSectionNames": ["פסקה"]},
                    ],
                }
            })
            .to_string(),
        )
        .unwrap();
        assert_eq!(
            sections.section_of("חלק א' בית נתיבות"),
            Some("part_one"),
            "the colon is not a letter of the name"
        );
        assert_eq!(sections.section_of("חלק א בית נתיבות"), Some("part_one"));
        // The address, cut on the colon and put back together across levels.
        assert_eq!(
            sections
                .slugged(&Address::parse("חלק א':בית נתיבות:1").unwrap())
                .to_string(),
            "part_one:1"
        );
        // And a name that is genuinely another section is still another
        // section: widening what counts as the same spelling must not make two
        // titles one.
        assert_eq!(sections.section_of("חלק ב' בית מועד"), Some("part_two"));
    }

    #[test]
    fn a_name_two_sections_share_is_ambiguous_rather_than_absent() {
        // The Chafetz Chaim's schema really does call two different sections
        // `הקדמה`. `section_of` refuses it — rule 6 — and a measurement that
        // could not tell that refusal apart from a name nobody knows would
        // report a working guard as 192 defects.
        let both = Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "Chafetz Chaim", "heTitle": "חפץ חיים",
                    "nodes": [
                        {"title": "Preface", "heTitle": "הקדמה", "heSectionNames": ["פסקה"]},
                        {"title": "Opening Comments", "heTitle": "הקדמה", "heSectionNames": ["פסקה"]},
                    ],
                }
            })
            .to_string(),
        )
        .unwrap();
        assert!(both.ambiguous("הקדמה"));
        assert_eq!(both.section_of("הקדמה"), None);
        // Not ambiguous: a name nobody has, and a name exactly one section has.
        assert!(!both.ambiguous("שער"));
        assert!(!maarachot().ambiguous("מערכת א"));
        assert_eq!(maarachot().section_of("מערכת א"), Some("maarechet_alef"));
    }

    #[test]
    fn two_sections_reading_as_one_number_are_refused_rather_than_picked() {
        // BUILDER rule 6, on the numeral path. Constructed, because nothing on
        // the shelf collides today — which is exactly why the refusal has to
        // be asserted rather than observed.
        //
        // **This fixture had to be rebuilt, and the reason is worth reading.**
        // It used to be `בא` against a section a schema simply calls `ג`, both
        // reading as three. The canonical-numeral rule stopped `בא` being a
        // numeral, so the collision evaporated and the guard sailed through
        // testing nothing — passing for the one reason a guard must never
        // pass, which is that it was never asked.
        //
        // A collision now needs two names that are each a canonical numeral,
        // and canonical spellings are unique — so it cannot come from two
        // whole titles. It comes from the *three readings* `read_as` tries:
        // `נח` reads as 58 as a whole title, and `שער נח` reads as 58 through
        // its last word, after the resolver has taken `שער` for a level label.
        // Two different sections, one number, by two different paths.
        let both = Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "S", "heTitle": "ס",
                    "nodes": [
                        {"title": "Noach", "heTitle": "נח", "heSectionNames": ["פסקה"]},
                        {"title": "Gate of Noach", "heTitle": "שער נח",
                         "heSectionNames": ["פסקה"]},
                    ],
                }
            })
            .to_string(),
        )
        .unwrap();
        assert_eq!(
            both.slugged(&Address::parse("58:1").unwrap()).to_string(),
            "58:1"
        );
    }

    #[test]
    fn a_work_that_counts_its_own_top_level_keeps_every_number_it_is_given() {
        // The guard. A schema that says how it counts at the root is a work
        // where a leading number is an address, and nothing may touch it.
        let counted = Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "S", "heTitle": "ס",
                    "heSectionNames": ["סימן", "סעיף"],
                    "nodes": [{"title": "Yoreh Deah", "heTitle": "יורה דעה",
                               "heSectionNames": ["סימן"]}],
                }
            })
            .to_string(),
        )
        .unwrap();
        assert_eq!(
            counted
                .slugged(&Address::parse("20:1").unwrap())
                .to_string(),
            "20:1"
        );
    }

    #[test]
    fn a_schema_that_calls_two_sections_one_name_refuses_rather_than_picks() {
        // BUILDER rule 6. Nothing on the shelf does this today; the assertion
        // is that it would be refused if something did.
        let both = Sections::of(
            &serde_json::json!({
                "schema": {
                    "title": "S", "heTitle": "ס",
                    "nodes": [
                        {"title": "First", "heTitle": "חלק", "heSectionNames": ["סימן"]},
                        {"title": "Second", "heTitle": "חלק", "heSectionNames": ["סימן"]},
                    ],
                }
            })
            .to_string(),
        )
        .unwrap();
        assert_eq!(both.section_of("חלק"), None);
    }

    #[test]
    fn a_work_with_no_schema_says_nothing_rather_than_guessing() {
        assert_eq!(Sections::read(Path::new("no/such/schema.json")), None);
        let empty = Sections::default();
        assert!(empty.is_empty());
        assert_eq!(empty.titled("orach_chayim"), None);
        assert!(empty.levels(&["1".to_string()]).is_empty());
    }
}
