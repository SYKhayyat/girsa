//! `comments-on` points one way, and the mefarshim are reachable from the daf.
//!
//! # Why this test exists, and why nothing else caught it
//!
//! Every internal check of the link graph passed. `inbound::built` was true,
//! four million edges imported, `girsa-link-types` printed
//! `link type comments-on 5388`, and W8's own acceptance test — Mishnah
//! Berakhot 1:1 reaching the Rambam on it — was green. The graph was
//! self-consistent and looked complete.
//!
//! It was consistent about the wrong thing. Sefaria's `links*.csv` does not
//! promise which of its two citation columns is the commentary, and
//! [`crate::sefaria`] recorded them in the order it read them, so half the
//! commentary in the corpus is stored as *base → commentary*:
//!
//! ```text
//! girsa:bavli/berakhot/10a:1#418  --comments-on-->  girsa:bavli/rashi-on-berakhot/10a:1:1#367
//! ```
//!
//! Read aloud, that says the gemara is a commentary on Rashi. 15,394 edges on
//! Berakhot alone are written that way, Rashi's 3,139 among them.
//!
//! Nothing internal could see it, because a reversed edge is still an edge: it
//! has two real ends, both resolve to segments that exist, the type is right,
//! and the count is right. `inbound.jsonl` — keyed by whichever end the row
//! called `to` — dutifully filed Rashi's commentary under *Rashi*, so
//! `girsa-app`'s panel asks "what lands on this daf?" and is told: Ben
//! Yehoyada and Benayahu. Two aggadic commentaries, out of forty.
//!
//! What catches it is knowing, from outside the corpus, that **Rashi is on
//! Berakhot** — a fact no amount of internal consistency can supply. That is
//! what the table below is: not derived from the data, checked against it.
//!
//! # Why it now runs without the corpus, and why that is not a weaker test
//!
//! It used to `return` when the link graph was absent, so on a fresh clone it
//! printed `3 passed` in 0.00s having checked nothing — the audit that caught
//! this defect could not itself run anywhere the defect could be reintroduced.
//!
//! It runs on [`girsa_fixture`] now, and the reason that is a real check rather
//! than a fixture asserting itself is that **the fixture writes its
//! `comments-on` rows in both column orders, exactly as Sefaria's export does**,
//! and lets `girsa_link::orient` sort them out. Eight of its rows are written
//! base-first. If orientation regresses they come out backwards, the mefarshim
//! stop being reachable from the daf, and this fails — on a shelf built in a
//! second. The table below is unchanged and every pair in it is on that shelf.
//!
//! The same three checks against the real download are at the bottom, `#[ignore]`d
//! rather than skipped, because *forty commentaries land on Berakhot* is a fact
//! about a Sefaria release and no fixture can stand in for it.
//!
//! # Why the expectations are named and not computed
//!
//! Deriving them from slugs is forbidden, and rightly (BUILDER.md rule 6, and
//! the note on [`girsa_corpus::work::Work::commentary_on`]): `X-on-Y` would
//! attach `Rashi on Berakhot` to the Yerushalmi masechta of the same name. An
//! earlier draft of this audit did exactly that and produced four different
//! wrong answers in a row. The pairs here are spelled out, and the orientation
//! invariant is checked against `commentary_on`, which is read from Sefaria's
//! schema rather than guessed from a title.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use girsa_link::{inbound, store, EdgeType};

/// The fixture shelf, with its graph imported and its inbound cache built.
fn fixture() -> &'static Path {
    girsa_fixture::linked().root()
}

/// The real download, for the three `#[ignore]`d checks at the bottom.
fn corpus() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    assert!(
        root.join("links").is_dir() && inbound::built(&root),
        "no link graph with an inbound cache at {} — run girsa-link-import then          girsa-link-types. This check is #[ignore]d so its absence is never read          as a pass.",
        root.display()
    );
    root
}

/// slug -> the bases that work declares itself a commentary on.
fn declared(root: &Path) -> HashMap<String, Vec<String>> {
    let body = std::fs::read_to_string(root.join("works/index.jsonl")).expect("work index reads");
    body.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<girsa_corpus::work::Work>(l).ok())
        .map(|w| {
            let bases = w.commentary_on.into_iter().map(|b| b.slug).collect();
            (w.slug, bases)
        })
        .collect()
}

/// The works whose `comments-on` edges land on `base`, as the reverse index
/// holds them — which is the exact question `girsa-app`'s panel asks.
fn commentaries_on(root: &Path, base: &str) -> HashSet<String> {
    inbound::read_back(root, base)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.edge_type == EdgeType::CommentsOn)
        .map(|e| e.from.from.work().to_string())
        .collect()
}

/// Which mefarshim sit on which sefer. Known from learning them, not from the
/// corpus — that is the whole point. Every slug here was checked to exist and
/// to declare the matching base in `commentary_on` before being written down.
const ON_THE_PAGE: &[(&str, &[&str])] = &[
    // The daf. If any single row of this table matters, it is this one.
    (
        "bavli/berakhot",
        &["bavli/rashi-on-berakhot", "bavli/tosafot-on-berakhot"],
    ),
    // The chumash. Rashi alone is not a chumash.
    (
        "genesis",
        &[
            "rashi-on-genesis",
            "ramban-on-genesis",
            "ibn-ezra-on-genesis",
            "sforno-on-genesis",
            "radak-on-genesis",
            "rashbam-on-genesis",
            "onkelos-genesis",
        ],
    ),
    // Mishnah: the Rambam's peirush and the Bartenura are the two printed on
    // every page of every tractate.
    (
        "mishnah-berakhot",
        &[
            "rambam-on-mishnah-berakhot",
            "bartenura-on-mishnah-berakhot",
        ],
    ),
    // Shulchan Arukh. The Taz is beside the Magen Avraham on every daf of
    // Orach Chayim, and beside the Shach on every daf of Yoreh De'ah.
    (
        "shulchan-arukh/orach-chayim",
        &[
            "magen-avraham",
            "mishnah-berurah",
            "turei-zahav-on-shulchan-arukh/orach-chayim",
        ],
    ),
    (
        "shulchan-arukh/yoreh-deah",
        &[
            "siftei-kohen-on-shulchan-arukh/yoreh-deah",
            "turei-zahav-on-shulchan-arukh/yoreh-deah",
        ],
    ),
];

