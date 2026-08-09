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

/// The shelf as it is **read back from disk**, resolving through the redirect
/// rows that were written there.
///
/// This said *"live first, then ancestry, then the redirect table — what a link,
/// a correction or a Ksav citation goes through"*, and it does not. It resolves
/// through explicit redirect rows only: `read_back` reads what the importer
/// wrote, and the ancestry walk the window uses (`shelf.rs`'s `covered_by`) is
/// not in this path at all.
///
/// The claim mattered because this is the **flagship §3 test** — *a reimport
/// keeps every name* — so a helper describing the window's resolution order was
/// a certifying test appearing to exercise a path it never touched. What it
/// does certify is real and worth having: that the rows the importer writes are
/// enough to find every previous name. The ancestry half is covered where it
/// lives, in `girsa-app`.
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
    let pieces: Vec<String> = shelf(&root).keys().cloned().collect();
    assert!(pieces.len() > 2, "cut into several: {pieces:?}");

    // The same place, shortened upstream until it no longer needs cutting. It
    // keeps its name — same address, same opening — so the `cut` rows have
    // nothing left to describe.
    let short = "מאימתי קורין את שמע בערבית:";
    let imported = import_over(&root, vec![raw(&["1", "1"], short)]);
    assert_eq!(imported.continuity.kept, 1);
    assert!(
        !imported.redirects.iter().any(|r| r.why == Why::Cut),
        "nothing is cut now, so nothing claims to be: {:?}",
        imported.redirects
    );

    // This said `redirects.is_empty()`, and that was the assertion rather than
    // the argument. The paragraph above is about the `cut` row for `#1`, which
    // is live again and must not be redirected over. Its children are the
    // opposite case: names this importer minted, on disk for a whole release,
    // and now on nothing. Saying nothing about them is how an anchor a reader
    // wrote comes to resolve to silence.
    let forwarded: Vec<String> = imported
        .redirects
        .iter()
        .map(|r| format!("{} → {:?} ({:?})", r.from, r.to, r.why))
        .collect();
    let shed: Vec<String> = imported
        .redirects
        .iter()
        .map(|r| r.from.to_string())
        .collect();
    assert_eq!(
        shed, pieces,
        "every child it had and nothing else: {forwarded:?}"
    );
    for row in &imported.redirects {
        assert_eq!(row.why, Why::Resegmented);
        assert_eq!(
            row.to.len(),
            1,
            "the whole place, which is one record again"
        );
        assert!(row.to[0].to_string().ends_with("#1"), "{forwarded:?}");
    }
    assert!(path.exists(), "dead names are rows: {forwarded:?}");
}

