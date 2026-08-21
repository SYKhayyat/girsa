//! The catalogue: every work in the union, and which corpus its text comes from.
//!
//! spec.md §2.3 measured the overlap between the two corpora — 5,637 shared,
//! 978 Otzaria-only, 961 Sefaria-only, ~7,576 in the union — and §2.3b turned
//! that into a rule: **Sefaria spine, Otzaria fill**. For a work both have,
//! Sefaria supplies the text *as well as* the structure; Otzaria supplies the
//! 978 it alone has, which are disproportionately the acharonim you actually
//! need at 11pm.
//!
//! This module is where a work is assigned to one source and one only. §2.3b:
//!
//! > **Never graft** a Sefaria schema onto an Otzaria text file for the same
//! > work. That is a line-by-line alignment problem across thousands of books
//! > and it will eat the schedule.
//!
//! # Matching two corpora that never agreed on a title
//!
//! Sefaria writes `שולחן ערוך, אורח חיים`; Otzaria names a file
//! `שולחן ערוך אורח חיים.txt`. They are the same sefer, and every difference
//! between those two strings — the comma, the gershayim, whichever nikud
//! survived a conversion — is noise. So a work is matched on its
//! [`match_key`]: [`girsa_hebrew::normalize`] with the quote marks and the
//! spaces taken out too, which is the smallest key under which the two
//! corpora's spellings of one sefer collide and two different seforim still
//! do not.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Which corpus a work's text is taken from.
///
/// One per work, forever — see the module note. The field is recorded on disk
/// so that a reader looking at a passage can be told where it came from, which
/// spec.md §13 asks for and which costs nothing now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Text and structure from `gs://sefaria-export`.
    Sefaria,
    /// Text from the Otzaria `.txt` tree, structured from its own headings.
    Otzaria,
    /// Yours. A file you dropped in — spec.md §5, *not an onboarding step, not
    /// a second-class attachment*. It is in the same enum as the other two
    /// because that is what first-class means: everything downstream that asks
    /// a work where its text came from gets an answer of the same kind.
    Mine,
}

crate::spelled!(Source {
    Sefaria => "sefaria",
    Otzaria => "otzaria",
    Mine => "mine",
});

impl Source {}

/// One work in the union, before its text has been read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Work {
    /// The ref slug — `shulchan-arukh/orach-chayim`. What every segment id in
    /// this work begins with, and what the resolver's lexicon maps titles onto.
    pub slug: String,
    pub he_title: String,
    pub en_title: String,
    /// Sefaria's category path, or Otzaria's folder path. Drives the shelf
    /// (spec.md §5) and the search facets (§9.8).
    pub categories: Vec<String>,
    /// Where this sefer stands among the ones beside it — **the order it is
    /// printed in**, not the order its name happens to sort in.
    ///
    /// Bereshis, Shemos, Vayikra; Berakhos, Shabbos, Eruvin. Every list in this
    /// application sorted seforim by [`Work::he_title`], so a shelf of the
    /// Chumash read *במדבר · בראשית · דברים · ויקרא · שמות* — alphabetical
    /// order over a sequence that has had an order for two thousand years.
    ///
    /// Sefaria states it: `order` in the schema, `[1, 1]` for Genesis, `[2, 1]`
    /// for Shabbat. 551 of the 6,595 schemas carry one, and they are exactly the
    /// seforim a reader browses to — Tanach, Shas, Mishnah, the chalakim of the
    /// Shulchan Arukh. A commentary carries none and **inherits its base's**
    /// (see [`Catalogue::build`]), so the Rashi on Bereshis comes before the
    /// Rashi on Shemos for the same reason Bereshis does.
    ///
    /// In **tenths**, so that the one schema in the corpus with a fractional
    /// order (`Rambam_Introduction_to_Seder_Kodashim`, `[40.5]`) sits where it
    /// says it does and two works with the same order still compare equal after
    /// a round trip through the catalogue. Empty is *the corpus does not say*,
    /// which sorts after everything that does.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<i32>,
    pub source: Source,
    /// The file the text is read from.
    pub origin: PathBuf,
    /// Sefaria's schema for this work, where there is one.
    ///
    /// spec.md §2.2 calls the schemas the prize: Otzaria has a line that *says*
    /// `סימן א`, and the schema knows what a siman **is** — that this sefer has
    /// 697 of them containing 4,171 se'ifim, and at which level a commentary
    /// attaches. The importer reads structure from here rather than guessing it
    /// back out of the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<PathBuf>,
    /// What the schema calls the levels of an address, outermost first —
    /// `["סימן", "סעיף"]`.
    ///
    /// The same field the importer reads structure from (`heSectionNames`),
    /// kept because a citation is printed with it: `girsa-cite` writes
    /// `שולחן ערוך, אורח חיים סימן א' סעיף א'` and has no other way to know
    /// that the first number is a siman. Empty where the schema does not say —
    /// 1,101 of Sefaria's 6,595 are branch schemas and no Otzaria work has a
    /// schema at all — and a sefer with no words is cited by number, which is
    /// an ordinary way to write a mekor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub he_sections: Vec<String>,
    /// Author, era and place, where the corpus knows them. spec.md §5 —
    /// these drive the era filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub era: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_date: Option<String>,
    /// Where this text came from and under what terms.
    ///
    /// spec.md §13: *carry each text's source and license in its metadata —
    /// costs nothing now, and it is the only thing preserving the option to
    /// distribute publicly later.*
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,
    /// The seforim this one is a commentary on, as the corpus declares them.
    ///
    /// This is what lets W9 put Rashi in the column beside the Gemara and keep
    /// the two together as you scroll. It is **read from the schema, not
    /// inferred from the title**: Sefaria states `dependence: "Commentary"` and
    /// `base_text_titles` on every one, and guessing it from `X on Y` would
    /// attach `Rashi on Berakhot` to the Yerushalmi masechta of the same name
    /// (BUILDER.md rule 6).
    ///
    /// A list rather than one, because a work can sit on more than one base and
    /// choosing between them is not this layer's business — the pane beside you
    /// already names which sefer it is holding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commentary_on: Vec<BaseText>,
}

