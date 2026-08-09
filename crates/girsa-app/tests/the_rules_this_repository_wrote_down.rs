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
    // stays for the write path, where the exact id is the right question — and
    // for the reading pane's one shortcut, which is why this check has two
    // halves. Deriving a `Standing` per segment cost 3.6 of the three seconds
    // in `three_seconds.rs`, so `corrected_by` asks `moved` once for the whole
    // work first: has anything under these corrections been re-segmented at
    // all. When the answer is no, `Standing::derived` would return `{id}` for
    // every segment and the exact lookup is the same lookup.
    //
    // What a source scan can check is that both halves are still there: the
    // `Standing` path, and the precheck that is the *only* thing making the
    // exact-id path safe. That the `if` still connects them is what
    // `an_anchor_survives_a_split_at_import.rs` runs.
    let root = repo();
    let shelf = std::fs::read_to_string(root.join("crates/girsa-app/src/shelf.rs"))
        .unwrap_or_else(|e| panic!("shelf.rs reads: {e}"));
    assert!(
        shelf.contains("apply_at(standing"),
        "the reading pane stopped asking `Layer::apply_at`. A correction is stored under the name the place had when it was made, and an exact lookup will miss it the day upstream re-segments the work — see `an_anchor_survives_a_split_at_import.rs`."
    );
    assert!(
        shelf.contains("let moved = fixes"),
        "the reading pane takes the exact-id shortcut without asking whether anything moved. `Layer::apply` is only the same question as `apply_at` on a work nothing has re-segmented; without the precheck it is a correction that silently stops applying."
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
        "girsa-nearby/src/unseen.rs",
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
         and `girsa_nearby::Unseen` decides which clauses are one answer, which is \
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
    //
    // Both sides are read out of the source rather than one of them imported,
    // because `girsa-lane` is no longer a dependency of this crate — it is
    // `girsa-nearby`'s, so the reading workspace stops building a BERT to
    // retest the taxonomy. Taking it as a dev-dependency to keep one `assert_eq`
    // typed would put candle back in `cargo test -p girsa-app` for a string.
    // A rename still fails here: the constant has to be found by name.
    let root = repo();
    let api = std::fs::read_to_string(root.join("app/src/api.ts"))
        .unwrap_or_else(|e| panic!("api.ts reads: {e}"));
    let marker = "export const NOTHING_YET = \"";
    let at = api
        .find(marker)
        .unwrap_or_else(|| panic!("api.ts declares NOTHING_YET"));
    let rest = &api[at + marker.len()..];
    let said = &rest[..rest.find('"').unwrap_or(0)];
    let rust = std::fs::read_to_string(root.join("crates/girsa-lane/src/coverage.rs"))
        .unwrap_or_else(|e| panic!("coverage.rs reads: {e}"));
    let declared = "pub const NOTHING_YET: &str = \"";
    let at = rust
        .find(declared)
        .unwrap_or_else(|| panic!("girsa_lane::coverage declares NOTHING_YET"));
    let rest = &rust[at + declared.len()..];
    let means = &rest[..rest.find('"').unwrap_or(0)];
    assert_eq!(
        said, means,
        "the browser build says one thing about an empty lane and Rust says another"
    );
    assert_eq!(
        api.matches("nothing is in the semantic lane yet").count(),
        1,
        "the sentence is typed out more than once in api.ts"
    );
}

