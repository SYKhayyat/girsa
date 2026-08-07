//! Invariants this repository states in doc comments, checked.
//!
//! # The diagnosis this file is the answer to
//!
//! The 6 August 2026 report's central finding was not any one defect. It was
//! that **this codebase reliably wins the argument it is having, and reliably
//! does not notice it has had the argument before** — and its evidence was a
//! table of ten doc comments, each stating an invariant, each sitting next to a
//! caller that violated it. Its conclusion:
//!
//! > The design lives in prose, and prose is not checkable. Every invariant in
//! > the table above was written down beautifully — and writing it down was
//! > mistaken for enforcing it.
//!
//! So the highest-leverage change in the repository is not any individual fix.
//! It is turning those doc comments into tests. This is where they go.
//!
//! # Why these are source scans, and when a source scan is the wrong tool
//!
//! Most invariants here are better expressed as types or as behaviour, and where
//! they can be, they are — `Bases::orient` is private so `girsa-link-orient`
//! cannot count without stamping; `Touching` has three variants so no caller can
//! read a missing cache as a zero. Those need no test in this file.
//!
//! What is left is the shape a type cannot hold: **"there is one implementation
//! of this."** Nothing in Rust says *no second function anywhere may be named
//! `slug_of`*, and every instance in that table was exactly a second
//! implementation. A scan over the tracked source is blunt, and it is what
//! catches a copy the day it is pasted rather than in an audit six weeks later.
//!
//! Each check names the doc comment it enforces, so a reader who trips one can
//! go and read the argument rather than guess at it.

use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| panic!("the repository root resolves: {e}"))
}

/// Every `.rs` file in the workspace and the shell, with its repo-relative path.
///
/// Not `target/`, and not the sibling checkouts — `sefer-crates` is a different
/// repository with its own rules, and a rule this one wrote down does not bind
/// it.
fn sources(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut stack = vec![root.join("crates"), root.join("app/src-tauri/src")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|x| x == "rs") {
                let body = std::fs::read_to_string(&path).unwrap_or_default();
                let named = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                found.push((named, body));
            }
        }
    }
    assert!(
        found.len() > 50,
        "only {} source files found — the walk is wrong, and a check that cannot \
         find what it checks is the one thing this repository forbids by name",
        found.len()
    );
    found
}

/// This file, which names every signature it forbids and would otherwise be
/// the only violation of every rule in it.
const SELF: &str = "the_rules_this_repository_wrote_down.rs";

/// Where a function of this name is defined, across the tree.
fn defined_in(root: &Path, signature: &str) -> Vec<String> {
    sources(root)
        .into_iter()
        .filter(|(named, body)| !named.ends_with(SELF) && body.contains(signature))
        .map(|(named, _)| named)
        .collect()
}

#[test]
fn slug_of_has_one_implementation() {
    // `girsa_corpus::work::slug_of`:
    //
    //   **This is the same function the lexicon is built with**, and it has to
    //   be: the lexicon maps a citation onto a slug and the importer names
    //   segments after one, so a second implementation that drifted by a hyphen
    //   would resolve citations onto works that do not exist.
    //
    // `examples/build-lexicon.rs` was a second implementation — byte-identical,
    // thirty-seven lines, directly beneath that sentence. So was the sentence.
    let root = repo();
    let mut found = defined_in(&root, "fn slug_of(");
    found.sort();
    assert_eq!(
        found,
        vec!["crates/girsa-corpus/src/work.rs".to_string()],
        "a second `slug_of`. The lexicon and the importer have to spell a work \
         the same way; see the doc comment on the original."
    );
}

#[test]
fn find_index_has_one_implementation() {
    // `girsa_note::since::index_candidates`:
    //
    //   Shared, so a search panel that finds an index and a `girsa-read` that
    //   does not cannot be two answers to one question.
    //
    // `app/src-tauri/src/lib.rs:855` was a second one — **forty lines above a
    // call to the shared one, in the same file** — with the same three
    // candidates in the same order and a different accept predicate. It took
    // only `girsa-cache.json`; the shared one also takes a bare tantivy
    // `meta.json`. So a directory `girsa-read` called an index, the window
    // called *no search index*.
    let root = repo();
    let mut found = defined_in(&root, "fn find_index(");
    found.sort();
    assert_eq!(
        found,
        vec!["crates/girsa-note/src/since.rs".to_string()],
        "a second `find_index`. Two of these had two accept predicates and disagreed about whether a directory was an index."
    );
}