impl Work {
    /// Two seforim, in the order they stand on a shelf.
    ///
    /// **The one comparator.** Every list of seforim in this application goes
    /// through it: the bookcase, the mefarshim behind the door, the picker, the
    /// facets. There were five of them and every one was `a.he_title.cmp(&b.he_title)`,
    /// which is why the Chumash came back in alphabetical order in all five
    /// places at once.
    ///
    /// The rule, in order:
    ///
    /// 1. what the corpus says ([`Work::order`]) — and a sefer the corpus has
    ///    ordered comes before one it has not, because an unordered sefer is
    ///    one nobody has placed and it belongs at the end rather than in the
    ///    middle of a sequence;
    /// 2. the title, so the answer is stable and a shelf of unordered seforim
    ///    is still alphabetical rather than arbitrary;
    /// 3. the slug, so the same shelf drawn twice is the same shelf.
    #[must_use]
    pub fn by_order(a: &Self, b: &Self) -> std::cmp::Ordering {
        match (a.order.is_empty(), b.order.is_empty()) {
            (false, true) => return std::cmp::Ordering::Less,
            (true, false) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        a.order
            .cmp(&b.order)
            .then_with(|| a.he_title.cmp(&b.he_title))
            .then_with(|| a.slug.cmp(&b.slug))
    }
}

/// A sefer this work comments on, and how the two line up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseText {
    /// The base work's slug. Only recorded when that work is on the shelf, so
    /// this always names something openable.
    pub slug: String,
    pub mapping: Mapping,
}

/// How a commentary's addresses relate to the addresses of its base text.
///
/// Sefaria's `base_text_mapping`. `Rashi on Berakhot 2a:1:3` is the third
/// comment on `Berakhot 2a:1` — the base text's address with a level added —
/// and *many to one* is the corpus saying so.
///
/// It is recorded rather than acted on. What puts two panes together is the
/// declaration itself plus the addresses the two works actually have; this says
/// how many of one to expect per one of the other, which is a thing to tell a
/// reader and later a thing for W24 to anchor spans with. 3,091 works say many
/// to one, 132 say one to one, and about 2,200 more declare a base text and say
/// nothing about the mapping — which is why nothing depends on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mapping {
    /// Many comments to one segment of the base text — the usual case.
    ManyToOne,
    /// One to one.
    OneToOne,
    /// Declared, in terms this does not model — or not declared at all.
    #[serde(other)]
    Unstated,
}

impl Mapping {
    #[must_use]
    fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("many_to_one") => Self::ManyToOne,
            Some("one_to_one") => Self::OneToOne,
            _ => Self::Unstated,
        }
    }
}

/// Which edition a text is, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub edition: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// The key two corpora's spellings of one sefer collide under.
///
/// [`girsa_hebrew::normalize`] already strips nikud and te'amim and folds every
/// spelling of geresh and gershayim onto `'` and `"`. Two more things go, and
/// only here — not in the normal form the index is built on:
///
/// - **the quote marks themselves**, because Otzaria writes `חזון איש` where
///   Sefaria writes `חזו"א`-era variants and a title is not a place to be
///   precious about a gershayim;
/// - **spaces**, because the comma in `שולחן ערוך, אורח חיים` normalizes to one
///   and the Otzaria filename has none.
#[must_use]
pub fn match_key(title: &str) -> String {
    girsa_hebrew::normalize(title)
        .chars()
        .filter(|c| !matches!(c, '\'' | '"' | ' '))
        .collect()
}

