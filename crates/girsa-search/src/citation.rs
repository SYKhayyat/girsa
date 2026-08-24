//! Citation — mode 4: *type a mareh makom, jump* (spec.md §9.3, §4.3).
//!
//! The resolver (W3) turns any way a person writes a citation into a ref. This
//! puts that ref on the shelf: which segments does it name, and is the sefer
//! even here?
//!
//! # Three answers, and the middle one is not a failure
//!
//! `או"ח` means the Orach Chayim of the Shulchan Arukh *and* of the Tur *and*
//! of a hundred sets of responsa. spec.md §4.3 and BUILDER.md rule 6: ambiguity
//! is surfaced as **a choice**, never resolved by picking the first. So a
//! lookup comes back with none, one, or several [`Place`]s, and the several is
//! shown as several.
//!
//! # Why a wrong jump is the worst kind of wrong
//!
//! A near-miss resolves, opens a page, and the page is the wrong one. Nothing
//! about it looks like an error — and if the reader copies it into a Ksav
//! document, the wrong mareh makom is now in a printed sefer. So the address
//! lookup will not fall back to the nearest thing (`girsa_corpus::index`), and
//! this layer will not either.
//!
//! What it does instead is **offer**, which is §9.6's rule for this mode:
//!
//! | what happened | what is offered |
//! |---|---|
//! | several candidates | all of them, as a choice |
//! | the sefer is here, the address is not | the sefer, and what it does have |
//! | the sefer is catalogued and has no Hebrew text | said as such, so it is not read as a typo |
//! | nothing resolved | titles close to what was typed — never applied |
//!
//! The last row is the only one that is a suggestion rather than a fact, and it
//! is offered as a list of spellings a reader picks from, exactly like the
//! ambiguity path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use girsa_corpus::import::{self, Segment};
use girsa_corpus::index::{Run, WorkSegments};
use girsa_corpus::sections::Sections;
use girsa_ref::lexicon::Lexicon;
use girsa_ref::resolve::{resolve_in_context, resolve_labels_as_names, Context, Resolution};
use girsa_ref::{Level, Ref};

/// How many spellings are offered when nothing resolved.
///
/// A list nobody reads is the same as no list. Anything cut is **counted** in
/// [`Landing::more_spellings`], because a list that silently stops reads as
/// *these are all of them*.
pub const MOST_SUGGESTIONS: usize = 8;

/// Why the citation lookup could not be set up.
#[derive(Debug, thiserror::Error)]
pub enum CitationError {
    #[error("no lexicon at {path} — has girsa-import run? {source}")]
    NoLexicon {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// A place a citation names, on this shelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The ref, canonical.
    pub reference: Ref,
    /// The segments it names — one, or a run in reading order.
    pub run: Run,
}

/// Something the reader may have meant, offered and never taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NearMiss {
    /// The sefer is on the shelf and does not have that address.
    ///
    /// A real and common case: `Berakhot 99a` parses perfectly and there is no
    /// daf 99. Offering the sefer is offering to open something the reader
    /// certainly meant; jumping there would be inventing a place.
    AddressNotThere { reference: Ref, work: String },
    /// Sefaria catalogues the sefer and the export has no Hebrew text for it —
    /// 387 titles on this shelf. Nothing to open, and it is not a typo.
    NotOnTheShelf { reference: Ref, work: String },
    /// A title in the lexicon close to what was typed.
    OtherTitle { spelling: String, slug: String },
}

/// What a typed citation turned out to be.
#[derive(Debug, Clone)]
pub struct Landing {
    pub typed: String,
    /// What the resolver said, before the shelf was consulted.
    pub resolution: Resolution,
    /// Every candidate that is really there. Several is a **choice**.
    pub places: Vec<Place>,
    /// What is offered instead, or as well.
    pub near: Vec<NearMiss>,
    /// Spellings not shown, because the list was cut.
    pub more_spellings: usize,
}

impl Landing {
    /// Candidates the shelf could not rule out.
    ///
    /// A candidate is eliminated **only when the shelf can refute it** — the
    /// sefer is here and the address is not in it. A candidate whose sefer is
    /// *not* here is never eliminated, because nothing here knows what is
    /// inside a sefer it does not have.
    ///
    /// This is W8's rule, and it is the same rule for the same reason: refuting
    /// needs evidence, and an absent sefer is not evidence about its contents.
    /// One of these surviving keeps the whole thing a choice.
    #[must_use]
    pub fn unrefuted(&self) -> usize {
        self.near
            .iter()
            .filter(|n| matches!(n, NearMiss::NotOnTheShelf { .. }))
            .count()
    }

