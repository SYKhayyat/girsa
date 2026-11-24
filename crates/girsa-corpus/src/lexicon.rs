//! The vocabulary the resolver is allowed to be asked about.
//!
//! Two files ship it, and **`girsa-import` writes both**. `lexicon.tsv` is every
//! spelling of every work Sefaria has a schema for, collected here by
//! [`from_schemas`]; `lexicon-otzaria.tsv` is the 978 works Sefaria never had,
//! written out of [`crate::work::Catalogue::otzaria_lexicon_rows`].
//!
//! The first one used to be `examples/build-lexicon`'s job and nothing else's —
//! an example is not a step you can put in an order, so `tools/build-a-shelf.mjs`
//! did not have it and a reader who followed the one-command shelf build got
//! `cannot load the lexicon` from `girsa-link-import` (#65). An input that a
//! later step requires belongs in the step that produces what it describes, not
//! in a program the documentation has to remember. The example is still there
//! for regenerating one file without a full import, and it calls [`from_schemas`]
//! rather than carrying the second copy this module's six-copies paragraph is
//! about. Reading the first and
//! appending the second is nine lines, and there were **six hand-written
//! copies** of those nine lines — the window's linkify, the search bar's
//! citation mode, `girsa-link-import`, `why-dropped`, and the two examples that
//! send a citation to the pen.
//!
//! Six copies had already drifted. Four joined the two files with `\n` between
//! them and two concatenated them bare, so a `lexicon.tsv` that did not happen
//! to end in a newline would have glued its last title onto Otzaria's first and
//! produced one row naming neither work. `build-lexicon` does end its file with
//! a newline, which makes those two correct by luck rather than by
//! construction — and luck is not a property you can check.
//!
//! And none of the six knew your own seforim existed. A sefer you dropped on
//! the shelf was opened by title and filed by title and could not be *cited* by
//! name, because the only vocabulary the resolver ever saw came out of the
//! corpus. That is the gap this module closes, and it is why it is one loader
//! rather than a seventh copy.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use girsa_ref::Lexicon;

use crate::work::{Source, Work};

/// Every title the resolver may be asked about, and which of them are yours.
///
/// Holds the TSV rather than the built [`Lexicon`], because one caller — the
/// citation resolver's near-miss list — walks the rows itself and would
/// otherwise have to ask the lexicon a question the lexicon exists not to
/// answer.
#[derive(Debug, Default, Clone)]
pub struct Titles {
    tsv: String,
    mine: BTreeSet<String>,
}

impl Titles {
    /// What the corpus shipped, and nothing else.
    ///
    /// For the two callers that resolve **Sefaria's own citations against
    /// Sefaria's own corpus**: `girsa-link-import`, which turns 4.1 million
    /// rows of `links0.csv` into edges, and `why-dropped`, which measures that
    /// same run. No row in Sefaria's link export names a sefer of yours, so
    /// your titles could not help there — and a title of yours that collided
    /// with one of Sefaria's would turn a lookup that resolved into one that is
    /// ambiguous, which drops the edge. Your layer is not an improvement in
    /// those two; it is noise with a cost.
    ///
    /// # Errors
    ///
    /// If `lexicon.tsv` is not there. That is a corpus which has not been
    /// imported, and every citation into it would go unresolved — which reads
    /// exactly like a shelf that does not have the sefer, so it is refused with
    /// the path instead.
    pub fn of(root: &Path) -> Result<Self, std::io::Error> {
        Self::read(root, None)
    }

    /// The corpus, plus the seforim and the notes in your own layer.
    ///
    /// For every caller where a **person typed a title**: the search bar's
    /// citation mode, the window's linkify, and the two examples that send a
    /// citation to the pen. A reader who put a sefer on their own shelf and
    /// then typed its name meant that sefer.
    ///
    /// A personal root that is not there yet is not an error — it is a fresh
    /// install, and it contributes no rows.
    ///
    /// # Errors
    ///
    /// As [`Titles::of`]: only the corpus half is required.
    pub fn across(root: &Path, personal: &Path) -> Result<Self, std::io::Error> {
        Self::read(root, Some(personal))
    }