/// The ref slug for a work with an English title.
///
/// A comma in a Sefaria title separates a work from its volume — `Shulchan
/// Arukh, Orach Chayim` — and that is exactly the `/` of a ref work path, which
/// is why spec.md §4.2 writes `girsa:shulchan-arukh/orach-chayim`.
///
/// Talmud is the exception worth handling: Sefaria titles a masechta `Berakhot`
/// and puts `Bavli` in its categories, but a ref has to say which Talmud —
/// there is a Yerushalmi Berakhot too, and they are different seforim.
///
/// **This is the same function the lexicon is built with**, and it has to be:
/// the lexicon maps a citation onto a slug and the importer names segments
/// after one, so a second implementation that drifted by a hyphen would resolve
/// citations onto works that do not exist.
///
/// That sentence was here, correct, for a week, with a byte-identical copy of
/// this function thirty-seven lines beneath it in `examples/build-lexicon.rs`.
/// It is now `slug_of_has_one_implementation` in
/// `crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs`, because
/// writing an invariant down is not enforcing it.
#[must_use]
pub fn slug_of(title: &str, categories: &[String]) -> String {
    let mut prefix = String::new();
    if categories.first().is_some_and(|c| c == "Talmud") {
        match categories.get(1).map(String::as_str) {
            Some("Bavli") => prefix.push_str("bavli/"),
            Some("Yerushalmi") => prefix.push_str("yerushalmi/"),
            _ => {}
        }
    }

    let mut slug = String::with_capacity(title.len());
    for c in title.chars() {
        match c {
            ',' => slug.push('/'),
            ' ' | '_' => slug.push('-'),
            c if c.is_ascii_alphanumeric() => slug.push(c.to_ascii_lowercase()),
            c if c == '-' || c == '/' => slug.push(c),
            _ => {}
        }
    }

    // `Shulchan Arukh, Orach Chayim` becomes `shulchan-arukh/-orach-chayim`
    // because of the space after the comma, and doubled separators read badly
    // in every ref that work ever appears in.
    let slug = slug.replace("/-", "/").replace("--", "-");
    format!("{prefix}{}", slug.trim_matches(['-', '/']))
}

/// The ref slug for a work that has only a Hebrew title.
///
/// The 978 Otzaria-only works have no Sefaria schema and therefore no English
/// title, and [`slug_of`] would reduce `קרן אורה על נדרים` to the empty string
/// — every one of them would collide on it.
///
/// So their slugs are Hebrew. A ref is text stored inside a Ksav document and
/// read by a person; `girsa:קרן-אורה-על-נדרים/2a:1` is more use to that person
/// than a transliteration nobody would have chosen. The grammar's own
/// separators are what must not appear, and nothing else.
/// Built from the letters as printed rather than from
/// [`girsa_hebrew::normalize`], because that folds final letters —
/// `קרן אורה` would slug to `קרנ אורה`, and a slug is read by a person, in
/// every ref of every sefer they never heard of. Marks come off, a gershayim is
/// dropped rather than becoming a separator (`שו"ת` → `שות`, one word, as it is
/// said), and whitespace is what separates.
#[must_use]
pub fn hebrew_slug_of(title: &str) -> String {
    slug_with(title, '-')
}

/// The same, for a **section of a work** rather than a work.
///
/// The words are joined with `_` and not `-`, and the reason is not taste.
/// `girsa-ref` reads a hyphen in the address as the separator of a **span** —
/// `2a:1-2b:4` — on the stated assumption that no address level contains one.
/// Slugging `Orach Chayim` to `orach-chayim` made that false: `girsa:tur/orach-chayim:240:1`
/// read back as a span from `orach` to `chayim:240:1`, which is not a place and
/// is not an error either.
///
/// So work slugs keep the hyphen §4.2 writes them with — they sit before the
/// last `/` and are never part of an address — and section labels take the
/// underscore.
///
/// `girsa-ref` 0.2.0 fixed the misreading at its own source — a hyphen now
/// separates two addresses only when what follows it is written entirely in
/// numbers — so this is defence in depth rather than the fix. It stays because
/// segment ids are permanent: an id minted today is read back in ten years by
/// whatever the parser is then.
#[must_use]
pub fn section_label_of(title: &str) -> String {
    slug_with(title, '_')
}

fn slug_with(title: &str, joiner: char) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut pending_dash = false;
    for c in title.chars() {
        if girsa_hebrew::is_mark(c) {
            continue;
        }
        let keep = if girsa_hebrew::is_hebrew_letter(c) {
            Some(c)
        } else if c.is_ascii_alphanumeric() {
            Some(c.to_ascii_lowercase())
        } else {
            None
        };
        match keep {
            Some(c) => {
                if pending_dash && !slug.is_empty() {
                    slug.push(joiner);
                }
                pending_dash = false;
                slug.push(c);
            }
            // Whitespace, a maqaf and an ASCII hyphen separate words; every
            // other mark of punctuation is noise between letters of one word.
            None if c.is_whitespace()
                || girsa_hebrew::is_word_breaking_punctuation(c)
                || matches!(c, '-' | '_') =>
            {
                pending_dash = true;
            }
            None => {}
        }
    }
    slug
}

/// Every work in the union, each assigned to exactly one source.
#[derive(Debug, Clone, Default)]
pub struct Catalogue {
    works: Vec<Work>,
    /// How many of the Sefaria works had an Otzaria file for the same sefer.
    shared: usize,
}

/// What the union turned out to be, for asserting against spec.md §2.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overlap {
    pub shared: usize,
    pub otzaria_only: usize,
    pub sefaria_only: usize,
}

impl Overlap {
    #[must_use]
    pub fn union(&self) -> usize {
        self.shared + self.otzaria_only + self.sefaria_only
    }
}