#[test]
fn a_prepared_query_is_never_rebuilt_to_be_asked_again() {
    // `girsa_search::index::Prepared`:
    //
    //   Hits, a total and the facet counts are three questions about **one**
    //   query, and a facet computed from a differently-built copy of it would
    //   be a column of numbers that did not add up to the header.
    //
    // `Bar::literally` built one for the facets and let `search_in` build a
    // second, private one for the hits. `Bar::smartly` was worse: it called
    // `prepare_widened(&answered.widened)` — rebuilding off the field whose own
    // doc comment says the facets must come from *"the search that ran rather
    // than one built afterwards to look like it."*
    //
    // The seam is `SearchIndex::found_with`, which takes a `Prepared`. So: no
    // caller of `prepare` may also call a `search_*` that prepares privately,
    // and the way to hold that is that `bar.rs` does not name those functions
    // at all any more.
    let root = repo();
    // Comments stripped, because the note above the fixed call site names the
    // call it replaced — and a check that a *comment* cannot mention what it
    // forbids is a check that makes the code harder to explain.
    let bar: String = std::fs::read_to_string(root.join("crates/girsa-search/src/bar.rs"))
        .unwrap_or_else(|e| panic!("bar.rs reads: {e}"))
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    for rebuilds in ["search_in(", "search_widened_in(", "prepare_widened("] {
        assert!(
            !bar.contains(rebuilds),
            "`bar.rs` calls `{rebuilds}`, which prepares a query privately — and it already holds a `Prepared` for the facets. Two builds of one query is two chances for a facet column not to add up to the header above it. Use `SearchIndex::found_with`."
        );
    }
}

#[test]
fn nothing_re_anchors_by_exact_id_where_a_standing_is_the_question() {
    // `girsa_app::shelf::Open::standing`:
    //
    //   The one derivation of `Standing` … Every consumer that used to ask
    //   `SegmentId::covers` asks this instead.
    //
    // Every consumer but one. `girsa_fix::Layer::apply` looked its patches up
    // with `by_segment.get(&id)`, so a correction made before the corpus folded
    // that se'if into another stopped applying — silently, because `apply`
    // reports a patch whose *letters* it cannot find and this one was never
    // looked up at all.
    //
    // The seam is `Layer::apply_at`, which takes a `Standing`. `Layer::apply`
    // stays for the write path, where the exact id is the right question.
    let root = repo();
    let shelf = std::fs::read_to_string(root.join("crates/girsa-app/src/shelf.rs"))
        .unwrap_or_else(|e| panic!("shelf.rs reads: {e}"));
    assert!(
        shelf.contains("apply_at(&standing"),
        "the reading pane stopped asking `Layer::apply_at`. A correction is stored under the name the place had when it was made, and an exact lookup will miss it the day upstream re-segments the work — see `an_anchor_survives_a_split_at_import.rs`."
    );
}

#[test]
fn a_number_a_reader_can_change_is_clamped_in_one_place() {
    // `girsa_app::session::Look::sane`:
    //
    //   Clamped in **one** place, here, rather than in the window and again in
    //   the command.
    //
    // Three numbers, three places, one of them in another language.
    // `set_text_size` clamped inline — `percent.clamp(60, 250)` — sixty-eight
    // lines below that sentence; `Look::sane` itself ran only from `set_look`,
    // never on load, so a hand-edited session file was believed; and the split
    // ratio's real bounds, 15–85%, existed **only** in `layout.ts`, against
    // `ratio.min(1000)` in Rust.
    //
    // `Session::sane` is the one place now, and `Session::load` runs it — a
    // clamp that only fires in a setter is a rule about a code path rather than
    // about the value.
    let root = repo();
    for (file, forbidden, why) in [
        (
            "app/src-tauri/src/lib.rs",
            "clamp(60, 250)",
            "the shell clamps the reading size itself",
        ),
        (
            "app/src/layout.ts",
            "Math.max(15,",
            "the window holds its own split bounds",
        ),
    ] {
        let body = std::fs::read_to_string(root.join(file))
            .unwrap_or_else(|e| panic!("{file} reads: {e}"))
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                !line.starts_with("//") && !line.starts_with("///")
            })
            .collect::<Vec<_>>()
            .join(
                "
",
            );
        assert!(
            !body.contains(forbidden),
            "{file}: {why}. `girsa_app::session::Session::sane` is the one place, and `Session::load` runs it."
        );
    }
}

