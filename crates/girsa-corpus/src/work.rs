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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// Text and structure from `gs://sefaria-export`.
    Sefaria,
    /// Text from the Otzaria `.txt` tree, structured from its own headings.
    Otzaria,
}

impl Source {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sefaria => "sefaria",
            Self::Otzaria => "otzaria",
        }
    }
}

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
                    slug.push('-');
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
    /// Read both corpora and decide, per work, where its text comes from.
    ///
    /// `sefaria_root` is the directory holding `schemas/` and `json/`;
    /// `otzaria_root` is the one holding `אוצריא/` and `metadata.json`.
    ///
    /// # Errors
    ///
    /// If either root cannot be read at all. A single unreadable schema is
    /// counted and skipped — one sefer missing is a smaller failure than no
    /// catalogue — and the count is returned so the caller can be loud.
    pub fn build(
        sefaria_root: &Path,
        otzaria_root: &Path,
    ) -> Result<(Self, usize), CatalogueError> {
        let (sefaria, skipped) = read_sefaria(sefaria_root)?;
        let otzaria = read_otzaria(otzaria_root)?;

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
fn read_sefaria(root: &Path) -> Result<(Vec<Work>, usize), CatalogueError> {
    let schemas = root.join("schemas");
    if !schemas.is_dir() {
        return Err(CatalogueError::Missing(schemas.display().to_string()));
    }
    let texts = index_hebrew_texts(&root.join("json"))?;

    let mut works = Vec::new();
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
            categories,
            source: Source::Sefaria,
            origin: origin.clone(),
            schema: Some(path.clone()),
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
        });
    }
    Ok((works, skipped))
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

/// Every `.txt` in Otzaria's `אוצריא/` tree, with what `metadata.json` knows.
fn read_otzaria(root: &Path) -> Result<Vec<Work>, CatalogueError> {
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
                author: meta.and_then(|m| m.author.clone()),
                era: None,
                comp_date: meta.and_then(|m| m.comp_date.clone()),
                version: Some(Version {
                    edition: "Otzaria".to_string(),
                    provenance: Some("https://github.com/Sivan22/otzaria-library".to_string()),
                    license: Some("Unlicense".to_string()),
                }),
            });
        }
    }
    Ok(works)
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
