//! The other end of spec.md §12 — MCP that writes, and everything it will not
//! touch.
//!
//! The server answered nine questions and wrote nothing. That is a defensible
//! place to stop and it is not where §12 stops: *MCP on both ends* is the work
//! order, and a library a program can read but not add to is a library an agent
//! learns beside rather than with.
//!
//! Three tools, and they are the three the record named: a note, a link, a
//! correction. What is asserted here is the shape of the permission around
//! them, because that is the part that goes quietly wrong:
//!
//! 1. **Off unless asked.** The corpus is a download and your own layer is not;
//!    nothing in it can be recovered by re-fetching, so the case where nobody
//!    has thought about whether an agent should write there is the case that
//!    has to be safe.
//! 2. **Absent from the catalogue, not listed and refused.** A tool list is
//!    what a program plans against, and one advertising a door it cannot open
//!    gets an agent halfway through a plan before the refusal lands.
//! 3. **And refused at the door anyway**, because a client that remembered the
//!    tools from a writable session will call them.
//! 4. **Into `personal/`, never the corpus.** The same wall the window is
//!    behind, and the reason spec.md §4.1 can promise the download stays
//!    replaceable.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use serde_json::{json, Value};

use girsa_mcp::Server;

/// A layer of its own per test: these run in parallel and they all write.
fn layer(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("girsa-mcp-writes-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a personal layer");
    dir
}

fn shelf() -> &'static girsa_fixture::Shelf {
    girsa_fixture::indexed()
}

fn server(name: &str, writable: bool) -> (Server, std::path::PathBuf) {
    let personal = layer(name);
    let shelf = shelf();
    let opened = Server::open(shelf.root(), &personal, shelf.index())
        .unwrap_or_else(|e| panic!("the fixture server will not open: {e}"));
    let opened = if writable { opened.writable() } else { opened };
    (opened, personal)
}

fn ask(server: &mut Server, method: &str, params: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": 7, "method": method, "params": params});
    let answer = server
        .serve(&line.to_string())
        .expect("a call with an id is answered");
    serde_json::from_str(&answer).expect("the answer is json")
}

fn handshake(server: &mut Server) {
    ask(
        server,
        "initialize",
        json!({"protocolVersion": "2025-06-18", "capabilities": {}}),
    );
}

/// A `tools/call` result, whole — so a test can read `isError` as well as the
/// answer.
fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
    ask(
        server,
        "tools/call",
        json!({"name": name, "arguments": arguments}),
    )["result"]
        .clone()
}

fn tool_names(server: &mut Server) -> Vec<String> {
    ask(server, "tools/list", json!({}))["result"]["tools"]
        .as_array()
        .expect("a list of tools")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

/// A place on the fixture shelf, by its address rather than its ordinal.
fn at(slug: &str, address: &[&str]) -> String {
    let work = girsa_corpus::import::read_back(shelf().root(), slug)
        .unwrap_or_else(|e| panic!("{slug}: {e}"));
    work.segments
        .iter()
        .find(|s| s.id.path() == address)
        .map(|s| s.id.to_string())
        .unwrap_or_else(|| panic!("{slug} has nothing at {address:?}"))
}

/// The first segment of a sefer, for a test that wants *somewhere else* and
/// does not care where. Its address is the fixture's business.
fn first_of(slug: &str) -> String {
    girsa_corpus::import::read_back(shelf().root(), slug)
        .unwrap_or_else(|e| panic!("{slug}: {e}"))
        .segments
        .first()
        .map(|s| s.id.to_string())
        .unwrap_or_else(|| panic!("{slug} has no segments"))
}

const MISHNAH: &str = "mishnah-berakhot";

#[test]
fn a_read_only_server_does_not_advertise_the_writes() {
    let (mut server, _) = server("not-advertised", false);
    handshake(&mut server);
    let names = tool_names(&mut server);

    assert!(names.contains(&"search".to_string()), "the reads are there");
    for write in ["write_note", "draw_link", "correct"] {
        assert!(
            !names.contains(&write.to_string()),
            "{write} is not in a read-only server's catalogue"
        );
    }
}

#[test]
fn a_writable_server_advertises_them_and_says_they_write() {
    let (mut server, _) = server("advertised", true);
    handshake(&mut server);

    let tools = ask(&mut server, "tools/list", json!({}))["result"]["tools"].clone();
    let listed = tools.as_array().expect("a list");
    for write in ["write_note", "draw_link", "correct"] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == json!(write))
            .unwrap_or_else(|| panic!("{write} is in a writable server's catalogue"));
        // The hint a client reads before asking its user. A claim about the
        // tool, not a promise about the client — which is exactly why the flag
        // that turns these on exists at all.
        assert_eq!(
            tool["annotations"]["readOnlyHint"],
            json!(false),
            "{write} says it writes"
        );
    }
}

#[test]
fn a_write_against_a_read_only_server_is_refused_at_the_door() {
    // Not merely absent from the list. A client that remembered the tools from
    // a writable session will call them, and *no such tool* would be a lie
    // about why.
    let (mut server, personal) = server("refused", false);
    handshake(&mut server);

    let answered = call(
        &mut server,
        "write_note",
        json!({"at": at(MISHNAH, &["1", "1"]), "text": "לא ייכתב"}),
    );
    assert_eq!(answered["isError"], json!(true));
    let said = answered["structuredContent"]["refused"]
        .as_str()
        .expect("a reason");
    assert!(
        said.contains("--writable"),
        "the refusal names what would let it through: {said}"
    );
    assert!(!personal.join("notes").exists(), "and nothing was written");
}