impl Catalogue {
    /// Read the corpora and decide, per work, where its text comes from.
    ///
    /// `sefaria_root` is the directory holding `schemas/` and `json/`.
    /// `libraries` are the `.txt` trees, each holding `אוצריא/` — more than one,
    /// because a shelf may be built from Otzaria's library **and** another one
    /// beside it, and each says for itself what it is (see [`Library`]).
    ///
    /// Read in the order given, and a work already claimed by an earlier
    /// library is not read again from a later one: the first library named owns
    /// any title two of them share, which is the same precedence Sefaria has
    /// over all of them.
    ///
    /// # Errors
    ///
    /// If a root cannot be read at all. A single unreadable schema is counted
    /// and skipped — one sefer missing is a smaller failure than no catalogue —
    /// and the count is returned so the caller can be loud.
    pub fn build(
        sefaria_root: &Path,
        libraries: &[Library],
    ) -> Result<(Self, usize), CatalogueError> {
        let (mut sefaria, skipped, declared) = read_sefaria(sefaria_root)?;
        let mut otzaria: Vec<Work> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for library in libraries {
            for work in read_otzaria(library)? {
                if seen.insert(match_key(&work.he_title)) {
                    otzaria.push(work);
                }
            }
        }

        // A declared base text is a **title**; a pane needs a slug. This is the
        // first point at which every title on the shelf is known, so it is the
        // only place the two can be joined — and a title naming a work that is
        // not here is dropped, because a dangling name would look exactly like
        // a slug and open nothing.
        let slug_of_title: BTreeMap<&str, &str> = sefaria
            .iter()
            .map(|w| (w.en_title.as_str(), w.slug.as_str()))
            .collect();
        let resolved: Vec<Vec<BaseText>> = declared
            .iter()
            .map(|(titles, mapping)| {
                titles
                    .iter()
                    .filter_map(|t| slug_of_title.get(t.as_str()))
                    .map(|slug| BaseText {
                        slug: (*slug).to_string(),
                        mapping: *mapping,
                    })
                    .collect()
            })
            .collect();
        for (work, bases) in sefaria.iter_mut().zip(resolved) {
            // A work is not a commentary on itself. Sefaria has a handful that
            // list their own title, and a pane that followed one would sit
            // beside a second copy of what you are already reading.
            work.commentary_on = bases.into_iter().filter(|b| b.slug != work.slug).collect();
        }

        // A commentary takes its base's place in the sequence.
        //
        // Sefaria states an `order` on the seforim that have one — the chumashim,
        // the masechtos, the chalakim — and on almost no commentary. So the
        // rishonim on the Torah, which are five works apiece for sixteen
        // authors, sorted alphabetically inside their folder: *Rashi on
        // Deuteronomy* above *Rashi on Genesis*, on a shelf where the reader
        // knows exactly what order those five come in.
        //
        // Done here because this is the first point that knows every work, and
        // done from the **declaration** rather than from the title — `X on Y` in
        // a slug is not evidence (BUILDER.md rule 6). A commentary on more than
        // one sefer takes the first base that has an order, which is the one
        // Sefaria lists first.
        let ordered: BTreeMap<&str, Vec<i32>> = sefaria
            .iter()
            .filter(|w| !w.order.is_empty())
            .map(|w| (w.slug.as_str(), w.order.clone()))
            .collect();
        let inherited: Vec<Option<Vec<i32>>> = sefaria
            .iter()
            .map(|work| {
                if !work.order.is_empty() {
                    return None;
                }
                work.commentary_on
                    .iter()
                    .find_map(|base| ordered.get(base.slug.as_str()))
                    .cloned()
            })
            .collect();
        for (work, order) in sefaria.iter_mut().zip(inherited) {
            if let Some(order) = order {
                work.order = order;
            }
        }

        let sefaria_keys: BTreeMap<String, ()> = sefaria
            .iter()
            .map(|w| (match_key(&w.he_title), ()))
            .chain(sefaria.iter().map(|w| (match_key(&w.en_title), ())))
            .collect();

        let mut works = sefaria;
        let mut shared = 0usize;
        for work in otzaria {
            if sefaria_keys.contains_key(&match_key(&work.he_title)) {
                // Sefaria has this sefer, so Sefaria supplies it — text and
                // structure both (spec.md §2.3b, decision 1). The Otzaria file
                // is not a second opinion to be merged in; it is simply not
                // used for this work.
                shared += 1;
                continue;
            }
            works.push(work);
        }

        works.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok((Self { works, shared }, skipped))
    }

    #[must_use]
    pub fn works(&self) -> &[Work] {
        &self.works
    }

    /// The measured overlap, to be checked against spec.md §2.3.
    #[must_use]
    pub fn overlap(&self) -> Overlap {
        let otzaria_only = self
            .works
            .iter()
            .filter(|w| w.source == Source::Otzaria)
            .count();
        Overlap {
            shared: self.shared,
            otzaria_only,
            sefaria_only: self.works.len() - otzaria_only - self.shared,
        }
    }

    /// Lexicon rows for the works Sefaria never had.
    ///
    /// The resolver's lexicon is seeded from Sefaria's schemas (W3), so the 978
    /// Otzaria-only works are invisible to it — and W8 has to resolve links
    /// *into* those works. One row per title, in the same
    /// `variant \t slug \t he \t en` shape `girsa_ref::Lexicon::from_tsv`
    /// reads.
    #[must_use]
    pub fn otzaria_lexicon_rows(&self) -> String {
        let mut out = String::new();
        for work in self.works.iter().filter(|w| w.source == Source::Otzaria) {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                work.he_title.trim(),
                work.slug,
                work.he_title.trim(),
                work.en_title.trim()
            ));
        }
        out
    }
}

