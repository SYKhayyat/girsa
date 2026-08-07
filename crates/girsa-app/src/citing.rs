//! *Where did I use this?* — and turning prose full of citations into refs.
//!
//! spec.md §10.4, BUILDER.md W19. Two halves of closing the loop, and they are
//! the same fact seen twice: **a document that stores refs can be asked
//! questions about places**, and a document that stores only printed strings
//! cannot be asked anything at all.
//!
//! # Where did I use this
//!
//! Standing on a passage in Girsa, see which of your own documents cite it. It
//! is a scan of your own layer for `מקור:` refs (see [`girsa_ksav::refs_in`]),
//! and it is cheap because the refs are already there — nothing has to be
//! parsed back out of prose, and nothing has to be guessed.
//!
//! # Linkify
//!
//! **High-confidence patterns only** (spec.md §10.5, decision 12). Prose full
//! of citations becomes live refs, and *anything ambiguous stays plain text*.
//! The rules are deliberately narrow:
//!
//! 1. the resolver must return [`Resolution::Exact`] — a citation that names
//!    two seforim is left alone, not narrowed to the first;
//! 2. the citation must carry an **address**. A bare title in prose is usually
//!    a subject and not a mekor: *the Mishnah Berurah writes at length* is not
//!    a citation of anything, and linking it would put a ref on a sentence
//!    nobody was citing in;
//! 3. **every level of that address is a number or a daf.** The resolver will
//!    read a named level, which is right for a citation somebody typed into a
//!    box and wrong for prose: `ברכות ב. ועיין שו"ע` resolves as Berakhot at a
//!    section called `ועיין שו"ע`, swallowing the next citation whole. In prose
//!    a mekor is a title and numbers.
//!
//! Both of those refuse more than they accept, on purpose. A wrong link in a
//! sefer somebody prints is worse than a plain string, and there is no way to
//! tell from the printed page which one it was.

use std::path::Path;

use girsa_ref::{resolve, Lexicon, Ref, Resolution};

use crate::buffer::Buffer;
use crate::documents::Documents;

/// One of your documents, and the places in it that cite something.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Citing {
    /// What you called it — a buffer's name, or the document's.
    pub name: String,
    /// The refs in it that answer the question asked.
    pub refs: Vec<String>,
    /// Where it is on disk, for a document the registry holds. `None` for a
    /// buffer in the toy editor, which lives in the personal layer under its
    /// name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The registry knows about it and the file is not there — a stick that is
    /// not plugged in, a folder that has not synced. **The cached refs are
    /// still answered from**, and this says they are cached.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub away: bool,
}

/// Which of your own documents cite this place.
///
/// A document cites a place if it stores a ref that **covers** it: a citation
/// of `1:1` answers a question about `1:1`, and a citation of the span
/// `1:1-1:3` answers a question about `1:2` as well. Anything less would miss
/// the commonest case, which is quoting a passage and asking about a line of
/// it.
///
/// # Two places to look, and one of them used to be the only one
///
/// The toy editor's buffers — `personal/ksav/*.ksav`, W17 — **and** the
/// documents the registry holds ([`crate::documents`]). This read the buffers
/// alone, so a `.ksav` written in the real Ksav, which is the application this
/// whole pairing exists for, was never found: the reader's actual work in the
/// actual editor answered *nothing cites this*.
///
/// The registry is not re-read here. [`crate::documents::Documents::refreshed`]
/// is the caller's to run, because it is a `stat` per document and this is
/// asked on a click.
#[must_use]
pub fn who_cites(personal: &Path, documents: &Documents, place: &Ref) -> Vec<Citing> {
    let answers = |refs: Vec<String>| -> Vec<String> {
        refs.into_iter()
            .filter(|text| {
                text.parse::<Ref>()
                    .map(|stored| covers(&stored, place))
                    .unwrap_or(false)
            })
            .collect()
    };

    let mut out = Vec::new();
    for name in Buffer::list(personal) {
        let Ok(buffer) = Buffer::open(personal, &name) else {
            continue;
        };
        let refs = answers(girsa_ksav::refs_in(&buffer.text));
        if !refs.is_empty() {
            out.push(Citing {
                name,
                refs,
                path: None,
                away: false,
            });
        }
    }
    for document in documents.all() {
        let refs = answers(document.refs.clone());
        if !refs.is_empty() {
            out.push(Citing {
                name: document.name.clone(),
                refs,
                path: Some(document.path.clone()),
                away: !document.is_here(),
            });
        }
    }
    out
}

