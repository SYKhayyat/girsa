//! Provenance must survive the trip from the corpus to a Ksav document.
//!
//! **These tests are red on purpose.** They were written by a grader, not a
//! builder, and they pin a defect rather than a feature (`GRADE` finding N-1).
//!
//! spec.md §13, `girsa-corpus::work::Work::version` and
//! `girsa-source`'s module note all say the same thing in three places:
//!
//! > *Carry each text's source and license in its metadata — costs nothing now,
//! > and it is the only thing preserving the option to distribute publicly
//! > later.* A sefer typeset from quotes whose provenance was dropped cannot be
//! > un-dropped.
//!
//! It is dropped. `girsa-import` writes each work's `work.json` from the value
//! `import::read` returns — which has the printed edition read out of the text
//! file — and then writes `works/index.jsonl` from `catalogue.works()`, whose
//! own doc-comment says it "does **not** know the printed edition".
//! `Shelf::open` reads *only* `index.jsonl`. So the edition is on disk, one
//! directory away, and every Sefaria-sourced quote reaches Ksav with
//! `"version":{}`.
//!
//! Reproduction against the real corpus in this checkout:
//!
//! ```text
//! $ cargo run -p girsa-app --example send -- corpus \
//!       "שולחן ערוך, אורח חיים סימן א' סעיף ג'" | tail -1
//! {"schema":1,…,"version":{}}
//!
//! $ grep -o '"version":.*' \
//!       corpus/works/shulchan-arukh/orach-chayim/work.json
//! "version": {"edition": "Maginei Eretz: Shulchan Aruch Orach Chaim,
//!              Lemberg, 1893", "provenance": "https://www.sefaria.org/…"}
//! ```
//!
//! 6,211 of the 7,189 works in this corpus are in that state. The 978 that are
//! not are the Otzaria-only ones, whose version the catalogue sets itself and
//! so never had to survive the round trip.
//!
//! The unit test that guards this — `sending::tests::
//! provenance_travels_with_the_quote` — passes, because it builds its `Work` in
//! memory with the version already set. It asserts on the claim; these assert
//! on the system.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use girsa_app::sending::{send, Selection};
use girsa_app::shelf::Shelf;
use girsa_cite::CiteStyle;
use girsa_corpus::import::{self, ImportedWork, RawSegment, SegmentKind};
use girsa_corpus::work::{Source, Version, Work};

const SLUG: &str = "shulchan-arukh/orach-chayim";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-provenance-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The edition this text was read out of — what `sefaria::version_of` recovers
/// from `merged.json` and what `work.json` therefore holds on disk.
fn edition() -> Version {
    Version {
        edition: "Maginei Eretz: Shulchan Aruch Orach Chaim, Lemberg, 1893".into(),
        license: Some("Public Domain".into()),
        provenance: Some("https://www.sefaria.org/Shulchan_Arukh,_Orach_Chayim".into()),
    }
}

/// A corpus in exactly the state `girsa-import` leaves one in: the per-work
/// `work.json` carries the edition, and `works/index.jsonl` does not.
///
/// That asymmetry is not invented here — it is what the two writers in
/// `bin/girsa-import.rs` actually produce (`import_all` → `work.json` via
/// `import::read`, and `write_index(root, catalogue.works(), …)` → the
/// catalogue).
fn corpus_as_the_importer_leaves_it(root: &Path) {
    let raw = vec![
        RawSegment {
            path: vec!["1".into()],
            kind: SegmentKind::Heading,
            text: "סימן א".into(),
        },
        RawSegment {
            path: vec!["1".into(), "3".into()],
            kind: SegmentKind::Text,
            text: "ראוי לכל ירא שמים שיהא מיצר ודואג על חורבן בית המקדש:".into(),
        },
    ];
    let work = Work {
        slug: SLUG.to_string(),
        he_title: "שולחן ערוך, אורח חיים".into(),
        en_title: "Shulchan Arukh, Orach Chayim".into(),
        categories: vec!["Halakhah".into(), "Shulchan Arukh".into()],
        source: Source::Sefaria,
        origin: PathBuf::new(),
        schema: None,
        author: Some("יוסף קארו".into()),
        era: Some("AH".into()),
        comp_date: Some("1563 CE".into()),
        version: Some(edition()),
        he_sections: vec!["סימן".into(), "סעיף".into()],
        commentary_on: Vec::new(),
    };

    // `work.json`, with the edition — the state `import_all` writes.
    let imported = ImportedWork::assemble(work.clone(), raw);
    import::write(root, &imported).expect("the work is written");

    // `index.jsonl`, from the catalogue's Work — which never learned the
    // edition. This is the one line that reproduces the defect.
    let mut catalogued = work;
    catalogued.version = None;
    std::fs::create_dir_all(root.join("works")).expect("a works dir");
    std::fs::write(
        root.join("works/index.jsonl"),
        format!(
            "{}\n",
            serde_json::to_string(&catalogued).expect("it serializes")
        ),
    )
    .expect("a catalogue");
}

