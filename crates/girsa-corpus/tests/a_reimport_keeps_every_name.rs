//! The test `anchors_survive_editing.rs` could not have caught.
//!
//! That one runs entirely in memory: it builds a [`SegmentStore`], splits a
//! segment, and asserts the anchors held. Every assertion in it is true and
//! none of them touch a disk. `import::write` emitted two files and neither was
//! a redirect table, so a store round-tripped through the corpus lost every row
//! it held — and `girsa-import` re-ran `Ordinal::root(i + 1)` over the whole
//! catalogue on every invocation and overwrote what was there.
//!
//! So the scenario spec.md §3 was written about — *"when upstream re-segments a
//! text, a redirect table absorbs it"* — went **through disk**, which is the one
//! path nothing exercised. Sefaria adds one se'if to siman 1 of Orach Chayim,
//! you re-run the importer, and 4,170 segments renumber by one: not a broken
//! link, the wrong text silently, which is T1 verbatim at import granularity.
//!
//! Everything here goes through `write` and `read_back`.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use girsa_corpus::import::{self, ImportedWork, Previous, RawSegment, SegmentKind, Why};
use girsa_corpus::store::{Anchors, SegmentStore};
use girsa_corpus::work::{Source, Work};

/// spec.md §2.2: Shulchan Arukh, Orach Chayim is 697 simanim / 4,171 se'ifim.
/// The size matters — the defect is proportional to how much sits below the
/// insert, and at n=3 nothing is proportional to anything.
const SIMANIM: usize = 697;

fn work() -> Work {
    Work {
        slug: "shulchan-arukh/orach-chayim".into(),
        he_title: "שולחן ערוך, אורח חיים".into(),
        en_title: "Shulchan Arukh, Orach Chayim".into(),
        categories: vec!["Halakhah".into()],
        source: Source::Sefaria,
        origin: PathBuf::from("merged.json"),
        schema: None,
        author: None,
        era: None,
        comp_date: None,
        version: None,
        he_sections: Vec::new(),
        commentary_on: Vec::new(),
    }
}

fn raw(path: &[&str], text: &str) -> RawSegment {
    RawSegment {
        path: path.iter().map(|p| (*p).to_string()).collect(),
        kind: SegmentKind::Text,
        text: text.to_string(),
    }
}