#[test]
fn a_shelf_that_will_not_parse_does_not_leave_its_redirects_behind() {
    // Where the removal branch is still reached, and it is the case that
    // matters most.
    //
    // The test above used to reach it by shortening a cut se'if, and that path
    // is gone: the children it sheds are dead names and they get rows. What is
    // left is `Previous::on_the_shelf`'s other answer — *"a work that will not
    // parse reads as nothing rather than failing the import"* — where the run
    // that follows has no previous names to keep and no rows to write. A
    // redirect file surviving that is the worst version of a stale one: every
    // row in it names records the run beneath it never wrote.
    let root = scratch("damaged");
    let long = "מאימתי קורין את שמע בערבית: ".repeat(1_200);
    import_over(&root, vec![raw(&["1", "1"], &long)]);
    let dir = import::work_dir(&root, &work().slug);
    let path = dir.join(import::REDIRECTS);
    assert!(path.exists(), "a cut se'if writes a row");

    std::fs::write(dir.join("segments.jsonl"), "{ half a line").expect("damages the shelf");
    let previous = Previous::on_the_shelf(&root, &work().slug);
    assert!(previous.is_empty(), "a damaged shelf reads as nothing");

    let imported = import_over(&root, vec![raw(&["1", "1"], "קצר")]);
    assert!(imported.redirects.is_empty());
    assert!(
        !path.exists(),
        "nothing is redirected, so nothing claims to be"
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

// ---------------------------------------------------------------------------
// The other half of the name supply
//
// Everything above is about the names `mint_between` hands out, and every one of
// them comes out of `taken`. The oversized cutter hands out names too — `#7.1`,
// `#7.2` — and it was taking them off `Ordinal::child` directly, asking nothing.
// `standing.rs` opens by explaining that those two callers mean opposite things
// by a dotted name; what neither of them said is that only one was asking
// permission.

/// Every **name** on the shelf, which is the ordinal and not the printed id.
///
/// The address inside an id is upstream's and moves; the ordinal is the durable
/// name and is what two records may not share. Comparing printed ids instead
/// hides the collision this section is about — a cut child at `1:1#1.1` and an
/// inserted se'if at `1:2#1.1` are two different strings and one name, which is
/// exactly the failure, and `SegmentId`'s own `Ord` ignores the path for this
/// reason.
fn names(root: &Path) -> Vec<String> {
    import::read_back(root, &work().slug)
        .expect("reads back")
        .segments
        .iter()
        .map(|s| s.id.ordinal().to_string())
        .collect()
}

/// The shelf in reading order — which is ordinal order, never file order.
fn in_reading_order(root: &Path) -> Vec<(String, String)> {
    let mut segments = import::read_back(root, &work().slug)
        .expect("reads back")
        .segments;
    segments.sort_by(|a, b| a.id.cmp(&b.id));
    segments
        .into_iter()
        .map(|s| (s.id.ordinal().to_string(), s.text))
        .collect()
}

/// Long enough to be cut into `n` pieces, in words that say which sefer they are.
fn oversized(chars: usize) -> String {
    let unit = "מאימתי קורין את שמע בערבית: ";
    unit.repeat(chars / unit.chars().count() + 1)
}

#[test]
fn an_inserted_seif_cannot_be_handed_a_cut_childs_name() {
    // The collision, end to end. `#1` is over-long and is on disk as `#1.1`,
    // `#1.2`, `#1.3`. Upstream then inserts a se'if between it and `#2`, and the
    // only name that sorts between `#1` and `#2` is — by the old arithmetic —
    // `#1.1`, which is three-quarters of a page of somebody else's words.
    //
    // Two records, one id, one file, and nothing in the importer to notice.
    let root = scratch("cut-child-vs-insert");
    let long = oversized(16_000);
    import_over(
        &root,
        vec![raw(&["1", "1"], &long), raw(&["1", "2"], "האחרון")],
    );
    let cut = names(&root);
    assert_eq!(cut.len(), 4, "three pieces and a short se'if: {cut:?}");

    let after = import_over(
        &root,
        vec![
            raw(&["1", "1"], &long),
            raw(&["1", "2"], "החדש"),
            raw(&["1", "3"], "האחרון"),
        ],
    );
    let ids: Vec<String> = after
        .segments
        .iter()
        .map(|s| s.id.ordinal().to_string())
        .collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "one name, one record: {ids:?}");

    // And the new se'if is after the whole of `#1`, not inside it. `#1.4` reads
    // oddly and is the honest name: it sorts after every piece of `#1` and
    // before `#2`, which is exactly where the words are.
    let new = after
        .segments
        .iter()
        .find(|s| s.text == "החדש")
        .expect("the inserted se'if is on the shelf");
    assert!(
        !cut.contains(&new.id.ordinal().to_string()),
        "it took a name that was already on disk: {}",
        new.id
    );
    let pieces: Vec<&str> = after
        .segments
        .iter()
        .filter(|s| s.id.ordinal().to_string().starts_with("1."))
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(
        pieces.last(),
        Some(&"החדש"),
        "the insertion sorts after every piece of the se'if it follows"
    );
}

#[test]
fn a_child_a_shorter_cut_no_longer_produces_says_where_the_words_went() {
    // The same place, re-divided. It keeps its name by address, so `went` never
    // hears about it — and the child it stopped producing had no way of being
    // mentioned at all. `Why::Gone`'s own comment reserves resolving to nothing
    // for a place upstream does not have, "not the same as an id nobody ever
    // minted". This importer minted `#2.3`.
    let root = scratch("shorter-cut");
    let three = oversized(16_000);
    import_over(
        &root,
        vec![
            raw(&["1", "1"], "ראשון"),
            raw(&["1", "2"], &three),
            raw(&["1", "3"], "אחרון"),
        ],
    );
    let before: Vec<String> = shelf(&root).keys().cloned().collect();
    assert_eq!(before.len(), 5, "one, three pieces, and one: {before:?}");

    let two = oversized(12_000);
    let after = import_over(
        &root,
        vec![
            raw(&["1", "1"], "ראשון"),
            raw(&["1", "2"], &two),
            raw(&["1", "3"], "אחרון"),
        ],
    );
    assert_eq!(after.continuity.kept, 3, "the place kept its name");

    let shed: Vec<&girsa_corpus::import::Redirect> = after
        .redirects
        .iter()
        .filter(|r| r.from.to_string().ends_with("#2.3"))
        .collect();
    assert_eq!(
        shed.len(),
        1,
        "the shed child has a row: {:?}",
        after.redirects
    );
    assert_eq!(shed[0].why, Why::Resegmented);
    let to: Vec<String> = shed[0].to.iter().map(ToString::to_string).collect();
    assert_eq!(to.len(), 2, "pointed at the two records it is now: {to:?}");
    assert!(to.iter().all(|id| id.contains("#2.")), "{to:?}");

    // And the two names that survived still name the words of the same place —
    // it is the *place* that is durable, and its pieces are how it is stored.
    let live = shelf(&root);
    assert!(live.contains_key(&to[0]), "{to:?}");
    assert!(!live.contains_key(&shed[0].from.to_string()));
}

#[test]
fn a_name_a_cut_gave_up_is_not_minted_for_a_new_seif() {
    // The seeding half, which the test above does not reach.
    //
    // `places_of` folds `#1.1 #1.2 #1.3` back into one place called `#1`, so
    // `taken` — seeded from the places — never heard of the three names those
    // words were actually on disk under. While the se'if is still over-long that
    // costs nothing, because it claims them back. Shorten it, and the three
    // names are free: the very next se'if upstream inserts after `#1` is minted
    // at `#1.1`, which was three-quarters of a page in the release a reader
    // wrote their anchor against.
    //
    // That is spec.md §3's failure exactly, and it is the silent one — the
    // anchor resolves, to different words.
    let root = scratch("gave-up-a-name");
    let long = oversized(16_000);
    import_over(
        &root,
        vec![raw(&["1", "1"], &long), raw(&["1", "2"], "האחרון")],
    );
    let was = names(&root);
    assert_eq!(was.len(), 4, "three pieces and a short se'if: {was:?}");

    // Upstream shortens it and inserts a se'if after it, in one release. The
    // shortened se'if keeps its opening, because that is what `same_opening`
    // wants before it will hand a name over — a se'if whose first word changed
    // is a different se'if as far as the alignment is concerned, and it takes
    // the `went` path instead of this one.
    let short = "מאימתי קורין את שמע בערבית: ".repeat(3);
    let after = import_over(
        &root,
        vec![
            raw(&["1", "1"], &short),
            raw(&["1", "2"], "החדש"),
            raw(&["1", "3"], "האחרון"),
        ],
    );
    let new = after
        .segments
        .iter()
        .find(|s| s.text == "החדש")
        .expect("the inserted se'if is on the shelf");
    assert!(
        !was.contains(&new.id.ordinal().to_string()),
        "minted a name that was on disk last release: {}",
        new.id
    );

    // And the names it gave up say where the words went rather than nothing.
    let shed: Vec<String> = after
        .redirects
        .iter()
        .filter(|r| r.why == Why::Resegmented)
        .map(|r| r.from.to_string())
        .collect();
    assert_eq!(shed.len(), 3, "every child of the se'if it was: {shed:?}");
}

#[test]
fn a_cut_cannot_take_the_name_of_a_seif_inserted_inside_it() {
    // The collision that needs three imports, and the one the old arithmetic
    // could not avoid at all.
    //
    // A se'if inserted after `#1` is minted at `#1.1` — the only name that sorts
    // between `#1` and `#2` — and it is live. Two releases later `#1` grows past
    // the threshold, and `id.split(3)` returns `#1.1 #1.2 #1.3`: the first of
    // them is a name a different se'if is currently living under.
    //
    // There is no arrangement of `#1.k` that both avoids it and stays in reading
    // order, because the insertion sits *inside* the range the pieces need. So
    // the pieces go where an insertion would — under `#1.0` — which is what
    // `mint_between` has always done for exactly this shape of problem.
    let root = scratch("insert-then-cut");
    let unit = "מאימתי קורין את שמע בערבית: ";
    import_over(
        &root,
        vec![raw(&["1", "1"], unit), raw(&["1", "2"], "האחרון")],
    );

    import_over(
        &root,
        vec![
            raw(&["1", "1"], unit),
            raw(&["1", "2"], "החדש"),
            raw(&["1", "3"], "האחרון"),
        ],
    );
    let inserted = in_reading_order(&root);
    assert_eq!(
        inserted.iter().map(|(o, _)| o.as_str()).collect::<Vec<_>>(),
        ["1", "1.1", "2"],
        "the insertion is named between its neighbours"
    );

    // And now the first se'if is over-long.
    let long = oversized(16_000);
    let after = import_over(
        &root,
        vec![
            raw(&["1", "1"], &long),
            raw(&["1", "2"], "החדש"),
            raw(&["1", "3"], "האחרון"),
        ],
    );
    let names: Vec<String> = after
        .segments
        .iter()
        .map(|s| s.id.ordinal().to_string())
        .collect();
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "one name, one record: {names:?}");

    let shelf = in_reading_order(&root);
    let order: Vec<String> = shelf.iter().map(|(name, _)| name.clone()).collect();
    let words: Vec<&str> = shelf.iter().map(|(_, text)| text.trim()).collect();
    assert_eq!(
        words.iter().filter(|t| **t == "החדש").count(),
        1,
        "the inserted se'if is still there, once: {order:?}"
    );

    // Reading order is the property a name collision would have destroyed
    // quietly, and it is the reason the pieces are called what they are:
    // `#1.0`, `#1.0.1`, `#1.0.1.1` all sort below `#1.1`, so the whole of the
    // first se'if is read before the se'if inserted after it.
    let at_new = words.iter().position(|t| *t == "החדש").expect("is there");
    let at_last = words.iter().position(|t| *t == "האחרון").expect("is there");
    assert_eq!(at_new + 1, at_last, "and the insertion is still before it");
    assert_eq!(at_new, 3, "cut into three: {order:?}");
    assert!(
        words[..at_new].iter().all(|t| t.starts_with("מאימתי")),
        "every piece of the cut se'if comes first: {order:?}"
    );
    assert_eq!(order[at_new], "1.1", "and the insertion kept its name");
}