#[test]
fn one_rule_says_what_a_place_is_called_and_where_it_sits() {
    // Six rows described a segment for a reader — `HitRow` twice over, `Near`,
    // `mcp::named`, `girsa-chain`'s printer, `PatchRow`, `SuspectRow`, a folder
    // member — and each worked out the title, the address and the date itself.
    //
    // Read the columns rather than the rows and none of the differences was a
    // decision. `HitRow` honoured the language the reader set, because it was
    // built where a `Session` was in scope; `mcp::named` carried the years,
    // because it was built where a `Timeline` was; `Near` had **no address at
    // all**, so the window and `girsa-lane ask` each invented one and invented
    // different ones. Nobody chose that. It is what was reachable from where
    // the code happened to be written.
    //
    // `girsa_app::Naming` is the rule and `girsa_app::Names` is what it takes
    // to apply it — a shelf, a timeline and a language, passed *instead of* a
    // bare `&Shelf` so that a caller with no dates says so once rather than by
    // leaving a column quietly empty.
    let root = repo();

    // The address. `SegmentId::address` is the spelling; there were seventeen
    // of the expression and eleven more sites that skipped it and printed the
    // whole permanent id where an address goes.
    let mut wrong = Vec::new();
    for (named, body) in sources(&root) {
        if named.ends_with(SELF) || named.ends_with("girsa-corpus/src/segment.rs") {
            continue;
        }
        let code: String = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        if code.contains("path().join(\":\")") {
            wrong.push(named);
        }
    }
    assert!(
        wrong.is_empty(),
        "an address spelled by hand:\n  {}\n\nIt is `SegmentId::address`.",
        wrong.join("\n  ")
    );

    // The title. `Language::title_of` is the rule and `Names::of` is where a
    // row reaches it.
    //
    // The one exception is `LinkRow::of`, and it is a real one: `girsa_app::Link`
    // carries **both** names of the sefer at the other end, the way `Card` does,
    // and the surface picks. That is the other correct shape — a row that
    // carries a sefer rather than a name to print — and the defect was never
    // that two shapes exist. It was six rows of the first shape, each deciding
    // privately.
    //
    // `view.rs` is on this list for that one constructor. If a second row in
    // there starts choosing, this check will not catch it, which is the cost of
    // a whitelist being per-file — and it is the reason the list is three
    // entries long rather than a habit.
    const MAY_CHOOSE: [&str; 4] = [
        "girsa-app/src/naming.rs",
        "girsa-app/src/session.rs",
        "girsa-app/src/view.rs",
        "app/src-tauri/src/lib.rs",
    ];
    let mut chose = Vec::new();
    for (named, body) in sources(&root) {
        if named.ends_with(SELF) || MAY_CHOOSE.iter().any(|ok| named.ends_with(ok)) {
            continue;
        }
        // `Language::title_of(&w.he_title, &w.en_title)` — the call that picks
        // one of a sefer's two names. On one line, so this is not
        // `Arrangement::title_of`, which names a *shelf* by its key and is a
        // different question with an unfortunately similar name.
        //
        // A row that carries **both** names, the way `Card` and
        // `mefarshim::Choice` do, is the other correct shape and is not this:
        // it hands the choice to `names.ts`, which is the one place in the
        // window allowed to make it.
        if body
            .lines()
            .any(|line| line.contains("title_of(") && line.contains("he_title"))
        {
            chose.push(named);
        }
    }
    assert!(
        chose.is_empty(),
        "a row deciding for itself which of a sefer's two names to print:\n  {}\n\n\
         `girsa_app::Names::of` is where a row reaches `Language::title_of`.",
        chose.join("\n  ")
    );
}