/// A shelf-sized sefer whose every se'if says something different, so a name
/// landing on the wrong words is detectable rather than a coincidence.
fn orach_chayim() -> Vec<RawSegment> {
    let mut out = Vec::new();
    for siman in 1..=SIMANIM {
        for seif in 1..=6 {
            out.push(raw(
                &[&siman.to_string(), &seif.to_string()],
                &format!("סימן {siman} סעיף {seif} · והנה האדם הזה אף שבכמותו הוא מקטני הברואים"),
            ));
        }
    }
    out
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-reimport-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Import a work over whatever is already at `root`, and write it.
fn import_over(root: &Path, raw: Vec<RawSegment>) -> ImportedWork {
    let previous = Previous::on_the_shelf(root, &work().slug);
    let imported = ImportedWork::assemble_after(work(), raw, &previous);
    import::write(root, &imported).expect("the sefer writes");
    imported
}

/// What every name on the shelf says right now: id → words.
fn shelf(root: &Path) -> BTreeMap<String, String> {
    import::read_back(root, &work().slug)
        .expect("reads back")
        .segments
        .into_iter()
        .map(|s| (s.id.to_string(), s.text))
        .collect()
}

/// Every anchor a reader could have written down, and the words it named.
fn anchors(root: &Path) -> Vec<(girsa_corpus::segment::SegmentId, String)> {
    import::read_back(root, &work().slug)
        .expect("reads back")
        .segments
        .into_iter()
        .map(|s| (s.id, s.text))
        .collect()
}

/// The shelf as the reader resolves it: live first, then ancestry, then the
/// redirect table. What a link, a correction or a Ksav citation goes through.
fn as_resolved(root: &Path) -> SegmentStore {
    SegmentStore::from_disk(&import::read_back(root, &work().slug).expect("reads back"))
}

// ---------------------------------------------------------------------------

#[test]
fn one_seif_added_upstream_renames_nothing_and_mints_one_name() {
    // The finding, at the size it was found at.
    let root = scratch("one-seif");
    import_over(&root, orach_chayim());
    let before = anchors(&root);
    assert_eq!(before.len(), SIMANIM * 6);

    // Sefaria's next release has one more se'if in siman 1, and re-addresses
    // every se'if after it — inside siman 1 by one, which is exactly the shape
    // of the change a corpus update makes.
    let mut now = Vec::new();
    for siman in 1..=SIMANIM {
        for seif in 1..=6 {
            if siman == 1 && seif == 3 {
                now.push(raw(&["1", "3"], "סימן 1 סעיף חדש · תוספת שלא היתה כאן"));
            }
            let address = if siman == 1 && seif >= 3 {
                (seif + 1).to_string()
            } else {
                seif.to_string()
            };
            now.push(raw(
                &[&siman.to_string(), &address],
                &format!("סימן {siman} סעיף {seif} · והנה האדם הזה אף שבכמותו הוא מקטני הברואים"),
            ));
        }
    }
    let imported = import_over(&root, now);
    let after = as_resolved(&root);

    // Resolved the way a link, a correction or a citation in somebody's Ksav
    // document is resolved — not by string equality of the printed id. The
    // *address* inside an id is upstream's and it did change for the four
    // se'ifim below the insert; the **ordinal is the durable name** and it is
    // what has to keep naming the same words (see `SegmentId::path`).
    let mut moved = Vec::new();
    for (anchor, was) in &before {
        match after.text_at(anchor) {
            Some(now) if now == *was => {}
            Some(now) => moved.push(format!("{anchor}\n  was: {was}\n  now: {now}")),
            None => moved.push(format!("{anchor}\n  was: {was}\n  now: <gone>")),
        }
    }
    assert!(
        moved.is_empty(),
        "{} of {} anchors moved across a re-import:\n{}",
        moved.len(),
        before.len(),
        moved.iter().take(3).cloned().collect::<Vec<_>>().join("\n")
    );

    assert_eq!(imported.continuity.kept, before.len());
    assert_eq!(imported.continuity.minted, 1);
    assert_eq!(imported.continuity.gone, 0);
    assert_eq!(after.len(), before.len() + 1);

    // And the new se'if sorts where it belongs, not at the end of the sefer.
    let was: Vec<String> = before
        .iter()
        .map(|(id, _)| id.ordinal().to_string())
        .collect();
    let read: Vec<String> = import::read_back(&root, &work().slug)
        .expect("reads")
        .segments
        .iter()
        .map(|s| s.id.ordinal().to_string())
        .collect();
    let at = read
        .iter()
        .position(|o| !was.contains(o))
        .expect("one name was minted");
    assert_eq!(at, 2, "{} landed at {at}, not third in the sefer", read[at]);
}

#[test]
fn the_same_edit_on_the_old_importer_would_have_renamed_4170_segments() {
    // The counter-example, asserted rather than described — the same argument
    // `LineIndexStore` is kept for, one level up. `assemble` with no previous
    // run is what every import used to do, and it is what a first import still
    // does; running it twice is the defect.
    let before = ImportedWork::assemble(work(), orach_chayim());
    let mut now = orach_chayim();
    now.insert(2, raw(&["1", "3"], "תוספת"));
    let after = ImportedWork::assemble(work(), now);

    // By ordinal, because the ordinal is the name an anchor resolves through.
    let was: BTreeMap<String, String> = before
        .segments
        .iter()
        .map(|s| (s.id.ordinal().to_string(), s.text.clone()))
        .collect();
    let moved = after
        .segments
        .iter()
        .filter(|s| {
            was.get(&s.id.ordinal().to_string())
                .is_some_and(|t| *t != s.text)
        })
        .count();
    assert!(
        moved >= 4_170,
        "position-derived names stopped being dangerous ({moved} moved) — \
         if they became safe, the whole of spec.md §3 needs rewriting"
    );
}

#[test]
fn a_store_round_tripped_through_disk_keeps_its_redirects() {
    // Fact one of the finding. `SegmentStore` has held a redirect table since
    // W6 and there was no slot on disk for it, so this was false for the whole
    // life of the project and no test looked.
    let root = scratch("round-trip");
    let long = "מאימתי קורין את שמע בערבית: ".repeat(1_200);
    let imported = import_over(
        &root,
        vec![raw(&["1", "1"], "קצר"), raw(&["1", "2"], &long)],
    );
    assert!(
        imported.redirects.iter().any(|r| r.why == Why::Cut),
        "cutting a 33,600-character se'if is a redirect and has to be written down"
    );

    let read = import::read_back(&root, &work().slug).expect("reads back");
    let store = SegmentStore::from_disk(&read);
    let parent = imported
        .redirects
        .iter()
        .find(|r| r.why == Why::Cut)
        .map(|r| r.from.clone())
        .expect("the cut parent");

    // The parent is not a record on disk. It still names the words.
    assert!(
        read.segments.iter().all(|s| s.id != parent),
        "a cut parent is not a segment any more"
    );
    assert_eq!(
        store.text_at(&parent).as_deref(),
        Some(long.as_str()),
        "{parent} stopped naming its words across a round trip through disk"
    );
    assert_eq!(store.redirects().count(), 1);
}

#[test]
fn a_cut_seif_is_one_place_again_on_the_next_import() {
    // What the `cut` rows are for on the way back in. Without them the next
    // import compares a whole se'if against a third of one, matches neither,
    // and mints three new names for words that already had them.
    let root = scratch("cut-is-one-place");
    let long = "מאימתי קורין את שמע בערבית: ".repeat(1_200);
    import_over(
        &root,
        vec![raw(&["1", "1"], "קצר"), raw(&["1", "2"], &long)],
    );
    let before = shelf(&root);

    let again = import_over(
        &root,
        vec![raw(&["1", "1"], "קצר"), raw(&["1", "2"], &long)],
    );
    assert_eq!(shelf(&root), before, "an unchanged work must be unchanged");
    assert_eq!(again.continuity.minted, 0);
    assert_eq!(again.continuity.kept, 2, "two places, not four records");
}

#[test]
fn re_sectioning_a_whole_work_costs_no_name_at_all() {
    // The case spec.md §3 was actually written about. Every address changes;
    // no words do. Matching on the address would rename all 4,182 of them.
    let root = scratch("re-sectioned");
    import_over(&root, orach_chayim());
    let before = shelf(&root);

    let now: Vec<RawSegment> = orach_chayim()
        .into_iter()
        .map(|r| RawSegment {
            path: vec![format!("פרק {}", r.path[0]), format!("הלכה {}", r.path[1])],
            ..r
        })
        .collect();
    let imported = import_over(&root, now);

    assert_eq!(imported.continuity.kept, before.len());
    assert_eq!(imported.continuity.minted, 0);
    let after = shelf(&root);
    for (id, was) in &before {
        // The id's *address* is new — it is what a reader sees — and the
        // ordinal is the durable part, so the words are found by ordinal.
        let ordinal = id.rsplit('#').next().expect("an ordinal");
        let found = after
            .iter()
            .find(|(id, _)| id.rsplit('#').next() == Some(ordinal))
            .map(|(_, text)| text.clone());
        assert_eq!(found.as_ref(), Some(was), "#{ordinal} moved to other words");
    }
}

#[test]
fn a_seif_upstream_dropped_says_so_rather_than_handing_its_name_away() {
    let root = scratch("dropped");
    import_over(
        &root,
        vec![
            raw(&["1", "1"], "אחד"),
            raw(&["1", "2"], "שנים"),
            raw(&["1", "3"], "שלשה"),
        ],
    );

    let imported = import_over(
        &root,
        vec![raw(&["1", "1"], "אחד"), raw(&["1", "2"], "שלשה")],
    );
    assert_eq!(imported.continuity.gone, 1);
    let row = imported
        .redirects
        .iter()
        .find(|r| r.why == Why::Gone)
        .expect("a place upstream dropped is a row and not a silence");
    assert!(row.from.to_string().ends_with("#2"), "{}", row.from);
    assert!(row.to.is_empty());

    // And the name it vacated is not handed to anything. `#2` named "שנים";
    // nothing on the shelf may be called `#2` now.
    let after = shelf(&root);
    assert!(
        after.keys().all(|id| !id.ends_with("#2")),
        "the name of a dropped se'if was reused: {after:?}"
    );
    // Which is the whole point: it is still on disk, still names the same
    // ordinal, and resolves to nothing rather than to somebody else's words.
    let read = import::read_back(&root, &work().slug).expect("reads");
    let store = SegmentStore::from_disk(&read);
    assert_eq!(store.text_at(&row.from), None);
}

#[test]
fn two_seifim_merged_upstream_redirect_to_the_one_that_absorbed_them() {
    let root = scratch("merged");
    import_over(
        &root,
        vec![
            raw(&["1", "1"], "אחד"),
            raw(&["1", "2"], "שנים"),
            raw(&["1", "3"], "שלשה"),
        ],
    );

    let imported = import_over(
        &root,
        vec![
            raw(&["1", "1"], "אחד"),
            raw(&["1", "2"], "שנים שלשה"),
            raw(&["1", "3"], "ארבעה"),
        ],
    );
    assert_eq!(imported.continuity.resegmented, 1);

    let read = import::read_back(&root, &work().slug).expect("reads");
    let store = SegmentStore::from_disk(&read);
    let row = imported
        .redirects
        .iter()
        .find(|r| r.why == Why::Resegmented)
        .expect("the merged se'if is redirected");
    let now = store.text_at(&row.from).expect("it still names words");
    assert!(now.contains("שנים"), "{} lost its words: {now}", row.from);
}

#[test]
fn a_first_import_is_exactly_what_it_always_was() {
    // The continuity machinery must cost a fresh corpus nothing — the names
    // still come out of reading order, one-based, and 7,189 directories of
    // them are already on somebody's disk.
    let imported = ImportedWork::assemble(
        work(),
        vec![
            raw(&["1", "1"], "יתגבר כארי"),
            raw(&["1", "2"], "ולא יתבייש"),
            raw(&["2", "1"], "המשכים"),
        ],
    );
    let ids: Vec<String> = imported.segments.iter().map(|s| s.id.to_string()).collect();
    assert_eq!(
        ids,
        [
            "girsa:shulchan-arukh/orach-chayim/1:1#1",
            "girsa:shulchan-arukh/orach-chayim/1:2#2",
            "girsa:shulchan-arukh/orach-chayim/2:1#3",
        ]
    );
    assert!(imported.redirects.is_empty());
    assert!(imported.continuity.is_empty());
    assert!(imported.continuity.said().is_empty());
}

#[test]
fn a_shelf_written_before_the_redirect_table_reads_as_one_with_nothing_redirected() {
    // Every one of the 7,189 works on disk right now. A missing file is not a
    // failure, and it must not become one — that would be a corpus that can
    // only be read by the build that wrote it.
    let root = scratch("old-shelf");
    import_over(
        &root,
        vec![raw(&["1", "1"], "אחד"), raw(&["1", "2"], "שנים")],
    );
    let path = import::work_dir(&root, &work().slug).join(import::REDIRECTS);
    assert!(
        !path.exists(),
        "nothing was redirected, so there is no file"
    );

    let read = import::read_back(&root, &work().slug).expect("reads back");
    assert!(read.redirects.is_empty());

    // And a re-import over it keeps every name, which is the only thing that
    // matters for the corpus already on disk.
    let again = import_over(
        &root,
        vec![raw(&["1", "1"], "אחד"), raw(&["1", "2"], "שנים")],
    );
    assert_eq!(again.continuity.kept, 2);
    assert_eq!(again.continuity.minted, 0);
}

#[test]
fn a_stale_redirect_file_is_taken_away_rather_than_left_to_lie() {
    // A work that stops having anything redirected loses the file it had.
    // Leaving it would be worse than never writing one: `Previous` reads it,
    // and a `cut` row for a name that is live again sends a reader to records
    // that do not exist.
    let root = scratch("stale");
    let long = "מאימתי קורין את שמע בערבית: ".repeat(1_200);
    import_over(&root, vec![raw(&["1", "1"], &long)]);
    let path = import::work_dir(&root, &work().slug).join(import::REDIRECTS);
    assert!(path.exists(), "a cut se'if writes a row");

    // The same place, shortened upstream until it no longer needs cutting. It
    // keeps its name — same address, same opening — so the `cut` rows have
    // nothing left to describe.
    let short = "מאימתי קורין את שמע בערבית:";
    let imported = import_over(&root, vec![raw(&["1", "1"], short)]);
    assert_eq!(imported.continuity.kept, 1);
    assert!(imported.redirects.is_empty());
    assert!(
        !path.exists(),
        "nothing is cut now, so nothing claims to be"
    );
}

#[test]
fn the_report_says_what_the_pass_did_rather_than_leaving_it_to_be_inferred() {
    let root = scratch("report");
    import_over(&root, orach_chayim());
    let mut now = orach_chayim();
    now.remove(0);
    let imported = import_over(&root, now);

    let said = imported.continuity.said();
    assert!(said.iter().any(|l| l.contains("kept their id")), "{said:?}");
    assert!(
        said.iter().any(|l| l.contains("no longer upstream")),
        "a place that went away is said out loud: {said:?}"
    );
    assert_eq!(imported.continuity.works(), 1);
}
