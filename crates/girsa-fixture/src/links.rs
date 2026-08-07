//! The link graph over the fixture shelf, imported from a `links0.csv`.
//!
//! Same pipeline as `girsa-link-import` followed by `girsa-link-types`, over
//! twenty-odd rows instead of five million: resolve the citations through the
//! lexicon, orient the commentary rows, write the shards, then build the
//! `touching` and `inbound` caches over them and sort the landing index.
//!
//! # The column order is wrong on purpose
//!
//! Sefaria's `links*.csv` does **not** promise which of its two citation columns
//! is the commentary. `girsa_link::sefaria` recorded them in the order it read
//! them, and the result was that half the commentary in the corpus was stored as
//! *base → commentary*:
//!
//! ```text
//! girsa:bavli/berakhot/10a:1#418  --comments-on-->  girsa:bavli/rashi-on-berakhot/10a:1:1#367
//! ```
//!
//! Read aloud that says the gemara is a commentary on Rashi. 15,394 edges on
//! Berakhot alone were written that way, and nothing internal could see it: a
//! reversed edge has two real ends, both resolve, the type is right and the count
//! is right. What the reader got was a daf whose mefarshim were two aggadic
//! commentaries out of forty.
//!
//! So [`ROWS`] writes its `comments-on` rows **in both orders**, marked, exactly
//! as the export has them. `girsa_link::orient::Orienting` is what has to put
//! them right, and if it stops doing so the fixture graph comes out backwards and
//! `the_meforshim_are_on_the_daf` fails — with no download, on a shelf built in
//! about a second.
//!
//! A fixture that wrote `edges.jsonl` directly could not check any of that. It
//! would be asserting the column order this file chose.

use std::path::Path;

use girsa_corpus::csv::link_columns;
use girsa_corpus::index::SegmentIndex;
use girsa_corpus::work::Work;
use girsa_link::store::Row;
use girsa_link::{inbound, orient, store, touching, EdgeType};
use girsa_ref::Lexicon;

/// One row of `links0.csv`: two citations and Sefaria's own word for what joins
/// them.
///
/// `backwards` is documentation, not data — the file on disk carries no such
/// column, which is the entire problem. It marks the rows written base-first so
/// that a reader of this file can see the trap being set.
struct Link {
    citation_1: &'static str,
    citation_2: &'static str,
    /// Sefaria's `Conection Type`, misspelling and all. Blank is the common case
    /// — 74% of the corpus — and lands on `references`, the catch-all.
    label: &'static str,
    backwards: bool,
}

const fn row(citation_1: &'static str, citation_2: &'static str, label: &'static str) -> Link {
    Link {
        citation_1,
        citation_2,
        label,
        backwards: false,
    }
}

/// The same, written base-first, the way the export writes about half of them.
const fn flipped(base: &'static str, commentary: &'static str, label: &'static str) -> Link {
    Link {
        citation_1: base,
        citation_2: commentary,
        label,
        backwards: true,
    }
}

