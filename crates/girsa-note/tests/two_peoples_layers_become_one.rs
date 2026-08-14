//! Taking somebody else's layer, and the one thing it must never do.
//!
//! spec.md §11, BUILDER.md W27/B22. Corrections have had `girsa-fix merge`
//! since W20; notes, marks, saved questions and folders did not, so two copies
//! of `personal/` were two copies and the only way to put a chaburah together
//! was to pick one.
//!
//! The assertions here are about the refusal, not about the taking. Taking is
//! the easy half and it is the half that cannot go quietly wrong — a note that
//! did not arrive is a note you notice is missing. **Overwriting is the half
//! that is silent**: two people learning one sugya both call the note
//! `מאימתי`, and a merge that kept the newer one would replace a morning's
//! writing with a stranger's and leave nothing on the screen to say so. Your
//! own layer is the one kind of material in this library that nobody else has a
//! copy of.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_note::{Collections, LayerMut, Marks, Notes, Queries, Taggable};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A layer with one note, one saved question and one folder in it.
struct Layer {
    root: PathBuf,
    notes: Notes,
    marks: Marks,
    queries: Queries,
    collections: Collections,
}

impl Layer {
    fn at(root: &Path) -> Self {
        let (notes, _) = Notes::open(root);
        let (marks, _) = Marks::open(root);
        let (queries, _) = Queries::open(root);
        let (collections, _) = Collections::open(root);
        Self {
            root: root.to_path_buf(),
            notes,
            marks,
            queries,
            collections,
        }
    }

    fn note(&mut self, name: &str, words: &str) {
        let mut note = self.notes.start(name, "who");
        note.append(words.to_string());
        self.notes.write(note).expect("a note writes");
    }

    fn query(&mut self, name: &str, typed: &str) {
        self.queries
            .save(girsa_note::SavedQuery::new(name, typed))
            .expect("a question is kept");
    }

    fn mut_view(&mut self) -> LayerMut<'_> {
        LayerMut {
            notes: &mut self.notes,
            marks: &mut self.marks,
            queries: &mut self.queries,
            collections: &mut self.collections,
        }
    }

    /// Re-read from disk, so an assertion is about the files and not about the
    /// index that happens to be in memory.
    fn reread(&self) -> Self {
        Self::at(&self.root)
    }
}

#[test]
fn what_is_theirs_arrives_and_what_is_yours_is_not_touched() {
    let dir = scratch("girsa-merge-layers");
    let mine_at = dir.join("mine");
    let theirs_at = dir.join("theirs");
    std::fs::create_dir_all(&mine_at).expect("a layer");
    std::fs::create_dir_all(&theirs_at).expect("another");

    let mut mine = Layer::at(&mine_at);
    mine.note("מאימתי", "מה שכתבתי אני");
    mine.note("שלי-בלבד", "רק אצלי");
    mine.query("שאלה", "\"מאימתי קורין\"");

    let mut theirs = Layer::at(&theirs_at);
    // The collision: the same name, different words.
    theirs.note("מאימתי", "מה שכתב הוא");
    theirs.note("שלו-בלבד", "רק אצלו");
    // And the same on a saved question, which is keyed by its name too.
    theirs.query("שאלה", "\"ערבית רשות\"");
    theirs.query("שאלה-שלו", "\"תפילת הדרך\"");

    let took = girsa_note::merge(&mut mine.mut_view(), &theirs_at).expect("a merge");

    // One note taken, one refused. And the refusal is the assertion:
    assert_eq!(took.of(Taggable::Note).taken, 1);
    assert_eq!(took.of(Taggable::Note).refused, 1);
    assert_eq!(took.of(Taggable::Query).taken, 1);
    assert_eq!(took.of(Taggable::Query).refused, 1);

    let after = mine.reread();
    assert_eq!(
        after
            .notes
            .get("מאימתי")
            .expect("still mine")
            .paras()
            .first()
            .expect("a paragraph")
            .text,
        "מה שכתבתי אני",
        "the note that collided is still the one I wrote"
    );
    assert!(after.notes.get("שלו-בלבד").is_some(), "theirs arrived");
    assert!(after.notes.get("שלי-בלבד").is_some(), "mine is still here");
    assert_eq!(
        after
            .queries
            .all()
            .find(|q| q.name == "שאלה")
            .expect("still mine")
            .typed,
        "\"מאימתי קורין\"",
        "the question that collided is still the one I saved"
    );
}