/// Why a catalogue could not be built at all.
#[derive(Debug, thiserror::Error)]
pub enum CatalogueError {
    #[error("reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0} is not there — has the fetch run?")]
    Missing(String),
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> CatalogueError + '_ {
    move |source| CatalogueError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Every work Sefaria ships a schema for, whose Hebrew text also landed.
///
/// The third return is what each work *declared* about its base text, by
/// English title, aligned with `works`. Titles rather than slugs, because a
/// title can only be turned into a slug once every work on the shelf is known
/// and that is one level up.
type Declared = (Vec<String>, Mapping);

fn read_sefaria(root: &Path) -> Result<(Vec<Work>, usize, Vec<Declared>), CatalogueError> {
    let schemas = root.join("schemas");
    if !schemas.is_dir() {
        return Err(CatalogueError::Missing(schemas.display().to_string()));
    }
    let texts = index_hebrew_texts(&root.join("json"))?;

    let mut works = Vec::new();
    let mut declared: Vec<Declared> = Vec::new();
    let mut skipped = 0usize;
    let mut entries: Vec<_> = fs::read_dir(&schemas)
        .map_err(io(&schemas))?
        .flatten()
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            skipped += 1;
            continue;
        };
        let Ok(schema): Result<Value, _> = serde_json::from_str(&text) else {
            // `schemas/Sheet.json` is zero bytes in the bucket. Counted, not
            // silently dropped.
            skipped += 1;
            continue;
        };
        let Some(en_title) = schema.get("title").and_then(Value::as_str) else {
            skipped += 1;
            continue;
        };
        // T8: a leading space in a title is real corpus grime, and it survives
        // into every slug and every match key unless it is taken off here.
        let en_title = en_title.trim();
        let he_title = schema
            .get("heTitle")
            .and_then(Value::as_str)
            .unwrap_or(en_title)
            .trim();
        let categories = string_list(schema.get("categories"));

        // A schema with no Hebrew text is a work Sefaria describes and does not
        // have — a stub, or an English-only text this fetch deliberately
        // skipped. It is not part of the union: there is nothing to read.
        let Some(origin) = texts.get(en_title) else {
            continue;
        };

        works.push(Work {
            slug: slug_of(en_title, &categories),
            he_title: he_title.to_string(),
            en_title: en_title.to_string(),
            order: order_of(schema.get("order")),
            categories,
            source: Source::Sefaria,
            origin: origin.clone(),
            schema: Some(path.clone()),
            he_sections: schema.get("schema").map(section_names).unwrap_or_default(),
            author: schema
                .pointer("/authors/0/he")
                .or_else(|| schema.pointer("/authors/0/en"))
                .and_then(Value::as_str)
                .map(str::to_string),
            era: schema
                .get("era")
                .and_then(Value::as_str)
                .map(str::to_string),
            // T8 once more: this arrives as `" (1563 CE)"`, leading space and
            // parentheses, and it is shown to a reader in the era facet.
            comp_date: schema
                .get("compDateString")
                .and_then(|v| v.get("en"))
                .or_else(|| schema.get("compDateString"))
                .and_then(Value::as_str)
                .map(|s| s.trim().trim_matches(['(', ')']).trim().to_string())
                .filter(|s| !s.is_empty()),
            // Sefaria's export ships one merged text per work and names the
            // editions it was merged from inside the file; the edition list is
            // read with the text rather than here, where only the schema is
            // open.
            version: None,
            // Filled in by `Catalogue::build`, which is the first place that
            // knows every title on the shelf and so the first place a declared
            // base text can be turned into a slug.
            commentary_on: Vec::new(),
        });
        declared.push((
            string_list_at(schema.get("base_text_titles"), "en"),
            Mapping::parse(schema.get("base_text_mapping").and_then(Value::as_str)),
        ));
    }
    Ok((works, skipped, declared))
}

/// What a schema calls the levels of an address, outermost first.
///
/// A **jagged** node says so itself. A **branch** says it on each child, and
/// the one that matters is the *default* child — the body of the work, keyed
/// by the empty string and contributing no level of its own, which is what an
/// address like `121:3` names. The named children are its introductions and
/// appendices, and they are addressed by their own name first, so a citation
/// into one is recognisable without the words.
///
/// Blank names are dropped rather than kept: 2,271 nodes carry an empty
/// string, and a citation reading `שולחן ערוך  קכ"א` with a hole in it is
/// worse than one with no words at all.
#[must_use]
pub fn section_names(schema: &Value) -> Vec<String> {
    if let Some(names) = schema.get("heSectionNames").and_then(Value::as_array) {
        return names
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    schema
        .get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|child| {
            child
                .get("default")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || child
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .is_empty()
        })
        .map(section_names)
        .unwrap_or_default()
}

/// Sefaria's `order`, in tenths — see [`Work::order`].
///
/// Tenths and not the number as written, because one schema in the corpus says
/// `[40.5]` and a shelf order that rounded it would put the Rambam's
/// introduction to Seder Kodashim on top of the sefer it introduces. Anything
/// that is not a number is dropped rather than guessed at: an order nobody can
/// read is *no order*, which sorts after the ones that can.
fn order_of(value: Option<&Value>) -> Vec<i32> {
    let Some(list) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(list.len());
    for step in list {
        let Some(number) = step.as_f64() else {
            return Vec::new();
        };
        // The corpus's largest is 88 and its smallest fraction is a half; this
        // clamp is against a hand-edited schema, not against the download.
        out.push(
            (number * 10.0)
                .round()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32,
        );
    }
    out
}

