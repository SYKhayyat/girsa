//! The first thing a reader said about this application, five minutes in.
//!
//! > *"Bug - It sorts sefarim in categories by name, not by true order."*
//!
//! Every list of seforim in the window went through
//! `a.he_title.cmp(&b.he_title)`, so the Chumash came back
//! *במדבר · בראשית · דברים · ויקרא · שמות* — alphabetical order over a sequence
//! that has had an order for two thousand years, on the first shelf anybody
//! opens.
//!
//! # Two tests, and only one of them needs the download
//!
//! [`Work::by_order`] is a comparator over plain data and is tested here without
//! a corpus, because that is where the rule is. But the rule is only worth
//! anything if the **catalogue carries the numbers**, and that is a fact about
//! an import: `girsa-import` reads Sefaria's `order` off the schema and writes
//! it into `works/index.jsonl`, and a commentary that states none inherits its
//! base's. So the second test reads the real shelf and is `#[ignore]`d, which
//! makes a run without the corpus print `1 ignored` rather than a green tick
//! over nothing (BUILDER.md rule 7).
//!
//! ```sh
//! cargo test -p girsa-app --test the_shelf_is_in_the_order_it_is_printed_in -- --ignored
//! ```
//!
//! **A catalogue imported before this existed has no orders in it**, and every
//! shelf falls back to the title — which is the old behaviour, quietly. That is
//! the one honest failure mode, and it is one command away:
//!
//! ```sh
//! cargo run --release -p girsa-corpus --bin girsa-import -- --metadata-only corpus <otzaria>
//! ```

// A panic in a test is a failure report.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use girsa_app::taxonomy::Shipped;
use girsa_corpus::work::Work;
use std::path::{Path, PathBuf};

/// The real download, for the checks that are about **it** and not about this
/// code. `#[ignore]`d rather than skipped, so a run without it says so.
fn corpus() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    assert!(
        root.join("works/index.jsonl").is_file(),
        "no corpus at {} — this test is #[ignore]d and needs the real download",
        root.display()
    );
    root
}

fn catalogue(root: &Path) -> Vec<Work> {
    let index = root.join("works/index.jsonl");
    let body =
        std::fs::read_to_string(&index).unwrap_or_else(|e| panic!("{}: {e}", index.display()));
    body.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Work>(l).ok())
        .collect()
}