    /// Whether the reader has to choose.
    #[must_use]
    pub fn is_a_choice(&self) -> bool {
        self.places.len() + self.unrefuted() > 1
    }

    /// The one place, when the shelf can account for every other candidate.
    ///
    /// `None` for a choice — deliberately, so that no caller can turn an
    /// ambiguity into an answer by taking the first element. And `None` when
    /// one candidate is here and another is a sefer we do not have: that is
    /// still two candidates, and jumping to the one we happen to hold would be
    /// picking by what is downloaded rather than by what was written.
    #[must_use]
    pub fn only(&self) -> Option<&Place> {
        match self.places.as_slice() {
            [only] if self.unrefuted() == 0 => Some(only),
            _ => None,
        }
    }

    /// A line for the result header.
    #[must_use]
    pub fn describe(&self) -> String {
        if let Some(only) = self.only() {
            return only.reference.to_string();
        }
        let candidates = self.places.len() + self.unrefuted();
        match candidates {
            0 => format!("{} is not a place on this shelf", self.typed),
            n => format!("{} could be {n} places", self.typed),
        }
    }
}

/// The citation bar: a lexicon, and the shelf to check it against.
#[derive(Debug)]
pub struct Citations {
    root: PathBuf,
    /// Your own layer, when one was named. A sefer of yours is in the lexicon
    /// (W3, G1) and its text is not under the corpus root, so resolving its
    /// name and then reading its segments are two questions with two answers.
    personal: Option<PathBuf>,
    /// Which slugs came out of your layer, so [`Citations::segments_of`] reads
    /// each work from the root that actually holds it.
    mine: std::collections::BTreeSet<String>,
    lexicon: Lexicon,
    /// Every spelling in the lexicon, for the near-miss list. Kept beside the
    /// lexicon rather than asked of it: the resolver's job is exact lookup, and
    /// *what looks like this* is a different question that must never be
    /// allowed to answer the first one.
    ///
    /// **Three fields, not two: the normalized form is stored.** The near-miss
    /// scan called `girsa_hebrew::normalize` on every spelling in a 3.7 MB
    /// lexicon, per unresolved lookup — one allocation and one full pass over
    /// each of tens of thousands of titles, to compare against a head word that
    /// does not change. It is a property of the spelling, so it is computed
    /// where the spelling is read: once, at `open`.
    spellings: Vec<(String, String, String)>,
    /// Works already read back, by slug. A citation is a jump, not a keystroke,
    /// but a reader typing one asks for the same sefer several times.
    known: Mutex<BTreeMap<String, Option<WorkSegments>>>,
    /// What each work's schema calls the parts of its address, read the first
    /// time a citation names that work. See [`Citations::addressed`].
    schemas: Mutex<BTreeMap<String, Arc<Sections>>>,
}

impl Citations {
    /// Read the lexicon `girsa-import` wrote and get ready to look things up.
    ///
    /// # Errors
    ///
    /// If the lexicon is not there. Without it every citation is unresolved,
    /// which would look exactly like a shelf that does not have the sefer.
    pub fn open(root: &Path, personal: Option<&Path>) -> Result<Self, CitationError> {
        // Both shipped halves and, when a personal root was named, the seforim
        // in it. The 978 Otzaria-only works have no Sefaria schema and are in a
        // second file (W8); a shelf without it can still resolve everything
        // Sefaria has, and a citation into one of those seforim simply does not
        // resolve, which is the honest outcome.
        let titles = match personal {
            Some(personal) => girsa_corpus::lexicon::Titles::across(root, personal),
            None => girsa_corpus::lexicon::Titles::of(root),
        }
        .map_err(|source| CitationError::NoLexicon {
            path: root.join("lexicon.tsv").display().to_string(),
            source,
        })?;
        let spellings = read_spellings(titles.tsv());
        Ok(Self {
            root: root.to_path_buf(),
            personal: personal.map(Path::to_path_buf),
            mine: titles.mine().clone(),
            lexicon: titles.lexicon(),
            spellings,
            known: Mutex::new(BTreeMap::new()),
            schemas: Mutex::new(BTreeMap::new()),
        })
    }