/// `[{"en": "Berakhot", "he": "ברכות"}, …]` → the values under one key.
fn string_list_at(v: Option<&Value>, key: &str) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|item| item.get(key).and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// `title → path of its Hebrew merged.json`.
///
/// Built by walking `json/`, because the directory a work's text sits under is
/// its *category* path, which is not derivable from the title.
fn index_hebrew_texts(json_root: &Path) -> Result<BTreeMap<String, PathBuf>, CatalogueError> {
    let mut out = BTreeMap::new();
    if !json_root.is_dir() {
        return Err(CatalogueError::Missing(json_root.display().to_string()));
    }
    let mut stack = vec![json_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "merged.json")
                && path.parent().is_some_and(|p| {
                    p.file_name()
                        .is_some_and(|n| n.eq_ignore_ascii_case("Hebrew"))
                })
            {
                if let Some(title) = path
                    .parent()
                    .and_then(Path::parent)
                    .and_then(Path::file_name)
                    .and_then(|n| n.to_str())
                {
                    out.insert(title.to_string(), path);
                }
            }
        }
    }
    Ok(out)
}

/// Every `.txt` in a library's `אוצריא/` tree, with what `metadata.json` knows.
///
/// The edition every work here gets is [`Library::version`] — the tree's own
/// answer about itself, which may be nothing at all. It used to be a constant,
/// and [`Library`] is the record of what that cost.
fn read_otzaria(library: &Library) -> Result<Vec<Work>, CatalogueError> {
    let root = library.root();
    let tree = root.join("אוצריא");
    if !tree.is_dir() {
        return Err(CatalogueError::Missing(tree.display().to_string()));
    }
    let metadata = read_otzaria_metadata(&root.join("metadata.json"));

    let mut works = Vec::new();
    let mut stack = vec![tree.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "txt") {
                continue;
            }
            let Some(title) = path.file_stem().and_then(|n| n.to_str()) else {
                continue;
            };
            // T8 again: `metadata.json` carries `" דברי חמודות על ברכות"`,
            // leading space and all.
            let title = title.trim();
            let categories: Vec<String> = path
                .parent()
                .and_then(|p| p.strip_prefix(&tree).ok())
                .map(|rel| {
                    rel.components()
                        .filter_map(|c| c.as_os_str().to_str())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();

            let meta = metadata.get(&match_key(title));
            works.push(Work {
                slug: hebrew_slug_of(title),
                he_title: title.to_string(),
                en_title: title.to_string(),
                categories,
                source: Source::Otzaria,
                origin: path,
                schema: None,
                // No schema, so nothing says what a level of this sefer is
                // called. Cited by number.
                he_sections: Vec::new(),
                // Otzaria has no schemas and so states no order. Its shelves
                // fall back to the title, which is what `Work::by_order` does
                // with an empty one — and which is what the folder listing on
                // disk already is.
                order: Vec::new(),
                author: meta.and_then(|m| m.author.clone()),
                era: None,
                comp_date: meta.and_then(|m| m.comp_date.clone()),
                version: library.version().cloned(),
                // Otzaria ships no schemas, so nothing here declares a base
                // text. `קרן אורה על נדרים` is plainly a commentary on Nedarim
                // and this leaves it unsaid rather than reading it off the
                // title — the pane beside it is found through the link graph
                // instead, which is a thing somebody recorded.
                commentary_on: Vec::new(),
            });
        }
    }
    Ok(works)
}

/// A tree of `.txt` seforim, and what may **truthfully** be said about where it
/// came from.
///
/// # Why this is a type and not a constant
///
/// It was a constant. [`read_otzaria`] stamped every work it walked with
/// `edition: "Otzaria"`, Sivan22's repository as provenance, and `Unlicense` —
/// true of the one tree it had ever been pointed at, and false the moment a
/// second one existed. Point it at somebody else's seforim laid out the same
/// way and the shelf records, as fact, a repository they did not come from and
/// a licence they are not under.
///
/// spec.md §13 wants a work to be able to say where its text is from. A field
/// that answers wrongly is worse than a field that does not answer: `None` is
/// something a reader can act on and a confident wrong licence is not.
///
/// So a library **declares itself**, in a `library.json` at its root:
///
/// ```json
/// { "edition": "OtzarLib",
///   "provenance": "https://github.com/YairDaniel123/OtzarLib" }
/// ```
///
/// A tree that declares nothing has nothing claimed for it. The single
/// exception is Sivan22's own library, which is *recognised* — by the
/// `metadata.json` beside an `אוצריא/` that nothing else has — rather than
/// assumed, so nobody has to add a file to a 13 GB download for the shelf to go
/// on saying what it always correctly said. A recognition that can be wrong
/// about one specific tree is a different thing from a default that is wrong
/// about every tree but one.
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
    version: Option<Version>,
    trouble: Option<String>,
}