    fn read(root: &Path, personal: Option<&Path>) -> Result<Self, std::io::Error> {
        let mut tsv = std::fs::read_to_string(root.join("lexicon.tsv"))?;
        // Missing is survivable, and deliberately so: it costs the 978
        // Otzaria-only works, and a citation into one of them then does not
        // resolve — the honest outcome for a shelf that was never imported
        // from Otzaria.
        if let Ok(more) = std::fs::read_to_string(root.join("lexicon-otzaria.tsv")) {
            append(&mut tsv, &more);
        }
        let mut mine = BTreeSet::new();
        if let Some(personal) = personal {
            let rows = own_rows(personal, &mut mine);
            append(&mut tsv, &rows);
        }
        Ok(Self { tsv, mine })
    }

    /// The rows, as `girsa_ref::Lexicon::from_tsv` reads them.
    #[must_use]
    pub fn tsv(&self) -> &str {
        &self.tsv
    }

    /// The resolver's map, built from those rows.
    #[must_use]
    pub fn lexicon(&self) -> Lexicon {
        Lexicon::from_tsv(&self.tsv)
    }

    /// Whether this slug's text lives under the personal root.
    ///
    /// A resolved citation is a slug and an address, and the next thing any
    /// caller does is go and read that work's segments — from the corpus root
    /// for a sefer the corpus shipped and from yours for a sefer you wrote.
    /// Answered from the catalogue that was actually read rather than by
    /// trying one root and falling back to the other: a fallback cannot tell
    /// *your sefer* from *a corpus sefer that failed to load*, and it would
    /// report the second as the first.
    #[must_use]
    pub fn is_mine(&self, slug: &str) -> bool {
        self.mine.contains(slug)
    }

    /// The slugs that came out of your own layer.
    ///
    /// Handed out whole so a caller that keeps the answer around does not have
    /// to keep the 3.7 MB of TSV around with it. The search bar's citation mode
    /// is that caller, and it reads the lexicon lazily precisely because those
    /// megabytes are worth not holding.
    #[must_use]
    pub fn mine(&self) -> &BTreeSet<String> {
        &self.mine
    }
}

/// Join one file's rows onto another's, with a line between them.
fn append(tsv: &mut String, more: &str) {
    if !tsv.is_empty() && !tsv.ends_with('\n') {
        tsv.push('\n');
    }
    tsv.push_str(more);
}

/// One row per sefer in your own catalogue, in the shape the lexicon reads.
///
/// `variant \t slug \t he \t en`, the same four fields
/// [`crate::work::Catalogue::otzaria_lexicon_rows`] writes, so the personal
/// half is not a second format to keep in step with the first.
///
/// **Everything in `personal/works/index.jsonl`**: a file you dropped in, a
/// `.ksav` read onto the shelf, and a note — which this library holds to be a
/// sefer of yours and not a lesser thing (spec.md §5), so it is one here too.
/// A note titled the same as a masechta does not shadow it: `Lexicon::lookup`
/// returns **both** works for a shared spelling and the resolver offers the
/// choice, which is already what it does for או"ח in the Shulchan Arukh and in
/// the Tur.
fn own_rows(personal: &Path, mine: &mut BTreeSet<String>) -> String {
    let Ok(body) = std::fs::read_to_string(personal.join("works/index.jsonl")) else {
        return String::new();
    };
    let mut out = String::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(work) = serde_json::from_str::<Work>(line) else {
            continue;
        };
        if work.source != Source::Mine {
            continue;
        }
        mine.insert(work.slug.clone());
        let he = one_line(&work.he_title);
        let en = one_line(&work.en_title);
        // Both spellings, and each of them once. A sefer added by
        // `import::mine::add` and a note both carry the one title in both
        // fields, so the two are usually the same string; a `.ksav` or a
        // renamed sefer can carry two, and both are ways somebody would type
        // it.
        let mut variants: Vec<&str> = Vec::new();
        if !he.is_empty() {
            variants.push(&he);
        }
        if !en.is_empty() && en != he {
            variants.push(&en);
        }
        for variant in variants {
            out.push_str(variant);
            out.push('\t');
            out.push_str(&work.slug);
            out.push('\t');
            out.push_str(&he);
            out.push('\t');
            out.push_str(&en);
            out.push('\n');
        }
    }
    out
}

