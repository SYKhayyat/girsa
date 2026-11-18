//! A sefer that arrives while the program is running is not still missing.
//!
//! The citation bar remembers the works it has read, because a reader typing a
//! citation asks for the same sefer several times. What it must not remember is
//! a work it **did not find**: imports happen while a window is open — a sefer
//! of your own is dropped in, a corpus is re-imported in another terminal — and
//! an answer cached on the miss would keep reporting *not on this shelf* about
//! a sefer that is on the shelf until the process restarted. That is the poison
//! issue #58 is about, and the miss is the half a cache gets wrong by default.
//!
//! Watched failing before the fix: the second `look_up` returned the same
//! not-there answer the first one did.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use girsa_corpus::import::{self, ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::work::{Source, Work};
use girsa_ref::resolve::Context;
use girsa_search::citation::Citations;

const BAVLI: &str = "bavli/berakhot";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-late-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A corpus that names Berakhot and does not have it — the state of a shelf
/// between the lexicon and the text, which every import passes through.
fn corpus(dir: &Path) -> PathBuf {
    let root = dir.join("corpus");
    std::fs::create_dir_all(&root).expect("a corpus root");
    std::fs::write(
        root.join("lexicon.tsv"),
        "ברכות\tbavli/berakhot\tברכות\tBerakhot\n",
    )
    .expect("a lexicon");
    root
}

/// Berakhot 2a, through the real importer, the way a corpus arrives.
fn bring_berakhot(root: &Path) {
    let work = Work {
        slug: BAVLI.into(),
        he_title: "ברכות".into(),
        en_title: "Berakhot".into(),
        categories: vec!["Talmud".into()],
        order: Vec::new(),
        source: Source::Sefaria,
        origin: PathBuf::from("merged.json"),
        schema: None,
        author: None,
        era: None,
        comp_date: None,
        version: None,
        he_sections: Vec::new(),
        commentary_on: Vec::new(),
    };
    let raw = vec![
        RawSegment {
            path: vec!["2a".into(), "1".into()],
            kind: SegmentKind::Text,
            text: "מאימתי קורין את שמע בערבין".into(),
        },
        RawSegment {
            path: vec!["2a".into(), "2".into()],
            kind: SegmentKind::Text,
            text: "משעה שהכהנים נכנסין לאכול בתרומתן".into(),
        },
    ];
    let imported = ImportedWork::assemble(work, raw);
    import::write(root, &imported).expect("the sefer writes");
}

#[test]
fn a_sefer_that_arrives_is_readable_without_reopening_anything() {
    let dir = scratch("arrives");
    let root = corpus(&dir);
    let bar = Citations::open(&root, None).expect("a lexicon");

    // Nothing has been imported: the name is in the lexicon and the text is
    // not on the shelf, which is a fact about now and not about the name.
    let before = bar.look_up("ברכות ב.", &Context::default());
    assert!(
        before.only().is_none(),
        "there is no Berakhot on this shelf yet, and it landed: {}",
        before.describe()
    );

    bring_berakhot(&root);

    let after = bar.look_up("ברכות ב.", &Context::default());
    let place = after
        .only()
        .unwrap_or_else(|| panic!("Berakhot is on the shelf and it says: {}", after.describe()));
    assert_eq!(place.run.first.work(), BAVLI);
}