/// Every row the fixture graph is built from.
const ROWS: &[Link] = &[
    // --- the daf ----------------------------------------------------------
    row("Rashi on Berakhot 2a:1:1", "Berakhot 2a:1", "commentary"),
    row("Rashi on Berakhot 2a:1:2", "Berakhot 2a:1", "commentary"),
    // Backwards, as half of Berakhot's fifteen thousand are.
    flipped("Berakhot 2a:3", "Rashi on Berakhot 2a:3:1", "commentary"),
    flipped("Berakhot 2b:1", "Rashi on Berakhot 2b:1:1", "commentary"),
    row("Rashi on Berakhot 2b:2:1", "Berakhot 2b:2", "commentary"),
    row("Rashi on Berakhot 3a:1:1", "Berakhot 3a:1", "commentary"),
    row("Tosafot on Berakhot 2a:1:1", "Berakhot 2a:1", "commentary"),
    flipped("Berakhot 2b:1", "Tosafot on Berakhot 2b:1:1", "commentary"),
    row(
        "Penei Yehoshua on Berakhot 2a:1:1",
        "Berakhot 2a:1",
        "commentary",
    ),
    // A later sefer that had to deal with both readings of the same line. That
    // is what makes Rashi and Tosafos a *fork* rather than two commentaries:
    // `chain::forks` will not call two seforim a disagreement on its own say-so,
    // it wants somebody downstream who cited both.
    row(
        "Penei Yehoshua on Berakhot 2a:1:1",
        "Rashi on Berakhot 2a:1:1",
        "",
    ),
    row(
        "Penei Yehoshua on Berakhot 2a:1:1",
        "Tosafot on Berakhot 2a:1:1",
        "",
    ),
    // T5: the blank three quarters. Says "connected somehow" and not how, and
    // has to arrive as `references` rather than as an error or a guess.
    row("Berakhot 2a:1", "Mishnah Berakhot 1:1", ""),
    // The same pairing written the other way up, which the export also does.
    // It is not a commentary row, so `orient` leaves it exactly as it found it —
    // and the Mishnah therefore has a shard of its own, which is what
    // `no_edge_anywhere_anchors_to_something_that_is_not_on_the_shelf` walks. A
    // work whose every edge has been oriented away from it has no outgoing file,
    // which is correct and would have left that test reading an empty one.
    row("Mishnah Berakhot 1:2", "Berakhot 2a:2", ""),
    // And one that names its kind without naming a commentary.
    row("Berakhot 2b:1", "Genesis 1:1", "quotation"),
    // --- the mishnah ------------------------------------------------------
    // W8's acceptance, through the new addressing: the first mishnah of Berakhot
    // and the Rambam's commentary on it.
    row(
        "Rambam on Mishnah Berakhot 1:1:1",
        "Mishnah Berakhot 1:1",
        "commentary",
    ),
    row(
        "Rambam on Mishnah Berakhot 1:1:2",
        "Mishnah Berakhot 1:2",
        "commentary",
    ),
    flipped(
        "Mishnah Berakhot 1:1",
        "Bartenura on Mishnah Berakhot 1:1:1",
        "commentary",
    ),
    // --- the Shulchan Arukh and its nosei keilim --------------------------
    // Quoted, because the title has a comma in it and a CSV reader that split on
    // the comma alone would tear it in half and score the row unresolvable for a
    // reason that has nothing to do with the resolver.
    row(
        "Mishnah Berurah 58:1",
        "\"Shulchan Arukh, Orach Chayim 58:1\"",
        "commentary",
    ),
    row(
        "Mishnah Berurah 1:1",
        "\"Shulchan Arukh, Orach Chayim 1:1\"",
        "commentary",
    ),
    // The hop behind that one: the Shulchan Arukh is written on the Tur. Two
    // hops of one chain, each stored in the direction its own work was written,
    // which is what `how_it_became_halacha` traces backwards through.
    row(
        "\"Shulchan Arukh, Orach Chayim 58:1\"",
        "\"Tur, Orach Chayim 58:1\"",
        "commentary",
    ),
    // And where the halacha started: the sugya of krias shema. Untyped, because
    // this is what three quarters of the graph looks like — the path across the
    // centuries is found, and the answer says it runs through a link that only
    // claims the two places are connected.
    row("\"Shulchan Arukh, Orach Chayim 58:1\"", "Berakhot 2a:1", ""),
    flipped(
        "\"Shulchan Arukh, Orach Chayim 1:1\"",
        "Magen Avraham 1:1",
        "commentary",
    ),
    row(
        "\"Turei Zahav on Shulchan Arukh, Orach Chayim 1:1\"",
        "\"Shulchan Arukh, Orach Chayim 1:1\"",
        "commentary",
    ),
    row(
        "\"Siftei Kohen on Shulchan Arukh, Yoreh De'ah 1:1\"",
        "\"Shulchan Arukh, Yoreh De'ah 1:1\"",
        "commentary",
    ),
    flipped(
        "\"Shulchan Arukh, Yoreh De'ah 1:1\"",
        "\"Turei Zahav on Shulchan Arukh, Yoreh De'ah 1:1\"",
        "commentary",
    ),
    // --- the chumash ------------------------------------------------------
    row("Rashi on Genesis 1:1:1", "Genesis 1:1", "commentary"),
    flipped("Genesis 1:1", "Ramban on Genesis 1:1:1", "commentary"),
    row("Ibn Ezra on Genesis 1:1:1", "Genesis 1:1", "commentary"),
    flipped("Genesis 1:1", "Sforno on Genesis 1:1:1", "commentary"),
    row("Radak on Genesis 1:1:1", "Genesis 1:1", "commentary"),
    row("Rashbam on Genesis 1:1:1", "Genesis 1:1", "commentary"),
    // Sefaria types a targum as `targum`, which is `comments-on` and not a
    // ninth edge type — the one row that checks that mapping.
    row("Onkelos Genesis 1:1:1", "Genesis 1:1", "targum"),
];

/// Import the fixture link graph and build both caches over it.
///
/// # Panics
///
/// If the shelf is not there or the graph cannot be written.
pub fn build(root: &Path) {
    write_csv(root);
    import(root);
    caches(root);
}

/// `sefaria/links/links0.csv`, header and misspelling intact.
fn write_csv(root: &Path) {
    let mut body = String::from(link_columns::HEADER);
    body.push('\n');
    for link in ROWS {
        // Category 1 and Category 2 are in the header and are not read. Written
        // anyway: a row this file writes should be a row the file has.
        body.push_str(&format!(
            "{},{},{},,,,\n",
            link.citation_1, link.citation_2, link.label
        ));
    }
    crate::put(&root.join("sefaria/links/links0.csv"), &body);
}