/// A title on one line, with no tab anywhere in it.
///
/// A field separator inside a field shifts every field after it, so a title
/// with a tab in it would hand its own second half to the lexicon as a slug and
/// map the sefer onto a work that does not exist. `build-lexicon` does the same
/// to Sefaria's variants for the same reason; the difference is that these
/// titles were **typed by a person**, which is where a stray tab actually comes
/// from — a name pasted out of a spreadsheet.
fn one_line(title: &str) -> String {
    title.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How a reader spells what Sefaria catalogues (#69).
///
/// Sefaria's variants are exhaustive about **its** spelling — `Berachos`,
/// `Shulchan Arukh, Orach Chayim`, `Mishneh Torah, Repentance` — and #63's one
/// box takes what a *person* types. Measured through the resolver against the
/// shipped lexicon before this table existed: `brachos`, `shabbos`,
/// `mishnayos peah`, `shulchan aruch orech chaim` and `rambam hilchos teshuva`
/// were all unresolved, while the same citations in Hebrew were fine.
///
/// Every entry is a substitution, never a similarity score. `Brachos` and
/// `Berakhos` are one letter apart and so are `Bava Basra` and `Bava Batra`; a
/// threshold that catches the first eventually opens the second, and a wrong
/// sefer is the failure this whole system is arranged to prevent. A variant made
/// here can only turn *unresolved* into *resolved* or *ambiguous*, and ambiguous
/// is still shown to the reader as a choice.
pub const READER: &[(&str, &str)] = &[
    ("Berakhot", "Berachos"),
    ("Berachos", "Brachos"),
    ("Shabbat", "Shabbos"),
    ("Mishnah", "Mishnayos"),
    ("Hilchot", "Hilchos"),
    ("Orach Chayim", "Orech Chaim"),
    ("Orach Chayim", "Orach Chaim"),
    ("Arukh", "Aruch"),
    ("Chayim", "Chaim"),
];

/// The spellings [`READER`] produces from the ones a schema offers.
///
/// To a fixed point rather than one pass: `Shulchan Arukh, Orach Chayim` needs
/// two substitutions at once to become the `Shulchan Aruch, Orech Chaim` a
/// reader types, and one pass reaches each but never both. `Rambam` is added as
/// a prefix because the work is catalogued under its own name and typed under
/// the author's — which is also how `rambam hilchos teshuva` is reached, from a
/// variant that spells the subsection `Hilchot Teshuva`.
#[must_use]
pub fn reader_spellings(variants: &[String]) -> Vec<String> {
    let mut mine: Vec<String> = Vec::new();
    let mut frontier: Vec<String> = variants.to_vec();
    for _ in 0..3 {
        let mut next: Vec<String> = Vec::new();
        for variant in &frontier {
            for (says, reader_says) in READER {
                if variant.contains(says) {
                    next.push(variant.replace(says, reader_says));
                }
            }
            if let Some(section) = variant.strip_prefix("Mishneh Torah, ") {
                next.push(format!("Rambam {section}"));
            }
        }
        if next.is_empty() {
            break;
        }
        mine.extend(next.iter().cloned());
        frontier = next;
    }
    mine
}

/// What [`from_schemas`] made, and what it could not read.
///
/// The counts are not decoration: a schema that fails to parse is a sefer whose
/// citations will not resolve, and both callers print the number so a mysteriously
/// bad resolver at one shelf has a line to point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    /// The whole file, header and rows, ready to be written.
    pub tsv: String,
    /// Works that contributed at least their own two spellings.
    pub works: usize,
    /// Schema files that would not read or would not parse.
    pub skipped: usize,
}

/// `lexicon.tsv` from Sefaria's `schemas/` — the resolver's starting vocabulary.
///
/// spec.md §4.3: the resolver is *"seeded from Sefaria's `titles` and
/// `match_templates`, which is most of the lexicon for free"*. Every way each
/// work is written, in both languages, out of the files that already list them:
/// `Shulchan_Arukh,_Orach_Chayim.json` alone contributes 44 spellings, because
/// the spellings a reader actually types live in the schema's nested `titles`
/// and not only in its two headline names.
///
/// Sorted, so the output is byte-identical between runs and a regenerated
/// lexicon shows an empty diff when nothing changed.
///
/// # Errors
///
/// If `schema_dir` cannot be read at all. One unreadable schema is counted in
/// [`Built::skipped`] instead — a sefer the resolver cannot be asked about is a
/// smaller failure than no lexicon at all.
pub fn from_schemas(schema_dir: &Path) -> Result<Built, std::io::Error> {
    // variant → slug, and slug → (he title, en title).
    let mut rows: BTreeMap<(String, String), (String, String)> = BTreeMap::new();
    let mut works = 0usize;
    let mut skipped = 0usize;

    let mut entries: Vec<_> = std::fs::read_dir(schema_dir)?.flatten().collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            skipped += 1;
            continue;
        };
        let Ok(schema) = serde_json::from_str::<serde_json::Value>(&text) else {
            skipped += 1;
            continue;
        };

        let Some(en_title) = schema.get("title").and_then(serde_json::Value::as_str) else {
            skipped += 1;
            continue;
        };
        let he_title = schema
            .get("heTitle")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(en_title);
        let categories = strings_of(schema.get("categories"));
        works += 1;

        for variant in variants_of(&schema, en_title, he_title) {
            let variant = without_markup(&variant);
            if variant.is_empty() {
                continue;
            }
            rows.insert(
                (variant, crate::work::slug_of(en_title, &categories)),
                (he_title.to_string(), en_title.to_string()),
            );
        }
    }

    let mut tsv = String::new();
    tsv.push_str(
        "# girsa-ref lexicon. GENERATED from Sefaria's schemas by \
         girsa_corpus::lexicon::from_schemas, written by girsa-import.\n\
         # variant\tslug\the-title\ten-title\n",
    );
    for ((variant, slug), (he, en)) in &rows {
        tsv.push_str(&format!("{variant}\t{slug}\t{he}\t{en}\n"));
    }
    Ok(Built {
        tsv,
        works,
        skipped,
    })
}