/// Whether a stored ref covers a place.
///
/// Same work, and the address inside the span — where "inside" is settled by
/// [`girsa_ref::Address`]'s ordering, which is the same ordering the corpus is
/// in. A ref to a whole sefer covers every place in it, which is what a reader
/// means by citing a sefer.
#[must_use]
pub fn covers(stored: &Ref, place: &Ref) -> bool {
    if stored.work_slug() != place.work_slug() {
        return false;
    }
    if stored.from().is_empty() {
        return true;
    }
    match stored.to() {
        // `1:1-1:3` covers `1:2`. Compared as addresses rather than as text:
        // `1:10` sorts after `1:9`, which string comparison gets wrong.
        Some(to) => place.from() >= stored.from() && place.from() <= to,
        // A point covers itself, and covers anything under it: a citation of
        // siman 1 answers a question about se'if 3 of it. Compared level by
        // level, because `1` is a prefix of `10` as text and is not a prefix
        // of it as an address.
        None => {
            let (under, over) = (place.from().levels(), stored.from().levels());
            over.len() <= under.len() && over.iter().zip(under).all(|(a, b)| a == b)
        }
    }
}

/// A citation found in prose, and where it is.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Linked {
    /// Where it sits in the text, in **characters**.
    pub from: usize,
    pub to: usize,
    /// The citation as it was written.
    pub text: String,
    /// The ref it resolves to, exactly.
    pub reference: String,
}

/// The longest citation a lexicon can start with is this many words.
///
/// A title is up to `longest_variant_words` and the address after it is at
/// most a handful: `רמב"ם הלכות תפילה פרק ד' הלכה א'` is seven. The cap keeps
/// linkify linear in the length of the prose rather than quadratic.
const ADDRESS_WORDS: usize = 6;

/// Find the citations in a piece of prose — **the ones that are certain**.
///
/// See the module note for the two rules. What this returns is safe to turn
/// into links; everything else is left as somebody's words.
#[must_use]
pub fn linkify(lexicon: &Lexicon, prose: &str) -> Vec<Linked> {
    let words: Vec<(usize, &str)> = word_positions(prose);
    let longest = lexicon.longest_variant_words() + ADDRESS_WORDS;

    let mut out: Vec<Linked> = Vec::new();
    let mut at = 0;
    while at < words.len() {
        let mut taken = None;
        // Longest first: `שו"ע או"ח סימן א' סעיף ב'` has to beat `שו"ע`, or a
        // citation is linked to the whole Shulchan Arukh and lands on its
        // first page.
        for take in (1..=longest.min(words.len() - at)).rev() {
            let (start, _) = words[at];
            let (last_start, last_word) = words[at + take - 1];
            let end = last_start + last_word.chars().count();
            let text: String = prose.chars().skip(start).take(end - start).collect();

            // A leading prefix letter is how citations are written in prose:
            // *וכתב בשו"ע* is the commonest shape there is. Peeled only from
            // the first word, and only when what is left resolves exactly —
            // the rules below still have to hold, so this widens where a
            // citation is found and never what it is found to be.
            let (offset, reading) = match resolve(lexicon, text.trim()) {
                Resolution::Exact(_) => (0, text.trim().to_string()),
                _ => match peel(text.trim()) {
                    Some(peeled) => (1, peeled),
                    None => continue,
                },
            };
            let Resolution::Exact(reference) = resolve(lexicon, &reading) else {
                continue;
            };
            // Rule 2: a bare title in prose is a subject, not a mekor.
            if reference.from().is_empty() {
                continue;
            }
            // Rule 3: in prose, a mekor is a title and numbers. Anything else
            // is the resolver reading the rest of the sentence as a section
            // name.
            let numbered = |address: &girsa_ref::Address| {
                address.levels().iter().all(girsa_ref::Level::is_numbered)
            };
            if !numbered(reference.from()) || reference.to().is_some_and(|to| !numbered(to)) {
                continue;
            }
            taken = Some((
                take,
                Linked {
                    from: start + offset,
                    to: end,
                    text: reading,
                    reference: reference.to_string(),
                },
            ));
            break;
        }
        match taken {
            Some((take, linked)) => {
                out.push(linked);
                at += take;
            }
            None => at += 1,
        }
    }
    out
}

/// The same citation with a leading prefix letter taken off its first word.
///
/// `בשו"ע` → `שו"ע`. Only the eight letters that are prefixes in Hebrew, and
/// only when there is something left after them.
fn peel(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if !matches!(first, 'ו' | 'ה' | 'ב' | 'כ' | 'ל' | 'מ' | 'ש' | 'ד') {
        return None;
    }
    let rest = chars.as_str();
    (!rest.trim().is_empty()).then(|| rest.to_string())
}