/// The shelf is entitled to the edition, because it is on disk beside the text.
///
/// **Red today**: `Shelf::open` reads `index.jsonl` and nothing else, so the
/// `work.json` written one directory away is never opened.
#[test]
fn the_shelf_knows_which_edition_it_is_reading() {
    let root = scratch("shelf");
    corpus_as_the_importer_leaves_it(&root);

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let sefer = shelf.read(SLUG).expect("the sefer opens");

    assert!(
        sefer.work.version.is_some(),
        "the edition is in {}, and the shelf did not read it",
        import::work_dir(&root, SLUG).join("work.json").display()
    );
}

/// The packet Ksav receives has to say which edition the words came from.
///
/// **Red today**: the packet arrives with `"version":{}`, so a sefer typeset
/// from it has no provenance to recover — the thing spec.md §13 says cannot be
/// un-dropped.
#[test]
fn a_quote_on_its_way_to_ksav_carries_the_edition_it_came_from() {
    let root = scratch("packet");
    corpus_as_the_importer_leaves_it(&root);

    let shelf = Shelf::open(&root, &root.join("personal")).expect("the shelf opens");
    let sefer = shelf.read(SLUG).expect("the sefer opens");
    let at = sefer.at(&girsa_ref::Address::parse("1:3").expect("an address"));
    let first = at.first().expect("the se'if is there").clone();
    let selection = Selection {
        from: first.clone(),
        to: first,
        from_char: 0,
        to_char: None,
    };

    let sent = send(&sefer, &selection, CiteStyle::HebrewFull, false, None).expect("it sends");
    let json = sent.packet.to_json().expect("it serializes");

    assert!(
        sent.packet.version.edition.contains("Lemberg"),
        "the packet dropped the edition: {json}"
    );
    assert!(
        !sent.packet.version.provenance.is_empty(),
        "the packet dropped the provenance: {json}"
    );
}

/// The reproduction, against whatever corpus this checkout actually has.
///
/// `#[ignore]`, not a `return`. This used to argue that printing `SKIPPED` and
/// returning "says so out loud instead" — and it does not: `cargo test`
/// captures stderr on a *passing* test, so what CI printed was a green tick and
/// `finished in 0.00s`. It was the forty-fourth such function, and the only one
/// the sweep that removed the other forty-three did not reach, because it spells
/// its guard by hand instead of through `corpus_or_skip!`.
///
/// The property underneath — a `work.json` edition survives into the catalogue —
/// is asserted on a synthetic shelf by `the_shelf_knows_which_edition_it_is_reading`
/// above. This one is the reproduction against a real download, and reads as
/// `1 ignored` rather than as a green tick: `cargo test -- --ignored`.
#[test]
#[ignore = "needs an imported corpus; the_shelf_knows_which_edition_it_is_reading asserts the property"]
fn every_work_json_edition_survives_into_the_catalogue() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let index = root.join("works/index.jsonl");
    if !index.exists() {
        eprintln!(
            "SKIPPED: no corpus at {} — run girsa-import first. \
             This test did not examine anything.",
            index.display()
        );
        return;
    }

    let body = std::fs::read_to_string(&index).expect("the catalogue reads");
    let mut catalogued: Vec<(String, bool)> = Vec::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let work: Work = match serde_json::from_str(line) {
            Ok(w) => w,
            Err(e) => panic!("a catalogue line will not parse, and nothing said so: {e}"),
        };
        catalogued.push((work.slug.clone(), work.version.is_some()));
    }

    let mut dropped = Vec::new();
    for (slug, has_version) in &catalogued {
        if *has_version {
            continue;
        }
        let on_disk = import::work_dir(&root, slug).join("work.json");
        let Ok(body) = std::fs::read_to_string(&on_disk) else {
            continue;
        };
        if let Ok(work) = serde_json::from_str::<Work>(&body) {
            if work.version.is_some() {
                dropped.push(slug.clone());
            }
        }
    }

    assert!(
        dropped.is_empty(),
        "{} of {} works have an edition in work.json that index.jsonl dropped, \
         e.g. {:?}",
        dropped.len(),
        catalogued.len(),
        &dropped[..dropped.len().min(3)]
    );
}
