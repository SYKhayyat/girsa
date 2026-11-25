//! What a program is promised, checked against the corpus that is on disk.
//!
//! The risk this crate carries is not that a tool returns the wrong JSON. It is
//! that the tools become a *second* search engine — a looser one, written for a
//! caller that cannot complain — and that spec.md §9's guarantees quietly stop
//! applying on the side nobody is watching. So the tests here are about the
//! refusals rather than the answers:
//!
//! - literal unless asked otherwise, and the answer says which mode ran;
//! - a zero offers the ladder priced and applies none of it;
//! - a citation that does not name a place is not rounded to the nearest one.
//!
//! # It skipped everywhere, which is why none of that was ever checked
//!
//! `Server::open` takes a shelf, a personal layer and an index, and this file
//! `return`ed unless all three were present — the corpus fetched, imported,
//! indexed, with the link graph built over it. On a fresh clone and in CI that
//! is never, so what it printed was `8 passed` in 0.00s: eight refusals nobody
//! had checked, on the one surface whose caller cannot complain.
//!
//! It runs on [`girsa_fixture`], which builds all three in about two seconds.
//!
//! # And the ordinals are gone
//!
//! These used to name places by their permanent id —
//! `girsa:shulchan-arukh/orach-chayim/58:1#404`. The id is permanent; the
//! *number of se'ifim before it* is not, and writing one into a test makes the
//! test a hostage to the next re-import. What the assertions are actually about
//! is that a citation settles on the right **place**, so they name the address
//! and let the shelf say which segment that is.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use serde_json::{json, Value};

use girsa_mcp::Server;

/// The shelf, the personal layer and the index, all built by the fixture.
fn shelf() -> &'static girsa_fixture::Shelf {
    girsa_fixture::indexed()
}

/// A server over that shelf.
///
/// A failure to open is a failure, not a skip. A stale index used to be excused
/// here — *"the ordinary case after a schema bump"* — and the excuse applied to
/// every other reason too, including there being no index at all.
macro_rules! server_or_skip {
    () => {{
        let shelf = shelf();
        match Server::open(shelf.root(), shelf.personal(), shelf.index()) {
            Ok(server) => server,
            Err(e) => panic!("the fixture server will not open: {e}"),
        }
    }};
}

/// The permanent id of the segment at an address, off the shelf.
fn at(slug: &str, address: &[&str]) -> String {
    let work = girsa_corpus::import::read_back(shelf().root(), slug)
        .unwrap_or_else(|e| panic!("{slug}: {e}"));
    work.segments
        .iter()
        .find(|s| s.id.path() == address)
        .map(|s| s.id.to_string())
        .unwrap_or_else(|| panic!("{slug} has nothing at {address:?}"))
}

fn ask(server: &mut Server, id: u32, method: &str, params: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    let answer = server
        .serve(&line.to_string())
        .expect("a call with an id is answered");
    serde_json::from_str(&answer).expect("the answer is json")
}

/// The structured half of a `tools/call` result.
fn tool(server: &mut Server, name: &str, arguments: Value) -> Value {
    let answered = ask(
        server,
        9,
        "tools/call",
        json!({"name": name, "arguments": arguments}),
    );
    assert!(
        answered.get("error").is_none(),
        "a tool refusal is a result, not a transport error: {answered}"
    );
    answered["result"]["structuredContent"].clone()
}

fn handshake(server: &mut Server) {
    let hello = ask(
        server,
        1,
        "initialize",
        json!({"protocolVersion": "2025-06-18", "capabilities": {}}),
    );
    assert_eq!(hello["result"]["protocolVersion"], json!("2025-06-18"));
}