#[test]
fn a_wire_format_is_spelled_once_and_not_derived_from_an_identifier() {
    // `girsa_fix::Kind::as_str`:
    //
    //   What the window calls it. One implementation, so the word in the file,
    //   the word on the button and the word the tests use cannot drift.
    //
    // …on a type that also carried `#[serde(rename_all = "lowercase")]`. Two
    // spellings of one wire format, on one type, under a sentence about there
    // being one.
    //
    // The root is wider than the two that were doubled. `rename_all` derives
    // the **file format from the identifier**, so renaming a variant —
    // `FixedWithVariants` → `WithVariants` — silently changes what is on disk,
    // and the corrections a reader made last year stop reading. Eleven fieldless
    // enums across seven crates were spelled that way.
    //
    // So: a fieldless enum states its spellings, through
    // `girsa_corpus::spelled!`. Two shapes are deliberately *not* caught, and
    // both are right —
    //
    //   * a **tagged** union (`#[serde(tag = "does", rename_all = …)]`) has
    //     variants with fields and no `as_str`; the rename is about its tag.
    //   * `#[serde(other)]` is a catch-all: an unknown word becomes that
    //     variant on purpose, and `spelled!` refuses unknown words by design.
    //     Converting `work::Mapping` would have turned a tolerated Sefaria
    //     value into a work that will not parse.
    let root = repo();
    let mut wrong = Vec::new();
    for (named, body) in sources(&root) {
        if named.ends_with(SELF) || named.ends_with("girsa-corpus/src/lib.rs") {
            continue;
        }
        let lines: Vec<&str> = body.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            let line = line.trim();
            if !line.starts_with("#[serde(rename_all") || !line.ends_with(")]") {
                continue;
            }
            // A tagged union renames its tag, which is a different fact.
            if line.contains("tag") {
                continue;
            }
            let Some(head) = lines[at + 1..]
                .iter()
                .map(|l| l.trim())
                .find(|l| !l.is_empty() && !l.starts_with("///"))
            else {
                continue;
            };
            if !head.contains("enum ") {
                continue;
            }
            let body_of: String = lines[at..]
                .iter()
                .take_while(|l| !l.trim_start().starts_with('}'))
                .copied()
                .collect::<Vec<_>>()
                .join(
                    "
",
                );
            // Variants with fields, or a catch-all: not this rule's business.
            if body_of.contains('(') || body_of.contains("#[serde(other)]") {
                continue;
            }
            wrong.push(format!("{named}: {head}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "a fieldless enum whose wire spelling is derived from its variant names:
  {}

         Rename one of those variants and the file format changes with it. `girsa_corpus::spelled!` states the spellings and generates `as_str`, `named`, `Serialize` and `Deserialize` from that one list.",
        wrong.join("
  ")
    );
}

#[test]
fn one_answer_has_one_highlighting_rule() {
    // `girsa_search::torat_emet` states the hazard, about a different pair:
    //
    //   two descriptions of one rule drift
    //
    // `Found::marks` and `bar::Marker` were two descriptions of *this* rule —
    // character-identical, four lines each — and each had its own caller:
    // `girsa-index find` highlighted its results through one and the window
    // through the other. A widened hit is marked on the word that answered
    // (`וכשהמלך`) and not on the three letters the reader typed, and that
    // sentence was true twice.
    //
    // `Found::marker` is the one description now, and `Found::marks` calls it.
    let root = repo();
    let index = std::fs::read_to_string(root.join("crates/girsa-search/src/index.rs"))
        .unwrap_or_else(|e| panic!("index.rs reads: {e}"));
    assert!(
        !index.contains("matches_word"),
        "`index.rs` walks tokens against a widening itself again. That rule is `bar::Marker`, and `Found::marker` is how this file reaches it."
    );
    let bar = std::fs::read_to_string(root.join("crates/girsa-search/src/bar.rs"))
        .unwrap_or_else(|e| panic!("bar.rs reads: {e}"));
    assert!(
        bar.contains("found.marker()"),
        "`Bar::results` builds a `Marker` out of `Found`'s two fields itself again. `Found::marker` is that."
    );
}

#[test]
fn hebrew_is_put_into_markup_by_one_rule() {
    // Three copies of one rule about Hebrew punctuation, in one crate.
    // `sending.rs` and `scanning.rs` each carried an `escape_text` and an
    // `escape_attr`, **byte-identical, including a five-line comment about
    // gershayim** — so the paragraph explaining why the quote mark must not be
    // escaped in text existed twice and could rot in one. `export.rs` carried a
    // third, as a chain of `replace` calls.
    //
    // `girsa_app::markup` is the one. `"` and `'` are how Hebrew writes
    // gershayim; escaping them in text turns `שו"ע או"ח סימן א'` into noise.
    let root = repo();
    let mut wrong = Vec::new();
    for (named, body) in sources(&root) {
        if named.ends_with(SELF) || named.ends_with("girsa-app/src/markup.rs") {
            continue;
        }
        // The *act* of escaping, not the string. Every false start here was a
        // test asserting on escaped output, or `mine.rs` decoding entities on
        // the way in — both of which name `&amp;` and neither of which is a
        // second escaper.
        for escaping in ["push_str(\"&amp;\")", "replace('&', \"&amp;\")"] {
            if body.contains(escaping) {
                wrong.push(named.clone());
                break;
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "a fourth HTML escaper:
  {}

It is `girsa_app::markup::text` and `::attr`, and the reason there are two of those and not one is a fact about Hebrew rather than about markup.",
        wrong.join("
  ")
    );
}

#[test]
fn one_sentence_says_what_an_answer_could_not_see() {
    // Three composers, each with a doc comment naming itself the only one:
    // `Coverage::said` ("the window, the command line, the MCP surface and the
    // test cannot drift apart"), `Gap::said` ("the window's line, the CLI's
    // line, the MCP server's line"), `Unindexed::said` ("the window's header,
    // `girsa-read`'s line, `girsa-index find`'s footer and the MCP server's
    // field"). Each was right about its own clause and none could see the other
    // two, so what drifted was everything between them: `Coverage` joined with a
    // semicolon, the other two with a middle dot; `Coverage` alone knew a
    // five-figure number wants a comma in it; and `Gap` joined an already-joined
    // string into its own join, so a four-clause sentence read correctly only
    // because both levels happened to pick the same separator.
    //
    // The rule that ends it: a module that words a clause of this sentence hands
    // it to `girsa_corpus::said::Clauses` and does no joining of its own. The
    // separator is spelled once, in the crate below all of them.
    const WORDS_A_CLAUSE: [&str; 4] = [
        "girsa-lane/src/coverage.rs",
        "girsa-app/src/reading.rs",
        "girsa-note/src/since.rs",
        "girsa-app/src/unseen.rs",
    ];
    let root = repo();
    let mut wrong = Vec::new();
    let mut found = 0;
    for (named, body) in sources(&root) {
        if !WORDS_A_CLAUSE.iter().any(|owner| named.ends_with(owner)) {
            continue;
        }
        found += 1;
        // Comments quote the sentences they are about, and a doc comment naming
        // the old separator is the argument for the new one.
        let code: String = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        if !code.contains("fn clauses(&self) -> Clauses") {
            wrong.push(format!(
                "{named}: words a clause and does not hand it over as `Clauses`"
            ));
        }
        // The two sentence separators. Not `.join(` — `Path::join` is all over
        // these files and `names.join(", ")` inside `Coverage::naming` joins
        // titles *within* one clause, which is that clause's own business. What
        // is forbidden is a module deciding how its clause sits beside somebody
        // else's.
        for spelt in ["\u{b7}", "\"; "] {
            if code.contains(spelt) {
                wrong.push(format!(
                    "{named}: spells `{spelt}` in code — the joining is `Clauses`"
                ));
            }
        }
    }
    assert_eq!(
        found,
        WORDS_A_CLAUSE.len(),
        "a clause module moved and this list did not"
    );
    assert!(
        wrong.is_empty(),
        "a module that words a clause is doing its own joining:\n  {}\n\n\
         The clause belongs to whoever knows the fact — that part of all three \
         doc comments was right. The joining is `girsa_corpus::said::Clauses`, \
         and `girsa_app::Unseen` decides which clauses are one answer, which is \
         the decision none of the three earlier composers was in a position to \
         make.",
        wrong.join("\n  ")
    );
}

#[test]
fn the_browser_stub_says_what_the_lane_says() {
    // `app/src/api.ts`'s stub had *"nothing is in the semantic lane yet"* typed
    // out twice, a fourth copy of `girsa_lane::coverage::NOTHING_YET` — in the
    // one language that cannot import it. It is one TypeScript constant now,
    // and this is the only thing that can hold the two together.
    let root = repo();
    let api = std::fs::read_to_string(root.join("app/src/api.ts"))
        .unwrap_or_else(|e| panic!("api.ts reads: {e}"));
    let marker = "export const NOTHING_YET = \"";
    let at = api
        .find(marker)
        .unwrap_or_else(|| panic!("api.ts declares NOTHING_YET"));
    let rest = &api[at + marker.len()..];
    let said = &rest[..rest.find('"').unwrap_or(0)];
    assert_eq!(
        said,
        girsa_lane::coverage::NOTHING_YET,
        "the browser build says one thing about an empty lane and Rust says another"
    );
    assert_eq!(
        api.matches("nothing is in the semantic lane yet").count(),
        1,
        "the sentence is typed out more than once in api.ts"
    );
}