/// Every word of the prose, with where it starts in characters.
fn word_positions(prose: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    let mut byte_start = 0;
    for (at, (byte, c)) in prose.char_indices().enumerate() {
        if c.is_whitespace() {
            if let Some(from) = start.take() {
                out.push((from, &prose[byte_start..byte]));
            }
        } else if start.is_none() {
            start = Some(at);
            byte_start = byte;
        }
    }
    if let Some(from) = start {
        out.push((from, &prose[byte_start..]));
    }
    out
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_ref::Work;

    fn lexicon() -> Lexicon {
        let mut lex = Lexicon::default();
        lex.add(
            Work {
                slug: "shulchan-arukh/orach-chayim".into(),
                he_title: "שולחן ערוך, אורח חיים".into(),
                en_title: "Shulchan Arukh, Orach Chayim".into(),
            },
            &["שו\"ע או\"ח", "שולחן ערוך אורח חיים", "או\"ח"],
        );
        lex.add(
            Work {
                slug: "tur/orach-chayim".into(),
                he_title: "טור, אורח חיים".into(),
                en_title: "Tur, Orach Chayim".into(),
            },
            // The same spelling as the Shulchan Arukh's volume: this is what
            // makes `או"ח` genuinely ambiguous, and it is true of the corpus.
            &["או\"ח"],
        );
        lex.add(
            Work {
                slug: "bavli/berakhot".into(),
                he_title: "ברכות".into(),
                en_title: "Berakhot".into(),
            },
            &["ברכות", "Berakhot"],
        );
        lex
    }

    fn r(text: &str) -> Ref {
        text.parse().expect("a ref")
    }

    #[test]
    fn a_citation_in_prose_becomes_a_ref() {
        let found = linkify(
            &lexicon(),
            "וכתב בשו\"ע או\"ח סימן א' סעיף ג' דראוי לכל ירא שמים",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].reference, "girsa:shulchan-arukh/orach-chayim/1:3");
        assert_eq!(found[0].text, "שו\"ע או\"ח סימן א' סעיף ג'");
    }

    #[test]
    fn a_citation_that_names_two_seforim_stays_plain_text() {
        // BUILDER rule 6 and spec.md §10.5: ambiguity is surfaced, never
        // resolved. `או"ח` is the Orach Chayim of the Shulchan Arukh *and* of
        // the Tur, and a linkify that picked one would be wrong half the time
        // in a way nobody could see on the printed page.
        assert!(linkify(&lexicon(), "עיין או\"ח סימן א'").is_empty());
    }

    #[test]
    fn a_bare_title_is_a_subject_and_not_a_mekor() {
        // *The Shulchan Arukh writes at length* is not a citation of anything.
        assert!(linkify(&lexicon(), "השולחן ערוך אורח חיים מאריך בזה").is_empty());
        assert!(linkify(&lexicon(), "ברכות היא מסכת ארוכה").is_empty());
    }

    #[test]
    fn the_longest_citation_wins_rather_than_the_first_title_that_matches() {
        let found = linkify(&lexicon(), "שולחן ערוך אורח חיים סימן קכ\"א סעיף ג'");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].reference,
            "girsa:shulchan-arukh/orach-chayim/121:3"
        );
    }

    #[test]
    fn two_citations_in_one_line_are_two_links() {
        let found = linkify(
            &lexicon(),
            "ברכות ב. ועיין שו\"ע או\"ח סימן נ\"ח סעיף א' בהגה",
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].reference, "girsa:bavli/berakhot/2a");
        assert_eq!(found[1].reference, "girsa:shulchan-arukh/orach-chayim/58:1");
        // And the spans are where they are: a linkifier that reported the
        // wrong offsets would put the link on the words beside the citation.
        let one: String = "ברכות ב. ועיין שו\"ע או\"ח סימן נ\"ח סעיף א' בהגה"
            .chars()
            .skip(found[1].from)
            .take(found[1].to - found[1].from)
            .collect();
        assert_eq!(one, found[1].text);
    }

    #[test]
    fn a_span_covers_the_lines_inside_it_and_nothing_outside() {
        assert!(covers(
            &r("girsa:bavli/berakhot/2a:1-2a:4"),
            &r("girsa:bavli/berakhot/2a:3")
        ));
        assert!(!covers(
            &r("girsa:bavli/berakhot/2a:1-2a:4"),
            &r("girsa:bavli/berakhot/2a:9")
        ));
        // A citation of a siman answers a question about a se'if in it.
        assert!(covers(
            &r("girsa:shulchan-arukh/orach-chayim/1"),
            &r("girsa:shulchan-arukh/orach-chayim/1:3")
        ));
        // …and not about the siman next door. `1:3` starts with `1`, `10:3`
        // does not.
        assert!(!covers(
            &r("girsa:shulchan-arukh/orach-chayim/1"),
            &r("girsa:shulchan-arukh/orach-chayim/10:3")
        ));
        // A different sefer is a different sefer.
        assert!(!covers(
            &r("girsa:bavli/berakhot/2a:1"),
            &r("girsa:bavli/shabbat/2a:1")
        ));
    }

    #[test]
    fn where_did_i_use_this_reads_the_refs_the_documents_already_store() {
        let personal = std::env::temp_dir().join("girsa-w19-citing");
        let _ = std::fs::remove_dir_all(&personal);

        let mut one = Buffer::new("חבורה");
        one.text = format!(
            "{}\nודו\"ק.\n",
            girsa_ksav::mekor("ברכות ב.", Some("girsa:bavli/berakhot/2a:1-2a:4"), None)
        );
        one.save(&personal).expect("saves");

        let mut two = Buffer::new("שיעור");
        two.text = girsa_ksav::mekor(
            "שו\"ע או\"ח א' ג'",
            Some("girsa:shulchan-arukh/orach-chayim/1:3"),
            None,
        );
        two.save(&personal).expect("saves");

        let (documents, _) = Documents::open(&personal);
        let asked = who_cites(&personal, &documents, &r("girsa:bavli/berakhot/2a:3"));
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].name, "חבורה");
        assert_eq!(asked[0].refs, ["girsa:bavli/berakhot/2a:1-2a:4"]);

        // A place nobody wrote about is nobody's — not the nearest document.
        assert!(who_cites(&personal, &documents, &r("girsa:bavli/shabbat/2a:1")).is_empty());
        let _ = std::fs::remove_dir_all(&personal);
    }

    #[test]
    fn a_document_written_in_the_real_ksav_is_found() {
        // The finding. This answered by walking `personal/ksav/` — the toy
        // editor's directory — so a `.ksav` written in the application this
        // whole pairing exists for was never found, and the reader's actual
        // work answered *nothing cites this*.
        let dir = std::env::temp_dir().join("girsa-who-cites-real");
        let _ = std::fs::remove_dir_all(&dir);
        let personal = dir.join("personal");
        std::fs::create_dir_all(&personal).expect("a layer");
        let shiurim = dir.join("shiurim");
        std::fs::create_dir_all(&shiurim).expect("somewhere else entirely");

        let doc = shiurim.join("חבורה.ksav");
        std::fs::write(
            &doc,
            girsa_ksav::mekor("ברכות ב.", Some("girsa:bavli/berakhot/2a:1-2a:4"), None),
        )
        .expect("Ksav wrote it");

        let (mut documents, _) = Documents::open(&personal);
        documents.remember(&doc, None).expect("the desk told us");
        documents.refreshed().expect("and it was read");

        let asked = who_cites(&personal, &documents, &r("girsa:bavli/berakhot/2a:3"));
        assert_eq!(asked.len(), 1, "{asked:?}");
        assert_eq!(asked[0].name, "חבורה");
        assert_eq!(
            asked[0].path.as_deref(),
            Some(doc.display().to_string().as_str())
        );
        assert!(!asked[0].away);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_document_that_is_not_here_still_answers_and_says_so() {
        // A stick that is not plugged in is not a document that was never
        // written. Dropping the row would be a silent gap; answering from the
        // cache without saying so would be a quiet lie.
        let dir = std::env::temp_dir().join("girsa-who-cites-away");
        let _ = std::fs::remove_dir_all(&dir);
        let personal = dir.join("personal");
        std::fs::create_dir_all(&personal).expect("a layer");
        let doc = dir.join("על-הכונן.ksav");
        std::fs::write(
            &doc,
            girsa_ksav::mekor("ברכות ב.", Some("girsa:bavli/berakhot/2a:1"), None),
        )
        .expect("written");

        let (mut documents, _) = Documents::open(&personal);
        documents.remember(&doc, None).expect("remembered");
        documents.refreshed().expect("read");
        std::fs::remove_file(&doc).expect("the stick came out");

        let asked = who_cites(&personal, &documents, &r("girsa:bavli/berakhot/2a:1"));
        assert_eq!(asked.len(), 1, "{asked:?}");
        assert!(asked[0].away, "it did not say the file is not here");
        assert_eq!(asked[0].refs, ["girsa:bavli/berakhot/2a:1"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