    /// How many works the lexicon knows.
    #[must_use]
    pub fn works(&self) -> usize {
        self.lexicon.len()
    }

    /// How many spellings of them.
    #[must_use]
    pub fn spellings(&self) -> usize {
        self.lexicon.variant_count()
    }

    /// Look a citation up, completing a partial one against where the reader
    /// is standing.
    #[must_use]
    pub fn look_up(&self, typed: &str, context: &Context) -> Landing {
        let resolution = self.reading_the_schema_confirms(typed, context);
        let mut places = Vec::new();
        let mut near = Vec::new();

        for reference in resolution.candidates() {
            let slug = reference.work_slug();
            match self.segments_of(&slug) {
                None => near.push(NearMiss::NotOnTheShelf {
                    reference: reference.clone(),
                    work: slug,
                }),
                Some(work) => {
                    // Addressed the way this work's segments are addressed
                    // before anything is asked of it, and **carried forward as
                    // the ref**: a place that answers `אורח חיים:1` and then
                    // reports itself as `girsa:tur/אורח חיים:1` would open here
                    // and fail everywhere the ref is stored.
                    let reference = self.addressed(&slug, reference);
                    match work.resolve_in(&slug, &reference) {
                        Some(run) => places.push(Place { reference, run }),
                        None => near.push(NearMiss::AddressNotThere {
                            reference,
                            work: slug,
                        }),
                    }
                }
            }
        }

        let mut more_spellings = 0;
        if matches!(resolution, Resolution::Unresolved) {
            let (suggestions, cut) = self.close_to(typed);
            more_spellings = cut;
            near.extend(suggestions);
        }

        Landing {
            typed: typed.to_string(),
            resolution,
            places,
            near,
            more_spellings,
        }
    }

    /// The text a place names, in reading order.
    ///
    /// # Errors
    ///
    /// If the work cannot be read back off the shelf.
    pub fn passage(
        &self,
        place: &Place,
        limit: usize,
    ) -> Result<Vec<Segment>, import::ImportError> {
        let work = import::read_back(&self.root, place.reference.work_slug().as_str())?;
        // A name that resolves but does not sit in the work is a refusal, not
        // an invitation to print the sefer's opening: the resolver said *this
        // place* and defaulting to segment 1 answered with a confident wrong
        // mareh makom.
        let first = work
            .segments
            .iter()
            .position(|s| s.id == place.run.first)
            .ok_or_else(|| import::ImportError::Malformed {
                path: format!("girsa:{}", place.reference.work_slug()),
                message: format!("{} is not in this work on the shelf", place.run.first),
            })?;
        let last = place
            .run
            .last
            .as_ref()
            .and_then(|id| work.segments.iter().position(|s| s.id == *id))
            .unwrap_or(first);
        Ok(work
            .segments
            .get(first..=last.max(first))
            .unwrap_or_default()
            .iter()
            .take(limit)
            .cloned()
            .collect())
    }

    /// One work's addresses, read back once.
    fn segments_of(&self, slug: &str) -> Option<WorkSegments> {
        let mut known = self.known.lock().ok()?;
        // The root that holds this work's text, not the root the resolver was
        // opened at. A sefer of yours resolves out of the same lexicon as a
        // sefer of Sefaria's and its `segments.jsonl` is somewhere else
        // entirely — read from the corpus it would come back missing, and a
        // missing work is reported as *not on the shelf*, which is the one
        // sentence a sefer sitting on your shelf must never produce.
        let from = match self.personal.as_deref() {
            Some(personal) if self.mine.contains(slug) => personal,
            _ => &self.root,
        };
        let entry = known.entry(slug.to_string()).or_insert_with(|| {
            WorkSegments::load(from, slug)
                .ok()
                .filter(|w| !w.is_empty())
        });
        entry.clone()
    }