#[test]
fn a_note_written_over_the_wire_is_a_sefer_on_your_shelf() {
    // spec.md §11's claim, asked of the other end: a note is a sefer, so one
    // written by a program has to arrive as one — with its own work.json, its
    // segments and a catalogue line — or it is a second-class note, which is
    // the thing §11 says a note is not.
    let (mut server, personal) = server("a-note", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "1"]);

    let answered = call(
        &mut server,
        "write_note",
        json!({
            "at": place,
            "title": "מאימתי",
            "text": "מה שראיתי כאן",
            "tags": ["חבורה"],
        }),
    );
    assert_ne!(answered["isError"], json!(true), "{answered}");
    let wrote = answered["structuredContent"]["wrote"]
        .as_str()
        .expect("the note is named");
    assert_eq!(answered["structuredContent"]["into"], json!("personal"));
    assert_eq!(
        answered["structuredContent"]["tags"],
        json!(["חבורה"]),
        "the tags it was given came back"
    );

    let (notes, trouble) = girsa_note::Notes::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    let held = notes.get(wrote).expect("the note is on the shelf");
    assert!(held.tags.iter().any(|tag| tag == "חבורה"));
    assert!(
        girsa_corpus::import::work_dir(&personal, &held.slug).is_dir(),
        "and it is a sefer with segments, not a loose file"
    );
}

#[test]
fn a_link_drawn_over_the_wire_is_yours_and_the_shipped_graph_is_untouched() {
    let (mut server, personal) = server("a-link", true);
    handshake(&mut server);

    let answered = call(
        &mut server,
        "draw_link",
        json!({
            "from": at(MISHNAH, &["1", "1"]),
            "to": first_of("rambam-on-mishnah-berakhot"),
            "type": "comments-on",
        }),
    );
    assert_ne!(answered["isError"], json!(true), "{answered}");
    assert_eq!(answered["structuredContent"]["into"], json!("personal"));

    let (repairs, trouble) = girsa_link::repair::Repairs::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(repairs.drawn().count(), 1, "one link, in your layer");
    // And the corpus is a download that has not been written to.
    assert!(
        !shelf().root().join("personal").exists(),
        "nothing was written under the corpus root"
    );
}

#[test]
fn a_link_type_nothing_understands_is_refused_with_the_list() {
    let (mut server, _) = server("a-bad-type", true);
    handshake(&mut server);

    let answered = call(
        &mut server,
        "draw_link",
        json!({
            "from": at(MISHNAH, &["1", "1"]),
            "to": first_of("rambam-on-mishnah-berakhot"),
            "type": "vaguely_about",
        }),
    );
    assert_eq!(answered["isError"], json!(true));
    let said = answered["structuredContent"]["refused"]
        .as_str()
        .expect("a reason");
    assert!(
        said.contains("quotes") && said.contains("comments-on"),
        "the refusal lists what it could have been: {said}"
    );
}

#[test]
fn a_correction_over_the_wire_is_an_overlay_and_the_text_on_disk_is_the_same() {
    // spec.md §4.1, asked of the other end. The whole argument for an overlay is
    // that the download stays replaceable, and a write tool that edited a
    // segment would end that quietly.
    let (mut server, personal) = server("a-correction", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "1"]);

    let before = girsa_corpus::import::read_back(shelf().root(), MISHNAH)
        .expect("the sefer reads")
        .segments
        .iter()
        .find(|s| s.id.to_string() == place)
        .map(|s| s.text.clone())
        .expect("the line");

    let answered = call(
        &mut server,
        "correct",
        json!({
            "at": place,
            "from_char": 0,
            "to_char": 4,
            "says": "מאימתי",
            "kind": "ocr",
            "note": "the scanner",
        }),
    );
    assert_ne!(answered["isError"], json!(true), "{answered}");
    assert_eq!(answered["structuredContent"]["kind"], json!("ocr"));
    assert_eq!(answered["structuredContent"]["into"], json!("personal"));

    let (layer, trouble) = girsa_fix::Layer::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(layer.count(), 1);

    let after = girsa_corpus::import::read_back(shelf().root(), MISHNAH)
        .expect("the sefer still reads")
        .segments
        .iter()
        .find(|s| s.id.to_string() == place)
        .map(|s| s.text.clone())
        .expect("the line");
    assert_eq!(before, after, "the corpus text on disk did not move");
}

#[test]
fn a_correction_is_a_scanning_error_unless_it_says_otherwise() {
    // `ocr` and `girsa` are the same machinery and two very different claims:
    // one says the scanner got a letter wrong, the other says this edition
    // reads differently. A default of `girsa` would let a program quietly file
    // emendations to the text of Shas.
    let (mut server, personal) = server("a-default-kind", true);
    handshake(&mut server);

    let answered = call(
        &mut server,
        "correct",
        json!({"at": at(MISHNAH, &["1", "1"]), "from_char": 0, "to_char": 4, "says": "מאימתי"}),
    );
    assert_ne!(answered["isError"], json!(true), "{answered}");
    assert_eq!(answered["structuredContent"]["kind"], json!("ocr"));

    let (layer, _) = girsa_fix::Layer::open(&personal);
    assert!(layer.all().all(|patch| patch.kind == girsa_fix::Kind::Ocr));
}