/// Every spelling a schema offers, before the markup is taken off it.
fn variants_of(schema: &serde_json::Value, en_title: &str, he_title: &str) -> Vec<String> {
    let mut out = vec![en_title.to_string(), he_title.to_string()];
    out.extend(strings_of(schema.get("titleVariants")));
    out.extend(strings_of(schema.get("heTitleVariants")));

    // The nested node carries its own `titles`, as `{text, lang}` pairs. This
    // is where the 44 spellings of Orach Chayim actually live.
    if let Some(titles) = schema
        .pointer("/schema/titles")
        .and_then(serde_json::Value::as_array)
    {
        out.extend(
            titles
                .iter()
                .filter_map(|t| t.get("text").and_then(serde_json::Value::as_str))
                .map(str::to_string),
        );
    }

    out.extend(reader_spellings(&out));
    out
}

fn strings_of(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Strip the markup Sefaria's title lists carry.
///
/// Several variants arrive wrapped in `<i>…</i>` — the transliterated ones,
/// `<i>Shulḥan Arukh, Oraḥ Ḥayyim</i>`. Left in, the tag becomes part of the
/// spelling and the variant matches nothing.
fn without_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    use std::path::PathBuf;

    /// A corpus with one sefer in its lexicon, and no trailing newline on it.
    ///
    /// Written without the newline on purpose: it is the shape two of the six
    /// copies assumed away.
    fn corpus(dir: &Path, tail: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("lexicon.tsv"),
            format!("ברכות\tbavli/berakhot\tברכות\tBerakhot{tail}"),
        )
        .unwrap();
        std::fs::write(
            dir.join("lexicon-otzaria.tsv"),
            "אור החיים\totzaria/or-hachaim\tאור החיים\tOr HaChaim\n",
        )
        .unwrap();
    }

    fn catalogued(personal: &Path, works: &[Work]) {
        std::fs::create_dir_all(personal.join("works")).unwrap();
        let body: String = works
            .iter()
            .map(|w| format!("{}\n", serde_json::to_string(w).unwrap()))
            .collect();
        std::fs::write(personal.join("works/index.jsonl"), body).unwrap();
    }

    fn mine_named(slug: &str, title: &str) -> Work {
        Work {
            slug: slug.to_string(),
            he_title: title.to_string(),
            en_title: title.to_string(),
            categories: vec!["שלי".to_string()],
            order: Vec::new(),
            source: Source::Mine,
            origin: PathBuf::new(),
            schema: None,
            he_sections: Vec::new(),
            author: None,
            era: None,
            comp_date: None,
            version: None,
            commentary_on: Vec::new(),
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-lexicon-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_sefer_of_yours_can_be_named() {
        let dir = tmp("mine");
        corpus(&dir.join("corpus"), "\n");
        catalogued(
            &dir.join("personal"),
            &[mine_named("mine/kuntres", "קונטרס הביאורים")],
        );

        let titles = Titles::across(&dir.join("corpus"), &dir.join("personal")).unwrap();
        let lexicon = titles.lexicon();
        let found = lexicon.lookup("קונטרס הביאורים");
        assert_eq!(found.len(), 1, "the title resolves to exactly one work");
        assert_eq!(found[0].slug, "mine/kuntres");
        assert!(titles.is_mine("mine/kuntres"));
        assert!(!titles.is_mine("bavli/berakhot"));
        assert_eq!(titles.mine().len(), 1);
    }

    #[test]
    fn the_corpus_alone_does_not_see_your_layer() {
        let dir = tmp("corpus-only");
        corpus(&dir.join("corpus"), "\n");
        catalogued(
            &dir.join("personal"),
            &[mine_named("mine/kuntres", "קונטרס הביאורים")],
        );

        // `Titles::of` is what `girsa-link-import` reads Sefaria's own link
        // export with. A title of yours there could only make one of Sefaria's
        // citations ambiguous.
        let titles = Titles::of(&dir.join("corpus")).unwrap();
        assert!(titles.lexicon().lookup("קונטרס הביאורים").is_empty());
        assert!(titles.mine().is_empty());
        assert_eq!(titles.lexicon().lookup("ברכות").len(), 1);
    }

    #[test]
    fn a_title_of_yours_that_collides_shadows_nothing() {
        let dir = tmp("collide");
        corpus(&dir.join("corpus"), "\n");
        catalogued(
            &dir.join("personal"),
            &[mine_named("mine/berakhot", "ברכות")],
        );

        // Two works, both offered. The resolver's own answer for a shared
        // spelling, and the one thing it must not do is pick.
        let titles = Titles::across(&dir.join("corpus"), &dir.join("personal")).unwrap();
        let lexicon = titles.lexicon();
        let found = lexicon.lookup("ברכות");
        assert_eq!(found.len(), 2);
        let slugs: BTreeSet<&str> = found.iter().map(|w| w.slug.as_str()).collect();
        assert!(slugs.contains("bavli/berakhot"));
        assert!(slugs.contains("mine/berakhot"));
    }

    #[test]
    fn the_two_shipped_halves_are_joined_by_a_line_and_not_by_luck() {
        let dir = tmp("no-trailing-newline");
        // No newline at the end of `lexicon.tsv`. Concatenated bare — which is
        // what two of the six copies did — the last Sefaria row and the first
        // Otzaria row become `…Berakhotאור החיים\totzaria/or-hachaim…`, and
        // both works go missing at once.
        corpus(&dir.join("corpus"), "");

        let titles = Titles::of(&dir.join("corpus")).unwrap();
        let lexicon = titles.lexicon();
        assert_eq!(lexicon.lookup("ברכות").len(), 1, "the last Sefaria row");
        assert_eq!(
            lexicon.lookup("אור החיים").len(),
            1,
            "the first Otzaria row"
        );
    }

    #[test]
    fn a_tab_typed_into_a_title_does_not_become_a_field() {
        let dir = tmp("tab");
        corpus(&dir.join("corpus"), "\n");
        catalogued(
            &dir.join("personal"),
            &[mine_named("mine/pasted", "שיעורים\tעל ברכות")],
        );

        let titles = Titles::across(&dir.join("corpus"), &dir.join("personal")).unwrap();
        let lexicon = titles.lexicon();
        let found = lexicon.lookup("שיעורים על ברכות");
        assert_eq!(found.len(), 1, "the whole title is one variant");
        assert_eq!(found[0].slug, "mine/pasted");
    }

    #[test]
    fn a_personal_root_that_is_not_there_yet_is_not_an_error() {
        let dir = tmp("fresh");
        corpus(&dir.join("corpus"), "\n");

        let titles = Titles::across(&dir.join("corpus"), &dir.join("personal")).unwrap();
        assert!(titles.mine().is_empty());
        assert_eq!(titles.lexicon().lookup("ברכות").len(), 1);
    }

    #[test]
    fn a_corpus_with_no_lexicon_is_refused_and_not_answered_empty() {
        let dir = tmp("bare");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(Titles::of(&dir).is_err());
    }

    /// The five citations #63 measured as unresolved, spelled the way a reader
    /// spells them rather than the way Sefaria catalogues them.
    #[test]
    fn the_spellings_a_reader_types_are_made_from_the_ones_sefaria_offers() {
        let offered = [
            "Berakhot",
            "Shabbat",
            "Mishnah Pe'ah",
            "Shulchan Arukh, Orach Chayim",
            "Mishneh Torah, Hilchot Teshuva",
            "Mishneh Torah, Repentance",
        ]
        .map(str::to_string);

        let mine = reader_spellings(&offered);
        for wanted in [
            "Brachos",
            "Shabbos",
            "Mishnayos Pe'ah",
            // Both substitutions at once, which one pass cannot reach.
            "Shulchan Aruch, Orech Chaim",
            // Substitution then prefix, which is the other composition.
            "Rambam Hilchos Teshuva",
            "Rambam Repentance",
        ] {
            assert!(mine.iter().any(|v| v == wanted), "{wanted} was not made");
        }
    }

    /// The one thing this must not be. `Bava Basra` is one letter from `Bava
    /// Batra` and a similarity threshold would open the wrong sefer, so an
    /// unlisted spelling produces nothing at all.
    #[test]
    fn an_unlisted_spelling_is_not_reached_by_being_close() {
        let mine = reader_spellings(&["Bava Batra".to_string()]);
        assert!(
            mine.iter().all(|v| !v.contains("Basra")),
            "a near miss was made"
        );
    }

    /// `girsa-import` writes this file now, so the collection itself is tested
    /// here rather than only through an example nothing runs (#65).
    #[test]
    fn the_lexicon_carries_every_way_a_work_is_written() {
        let dir = std::env::temp_dir().join("girsa-lexicon-from-schemas");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("schemas")).unwrap();
        // The shape Sefaria ships: a headline name in each language, and the
        // spellings a reader types nested under `schema.titles`, markup and all.
        std::fs::write(
            dir.join("schemas/Shulchan_Arukh,_Orach_Chayim.json"),
            r#"{"title": "Shulchan Arukh, Orach Chayim",
                "heTitle": "שולחן ערוך, אורח חיים",
                "categories": ["Halakhah"],
                "titleVariants": ["<i>Shulḥan Arukh, Oraḥ Ḥayyim</i>"],
                "schema": {"titles": [{"text": "Shulchan Aruch OC", "lang": "en"}]}}"#,
        )
        .unwrap();
        // And one that will not parse: counted, not fatal.
        std::fs::write(dir.join("schemas/Broken.json"), "{not json").unwrap();

        let built = from_schemas(&dir.join("schemas")).unwrap();
        assert_eq!(built.works, 1, "one schema named a work");
        assert_eq!(built.skipped, 1, "and the other is counted, not ignored");

        for variant in [
            "Shulchan Arukh, Orach Chayim",
            "Shulchan Aruch OC",
            // The markup is off the transliterated variant, or it matches
            // nothing a reader types.
            "Shulḥan Arukh, Oraḥ Ḥayyim",
            // And the reader's own spellings, from the same list.
            "Shulchan Aruch, Orech Chaim",
        ] {
            assert!(
                built.tsv.contains(&format!("\t{variant}\t"))
                    || built
                        .tsv
                        .lines()
                        .any(|line| line.starts_with(&format!("{variant}\t"))),
                "{variant} is not in it:\n{}",
                built.tsv
            );
        }
        assert!(
            !built.tsv.contains("<i>"),
            "markup in a spelling matches nothing:\n{}",
            built.tsv
        );
        assert!(
            built.tsv.ends_with('\n'),
            "so appending Otzaria's rows cannot glue"
        );

        // Byte-stable, so a regenerated lexicon diffs empty when nothing moved.
        let again = from_schemas(&dir.join("schemas")).unwrap();
        assert_eq!(built.tsv, again.tsv);
    }

    /// Two runs of one function, not two implementations of one file: the
    /// example that used to own this writes what the importer writes.
    #[test]
    fn the_lexicon_is_a_file_and_not_a_second_copy() {
        let dir = std::env::temp_dir().join("girsa-lexicon-one-copy");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("schemas")).unwrap();
        std::fs::write(
            dir.join("schemas/Berakhot.json"),
            r#"{"title": "Berakhot", "heTitle": "ברכות", "categories": ["Talmud"]}"#,
        )
        .unwrap();
        let built = from_schemas(&dir.join("schemas")).unwrap();
        let header = built.tsv.lines().next().unwrap();
        assert!(
            header.contains("from_schemas") && header.contains("girsa-import"),
            "the file says who writes it, and both do: {header}"
        );
    }
}