impl Library {
    /// Read what this tree says about itself.
    #[must_use]
    pub fn at(root: &Path) -> Self {
        let (version, trouble) = match fs::read_to_string(root.join("library.json")) {
            Ok(body) => match serde_json::from_str::<Version>(&body) {
                Ok(version) => (Some(version), None),
                // Not silence. A library that tried to say something and failed
                // must not fall back to being recognised as somebody else, and
                // must not disappear into a tree that says nothing — either way
                // the reason would be invisible at exactly the moment somebody
                // is asking where a sefer came from.
                Err(e) => (
                    None,
                    Some(format!("{}: {e}", root.join("library.json").display())),
                ),
            },
            Err(_) => (the_otzaria_library(root), None),
        };
        Self {
            root: root.to_path_buf(),
            version,
            trouble,
        }
    }

    /// Where this library's seforim sit.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// What every work read out of it may say about its edition, or nothing.
    #[must_use]
    pub fn version(&self) -> Option<&Version> {
        self.version.as_ref()
    }

    /// A declaration that would not parse, for the caller to be loud about.
    #[must_use]
    pub fn trouble(&self) -> Option<&str> {
        self.trouble.as_deref()
    }
}

/// Is this Sivan22's Otzaria library?
///
/// Both marks, and both are its own: a `metadata.json` at the root beside the
/// `אוצריא/` directory the seforim are under. A directory holding only one of
/// them is not being recognised on a guess.
fn the_otzaria_library(root: &Path) -> Option<Version> {
    (root.join("metadata.json").is_file() && root.join("אוצריא").is_dir()).then(|| Version {
        edition: "Otzaria".to_string(),
        provenance: Some("https://github.com/Sivan22/otzaria-library".to_string()),
        license: Some("Unlicense".to_string()),
    })
}

#[derive(Debug, Clone, Default)]
struct OtzariaMeta {
    author: Option<String>,
    comp_date: Option<String>,
}

/// Read `metadata.json`, keyed by match key.
///
/// Missing or malformed is not fatal: it carries author and date, which are
/// facets (spec.md §5), not text. A catalogue without them is worth having.
fn read_otzaria_metadata(path: &Path) -> BTreeMap<String, OtzariaMeta> {
    let mut out = BTreeMap::new();
    let Ok(text) = fs::read_to_string(path) else {
        return out;
    };
    let Ok(rows): Result<Vec<Value>, _> = serde_json::from_str(&text) else {
        return out;
    };
    for row in rows {
        // T8: `"Unnamed: 9"` is a leftover spreadsheet column and is read by
        // naming the fields we want rather than by taking whatever is there.
        let Some(title) = row.get("title").and_then(Value::as_str) else {
            continue;
        };
        out.insert(
            match_key(title.trim()),
            OtzariaMeta {
                author: row
                    .get("author")
                    .and_then(Value::as_str)
                    .map(|s| s.trim().to_string()),
                comp_date: row
                    .get("compDate")
                    .and_then(Value::as_str)
                    .map(|s| s.trim().to_string()),
            },
        );
    }
    out
}