    /// The same ref, with a chelek the reader named in Hebrew turned into the
    /// slug this work's segments are addressed by.
    ///
    /// # The finding
    ///
    /// Typed into the bar, `טור אורח חיים סימן א'` landed nowhere. So did
    /// `טור יורה דעה סימן א'`, `ערוך השולחן יורה דעה א'` and `שולחן ערוך הרב
    /// אורח חיים א'` — four of the most-cited codes after the Shulchan Arukh,
    /// none of them reachable by typing its name.
    ///
    /// The resolver was never the problem: it answers `Exact`, with
    /// `אורח חיים:1`. The Tur's segments say `orach_chayim:1:1`, and nothing
    /// between the two knew that `אורח חיים` and `orach_chayim` are one place.
    /// [`girsa_corpus::sections`] is the pairing, read out of the schema the
    /// importer already recorded.
    ///
    /// A work whose schema names no sections — which is nearly all of them —
    /// gets its ref back untouched.
    fn addressed(&self, slug: &str, reference: &Ref) -> Ref {
        let sections = self.sections_of(slug);
        if sections.is_empty() {
            return reference.clone();
        }
        let from = sections.slugged(reference.from());
        match reference.to() {
            Some(to) => Ref::span(reference.work().to_vec(), from, sections.slugged(to)),
            None => Ref::point(reference.work().to_vec(), from),
        }
    }

    /// The resolver reads a level word two ways; this asks the schema which.
    ///
    /// `סימן א'` is siman one and the word `סימן` addresses nothing — right
    /// almost always, and wrong in a sefer whose schema names a section by
    /// exactly that word. `עטרת זקנים שער א'` came back as `1`, which is a real
    /// perek of that sefer and not the place anybody asked for: **a wrong
    /// landing that looks exactly like a right one**, which this module's own
    /// header calls the worst kind of wrong there is.
    ///
    /// `girsa_ref::resolve::resolve_labels_as_names` is the same citation read
    /// with the word kept. Neither reading is better in general and this does
    /// not prefer one — the schema decides, and only when it says both of these
    /// things about the word:
    ///
    /// * it **is** the title of a section of this work
    ///   ([`Sections::section_of`], which refuses a name two sections share);
    /// * it is **not** a level name this work uses
    ///   ([`Sections::is_level_name`]) — so the label reading is labelling with
    ///   a word this sefer never labels anything with.
    ///
    /// Both, or nothing changes. Deliberately narrow in two more ways: it only
    /// looks when both readings come back `Exact`, so a citation that is
    /// already a choice between works stays exactly the choice it was; and it
    /// only swaps when the second reading names a place that is really there,
    /// so a schema-confirmed guess that lands nowhere never replaces a landing.
    ///
    /// Measured on the shelf: 166 chalakim of 7,627.
    fn reading_the_schema_confirms(&self, typed: &str, context: &Context) -> Resolution {
        let plain = resolve_in_context(&self.lexicon, typed, context);
        // A choice between works stays the choice it was. The two cases worth
        // a second reading are *one answer, and it may be the wrong one* and
        // *no answer at all* — the second because the ordinary reading refuses
        // an address that is a section name with no number after it, on the
        // ground that it cannot tell a section from a stray word. It cannot;
        // the schema can.
        match plain {
            Resolution::Exact(_) | Resolution::Unresolved => {}
            Resolution::Ambiguous(_) => return plain,
        }
        let named = resolve_labels_as_names(&self.lexicon, typed, context);
        let Some(kept) = named.exact() else {
            return plain;
        };
        if plain
            .exact()
            .is_some_and(|ordinary| kept.work() != ordinary.work())
        {
            return plain;
        }
        let slug = kept.work_slug();
        let sections = self.sections_of(&slug);
        // `addressed` is what turns names into the slugs the segments carry —
        // the same path `look_up` takes below — and asking it rather than
        // asking `section_of` about the raw words is what lets a level holding
        // **several** section names answer: one level can hold two of them, and
        // `Sections::slugged` is the one place that knows how to split it.
        let addressed = self.addressed(&slug, kept);
        let Some(word) = kept.from().levels().first().map(Level::as_str) else {
            return plain;
        };
        let Some(front) = addressed.from().levels().first().map(Level::as_str) else {
            return plain;
        };
        if sections.titled(front).is_none() || sections.is_level_name(word) {
            return plain;
        }
        match self.segments_of(&slug) {
            Some(work) if work.resolve_in(&slug, &addressed).is_some() => named,
            _ => plain,
        }
    }

    /// What this work's schema calls the parts of its address, read once.
    ///
    /// Beside `known` rather than inside it: the segments are read from
    /// whichever root holds the text and the schema is only ever under the
    /// corpus, and a sefer of the reader's own has segments and no schema at
    /// all.
    fn sections_of(&self, slug: &str) -> Arc<Sections> {
        if let Some(found) = self.schemas.lock().ok().and_then(|s| s.get(slug).cloned()) {
            return found;
        }
        let read = Arc::new(Sections::of_work(&self.root, slug));
        if let Ok(mut schemas) = self.schemas.lock() {
            schemas.insert(slug.to_string(), Arc::clone(&read));
        }
        read
    }

