//! The vocabulary the resolver is allowed to be asked about.
//!
//! Two files ship it. `lexicon.tsv` is every spelling of every work Sefaria has
//! a schema for, collected by `examples/build-lexicon`; `lexicon-otzaria.tsv`
//! is the 978 works Sefaria never had, written by `girsa-import` out of
//! [`crate::work::Catalogue::otzaria_lexicon_rows`]. Reading the first and
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

use std::collections::BTreeSet;
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
}