#[test]
fn every_binary_reads_its_command_line_the_same_way() {
    // Sixteen binaries, five conventions, and no shared line of code. The
    // `corpus personal` prefix had six answers — defaulted, required,
    // `<corpus> <otzaria>`, `<index> <personal>`, root-only, and after the
    // subcommand — and `girsa-lane` required it in the same directory as four
    // siblings that defaulted it.
    //
    // Three of the parsers cost something:
    //
    // - `girsa-chain`'s usage said `[--depth N]` and its parser accepted only
    //   `--depth=N`, so typing what the usage said left a bare `N` among the
    //   segment ids.
    // - `girsa-notes`' `split_flags` made **every** `--x` swallow the next
    //   token, so a switch ate a positional.
    // - `girsa-link-orient`'s `other => root = PathBuf::from(other)` turned a
    //   mistyped `--replce` into the corpus root, and the run read a directory
    //   of that name, found nothing, and reported that it had finished.
    //
    // Each is the same shape: a parser that cannot tell a switch from a value
    // option because nothing told it which is which. `girsa_corpus::argv` is
    // told.
    let root = repo();
    let mut wrong = Vec::new();
    let mut bins = 0;
    for (named, body) in sources(&root) {
        if !named.contains("/src/bin/") {
            continue;
        }
        bins += 1;
        // `girsa-card` prints the shortcut sheet and reads no arguments at all,
        // which is the one honest way not to use this.
        if !body.contains("std::env::args()") {
            continue;
        }
        if !body.contains("girsa_corpus::argv") {
            wrong.push(format!(
                "{named}: reads argv and not through `girsa_corpus::argv`"
            ));
        }
        // A `fn flag` or a `fn split_flags` of its own. There were three
        // functions named `flag`, with three signatures and two incompatible
        // value syntaxes between them.
        for spelt in ["fn flag(", "fn split_flags("] {
            if body.contains(spelt) {
                wrong.push(format!("{named}: has its own `{spelt}`"));
            }
        }
        // `ExitCode::FAILURE` for a mistyped verb. Four binaries did that,
        // through the same path as *the shelf will not open*, so a script could
        // not tell a typo from a broken corpus.
        if body.contains("no such command") && !body.contains("argv::refuse") {
            wrong.push(format!(
                "{named}: refuses a bad verb without `argv::refuse`"
            ));
        }
        // Every binary answers `--help`. Only `girsa-index` used to; typing
        // `girsa-shelf --help` set the corpus root to the string `"--help"`.
        if !body.contains("wants_help") {
            wrong.push(format!("{named}: does not answer --help"));
        }
    }
    assert!(
        bins >= 16,
        "only {bins} binaries walked — the check cannot find what it checks"
    );
    assert!(
        wrong.is_empty(),
        "a binary reading its own command line:\n  {}\n\n\
         `girsa_corpus::argv::Argv::of` takes the switches and the value \
         options by name, which is what makes `--near 5` and `--near=5` both \
         work, stops a switch eating the next word, and makes a typo an error \
         rather than a path.",
        wrong.join("\n  ")
    );
}