    /// Spellings that look like what was typed, and how many were not shown.
    ///
    /// Prefix matching, both ways round, over the normal forms — so `ברכ`
    /// offers `ברכות` and `ברכות רבה` offers `ברכות`. Not an edit distance:
    /// this is a list a reader picks from, and *starts with what you typed* is
    /// a rule they can see working.
    fn close_to(&self, typed: &str) -> (Vec<NearMiss>, usize) {
        let normal = girsa_hebrew::normalize(typed);
        let head = normal.split_whitespace().next().unwrap_or_default();
        if head.chars().count() < 2 {
            return (Vec::new(), 0);
        }
        let hits: Vec<&(String, String, String)> = self
            .spellings
            .iter()
            .filter(|(_, _, normal)| normal.starts_with(head) || head.starts_with(normal))
            .collect();
        // One spelling per work, keeping the closest. `dedup_by` only ever
        // compares *neighbours*, and the sort below is by length — so two
        // spellings of one sefer whose lengths differed were never neighbours,
        // both survived, and the suggestion list offered the same sefer twice.
        let mut closest: std::collections::HashMap<&str, &(String, String, String)> =
            std::collections::HashMap::new();
        for hit in hits {
            match closest.get(hit.1.as_str()) {
                Some(held)
                    if (held.0.chars().count(), held.0.as_str())
                        <= (hit.0.chars().count(), hit.0.as_str()) => {}
                _ => {
                    closest.insert(hit.1.as_str(), hit);
                }
            }
        }
        let mut kept: Vec<&&(String, String, String)> = closest.values().collect();
        kept.sort_by_key(|(spelling, _, _)| (spelling.chars().count(), spelling.clone()));
        let cut = kept.len().saturating_sub(MOST_SUGGESTIONS);
        (
            kept.into_iter()
                .take(MOST_SUGGESTIONS)
                .map(|(spelling, slug, _)| NearMiss::OtherTitle {
                    spelling: spelling.clone(),
                    slug: slug.clone(),
                })
                .collect(),
            cut,
        )
    }
}

/// A private extension: resolve a ref inside one work's addresses.
///
/// [`WorkSegments`] answers about a work it does not know the name of, and
/// [`girsa_corpus::index::SegmentIndex`] is the whole corpus. This is the one
/// work in between, which is all a citation needs.
trait ResolveIn {
    fn resolve_in(&self, slug: &str, reference: &Ref) -> Option<Run>;
}

impl ResolveIn for WorkSegments {
    fn resolve_in(&self, slug: &str, reference: &Ref) -> Option<Run> {
        let mut one = girsa_corpus::index::SegmentIndex::default();
        one.insert(slug, self.clone());
        one.resolve(reference)
    }
}