#[test]
fn the_mefarshim_printed_on_the_page_are_reachable_from_it() {
    mefarshim_are_reachable(fixture());
}

#[test]
#[ignore = "needs the fetched corpus: cargo test -p girsa-link -- --ignored"]
fn the_mefarshim_printed_on_the_page_are_reachable_from_the_real_daf() {
    mefarshim_are_reachable(&corpus());
}

fn mefarshim_are_reachable(root: &Path) {
    let declares = declared(root);
    let mut missing = Vec::new();

    for (base, expected) in ON_THE_PAGE {
        let live = commentaries_on(root, base);
        for commentary in *expected {
            // A wrong slug in this table would be a test that fails for a
            // reason that has nothing to do with the graph, so the declaration
            // is asserted rather than assumed.
            assert!(
                declares
                    .get(*commentary)
                    .is_some_and(|bases| bases.iter().any(|b| b == base)),
                "{commentary} does not declare itself a commentary on {base} — \
                 the expectation table is wrong, not the graph"
            );
            if !live.contains(*commentary) {
                missing.push(format!("{base} <- {commentary}"));
            }
        }
        println!("{base}: {} commentaries land on it", live.len());
    }

    assert!(
        missing.is_empty(),
        "{} of the mefarshim printed on the page cannot be reached from it:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}

#[test]
fn comments_on_points_from_the_commentary_to_the_sefer_it_comments_on() {
    comments_on_points_the_right_way(fixture());
}

#[test]
#[ignore = "needs the fetched corpus: cargo test -p girsa-link -- --ignored"]
fn comments_on_points_the_right_way_in_the_real_graph() {
    comments_on_points_the_right_way(&corpus());
}

fn comments_on_points_the_right_way(root: &Path) {
    let declares = declared(root);

    // Checked on the shards of a few base works rather than all seven
    // thousand: a reversed edge lives in the *base* work's outgoing shard, so
    // that is where to look, and one masechta is enough to prove the rule is
    // or is not being kept.
    for base in [
        "bavli/berakhot",
        "genesis",
        "mishnah-berakhot",
        "shulchan-arukh/orach-chayim",
    ] {
        // A base work every one of whose edges was oriented away from it has no
        // outgoing shard at all, which is the right answer and not a missing
        // file: `unwrap_or_default` reads it as the empty set it is.
        let edges = store::read_back(root, base).unwrap_or_default();
        let mut backwards = 0usize;
        let mut first = None;

        for edge in edges.iter().filter(|e| e.edge_type == EdgeType::CommentsOn) {
            let (from, to) = (edge.from.from.work(), edge.to.from.work());
            let says = |a: &str, b: &str| {
                declares
                    .get(a)
                    .is_some_and(|bases| bases.iter().any(|s| s == b))
            };
            // Right way round: the `from` end declares the `to` end as its
            // base. Backwards: the `to` end declares the `from` end. Neither
            // declaring anything is a separate question — an edge between two
            // works whose relationship the corpus never stated — and is not
            // counted here, because flipping it would be a guess.
            if !says(from, to) && says(to, from) {
                backwards += 1;
                if first.is_none() {
                    first = Some(format!(
                        "{} --comments-on--> {}",
                        edge.from.from, edge.to.from
                    ));
                }
            }
        }

        assert_eq!(
            backwards,
            0,
            "{base}: {backwards} comments-on edges point from the sefer to its own commentary, \
             e.g.\n    {}\n  which reads: the base text is a commentary on its commentary.",
            first.unwrap_or_default()
        );
    }
}

#[test]
fn a_commentary_is_not_attached_to_a_sefer_it_was_never_written_on() {
    nothing_is_attached_to_what_it_was_never_written_on(fixture());
}

#[test]
#[ignore = "needs the fetched corpus: cargo test -p girsa-link -- --ignored"]
fn nothing_in_the_real_graph_is_attached_to_what_it_was_never_written_on() {
    nothing_is_attached_to_what_it_was_never_written_on(&corpus());
}

fn nothing_is_attached_to_what_it_was_never_written_on(root: &Path) {
    // The counterpart to the table above, and the reason the fix cannot simply
    // flip every edge it is unsure about. The Mishnah Berurah is on Orach
    // Chayim and on nothing else; the Shach is on Yoreh De'ah and Choshen
    // Mishpat and not on Orach Chayim. An import that invented these would
    // score better on the test above and be wrong.
    const NEVER: &[(&str, &str)] = &[
        ("shulchan-arukh/yoreh-deah", "mishnah-berurah"),
        ("shulchan-arukh/choshen-mishpat", "mishnah-berurah"),
        (
            "shulchan-arukh/orach-chayim",
            "siftei-kohen-on-shulchan-arukh/yoreh-deah",
        ),
        ("shulchan-arukh/even-haezer", "magen-avraham"),
    ];
    for (base, never) in NEVER {
        assert!(
            !commentaries_on(root, base).contains(*never),
            "{never} is attached to {base}, which it was never written on"
        );
    }
}