#[test]
fn no_crate_reads_another_crates_file_by_string_surgery() {
    // `girsa-note` may not depend on `girsa-fix` — siblings, and neither may
    // name the other — so `since.rs` counted unindexed corrections like this:
    //
    //     if !body.contains("\"when\"") { … }
    //     line.split("\"when\"").nth(1).trim_start_matches([':', ' ', '"'])
    //
    // One crate parsing another's file by hand, with `serde_json` sitting
    // unused in its own manifest, purely because a type name was out of reach.
    // It was correct — a `"when"` inside a string value is escaped, so the
    // split could not land in one — and it was correct by luck, and would have
    // stayed silently correct until somebody added a field called `whenever`.
    //
    // The answer was not to name `Patch`. Counting records in a log is a fact
    // about the log format, so it is `girsa_personal::since`, which both crates
    // already depend on.
    let root = repo();
    let mut wrong = Vec::new();
    for (named, body) in sources(&root) {
        if named.ends_with(SELF) {
            continue;
        }
        for (n, line) in body.lines().enumerate() {
            let code = line.trim_start();
            // Comments quote the surgery they replaced, and tests assert on
            // what a store wrote — `line.contains("\"dir\":\"undeclared\"")` is
            // a store checking its own output, which is the opposite of this.
            if code.starts_with("//") || code.starts_with("assert") {
                continue;
            }
            if code.contains(".split(\"\\\"") || code.contains(".splitn(\"\\\"") {
                wrong.push(format!("{named}:{}", n + 1));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "a JSON field pulled out of a line by splitting on its name:\n  {}\n\n\
         If the type is reachable, deserialise it. If it is not — a sibling \
         crate's record — the question is a fact about the **log format**, \
         which is `girsa-personal`'s: see `girsa_personal::since`.",
        wrong.join("\n  ")
    );
}

/// The README: *"`app/` is the Tauri shell: a window and fifty commands, and
/// **nothing that decides anything**."*
///
/// Four kinds of decision were in there, and each of them was a decision the
/// README says lives in `girsa-app`.
#[test]
fn the_shell_decides_nothing_it_says_it_decides_nothing_about() {
    let root = repo();
    let shell: Vec<(String, String)> = sources(&root)
        .into_iter()
        .filter(|(path, _)| path.starts_with("app/src-tauri/"))
        .collect();
    assert!(!shell.is_empty(), "the shell's sources are readable");

    let mut wrong = Vec::new();
    for (path, body) in &shell {
        for (at, line) in body.lines().enumerate() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            let said = |what: &str| format!("{path}:{}: {what} — {line}", at + 1);

            // Who is writing. It read `USERNAME` first and had never heard of
            // `GIRSA_WHO`, so a reader who set the one variable this project
            // offers got that name on notes written from the terminal and
            // their operating-system login on every patch made in the window.
            if line.contains("\"USERNAME\"") || line.contains("\"USER\"") {
                wrong.push(said("who is writing is `girsa_personal::who`"));
            }
            // How long a sefer stays in memory, and which one goes.
            if line.contains("KEEP_OPEN") {
                wrong.push(said("how many seforim stay open is `girsa_app::held`"));
            }
            // Which fonts a Hebrew reading application offers.
            if line.contains("Frank Ruehl") || line.contains("SBL Hebrew") {
                wrong.push(said("the font families are `girsa_app::session::FONTS`"));
            }
            // What makes a directory a corpus.
            if line.contains("works/index.jsonl") {
                wrong.push(said("what makes a corpus is `girsa_corpus::roots`"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the shell decided something:\n{}",
        wrong.join("\n")
    );
}

/// `Chips::choose`: *"a value invented on read is a claim nobody made."*
///
/// Four chip families were read with a hand-written `match` whose last arm was
/// `_ => the default`, forty lines from a `link_repair` that refused an unknown
/// candidate by name. Two policies about one question, in one file, and the
/// quiet one was the one the search bar used.
#[test]
fn no_chip_family_is_read_with_a_silent_fallback() {
    let root = repo();
    let mut wrong = Vec::new();
    for (path, body) in sources(&root) {
        if !path.starts_with("app/src-tauri/") {
            continue;
        }
        for (at, line) in body.lines().enumerate() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            // `_ => Mode::X` and friends: a wire value nobody sent, invented
            // because one that was sent did not match.
            for family in ["Mode::", "Match::", "Sounding::", "Together::"] {
                if line.starts_with("_ =>") && line.contains(family) {
                    wrong.push(format!(
                        "{path}:{}: a chip read with a fallback — `Chips::choose` refuses \
                         instead: {line}",
                        at + 1
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_refusal_this_codebase_names_has_a_sentence_in_the_window() {
    // `app/src/trouble.ts` turned an error into a Hebrew sentence by matching
    // **twenty-one regular expressions against the English prose of Rust's
    // `Display` impls** — which makes every error string in this repository
    // load-bearing API, and the only test asserting any of them was on the
    // TypeScript side against a hand-typed copy. Reword `"there is no index
    // here"` and both halves stay green while the reader stops being told what
    // to run and gets the generic fallback.
    //
    // Seven of the twenty-one match prose this project does not own — `os error
    // 2`, `connection refused`, `EOF while parsing`. Those stay regexes, and
    // that is honest. The other fourteen carry a name now
    // (`girsa_app::trouble::Code`), and this is what holds the two lists
    // together across the language boundary.
    let root = repo();
    let ts = std::fs::read_to_string(root.join("app/src/trouble.ts"))
        .unwrap_or_else(|e| panic!("trouble.ts reads: {e}"));
    let table = ts
        .split_once("const CODED: Record<string, (doing: string) => string> = {")
        .map(|(_, rest)| rest.split_once("\n};").map_or(rest, |(table, _)| table))
        .unwrap_or_else(|| panic!("trouble.ts declares CODED"));

    let mut missing = Vec::new();
    for (_, spelt) in girsa_app::trouble::Code::SPELLINGS {
        // `"no-index":` for a name with a dash in it, `poisoned:` for one
        // without — TypeScript quotes a key only when it has to.
        let quoted = format!("\"{spelt}\":");
        let bare = format!("\n  {spelt}:");
        if !table.contains(&quoted) && !table.contains(&bare) {
            missing.push(*spelt);
        }
    }
    assert!(
        missing.is_empty(),
        "a refusal Rust can send that the window has no sentence for: {missing:?}\n\n\
         Add a line to `CODED` in `app/src/trouble.ts`. A code with no line \
         falls through to the generic *something went wrong*, which is the \
         silence this whole arrangement replaced.",
    );
    assert!(
        girsa_app::trouble::Code::SPELLINGS.len() >= 8,
        "the code list is suspiciously short"
    );

    // The same, for the one error type that crosses to the other repository.
    //
    // `girsa_post::PostError` is not this codebase's prose and it is not
    // somebody else's either — it is the shared crate's, which both
    // applications compile. `trouble.ts` had it under `FAMILIES` with the note
    // *"the refusals this codebase does not own… whatever a `PostError` says"*,
    // matched by four regexes that Ksav's `diagnostics.ts` also carried,
    // character for character. Two repositories keying on the English words of a
    // `Display` impl in a third, in the crate that exists so the two sides need
    // not agree in prose.
    //
    // `PostError::Io` and `::Json` are deliberately uncoded and are not checked
    // here: they forward the operating system's failure and serde's, and the
    // distinction a reader needs lives in those words alone.
    let mut missing = Vec::new();
    for code in girsa_post::PostError::CODES {
        if !table.contains(&format!("\"{code}\":")) {
            missing.push(*code);
        }
    }
    assert!(
        missing.is_empty(),
        "girsa-post can send {missing:?} and `CODED` has no line for it — the \
         reader would be shown the English, which is the bug `trouble.ts` and \
         `presence.ts` both cite as their reason for existing.",
    );
    assert!(
        !girsa_post::PostError::CODES.is_empty(),
        "PostError::CODES is empty, so the sweep above checked nothing"
    );
}

#[test]
fn the_reading_workspace_does_not_take_a_dependency_it_reads_nothing_from() {
    // `girsa-app` is *"the shelf, tabs and splits, and what keeps two columns
    // together"*, and its manifest was the confession: a BERT and three
    // `candle` crates for `adjacent.rs`, a document format for `buffer.rs`, and
    // `zip` because `export.rs` writes a `.docx`. Thirty modules, and
    // `cargo test -p girsa-app` built the forward pass in order to retest the
    // taxonomy.
    //
    // Three crates now sit *above* it — `girsa-nearby`, `girsa-desk`,
    // `girsa-export` — and the arrow runs one way. The seam is only worth what
    // it costs to keep, and what it costs is one line in a manifest.
    //
    // A dev-dependency is a different claim and is allowed: `girsa-ksav` is
    // there for one assertion about what a scan's packet becomes, and nothing a
    // reader runs compiles it.
    let root = repo();
    let manifest = std::fs::read_to_string(root.join("crates/girsa-app/Cargo.toml"))
        .unwrap_or_else(|e| panic!("girsa-app's Cargo.toml reads: {e}"));
    let (deps, _) = manifest
        .split_once("[dev-dependencies]")
        .unwrap_or((manifest.as_str(), ""));
    for (crate_name, whose) in [
        ("girsa-lane", "girsa-nearby"),
        ("girsa-ksav", "girsa-desk"),
        ("zip", "girsa-export"),
    ] {
        assert!(
            !deps.contains(&format!("
{crate_name}")),
            "`girsa-app` depends on `{crate_name}` again — that is `{whose}`'s, and the reading              workspace reads nothing from it. Whatever needed it belongs above this crate, not              inside it."
        );
    }
}

#[test]
fn nothing_below_the_desk_knows_what_ksav_looks_like() {
    // The other half of the same rule, and the one that matters more: a crate
    // boundary that only the manifest holds is a boundary somebody re-crosses
    // with a string literal. Ksav's markup is `#ציטוט[…]` and
    // `#מראה_מקום(מקור: …)`, and the moment a second place in this tree writes
    // one of those by hand, `girsa-desk` is no longer where the document format
    // lives — it is one of two places that disagree about it.
    // Three exclusions, and each is the difference between naming a format and
    // being a second implementation of it. A comment that explains what Ksav
    // writes is prose. `assert!(!markup.contains("#ציטוט"))` is a test holding
    // the writer to its word. And a `.ksav` document written out in a test is
    // *input* — `girsa-corpus` imports what you wrote (spec.md §10.4), so
    // reading the format is its job; the rule is about composing it.
    let mut wrong = Vec::new();
    for (path, text) in sources(&repo()) {
        if path.starts_with("crates/girsa-desk/") || !path.contains("/src/") {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || line.contains("assert") {
                continue;
            }
            for markup in ["#ציטוט", "#מראה_מקום"] {
                if line.contains(markup) {
                    wrong.push(format!("{path}:{} writes {markup} itself", n + 1));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "Ksav markup is written outside `girsa-desk`:
  {}

         The writer both applications compile is `girsa_ksav::to_ksav`. A second one here would          pass a `contains` for years and produce documents that differ.",
        wrong.join("
  ")
    );
}

#[test]
fn no_store_of_your_own_layer_rewrites_its_file_to_record_one_line() {
    // The fourth of the four doc comments §19 asked for as a test, and the one
    // that had no check.
    //
    // Six stores held a `BTreeMap`, and `save()` serialized the whole of it and
    // wrote the file — so correcting the eight thousandth typo wrote eight
    // thousand lines, and a lifetime of corrections was O(n²) bytes to record
    // O(n) statements. Every one of the six had a comment about atomicity above
    // its `write` + `rename`, which is the shape of this whole finding: the
    // rule was written down and the cost was not.
    //
    // `girsa_personal::Log` is the one file format now: append a line, tombstone
    // a line, compact when the tombstones outweigh the live rows. This checks
    // that the six are still on it and still append-only.
    //
    // Note what is *not* here. `note.rs` renames — a note is a `.md` file and
    // rewriting one is rewriting the note. `girsa-scan/store.rs`,
    // `girsa-lane/lane.rs` and `chosen.rs` rename too, and each writes a single
    // document rather than a list of statements. Atomic whole-file writes are
    // right for those and wrong for these, which is why this names the six.
    let root = repo();
    for (path, holds) in [
        ("crates/girsa-fix/src/lib.rs", "your corrections"),
        ("crates/girsa-fix/src/suspect.rs", "the OCR queue"),
        ("crates/girsa-note/src/mark.rs", "your marks"),
        ("crates/girsa-note/src/query.rs", "your saved queries"),
        (
            "crates/girsa-note/src/collection.rs",
            "your chaburah folders",
        ),
        (
            "crates/girsa-link/src/repair.rs",
            "your repairs to the link graph",
        ),
    ] {
        let text = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|e| panic!("{path} reads: {e}"));
        assert!(
            text.contains("Log::at") || text.contains("girsa_personal::Log"),
            "{path} holds {holds} and is no longer on `girsa_personal::Log`. A store that keeps \
             its own file writes the whole of it to record one line, which is what the log \
             replaced."
        );
        assert!(
            !text.contains("fs::rename"),
            "{path} holds {holds} and writes a whole file again — the temp-then-rename that \
             `girsa_personal::Log` exists to end. Appending one line does not need to be atomic \
             over the other eight thousand."
        );
    }
}

#[test]
fn what_two_open_panes_have_in_common_is_worked_out_once() {
    // `girsa_app::beside`:
    //
    //   Built once per pair of open panes.
    //
    // It was built per **scroll event**, out of a Tauri command that reads both
    // works' shards — so following a commentary down a page rebuilt the whole
    // joining, multi-MB shard reads and all, at the rate the reader's finger
    // moved. The doc comment above it said what the code did not do.
    //
    // It is held on the pane now and `Beside::over` re-aims the held one, so
    // the two callers below are the two places a *pair* changes: the pane that
    // has just been joined, and the one whose partner moved. A third would be a
    // third answer to what two panes have in common, and the fourth would be
    // back in the scroll handler.
    let root = repo();
    let shell = std::fs::read_to_string(root.join("app/src-tauri/src/lib.rs"))
        .unwrap_or_else(|e| panic!("the shell reads: {e}"));
    let built = shell.matches("Beside::over(").count();
    assert!(
        built <= 2,
        "the shell builds a `Beside` {built} times. `girsa_app::beside` says *built once per \
         pair of open panes*, and every extra site is a pair being re-derived somewhere that is \
         not a pairing — which is how it ended up in a scroll handler."
    );
    assert!(
        !shell.contains("Beside::between("),
        "the shell calls `Beside::between` itself. That is the expensive half — both works' \
         shards — and it belongs behind `Shelf`, which caches a pairing; `Beside::over` is what \
         a caller holding one already asks."
    );
}

#[test]
fn a_test_that_finds_nothing_says_so() {
    // `tools/check-ksav-fixture.sh`:
    //
    //   a check that passes because it could not find what it checks is the
    //   exact failure this script exists to end
    //
    // Written about a missing fixture, and true of a missing corpus: 43 test
    // functions built a shelf, found no seforim on it, and printed `ok` in
    // 0.00s. The rule was written down in one shell script and broken by four
    // dozen Rust tests.
    //
    // What the fixture crate fixed for the tests, this holds for the next one:
    // an acceptance test that walks a shelf has to assert that it found
    // something before it asserts anything about what it found. Checked by
    // asking for the one thing an empty run cannot produce — a non-zero
    // assertion on a count.
    let root = repo();
    let mut silent = Vec::new();
    for (path, text) in sources(&root) {
        if !path.contains("/tests/") {
            continue;
        }
        // A test file that builds a shelf from the fixture and never asserts
        // that anything is on it.
        if !text.contains("girsa_fixture") {
            continue;
        }
        let counts = text.contains("assert!(") || text.contains("assert_eq!(");
        let looks = text.contains("is_empty()") || text.contains(".len()") || text.contains("> 0");
        if !(counts && looks) {
            silent.push(path);
        }
    }
    assert!(
        silent.is_empty(),
        "a test builds a fixture shelf and never asserts that anything is on it:\n  {}\n\n\
         `tools/check-ksav-fixture.sh` wrote the rule down: a check that passes because it could \
         not find what it checks is the failure, not the skip.",
        silent.join("\n  ")
    );
}

#[test]
fn the_four_rules_this_file_was_asked_for_are_all_here() {
    // Not a source scan — a list, and the one check in this file whose subject
    // is the file itself.
    //
    // §19 named four doc comments and asked for four tests. Three of them were
    // written, the fourth was not, and nothing said which — the same shape as
    // every finding above it. So the four are named here: a rule that loses its
    // test loses it loudly.
    let root = repo();
    let me = std::fs::read_to_string(
        root.join("crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs"),
    )
    .unwrap_or_else(|e| panic!("this file reads: {e}"));
    for (check, rule) in [
        (
            "fn slug_of_has_one_implementation",
            "a slug is worked out once",
        ),
        (
            "fn find_index_has_one_implementation",
            "an index is found once",
        ),
        (
            "fn a_prepared_query_is_never_rebuilt_to_be_asked_again",
            "a query is prepared once",
        ),
        (
            "fn no_store_of_your_own_layer_rewrites_its_file_to_record_one_line",
            "a correction is one line, not a file",
        ),
    ] {
        assert!(
            me.contains(check),
            "the check for *{rule}* is gone. It is one of the four this file was written for."
        );
    }
}