/// `variant \t slug \t he \t en` — the shape `Lexicon::from_tsv` reads.
/// `(spelling, slug, normalized spelling)`.
///
/// The third is computed here — once, at open — because it is a property of the
/// spelling and nothing else. The near-miss scan used to call
/// `girsa_hebrew::normalize` on every one of them **per unresolved lookup**,
/// over a 3.7 MB lexicon, to compare against a head word that does not change.
fn read_spellings(tsv: &str) -> Vec<(String, String, String)> {
    tsv.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let variant = fields.next()?.trim();
            let slug = fields.next()?.trim();
            (!variant.is_empty() && !slug.is_empty()).then(|| {
                (
                    variant.to_string(),
                    slug.to_string(),
                    girsa_hebrew::normalize(variant),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_corpus::segment::{Ordinal, SegmentId};

    /// A shelf with one sefer of ten simanim, three se'ifim each, and a
    /// lexicon that spells it three ways — one of which it shares with a work
    /// that is catalogued and not here.
    fn shelf(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a root");
        std::fs::write(
            root.join("lexicon.tsv"),
            "# a lexicon\n\
             שולחן ערוך אורח חיים\tshulchan-arukh/orach-chayim\tשולחן ערוך\tShulchan Arukh\n\
             שוע אוח\tshulchan-arukh/orach-chayim\tשולחן ערוך\tShulchan Arukh\n\
             אוח\tshulchan-arukh/orach-chayim\tשולחן ערוך\tShulchan Arukh\n\
             אוח\ttur/orach-chayim\tטור\tTur\n\
             ברכות\tbavli/berakhot\tברכות\tBerakhot\n",
        )
        .expect("a lexicon");

        let mut body = String::new();
        let mut n = 0u32;
        for siman in 1..=10u32 {
            for seif in 1..=3u32 {
                n += 1;
                let id = SegmentId::new(
                    "shulchan-arukh/orach-chayim",
                    vec![siman.to_string(), seif.to_string()],
                    Ordinal::root(n),
                );
                body.push_str(&format!(
                    "{{\"id\":\"{id}\",\"kind\":\"text\",\"text\":\"סימן {siman} סעיף {seif}\"}}\n"
                ));
            }
        }
        let dir = import::work_dir(&root, "shulchan-arukh/orach-chayim");
        std::fs::create_dir_all(&dir).expect("a work dir");
        std::fs::write(dir.join("segments.jsonl"), body).expect("segments");
        root
    }

    #[test]
    fn a_mareh_makom_lands_on_the_segments_it_names() {
        let root = shelf("girsa-citation-jump");
        let bar = Citations::open(&root, None).expect("a lexicon");
        let landing = bar.look_up("שוע אוח א ב", &Context::default());
        let place = landing.only().expect("one place");
        assert_eq!(
            place.run.first.to_string(),
            "girsa:shulchan-arukh/orach-chayim/1:2#2"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_citation_that_could_be_two_seforim_is_a_choice_and_not_a_pick() {
        // `או"ח` is the Orach Chayim of the Shulchan Arukh and of the Tur.
        // BUILDER.md rule 6: the honest answer is both.
        let root = shelf("girsa-citation-choice");
        let bar = Citations::open(&root, None).expect("a lexicon");
        let landing = bar.look_up("אוח א א", &Context::default());
        assert!(matches!(landing.resolution, Resolution::Ambiguous(_)));
        assert!(landing.is_a_choice());
        assert_eq!(landing.only(), None, "no caller can take the first one");
        // One of the two is here; the other is catalogued and not on the shelf,
        // and that is said rather than dropped.
        assert_eq!(landing.places.len(), 1);
        assert!(landing.near.iter().any(
            |n| matches!(n, NearMiss::NotOnTheShelf { work, .. } if work == "tur/orach-chayim")
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_address_the_sefer_does_not_have_offers_the_sefer_rather_than_the_nearest_page() {
        // The failure this whole mode is arranged against: it parses, it
        // resolves, it opens a page, and the page is the wrong one.
        let root = shelf("girsa-citation-nowhere");
        let bar = Citations::open(&root, None).expect("a lexicon");
        let landing = bar.look_up("שוע אוח תתקצט א", &Context::default());
        assert!(landing.places.is_empty(), "{:?}", landing.places);
        assert!(
            landing
                .near
                .iter()
                .any(|n| matches!(n, NearMiss::AddressNotThere { .. })),
            "{:?}",
            landing.near
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_partial_citation_is_completed_against_where_the_reader_is_standing() {
        // spec.md §4.3: "see se'if 5" while standing in siman 1 means 1:5 and
        // cannot mean anything else — the reader supplied the context by being
        // there.
        let root = shelf("girsa-citation-context");
        let bar = Citations::open(&root, None).expect("a lexicon");
        let context = Context {
            work: Some(vec!["shulchan-arukh".into(), "orach-chayim".into()]),
            address: girsa_ref::Address::parse("2:1"),
        };
        let landing = bar.look_up("שוע אוח ב ג", &context);
        assert_eq!(
            landing.only().expect("one place").run.first.to_string(),
            "girsa:shulchan-arukh/orach-chayim/2:3#6"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_resolved_offers_spellings_and_applies_none_of_them() {
        let root = shelf("girsa-citation-nearmiss");
        let bar = Citations::open(&root, None).expect("a lexicon");
        let landing = bar.look_up("ברכ", &Context::default());
        assert!(landing.places.is_empty());
        assert!(
            landing.near.iter().any(|n| matches!(
                n,
                NearMiss::OtherTitle { spelling, .. } if spelling == "ברכות"
            )),
            "{:?}",
            landing.near
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