#[test]
fn a_tool_call_before_the_handshake_is_refused() {
    // A client that has not agreed a protocol version cannot be assumed to read
    // the answer, and answering it anyway is how a version mismatch becomes a
    // silently mis-parsed result rather than an error.
    let mut server = server_or_skip!();
    let answered = ask(
        &mut server,
        1,
        "tools/call",
        json!({"name": "seforim", "arguments": {"title": "משנה ברורה"}}),
    );
    assert!(answered["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("initialize"));
}

#[test]
fn a_notification_is_not_answered() {
    let mut server = server_or_skip!();
    assert!(
        server
            .serve(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none(),
        "a line on the pipe the client is not waiting for is read as the answer to its next call"
    );
}

#[test]
fn every_tool_the_list_names_is_a_tool_that_answers() {
    let mut server = server_or_skip!();
    handshake(&mut server);
    let listed = ask(&mut server, 2, "tools/list", json!({}));
    let tools = listed["result"]["tools"]
        .as_array()
        .expect("a list of tools")
        .clone();
    assert!(tools.len() >= 9);
    for entry in &tools {
        let name = entry["name"].as_str().expect("a name");
        let required = entry["inputSchema"]["required"].as_array();
        // Called with no arguments at all: what comes back must be coherent.
        // A tool that names required arguments refuses naming the missing one;
        // a tool whose schema requires nothing (`marks`, `folders`, `queries`)
        // answers with its whole answer, which is not a failure.
        let answered = ask(
            &mut server,
            3,
            "tools/call",
            json!({"name": name, "arguments": {}}),
        );
        assert!(
            answered.get("error").is_none(),
            "{name} answered the transport instead of the caller"
        );
        match required {
            Some(required) if !required.is_empty() => assert_eq!(
                answered["result"]["isError"],
                json!(true),
                "{name} with no arguments should refuse"
            ),
            _ => assert_eq!(
                answered["result"]["isError"],
                json!(false),
                "{name} requires nothing and should answer"
            ),
        }
    }
}

#[test]
fn a_concordance_is_that_form_and_every_place_it_is_said() {
    // #48, on the side of the engine a program sees. `ברישא` stands twice in the
    // fixture's Berakhot and three more times in the mefarshim that quote it —
    // one form, five places, four seforim, and a count a caller can check.
    let mut server = server_or_skip!();
    handshake(&mut server);
    let found = tool(
        &mut server,
        "concordance",
        json!({"form": "ברישא", "limit": 50}),
    );
    assert_eq!(found["indexed"], json!("ברישא"), "{found}");
    assert_eq!(found["total"], json!(5), "{found}");
    assert_eq!(found["shown"], json!(5), "{found}");
    assert_eq!(found["seforim"], json!(4), "{found}");
    let places = found["places"].as_array().expect("places").clone();
    let ids: Vec<&str> = places
        .iter()
        .map(|place| place["id"].as_str().expect("an id"))
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "reading order, not score order");
    for place in &places {
        let marks = place["marks"].as_array().expect("marks");
        assert!(!marks.is_empty(), "a place with no position: {place}");
        assert!(
            place["shown"].as_str().unwrap_or_default().contains('['),
            "the list does not show the word: {place}"
        );
    }
}

#[test]
fn a_concordance_refuses_a_phrase_instead_of_searching_it_loosely() {
    // The count is the whole value of a concordance, so a two-word ask cannot be
    // quietly answered as a phrase search: the count of *this word* and the count
    // of *these two words together* are different numbers about different things.
    let mut server = server_or_skip!();
    handshake(&mut server);
    let refused = tool(
        &mut server,
        "concordance",
        json!({"form": "לית\u{5d9} דשחרית"}),
    );
    let why = refused["refused"].as_str().unwrap_or_default();
    assert!(why.contains("find") && why.contains("where-from"), "{why}");
}

#[test]
fn a_commentator_is_a_person_and_not_the_works_the_shelf_files_under_him() {
    // #46, the acceptance as a program reads it: given a name, the era, the
    // years, what he comments on, and whether anybody doubts the attribution.
    let mut server = server_or_skip!();
    handshake(&mut server);
    let rashi = tool(&mut server, "commentator", json!({"name": "רש\"י"}));
    assert_eq!(rashi["era"], json!("ראשונים"), "{rashi}");
    assert_eq!(rashi["years"], json!("1090"), "{rashi}");
    assert!(
        rashi["on"]
            .as_array()
            .expect("on")
            .iter()
            .any(|slug| slug == "bavli/berakhot"),
        "{rashi}"
    );
    assert_eq!(rashi["works"][0]["attribution"]["how"], json!("stated"));
    assert_eq!(rashi["doubted"], json!(false));
    // A name nobody in the corpus wrote is refused rather than answered with an
    // empty profile that reads as *this mefaresh is unknown work*.
    let refused = tool(&mut server, "commentator", json!({"name": "אבן עזרא"}));
    assert!(
        refused["refused"]
            .as_str()
            .unwrap_or_default()
            .contains("not an author"),
        "{refused}"
    );
}

#[test]
fn who_comments_on_a_sefer_says_how_many_are_unnamed() {
    let mut server = server_or_skip!();
    handshake(&mut server);
    let here = tool(&mut server, "commentators", json!({"on": "bavli/berakhot"}));
    let names: Vec<&str> = here["commentators"]
        .as_array()
        .expect("commentators")
        .iter()
        .map(|row| row["name"].as_str().expect("a name"))
        .collect();
    assert!(names.contains(&"רש\"י"), "{names:?}");
    // Two commentaries on Berakhot are in the fixture with no author on them: in
    // the corpus and in no row here, so the answer has to say the list is short.
    assert_eq!(here["anonymous"], json!(2), "{here}");
    // A slug that is not a sefer at all is a different answer from *nobody
    // comments here*, and it is refused rather than answered with an empty list.
    let refused = tool(&mut server, "commentators", json!({"on": "bavli/nothing"}));
    assert!(
        refused["refused"]
            .as_str()
            .unwrap_or_default()
            .contains("not on this shelf"),
        "{refused}"
    );
}

#[test]
fn who_is_written_on_a_line_joins_the_graph_to_the_register() {
    // #46's third item as a program reads it: an anchor in, people out. The
    // graph says which classes have a unit here; the register says who they are.
    let mut server = server_or_skip!();
    handshake(&mut server);
    let here = tool(
        &mut server,
        "on_line",
        json!({"id": "girsa:bavli/berakhot/2a:1#1"}),
    );
    assert_eq!(here["built"], json!(true), "{here}");
    let rows = here["commentaries"].as_array().expect("commentaries");
    assert!(
        !rows.is_empty(),
        "nothing is written on the first line of the daf"
    );
    let rashi = rows
        .iter()
        .find(|row| row["work"] == json!("bavli/rashi-on-berakhot"))
        .unwrap_or_else(|| panic!("Rashi is not on 2a:1: {here}"));
    assert_eq!(rashi["name"], json!("רש\"י"), "{rashi}");
    assert_eq!(rashi["era"], json!("ראשונים"), "{rashi}");
    assert_eq!(rashi["years"], json!("1090"), "{rashi}");
    // The count is the list, so a caller reading either gets the same number —
    // and a commentary the corpus has no author for keeps its row with a null
    // name rather than vanishing from the answer.
    assert_eq!(here["total"], json!(rows.len()));
    assert!(
        rows.iter()
            .any(|row| row["name"].is_null() || row["name"].is_string()),
        "{here}"
    );
}

#[test]
fn search_is_literal_unless_a_program_asks_for_otherwise() {
    let mut server = server_or_skip!();
    handshake(&mut server);
    let found = tool(
        &mut server,
        "search",
        json!({"query": "מאימתי קורין", "limit": 3}),
    );
    assert_eq!(
        found["mode"],
        json!("torat-emet"),
        "the default is the literal mode, here as in the window"
    );
    assert!(found["total"].as_u64().unwrap_or(0) > 0);
    assert!(
        found["hits"][0]["id"]
            .as_str()
            .unwrap_or_default()
            .starts_with("girsa:"),
        "a hit is named by its permanent id, which is what a document can carry"
    );
    // And what was cut is on the answer, not implied by its length.
    assert!(found["not_shown"].is_number());

    let widened = tool(
        &mut server,
        "search",
        json!({"query": "מאימתי קורין", "mode": "smart", "limit": 3}),
    );
    assert_eq!(
        widened["mode"],
        json!("smart"),
        "and when a program asks for widening it is told that it got it"
    );
}

#[test]
fn a_citation_that_names_a_place_settles_and_one_that_does_not_is_not_guessed() {
    let mut server = server_or_skip!();
    handshake(&mut server);

    // A mareh makom that names a place settles to that place and no other.
    for (citation, slug, address) in [
        ("ברכות ב.", "bavli/berakhot", &["2a", "1"][..]),
        (
            "שו\"ע או\"ח נח:א",
            "shulchan-arukh/orach-chayim",
            &["58", "1"][..],
        ),
        ("בבא מציעא נט:", "bavli/bava-metzia", &["59b", "1"][..]),
    ] {
        let resolved = tool(&mut server, "resolve", json!({"citation": citation}));
        assert_eq!(resolved["settled"], json!(true), "{citation}");
        assert_eq!(
            resolved["places"][0]["id"],
            json!(at(slug, address)),
            "{citation}"
        );
    }

    // And one that does not is **not** rounded to the nearest thing. `ברכות ב`
    // has no amud on it, so it is not a daf; the engine answers with no place
    // and a near miss to offer, which is the whole of BUILDER.md rule 6. A
    // resolver that picked 2a here would be right about half the time, and a
    // document carrying the wrong half would never say so.
    let vague = tool(&mut server, "resolve", json!({"citation": "ברכות ב"}));
    assert_eq!(vague["settled"], json!(false));
    assert_eq!(vague["places"].as_array().map(Vec::len), Some(0));
    assert!(
        vague["near_misses"].as_u64().unwrap_or(0) > 0,
        "and something is offered instead, rather than silence"
    );

    // And every reply says what the citation was resolved **against** (the
    // audit's C6). Null: no standing — a relative mareh makom was completed
    // against nothing, and an agent reading `settled` alone would never know.
    for citation in ["ברכות ב.", "ברכות ב"] {
        let answered = tool(&mut server, "resolve", json!({"citation": citation}));
        assert_eq!(
            answered["resolved_against"],
            json!(null),
            "{citation}: nothing was asked for"
        );
    }
    let scoped = tool(
        &mut server,
        "resolve",
        json!({"citation": "ברכות ב.", "sefer": "bavli/berakhot"}),
    );
    assert_eq!(
        scoped["resolved_against"],
        json!("bavli/berakhot"),
        "the sefer it was completed against is named"
    );
}

#[test]
fn a_program_can_follow_a_ruling_back_to_the_code_it_is_written_on() {
    let mut server = server_or_skip!();
    handshake(&mut server);
    let traced = tool(
        &mut server,
        "trace",
        json!({
            "id": at("mishnah-berurah", &["58", "1"]),
            "direction": "back",
            "depth": 1,
            "width": 4
        }),
    );
    let reached: Vec<String> = traced["chains"]
        .as_array()
        .expect("chains")
        .iter()
        .filter_map(|chain| chain["hops"][0]["sefer"].as_str().map(str::to_string))
        .collect();
    assert!(
        reached.iter().any(|s| s == "shulchan-arukh/orach-chayim"),
        "back from the Mishnah Berurah is the Shulchan Arukh: {reached:?}"
    );
    // The refusals are part of the answer, not a debug aid.
    assert!(traced["not_followed"]["no_date_and_no_era"].is_number());
    assert!(traced["not_followed"]["other_way_in_time"].is_number());
    // And every hop says whether the corpus declared which way it points (the
    // audit's F1) — a coin-flip arrow is a different fact from a declared one,
    // and an agent ranking these is owed the difference. On this fixture the
    // commentary edges are declared: the Rambam names its base.
    let mut declared = false;
    for chain in traced["chains"].as_array().expect("chains") {
        for hop in chain["hops"].as_array().expect("hops") {
            match hop["direction"].as_str() {
                Some("declared") => declared = true,
                other => assert!(
                    other.is_none(),
                    "an undeclared hop says so explicitly, not with a missing field"
                ),
            }
        }
    }
    assert!(
        declared,
        "at least one hop back was oriented by declaration"
    );
}

#[test]
fn a_link_that_only_says_connected_somehow_says_so_on_the_row() {
    let mut server = server_or_skip!();
    handshake(&mut server);
    let touching = tool(
        &mut server,
        "links",
        json!({"id": at("bavli/berakhot", &["2a", "1"]), "limit": 20}),
    );
    let links = touching["links"].as_array().expect("links");
    assert!(!links.is_empty());
    let mut declared = false;
    for link in links {
        // Every row carries both, so a caller writing a mareh makom can tell a
        // commentary from a corpus shrug without asking a second question.
        assert!(link["type"].is_string());
        assert!(link["asserts_something"].is_boolean());
        // And which way it points, said rather than priced into a number the
        // caller cannot read (F1). The fixture's commentary edges were
        // oriented by declaration; at least one row here says so.
        match link["direction"].as_str() {
            Some("declared") => declared = true,
            other => assert!(
                other.is_none(),
                "an undeclared link says so explicitly, not with a missing field"
            ),
        }
    }
    assert!(
        declared,
        "the Rambam's edge on this mishnah was oriented by declaration"
    );
    assert_eq!(
        touching["incoming_half_unknown"],
        json!(false),
        "the inbound cache is built, so the incoming half is an answer and not a silence"
    );
}

#[test]
fn the_semantic_lane_is_its_own_tool_and_discloses_its_coverage() {
    // BUILDER.md W30's sibling clause: *the MCP surface must refuse and disclose
    // partial coverage exactly as the UI does.* An agent is the caller least able
    // to notice that a list of three Rishonim was drawn from eleven per cent of a
    // shelf, and most likely to write it up as though it were all of them.
    let mut server = server_or_skip!();
    handshake(&mut server);

    // It is listed, and it is listed as adjacent rather than as a search.
    let listed = ask(&mut server, 2, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let lane = tools
        .iter()
        .find(|tool| tool["name"] == json!("adjacent"))
        .expect("the lane is offered as its own tool");
    let told = lane["description"].as_str().unwrap_or_default();
    assert!(told.contains("ADJACENT"), "{told}");
    assert!(told.contains("coverage"), "{told}");
    assert!(told.contains("does not pasken"), "{told}");
    // And `search` is still the literal one. A caller must not be able to reach
    // the lane through it.
    let search = tools
        .iter()
        .find(|tool| tool["name"] == json!("search"))
        .expect("search");
    let modes = search["inputSchema"]["properties"]["mode"]["enum"]
        .as_array()
        .expect("the modes")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(modes, ["torat-emet", "smart", "regex"]);

    // Called, it answers with the label, the state and the coverage sentence —
    // whether or not it found anything. On a shelf where nobody turned the lane
    // on, that is a refusal with a reason and never an empty list.
    let answer = tool(
        &mut server,
        "adjacent",
        json!({"text": "מי שנשתכר ביין לא יעמוד", "limit": 5}),
    );
    assert!(answer["these_are"]
        .as_str()
        .unwrap_or_default()
        .contains("rather than by these words"));
    assert!(
        answer["coverage"].is_string()
            && !answer["coverage"].as_str().unwrap_or_default().is_empty(),
        "coverage is said in every answer: {answer}"
    );
    match answer["lane"].as_str() {
        Some("off") | Some("on, but no model will run") => {
            assert!(
                answer["refused"].is_string(),
                "nothing, with no reason attached: {answer}"
            );
            assert_eq!(answer["showing"], json!(0));
        }
        Some("on") => {
            assert!(answer["model"].is_string());
        }
        other => panic!("the lane has no state: {other:?}"),
    }
}
