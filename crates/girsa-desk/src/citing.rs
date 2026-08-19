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
//! The other half — prose full of citations becoming live refs — is
//! [`girsa_app::linkify`], and [`crate::linkify`] re-exports it so this is
//! still the path Ksav's loopback takes. It moved down a crate because it is a
//! function of a lexicon and a string and needs nothing from a desk, and
//! because up here the reading pane could not reach it: `girsa-desk` depends on
//! `girsa-app`, so a note read as a sefer had no way to ask which of its own
//! words were citations.

use std::path::Path;

use girsa_ref::Ref;

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
    let mut out = in_your_drawer(personal, place);
    out.extend(in_your_documents(documents, place));
    out
}

/// The refs of this document that cover `place`.
fn answering(refs: impl IntoIterator<Item = String>, place: &Ref) -> Vec<String> {
    refs.into_iter()
        .filter(|text| {
            text.parse::<Ref>()
                .map(|stored| covers(&stored, place))
                .unwrap_or(false)
        })
        .collect()
}

/// The half that **reads files**: every buffer in the drawer, opened and
/// scanned.
///
/// Split out from [`who_cites`] so a caller holding a lock can do this part
/// without it. The shell's `who_cites` command is asked on every click on a
/// line, and it used to run the whole of this under the global state guard —
/// a `read_dir` and a full read of every document the reader is writing, with
/// no cap on size or count, on the thread the scroll handler is waiting for.
///
/// Nothing here touches shared state, which is what makes the split honest
/// rather than a rearrangement: the argument is a path.
#[must_use]
pub fn in_your_drawer(personal: &Path, place: &Ref) -> Vec<Citing> {
    let mut out = Vec::new();
    for name in Buffer::list(personal) {
        let Ok(buffer) = Buffer::open(personal, &name) else {
            continue;
        };
        let refs = answering(girsa_ksav::refs_in(&buffer.text), place);
        if !refs.is_empty() {
            out.push(Citing {
                name,
                refs,
                path: None,
                away: false,
            });
        }
    }
    out
}

/// The half that **reads nothing**: the registry, which is already in memory.
///
/// String parsing over rows a caller is already holding. This is the part it
/// is fair to do under a lock.
#[must_use]
pub fn in_your_documents(documents: &Documents, place: &Ref) -> Vec<Citing> {
    let mut out = Vec::new();
    for document in documents.all() {
        let refs = answering(document.refs.clone(), place);
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

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn r(text: &str) -> Ref {
        text.parse().expect("a ref")
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