/// A work with a title and an order, and nothing else that matters here.
fn work(slug: &str, he: &str, order: &[i32]) -> Work {
    Work {
        slug: slug.into(),
        he_title: he.into(),
        en_title: slug.into(),
        categories: vec!["Tanakh".into(), "Torah".into()],
        order: order.to_vec(),
        source: girsa_corpus::work::Source::Sefaria,
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

#[test]
fn the_chumash_comes_back_in_the_order_it_is_printed_in() {
    let mut chumash = [
        work("numbers", "במדבר", &[40, 40]),
        work("genesis", "בראשית", &[10, 10]),
        work("deuteronomy", "דברים", &[50, 50]),
        work("leviticus", "ויקרא", &[30, 30]),
        work("exodus", "שמות", &[20, 20]),
    ];
    // Deliberately handed in alphabetically, which is what the old comparator
    // would have produced and what a reader saw.
    chumash.sort_by(Work::by_order);
    let said: Vec<&str> = chumash.iter().map(|w| w.he_title.as_str()).collect();
    assert_eq!(said, ["בראשית", "שמות", "ויקרא", "במדבר", "דברים"]);
}

#[test]
fn a_sefer_the_corpus_has_not_placed_sorts_after_the_ones_it_has() {
    // And not into the middle of them. An unordered sefer is one nobody has
    // placed; dropping it between Vayikra and Bamidbar would be inventing a
    // position for it.
    let mut shelf = [
        work("something", "אלף", &[]),
        work("exodus", "שמות", &[20, 20]),
        work("genesis", "בראשית", &[10, 10]),
        work("another", "בית", &[]),
    ];
    shelf.sort_by(Work::by_order);
    let said: Vec<&str> = shelf.iter().map(|w| w.he_title.as_str()).collect();
    assert_eq!(
        said,
        ["בראשית", "שמות", "אלף", "בית"],
        "the placed ones in their order, then the rest alphabetically"
    );
}

#[test]
fn the_order_is_stable_however_the_list_arrives() {
    // A shelf drawn twice has to be the same shelf. Two works with the same
    // order — the corpus has them — fall back to the title and then to the slug.
    let mut one = vec![
        work("b", "אותו שם", &[10]),
        work("a", "אותו שם", &[10]),
        work("c", "אחר", &[10]),
    ];
    let mut two: Vec<Work> = one.iter().rev().cloned().collect();
    one.sort_by(Work::by_order);
    two.sort_by(Work::by_order);
    let slugs = |works: &[Work]| -> Vec<String> { works.iter().map(|w| w.slug.clone()).collect() };
    assert_eq!(slugs(&one), slugs(&two));
    // `אותו` before `אחר`, because ו sorts before ח — the title decides, and then
    // the slug decides between the two that share a title.
    assert_eq!(slugs(&one), ["a", "b", "c"]);
}

// ── against the real download ───────────────────────────────────────────────

#[test]
#[ignore = "needs the imported corpus"]
fn the_real_chumash_and_the_real_shas_are_in_order() {
    let root = corpus();
    let works = catalogue(&root);
    let shipped = Shipped::of(&works);
    let arrangement = girsa_app::arrangement::Arrangement::default();

    let on = |key: &str| -> Vec<String> {
        let mut here: Vec<&Work> = works
            .iter()
            .filter(|w| girsa_app::taxonomy::shelf_key_of(w, &arrangement, &shipped) == key)
            .collect();
        here.sort_by(|a, b| Work::by_order(a, b));
        here.into_iter().map(|w| w.he_title.clone()).collect()
    };

    assert_eq!(
        on("תנ״ך/תורה"),
        ["בראשית", "שמות", "ויקרא", "במדבר", "דברים"],
        "the first shelf anybody opens"
    );

    let moed = on("תלמוד/בבלי/סדר מועד");
    assert_eq!(
        moed.first().map(String::as_str),
        Some("שבת"),
        "seder moed starts with Shabbos, not with whichever masechta sorts first: {moed:?}"
    );
    assert_eq!(moed.get(1).map(String::as_str), Some("עירובין"));
    assert_eq!(moed.get(2).map(String::as_str), Some("פסחים"));
}

#[test]
#[ignore = "needs the imported corpus"]
fn a_commentary_takes_its_base_s_place_in_the_sequence() {
    // Sefaria states an order on almost no commentary, so the five volumes of a
    // rishon on the Torah sorted alphabetically inside their folder — *Rashi on
    // Deuteronomy* above *Rashi on Genesis*, on a shelf where a reader knows
    // exactly what order those five come in. `Catalogue::build` gives each one
    // its declared base's order.
    let root = corpus();
    let works = catalogue(&root);
    let mut rashi: Vec<&Work> = works
        .iter()
        .filter(|w| {
            w.slug.starts_with("rashi-on-")
                && w.commentary_on.iter().any(|b| {
                    matches!(
                        b.slug.as_str(),
                        "genesis" | "exodus" | "leviticus" | "numbers" | "deuteronomy"
                    )
                })
        })
        .collect();
    assert_eq!(rashi.len(), 5, "the five chumashim: {rashi:?}");
    rashi.sort_by(|a, b| Work::by_order(a, b));
    let said: Vec<&str> = rashi.iter().map(|w| w.he_title.as_str()).collect();
    assert_eq!(
        said,
        [
            "רש\"י על בראשית",
            "רש\"י על שמות",
            "רש\"י על ויקרא",
            "רש\"י על במדבר",
            "רש\"י על דברים"
        ]
    );
}

#[test]
#[ignore = "needs the imported corpus"]
fn midrash_lekach_tov_is_not_filed_among_its_own_mefarshim() {
    // > *"medrash lekach tov seems to be in a separate category? i dont know
    // > why, but it looks confusing."*
    //
    // It declares the five chumashim, so W46's *a declared commentary is filed
    // one level down* moved it — into a `מפרשים` folder inside the shelf named
    // after **itself**, beside its own commentaries. The rule compares against
    // the base's actual shelf now, and over this corpus that is 25 works moved
    // and 5 left alone; these are the 5.
    let root = corpus();
    let works = catalogue(&root);
    let shipped = Shipped::of(&works);
    let arrangement = girsa_app::arrangement::Arrangement::default();
    let shelf_of = |slug: &str| -> Option<String> {
        works
            .iter()
            .find(|w| w.slug == slug)
            .map(|w| girsa_app::taxonomy::shelf_key_of(w, &arrangement, &shipped))
    };

    assert_eq!(
        shelf_of("midrash-lekach-tov").as_deref(),
        Some("מדרש/אגדה/Midrash Lekach Tov"),
        "the midrash itself"
    );
    assert_eq!(
        shelf_of("beur-hareem-on-midrash-lekach-tov").as_deref(),
        Some("מדרש/אגדה/Midrash Lekach Tov/מפרשים"),
        "and its mefaresh, which really is one and really does move"
    );

    // The work the rule was written for still moves: the Pri Megadim is filed
    // by Sefaria on the Shulchan Arukh's own shelf, as though it were a fifth
    // chelek.
    assert_eq!(
        shelf_of("peri-megadim-on-orach-chayim").as_deref(),
        Some("הלכה/שולחן ערוך/מפרשים")
    );
    assert_eq!(
        shelf_of("shulchan-arukh/orach-chayim").as_deref(),
        Some("הלכה/שולחן ערוך")
    );
}

#[test]
#[ignore = "needs the imported corpus"]
fn bereshis_is_offered_beside_onkelos_and_is_not_called_a_peirush_on_it() {
    // > *"bereishis is counted as a peirush on onkelos."*
    //
    // `onkelos-genesis` declares `commentary_on: genesis`, and `companions`
    // offered both directions of a declaration as one `declared: bool` — which
    // is what the window prints `פירוש` from.
    use girsa_app::shelf::{Related, Shelf};
    let root = corpus();
    let personal = std::env::temp_dir().join("girsa-order-test-personal");
    let _ = std::fs::remove_dir_all(&personal);
    let shelf = Shelf::open(&root, &personal).expect("the shelf opens");

    let offered = shelf.companions("onkelos-genesis");
    let bereshis = offered
        .iter()
        .find(|c| c.slug == "genesis")
        .expect("Bereshis is offered beside Onkelos, and it should be");
    assert_eq!(
        bereshis.stands,
        Some(Related::Base),
        "it is the sefer Onkelos was written about, not a peirush on Onkelos"
    );
    assert_eq!(bereshis.stands.map(Related::said), Some("הספר עצמו"));

    // And the other direction is still what it always was.
    let onkelos = shelf
        .companions("genesis")
        .into_iter()
        .find(|c| c.slug == "onkelos-genesis")
        .expect("Onkelos is offered beside Bereshis");
    assert_eq!(onkelos.stands, Some(Related::On));
    let _ = std::fs::remove_dir_all(&personal);
}