#[test]
fn taking_the_same_layer_twice_is_taking_it_once() {
    // Idempotence, which is what makes a merge safe to run when you are not
    // sure whether you already did. The second run must report *already had*
    // and not *taken* — a merge that reported four takings twice would be
    // indistinguishable, from the report, from one that duplicated everything.
    let dir = scratch("girsa-merge-twice");
    let mine_at = dir.join("mine");
    let theirs_at = dir.join("theirs");
    std::fs::create_dir_all(&mine_at).expect("a layer");
    std::fs::create_dir_all(&theirs_at).expect("another");

    let mut mine = Layer::at(&mine_at);
    let mut theirs = Layer::at(&theirs_at);
    theirs.note("שלו", "דברים");
    theirs.query("שאלתו", "\"ברכות\"");

    let first = girsa_note::merge(&mut mine.mut_view(), &theirs_at).expect("a merge");
    assert_eq!(first.all().taken, 2);
    assert_eq!(first.all().already_had, 0);

    let again = girsa_note::merge(&mut mine.mut_view(), &theirs_at).expect("a second merge");
    assert_eq!(again.all().taken, 0, "nothing was taken a second time");
    assert_eq!(again.all().already_had, 2);
    assert_eq!(
        again.all().refused,
        0,
        "mine is theirs, so it is not a clash"
    );

    assert_eq!(mine.reread().notes.count(), 1);
}

#[test]
fn a_note_of_theirs_arrives_as_a_sefer_on_your_shelf() {
    // The claim W27 is built on, applied to a note that came from somebody
    // else: a note is a sefer, so a note of theirs has to be one too. If this
    // only copied the `.md` file, their note would be readable in the notes
    // drawer and absent from the shelf, the panes and the next index build —
    // a second-class note, which is the thing §11 says a note is not.
    let dir = scratch("girsa-merge-shelved");
    let mine_at = dir.join("mine");
    let theirs_at = dir.join("theirs");
    std::fs::create_dir_all(&mine_at).expect("a layer");
    std::fs::create_dir_all(&theirs_at).expect("another");

    let mut mine = Layer::at(&mine_at);
    let mut theirs = Layer::at(&theirs_at);
    theirs.note("חבורה", "דברי תורה");

    girsa_note::merge(&mut mine.mut_view(), &theirs_at).expect("a merge");

    let catalogue = mine_at.join("works").join("index.jsonl");
    let body = std::fs::read_to_string(&catalogue).expect("a catalogue");
    assert!(
        body.contains("note/חבורה"),
        "their note is catalogued on my shelf: {body}"
    );
    assert!(
        girsa_corpus::import::work_dir(&mine_at, "note/חבורה").is_dir(),
        "and it has its segments"
    );
}

#[test]
fn a_layer_with_nothing_in_it_is_not_a_failure() {
    // Somebody hands you a directory that has no marks file because they have
    // never marked anything. That is a layer with no marks, not a broken
    // merge, and a reader must not have to create four empty files to be told
    // they have nothing.
    let dir = scratch("girsa-merge-empty");
    let mine_at = dir.join("mine");
    let theirs_at = dir.join("theirs");
    std::fs::create_dir_all(&mine_at).expect("a layer");
    std::fs::create_dir_all(&theirs_at).expect("an empty directory");

    let mut mine = Layer::at(&mine_at);
    let took = girsa_note::merge(&mut mine.mut_view(), &theirs_at).expect("not an error");
    assert_eq!(took.all().taken, 0);
    assert_eq!(took.all().refused, 0);
}

#[test]
fn an_export_is_a_layer_a_merge_can_read() {
    // The two halves of §11 meeting. `export` writes the four stores under
    // their own file names, which is the shape of a `personal/` root, so
    // handing somebody an export and handing them the directory arrive at the
    // same door. If these two ever disagreed about a file name, the export
    // would be a format that only its own writer could read — which is exactly
    // what §11's *exportable as plain files* is a claim against.
    let dir = scratch("girsa-merge-export");
    let mine_at = dir.join("mine");
    let theirs_at = dir.join("theirs");
    let handed_over = dir.join("handed-over");
    std::fs::create_dir_all(&mine_at).expect("a layer");
    std::fs::create_dir_all(&theirs_at).expect("another");

    let mut theirs = Layer::at(&theirs_at);
    theirs.note("שיעור", "מה שנאמר");
    theirs.query("שאלתו", "\"תפילה\"");
    girsa_note::export(
        &girsa_note::Layer {
            notes: &theirs.notes,
            marks: &theirs.marks,
            queries: &theirs.queries,
            collections: &theirs.collections,
        },
        &handed_over,
    )
    .expect("an export");

    let mut mine = Layer::at(&mine_at);
    let took = girsa_note::merge(&mut mine.mut_view(), &handed_over).expect("a merge");
    assert_eq!(took.of(Taggable::Note).taken, 1);
    assert_eq!(took.of(Taggable::Query).taken, 1);
}
