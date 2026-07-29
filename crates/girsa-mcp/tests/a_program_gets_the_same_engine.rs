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
//! # Why it skips when the corpus is absent
//!
//! It needs the fetched corpus, imported, indexed, with the link graph over it.
//! None of that is committed. A test that failed on a fresh clone would be
//! noise everybody learns to ignore.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use girsa_mcp::Server;

fn roots() -> Option<(PathBuf, PathBuf, PathBuf)> {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (root, personal, index) = (
        here.join("corpus"),
        here.join("personal"),
        here.join("index"),
    );
    (root.join("works/index.jsonl").is_file() && index.is_dir()).then_some((root, personal, index))
}

/// A server that has finished the handshake, or a skip.
macro_rules! server_or_skip {
    () => {{
        let Some((root, personal, index)) = roots() else {
            eprintln!("skipped: no imported corpus with an index beside it");
            return;
        };
        match Server::open(&root, &personal, &index) {
            Ok(server) => server,
            Err(e) => {
                // A stale index is the ordinary case after a schema bump and is
                // not this crate's failure. Say which it was, and skip.
                eprintln!("skipped: {e}");
                return;
            }
        }
    }};
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
        // Called with no arguments at all: what comes back must be a refusal
        // naming the missing argument, never a transport error and never a
        // panic.
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
        assert_eq!(
            answered["result"]["isError"],
            json!(true),
            "{name} with no arguments should refuse"
        );
    }
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
    for (citation, expected) in [
        ("ברכות ב.", "girsa:bavli/berakhot/2a:1#1"),
        (
            "שו\"ע או\"ח נח:א",
            "girsa:shulchan-arukh/orach-chayim/58:1#404",
        ),
        ("בבא מציעא נט:", "girsa:bavli/bava-metzia/59b:1#1813"),
    ] {
        let resolved = tool(&mut server, "resolve", json!({"citation": citation}));
        assert_eq!(resolved["settled"], json!(true), "{citation}");
        assert_eq!(resolved["places"][0]["id"], json!(expected), "{citation}");
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
}

#[test]
fn a_program_can_follow_a_ruling_back_to_the_code_it_is_written_on() {
    let mut server = server_or_skip!();
    handshake(&mut server);
    let traced = tool(
        &mut server,
        "trace",
        json!({
            "id": "girsa:mishnah-berurah/58:1#1496",
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
}

#[test]
fn a_link_that_only_says_connected_somehow_says_so_on_the_row() {
    let mut server = server_or_skip!();
    handshake(&mut server);
    let touching = tool(
        &mut server,
        "links",
        json!({"id": "girsa:bavli/berakhot/2a:1#1", "limit": 20}),
    );
    let links = touching["links"].as_array().expect("links");
    assert!(!links.is_empty());
    for link in links {
        // Every row carries both, so a caller writing a mareh makom can tell a
        // commentary from a corpus shrug without asking a second question.
        assert!(link["type"].is_string());
        assert!(link["asserts_something"].is_boolean());
    }
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