/// The `girsa-link-import` half: resolve, orient, shard.
fn import(root: &Path) {
    let tsv = std::fs::read_to_string(root.join("lexicon.tsv")).expect("the fixture lexicon");
    let lexicon = Lexicon::from_tsv(&tsv);
    let works = works(root);

    let (index, unreadable) = SegmentIndex::load(root).expect("the fixture shelf indexes");
    assert!(
        unreadable.is_empty(),
        "the fixture shelf has works that will not load: {unreadable:?}"
    );

    let mut resolver = girsa_link::sefaria::Resolver::new(&lexicon);
    let bases = orient::Bases::of(&works);
    let mut oriented = orient::Orienting::new(&bases);
    let mut writer = store::Writer::default();

    let path = root.join("sefaria/links/links0.csv");
    let tally = girsa_link::sefaria::read_file(&path, &mut resolver, &index, |edge| {
        let mut edge = edge;
        oriented.apply(&mut edge);
        writer.push(&edge);
    })
    .expect("the fixture links import");

    // A fixture whose citations stopped resolving would leave every link test
    // asserting over an empty graph and passing, which is the failure this whole
    // crate exists to end — so it is checked here, once, rather than discovered
    // as a puzzling assertion failure in five test files.
    assert_eq!(
        tally.imported,
        ROWS.len(),
        "{} of {} fixture link rows resolved — unresolved {}, ambiguous {}, \
         work not on shelf {}, address not in work {}",
        tally.imported,
        ROWS.len(),
        tally.unresolved_citation,
        tally.ambiguous,
        tally.work_not_on_shelf,
        tally.address_not_found,
    );
    writer.flush(root).expect("the fixture edges are written");
}

/// The `girsa-link-types` half: the inbound cache, the landing index over it,
/// and the per-segment link-type masks.
///
/// The same three phases in the same order the tool runs them, and the order is
/// load-bearing: the masks are positional, so they come after everything that
/// could still move a row.
fn caches(root: &Path) {
    let mut into = inbound::Writer::default();
    let mut edges = 0usize;

    for path in shards(&root.join("links")) {
        let body = std::fs::read_to_string(&path).expect("a fixture shard reads");
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            let row: Row = serde_json::from_str(line).expect("a fixture edge parses");
            let (Some(from), Some(to)) = (work_of(&row.from), work_of(&row.to)) else {
                panic!("a fixture edge names no work: {line}");
            };
            into.push_row(from, to, line);
            edges += 1;
        }
    }
    assert!(edges > 0, "the fixture graph has no edges");

    // Writes `links/inbound.built`, which is what `inbound::built` reads and what
    // keeps a missing cache from being read as a zero.
    into.flush(root).expect("the inbound cache is written");

    for path in inbound_files(&root.join("links")) {
        inbound::sort_and_index_at(&path).expect("the landing index is built");
    }

    let mut masked = 0usize;
    for work in works(root) {
        let Ok(ordered) = girsa_corpus::import::ordered_ids(root, &work.slug) else {
            continue;
        };
        if ordered.is_empty() {
            continue;
        }
        let (touching_edges, _) = inbound::touching_work(root, &work.slug).unwrap_or_default();
        let mut ends: Vec<(&girsa_link::Anchor, EdgeType)> = Vec::new();
        for edge in &touching_edges {
            if edge.from.from.work() == work.slug {
                ends.push((&edge.from, edge.edge_type));
            }
            if edge.to.from.work() == work.slug {
                ends.push((&edge.to, edge.edge_type));
            }
        }
        let masks = touching::masks_for(ends, &ordered);
        touching::write(root, &work.slug, &ordered, &masks).expect("the fixture masks are written");
        masked += usize::from(masks.iter().any(|m| !m.is_empty()));
    }
    assert!(
        masked > 0,
        "no fixture work has a link-type mask set — the facet would be empty and every \
         test over it would pass"
    );
}

fn works(root: &Path) -> Vec<Work> {
    let body = std::fs::read_to_string(root.join("works/index.jsonl")).expect("the fixture index");
    body.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Work>(l).expect("a fixture work parses"))
        .collect()
}

fn shards(links: &Path) -> Vec<std::path::PathBuf> {
    named(links, "edges.jsonl")
}

fn inbound_files(links: &Path) -> Vec<std::path::PathBuf> {
    named(links, "inbound.jsonl")
}

fn named(links: &Path, name: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![links.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == name) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// The work slug out of a written anchor, the same way `girsa-link-types` does
/// it.
fn work_of(anchor: &str) -> Option<&str> {
    let one = anchor.split("-girsa:").next().unwrap_or(anchor);
    let body = one.strip_prefix("girsa:")?;
    let cut = body.rfind('/')?;
    Some(&body[..cut])
}

/// How many rows the fixture graph is built from, for a test that wants to say
/// what fraction of it it looked at.
#[must_use]
pub fn rows() -> usize {
    ROWS.len()
}

/// How many of them are written base-first, which is what `orient` has to undo.
#[must_use]
pub fn backwards_rows() -> usize {
    ROWS.iter().filter(|r| r.backwards).count()
}