fn string_list(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_schema_says_what_the_levels_of_an_address_are_called() {
        // Verbatim shapes from `corpus/sefaria/schemas/`. A citation is
        // printed with these words and there is nowhere else to learn them.
        let jagged = serde_json::json!({
            "nodeType": "JaggedArrayNode", "depth": 2,
            "addressTypes": ["Siman", "Seif"],
            "heSectionNames": ["סימן", "סעיף"]});
        assert_eq!(section_names(&jagged), ["סימן", "סעיף"]);

        // Mishnah Berurah: a branch whose *body* is the default child, and
        // whose named children are its two introductions. The body is what
        // `121:3` addresses.
        let branch = serde_json::json!({
            "nodeType": "SchemaNode",
            "nodes": [
                {"title": "Introduction", "depth": 1, "heSectionNames": ["פסקה"]},
                {"title": "", "default": true, "depth": 2,
                 "heSectionNames": ["סימן", "סעיף קטן"]}]});
        assert_eq!(section_names(&branch), ["סימן", "סעיף קטן"]);

        // A schema that does not say, and one whose names are blank — 2,271
        // nodes carry an empty string. Neither invents a word.
        assert!(section_names(&serde_json::json!({"depth": 1})).is_empty());
        assert!(section_names(&serde_json::json!({"heSectionNames": ["", " "]})).is_empty());
    }

    #[test]
    fn a_volume_title_becomes_a_work_path() {
        assert_eq!(
            slug_of("Shulchan Arukh, Orach Chayim", &[]),
            "shulchan-arukh/orach-chayim"
        );
    }

    #[test]
    fn a_masechta_says_which_talmud_it_is_from() {
        let bavli = ["Talmud".into(), "Bavli".into(), "Seder Zeraim".into()];
        let yerushalmi = ["Talmud".into(), "Yerushalmi".into()];
        assert_eq!(slug_of("Berakhot", &bavli), "bavli/berakhot");
        assert_eq!(
            slug_of("Jerusalem Talmud Berakhot", &yerushalmi),
            "yerushalmi/jerusalem-talmud-berakhot"
        );
    }

    #[test]
    fn the_two_corpora_spellings_of_one_sefer_collide() {
        // Sefaria's title and Otzaria's filename for the same sefer. Every
        // difference between them is punctuation.
        assert_eq!(
            match_key("שולחן ערוך, אורח חיים"),
            match_key("שולחן ערוך אורח חיים")
        );
        assert_eq!(match_key("שו\"ע"), match_key("שו״ע"));
    }

    #[test]
    fn two_different_seforim_do_not_collide() {
        assert_ne!(match_key("קרן אורה"), match_key("אורה"));
        assert_ne!(
            match_key("Shulchan Arukh, Orach Chayim"),
            match_key("Shulchan Arukh, Yoreh Deah")
        );
    }

    #[test]
    fn a_hebrew_only_work_gets_a_slug_a_person_can_read() {
        // slug_of would reduce every one of the 978 to the empty string, and
        // they would all collide on it.
        assert_eq!(slug_of("קרן אורה על נדרים", &[]), "");
        assert_eq!(hebrew_slug_of("קרן אורה על נדרים"), "קרן-אורה-על-נדרים");
    }

    /// A Sefaria root holding just enough to be read: a schema per work, and a
    /// Hebrew `merged.json` under the title's directory so the work counts as
    /// one the export actually has text for.
    fn sefaria_root(name: &str, schemas: &[(&str, serde_json::Value)]) -> PathBuf {
        let root = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&root);
        for (title, schema) in schemas {
            let path = root.join("schemas").join(format!("{title}.json"));
            fs::create_dir_all(root.join("schemas")).expect("schemas dir");
            fs::write(&path, schema.to_string()).expect("schema");

            let text = root.join("json").join(title).join("Hebrew");
            fs::create_dir_all(&text).expect("text dir");
            fs::write(text.join("merged.json"), "{\"text\":[]}").expect("text");
        }
        fs::create_dir_all(root.join("otzaria/אוצריא")).expect("otzaria tree");
        root
    }

    #[test]
    fn a_commentary_records_the_sefer_it_sits_beside() {
        // Two panes track each other because the corpus *says* one work is a
        // commentary on the other — Sefaria declares `base_text_titles` and
        // `base_text_mapping` on every commentary's schema. Without that, the
        // only way to put Rashi beside the Gemara is to notice that one title
        // contains the other, which is a guess (BUILDER.md rule 6) and wrong
        // for `Rashi on Berakhot` versus `Berakhot` in the Yerushalmi.
        let root = sefaria_root(
            "girsa-work-basetext",
            &[
                (
                    "Berakhot",
                    serde_json::json!({
                        "title": "Berakhot",
                        "heTitle": "ברכות",
                        "categories": ["Talmud", "Bavli", "Seder Zeraim"],
                    }),
                ),
                (
                    "Rashi on Berakhot",
                    serde_json::json!({
                        "title": "Rashi on Berakhot",
                        "heTitle": "רש\"י על ברכות",
                        "categories": ["Talmud", "Bavli", "Rishonim on Talmud"],
                        "dependence": "Commentary",
                        "base_text_titles": [{"en": "Berakhot", "he": "ברכות"}],
                        "base_text_mapping": "many_to_one",
                    }),
                ),
            ],
        );

        let (catalogue, _) = Catalogue::build(&root, &[Library::at(&root.join("otzaria"))])
            .expect("the catalogue builds");
        let rashi = catalogue
            .works()
            .iter()
            .find(|w| w.slug == "bavli/rashi-on-berakhot")
            .expect("Rashi on Berakhot is in the catalogue");

        assert_eq!(
            rashi.commentary_on,
            vec![BaseText {
                slug: "bavli/berakhot".into(),
                mapping: Mapping::ManyToOne,
            }],
            "the declared base text is not recorded, so nothing can put the two side by side"
        );

        // And the base text does not claim to be a commentary on itself.
        let berakhot = catalogue
            .works()
            .iter()
            .find(|w| w.slug == "bavli/berakhot")
            .expect("Berakhot is in the catalogue");
        assert!(berakhot.commentary_on.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_base_text_that_is_not_on_the_shelf_is_dropped_rather_than_kept_as_a_title() {
        // Sefaria declares base texts it has no Hebrew for. A dangling name in
        // this field would be indistinguishable from a slug, and the pane
        // beside you would open nothing with no way to say why.
        let root = sefaria_root(
            "girsa-work-basetext-missing",
            &[(
                "Rashi on Nowhere",
                serde_json::json!({
                    "title": "Rashi on Nowhere",
                    "heTitle": "רש\"י על שומקום",
                    "categories": ["Tanakh"],
                    "dependence": "Commentary",
                    "base_text_titles": [{"en": "Nowhere", "he": "שומקום"}],
                    "base_text_mapping": "one_to_one",
                }),
            )],
        );
        let (catalogue, _) = Catalogue::build(&root, &[Library::at(&root.join("otzaria"))])
            .expect("the catalogue builds");
        assert!(catalogue.works()[0].commentary_on.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_hebrew_slug_never_carries_the_grammars_own_separators() {
        // `חזון איש, אורח חיים מועד` is a real Otzaria filename, and a `/` or a
        // `:` reaching a slug would make every id in that work re-read as a
        // different work at a different place.
        for title in [
            "חזון איש, אורח חיים מועד",
            "שער חמישי - שער ייחוד המעשה",
            " דברי חמודות על ברכות",
            "שו\"ת ישועות מלכו",
        ] {
            let slug = hebrew_slug_of(title);
            assert!(!slug.is_empty(), "{title} slugged to nothing");
            assert!(!slug.contains(['/', ':', '#']), "{title} slugged to {slug}");
            assert!(!slug.starts_with('-') && !slug.ends_with('-'), "{slug}");
        }
    }
}
