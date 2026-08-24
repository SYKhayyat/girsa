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
    // The layer reads are always there — seeing the layer you stand in is not
    // a permission question. The writes are not.
    for read in ["marks", "folders", "queries", "who_cites"] {
        assert!(names.contains(&read.to_string()), "{read} is listed");
    }
    for write in [
        "write_note",
        "draw_link",
        "correct",
        "forget_note",
        "undraw_link",
        "uncorrect",
        "bookmark",
        "forget_mark",
        "save_query",
        "forget_query",
    ] {
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
    for write in [
        "write_note",
        "draw_link",
        "correct",
        "bookmark",
        "forget_mark",
        "save_query",
        "forget_query",
    ] {
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
    // about why. Every write is tried, whatever arguments it would have taken:
    // the door is checked before anything else about the call.
    let (mut server, personal) = server("refused", false);
    handshake(&mut server);

    for (write, arguments) in [
        (
            "write_note",
            json!({"at": at(MISHNAH, &["1", "1"]), "text": "לא ייכתב"}),
        ),
        (
            "draw_link",
            json!({"from": "x", "to": "y", "type": "quotes"}),
        ),
        (
            "correct",
            json!({"at": "x", "from_char": 0, "to_char": 1, "says": "א"}),
        ),
        ("forget_note", json!({"note": "n", "saying": "s"})),
        (
            "undraw_link",
            json!({"from": "x", "to": "y", "type": "quotes"}),
        ),
        ("uncorrect", json!({"at": "x", "says": "א"})),
        ("bookmark", json!({"at": at(MISHNAH, &["1", "1"])})),
        ("forget_mark", json!({"id": "m"})),
        ("save_query", json!({"name": "q", "typed": "t"})),
        ("forget_query", json!({"name": "q", "typed": "t"})),
    ] {
        let answered = call(&mut server, write, arguments);
        assert_eq!(answered["isError"], json!(true), "{write}");
        let said = answered["structuredContent"]["refused"]
            .as_str()
            .unwrap_or_else(|| panic!("{write}: a reason"));
        assert!(
            said.contains("--writable"),
            "{write} names what would let it through: {said}"
        );
    }
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

// ---------------------------------------------------------------------------
// And the other direction: three writes with no way back was three writes.
//
// The record's own line was *"nothing there deletes one, because deleting is a
// decision and this end cannot show you what you are about to delete."* The
// first half was a gap and the second half is a real constraint, so each undo
// asks for something only a caller that read the thing can supply — and a wrong
// answer is refused with the thing left standing. What is asserted below is
// that shape, on all three, plus the two ways it could be a fiction: a refusal
// that still deletes, and a check that hands back the answer it just refused.

#[test]
fn a_read_only_server_advertises_no_undo_either_and_refuses_one() {
    let (mut server, personal) = server("no-undo", false);
    handshake(&mut server);
    let names = tool_names(&mut server);
    for undo in ["forget_note", "undraw_link", "uncorrect"] {
        assert!(
            !names.contains(&undo.to_string()),
            "{undo} is not in a read-only server's catalogue"
        );
    }

    let answered = call(
        &mut server,
        "forget_note",
        json!({"note": "whatever", "saying": "whatever"}),
    );
    assert_eq!(answered["isError"], json!(true));
    let said = answered["structuredContent"]["refused"]
        .as_str()
        .expect("a reason");
    assert!(said.contains("--writable"), "{said}");
    assert!(!personal.join("notes").exists());
}

#[test]
fn a_writable_server_advertises_the_undos_and_says_they_destroy() {
    let (mut server, _) = server("undo-advertised", true);
    handshake(&mut server);
    let tools = ask(&mut server, "tools/list", json!({}))["result"]["tools"].clone();
    let listed = tools.as_array().expect("a list");
    for undo in ["forget_note", "undraw_link", "uncorrect"] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == json!(undo))
            .unwrap_or_else(|| panic!("{undo} is in a writable server's catalogue"));
        assert_eq!(tool["annotations"]["readOnlyHint"], json!(false), "{undo}");
        // The one hint that separates these from the three that write: a client
        // that asks its user before a destructive call has to be able to tell
        // *this adds a note* from *this deletes one*.
        assert_eq!(
            tool["annotations"]["destructiveHint"],
            json!(true),
            "{undo} says it destroys"
        );
    }
}

#[test]
fn a_note_is_not_thrown_away_by_a_caller_that_has_not_read_it() {
    let (mut server, personal) = server("forget-unread", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "1"]);
    let wrote = call(
        &mut server,
        "write_note",
        json!({"at": place, "title": "חבורה", "text": "מה שראיתי"}),
    )["structuredContent"]["wrote"]
        .as_str()
        .expect("a name")
        .to_string();

    let answered = call(
        &mut server,
        "forget_note",
        json!({"note": &wrote, "saying": "something it does not say"}),
    );
    assert_eq!(answered["isError"], json!(true));
    let said = answered["structuredContent"]["refused"]
        .as_str()
        .expect("a reason");
    // And it does not hand back the words it just refused. Printing them would
    // make the check a two-call formality an agent passes without ever reading
    // the note, which is the whole thing being guarded against.
    assert!(
        !said.contains("מה שראיתי"),
        "the refusal does not answer its own question: {said}"
    );
    let (notes, _) = girsa_note::Notes::open(&personal);
    assert!(notes.get(&wrote).is_some(), "and the note is still there");
}

#[test]
fn a_note_the_caller_has_read_is_thrown_away_with_its_sefer() {
    let (mut server, personal) = server("forget-read", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "1"]);
    let wrote = call(
        &mut server,
        "write_note",
        json!({"at": place, "title": "חבורה", "text": "מה שראיתי"}),
    )["structuredContent"]["wrote"]
        .as_str()
        .expect("a name")
        .to_string();
    let slug = {
        let (notes, _) = girsa_note::Notes::open(&personal);
        notes.get(&wrote).expect("written").slug.clone()
    };
    assert!(girsa_corpus::import::work_dir(&personal, &slug).is_dir());

    let answered = call(
        &mut server,
        "forget_note",
        json!({"note": &wrote, "saying": "מה שראיתי"}),
    );
    assert_ne!(answered["isError"], json!(true), "{answered}");
    assert_eq!(answered["structuredContent"]["forgot"], json!(&wrote));

    let (notes, _) = girsa_note::Notes::open(&personal);
    assert!(notes.get(&wrote).is_none(), "the note is gone");
    assert!(
        !girsa_corpus::import::work_dir(&personal, &slug).is_dir(),
        "and so is the sefer it was — not just the catalogue line"
    );

    // Twice is not a silent success. A caller that cannot tell *deleted* from
    // *was not there* cannot tell whether its first call worked.
    let again = call(
        &mut server,
        "forget_note",
        json!({"note": &wrote, "saying": "מה שראיתי"}),
    );
    assert_eq!(again["isError"], json!(true));
}

#[test]
fn a_link_is_not_undrawn_by_a_caller_naming_the_wrong_type() {
    let (mut server, personal) = server("undraw-wrong-type", true);
    handshake(&mut server);
    let (from, to) = (
        at(MISHNAH, &["1", "1"]),
        first_of("rambam-on-mishnah-berakhot"),
    );
    call(
        &mut server,
        "draw_link",
        json!({"from": &from, "to": &to, "type": "comments-on"}),
    );

    let answered = call(
        &mut server,
        "undraw_link",
        json!({"from": &from, "to": &to, "type": "quotes"}),
    );
    assert_eq!(answered["isError"], json!(true));
    let said = answered["structuredContent"]["refused"]
        .as_str()
        .expect("a reason");
    assert!(
        !said.contains("comments-on"),
        "the refusal does not answer its own question: {said}"
    );
    let (repairs, _) = girsa_link::repair::Repairs::open(&personal);
    assert_eq!(repairs.drawn().count(), 1, "and the link is still drawn");
}

#[test]
fn an_edge_the_corpus_shipped_cannot_be_undrawn() {
    // The wall this tool is behind. Rejecting a shipped edge is a different
    // statement with its own record, and a tool that deleted one under the name
    // *undraw* would be a second way to change the graph.
    let (mut server, _) = server("undraw-shipped", true);
    handshake(&mut server);
    let from = at(MISHNAH, &["1", "1"]);
    let shipped = call(&mut server, "links", json!({"id": &from, "limit": 1}));
    let other = shipped["structuredContent"]["links"][0]["id"]
        .as_str()
        .expect("the fixture has an edge on that line")
        .to_string();

    let answered = call(
        &mut server,
        "undraw_link",
        json!({"from": &from, "to": &other, "type": "comments-on"}),
    );
    assert_eq!(answered["isError"], json!(true));
    let said = answered["structuredContent"]["refused"]
        .as_str()
        .expect("a reason");
    assert!(said.contains("not drawn a link"), "{said}");
}

#[test]
fn a_link_the_caller_has_read_is_taken_back_and_nothing_else_said_about_it_is() {
    let (mut server, personal) = server("undraw-read", true);
    handshake(&mut server);
    let (from, to) = (
        at(MISHNAH, &["1", "1"]),
        first_of("rambam-on-mishnah-berakhot"),
    );
    call(
        &mut server,
        "draw_link",
        json!({"from": &from, "to": &to, "type": "comments-on"}),
    );

    let answered = call(
        &mut server,
        "undraw_link",
        json!({"from": &from, "to": &to, "type": "comments-on"}),
    );
    assert_ne!(answered["isError"], json!(true), "{answered}");
    assert_eq!(
        answered["structuredContent"]["undrew"]["type"],
        json!("comments-on")
    );

    let (repairs, trouble) = girsa_link::repair::Repairs::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(
        repairs.drawn().count(),
        0,
        "the link is gone from your layer"
    );

    let again = call(
        &mut server,
        "undraw_link",
        json!({"from": &from, "to": &to, "type": "comments-on"}),
    );
    assert_eq!(again["isError"], json!(true), "and twice is not a success");
}

#[test]
fn a_correction_is_not_taken_back_by_a_caller_naming_words_it_does_not_say() {
    let (mut server, personal) = server("uncorrect-unread", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "1"]);
    call(
        &mut server,
        "correct",
        json!({"at": &place, "from_char": 0, "to_char": 4, "says": "מאימתי"}),
    );

    let answered = call(
        &mut server,
        "uncorrect",
        json!({"at": &place, "says": "לא זה"}),
    );
    assert_eq!(answered["isError"], json!(true));
    let said = answered["structuredContent"]["refused"]
        .as_str()
        .expect("a reason");
    assert!(!said.contains("מאימתי"), "{said}");
    let (layer, _) = girsa_fix::Layer::open(&personal);
    assert_eq!(layer.count(), 1, "and the correction stands");
}

#[test]
fn a_correction_the_caller_read_is_removed_as_an_overlay_and_restores_nothing() {
    // The sentence the answer has to carry: nothing was edited, so nothing is
    // reverted. `read` is where the caller got the words, which is the point of
    // it returning `corrections` at all.
    let (mut server, personal) = server("uncorrect-read", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "1"]);
    call(
        &mut server,
        "correct",
        json!({"at": &place, "from_char": 0, "to_char": 4, "says": "מאימתי"}),
    );

    let seen = call(&mut server, "read", json!({"id": &place}));
    let says = seen["structuredContent"]["segments"][0]["corrections"][0]["says"]
        .as_str()
        .expect("read reports the corrections on the line")
        .to_string();
    assert_eq!(says, "מאימתי");

    let answered = call(
        &mut server,
        "uncorrect",
        json!({"at": &place, "says": &says}),
    );
    assert_ne!(answered["isError"], json!(true), "{answered}");
    assert_eq!(
        answered["structuredContent"]["no_longer_says"],
        json!("מאימתי")
    );

    let (layer, trouble) = girsa_fix::Layer::open(&personal);
    assert!(trouble.is_empty(), "{trouble:?}");
    assert_eq!(layer.count(), 0, "the overlay is gone");

    let after = call(&mut server, "read", json!({"id": &place}));
    assert!(
        after["structuredContent"]["segments"][0]
            .get("corrections")
            .is_none(),
        "and `read` says so"
    );
}

#[test]
fn the_offsets_a_correction_takes_are_the_ones_read_hands_back() {
    // Found by driving the server against the real corpus. `read` returned the
    // segment as the corpus stores it — markup and all — and `correct` counted
    // into the same words with the markup out, while its own description said
    // the two were one string. On Berakhot 2a:1#1, `from_char: 0, to_char: 4`
    // reads as `<big` in what the caller was given and landed on `מֵאֵ`,
    // successfully, with nothing to tell it apart from a correct call.
    //
    // The repair is that `read` hands back the string the offsets are into.
    // What is asserted is the agreement, not either string: a caller that
    // counts characters in what it was given must name the characters it meant.
    let (mut server, _) = server("counting-agrees", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "1"]);

    let seen = call(&mut server, "read", json!({"id": &place}));
    let counting = seen["structuredContent"]["segments"][0]["counting"]
        .as_str()
        .expect("read returns the string to count into")
        .to_string();
    let meant: String = counting.chars().take(4).collect();

    let answered = call(
        &mut server,
        "correct",
        json!({"at": &place, "from_char": 0, "to_char": 4, "says": "מאימתי"}),
    );
    assert_ne!(answered["isError"], json!(true), "{answered}");
    assert_eq!(
        answered["structuredContent"]["was"],
        json!(meant),
        "the four characters corrected are the first four of `counting`"
    );
}

// ---------------------------------------------------------------------------
// The rest of the layer (the audit's F4): an agent that could write a note but
// not see marks, folders, saved queries, or which of your documents cite a
// place was standing in a library it could change and never read.
// ---------------------------------------------------------------------------

/// The answer body of a call that succeeded.
fn answered(server: &mut Server, name: &str, arguments: Value) -> Value {
    let result = call(server, name, arguments);
    assert_ne!(
        result["isError"],
        json!(true),
        "{name} refused: {:?}",
        result["structuredContent"]
    );
    result["structuredContent"].clone()
}

#[test]
fn a_bookmark_written_over_the_wire_is_read_back_and_taken_back() {
    let (mut server, personal) = server("a-bookmark", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "1"]);

    // Nothing there yet — and empty is an answer, not an error.
    let marks = answered(&mut server, "marks", json!({}));
    assert_eq!(marks["total"], json!(0));

    let marked = answered(
        &mut server,
        "bookmark",
        json!({"at": place, "label": "להתחיל כאן", "tag": "חבורה"}),
    );
    let id = marked["marked"]
        .as_str()
        .expect("the mark has an id")
        .to_string();
    assert!(!id.is_empty());

    let marks = answered(&mut server, "marks", json!({"bookmarks": true}));
    assert_eq!(marks["total"], json!(1));
    let row = &marks["marks"][0];
    assert_eq!(row["id"], json!(id));
    assert_eq!(row["kind"], json!("bookmark"));
    assert_eq!(row["label"], json!("להתחיל כאן"));
    assert_eq!(row["tags"], json!(["חבורה"]));
    assert_eq!(row["at"]["id"], json!(place), "and it names the place");

    // Back by the id `marks` gave — the proof of having looked.
    let forgot = answered(&mut server, "forget_mark", json!({"id": id}));
    assert!(forgot["forgot"].is_string());
    let marks = answered(&mut server, "marks", json!({}));
    assert_eq!(marks["total"], json!(0));

    // And twice is refused rather than a silent success.
    let again = call(&mut server, "forget_mark", json!({"id": marked["marked"]}));
    assert_eq!(again["isError"], json!(true));
    assert!(girsa_note::Marks::open(&personal).0.all().next().is_none());
}

#[test]
fn a_saved_query_round_trips_and_cannot_die_unread() {
    let (mut server, personal) = server("a-query", true);
    handshake(&mut server);

    let saved = answered(
        &mut server,
        "save_query",
        json!({"name": "מאימתי", "typed": "\"זמן צאת הכוכבים\""}),
    );
    assert!(saved["said"].is_string(), "the query says what it says");

    let queries = answered(&mut server, "queries", json!({}));
    assert_eq!(queries["total"], json!(1));
    assert_eq!(queries["queries"][0]["name"], json!("מאימתי"));
    assert_eq!(queries["queries"][0]["typed"], json!("\"זמן צאת הכוכבים\""));

    // The same proof `forget_note` asks for: what it says now, which cannot be
    // filled in without having looked. A mismatch is refused, the query stays.
    let wrong = call(
        &mut server,
        "forget_query",
        json!({"name": "מאימתי", "typed": "אחרת לגמרי"}),
    );
    assert_eq!(wrong["isError"], json!(true));
    assert_eq!(
        answered(&mut server, "queries", json!({}))["total"],
        json!(1)
    );

    let right = answered(
        &mut server,
        "forget_query",
        json!({"name": "מאימתי", "typed": "\"זמן צאת הכוכבים\""}),
    );
    assert_eq!(right["forgot"], json!("מאימתי"));
    assert_eq!(
        answered(&mut server, "queries", json!({}))["total"],
        json!(0)
    );
    assert!(girsa_note::Queries::open(&personal)
        .0
        .all()
        .next()
        .is_none());
}

#[test]
fn folders_are_read_with_their_members_in_order() {
    let personal = layer("folders");
    // Seeded before the server opens — the layer is held open in memory, and
    // that is true of the window too: a second writer behind the server's back
    // is not a shape this surface answers. Seeding here is the test standing in
    // for the reader's own history.
    let mut folder = girsa_note::Collection::new("thursday", "חבורה יום ה");
    let place = at(MISHNAH, &["1", "1"]);
    folder.put(girsa_note::Member::Place(
        place.parse().expect("the place reads as a segment id"),
    ));
    folder.put(girsa_note::Member::Query("מאימתי".to_string()));
    girsa_note::Collections::open(&personal)
        .0
        .save(folder)
        .expect("saves");

    let shelf = shelf();
    let mut server = Server::open(shelf.root(), &personal, shelf.index())
        .expect("opens")
        .writable();
    handshake(&mut server);

    // Folder *writing* is deliberately not on this surface: reordering
    // somebody's shiur is not an agent's call to make.
    let listed = answered(&mut server, "folders", json!({}));
    assert_eq!(listed["total"], json!(1));
    let row = &listed["folders"][0];
    assert_eq!(row["name"], json!("thursday"));
    assert_eq!(row["title"], json!("חבורה יום ה"));
    assert_eq!(row["members"][0]["member"], json!("place"));
    assert_eq!(
        row["members"][0]["id"],
        json!(place),
        "members named, not bare slugs"
    );
    assert_eq!(row["members"][1]["member"], json!("query"));

    // And the one filter the panel answers: does this line sit in any of them?
    let holding = answered(&mut server, "folders", json!({"holding": place}));
    assert_eq!(holding["total"], json!(1));

    let elsewhere = first_of("rambam-on-mishnah-berakhot");
    let holding = answered(&mut server, "folders", json!({"holding": elsewhere}));
    assert_eq!(holding["total"], json!(0));
}

#[test]
fn who_cites_names_your_own_writing_and_only_it() {
    let (mut server, personal) = server("who-cites", true);
    handshake(&mut server);
    let place = at(MISHNAH, &["1", "2"]);

    // A document in your own layer citing the place — real Ksav markup, built
    // by the same `#מראה_מקום[…]` composer the editor writes with.
    let mut buffer = girsa_desk::Buffer::new("chesbon");
    buffer.text = format!(
        "{}\nודו\"ק.\n",
        girsa_ksav::mekor("ברכות ב.", Some(&place), None)
    );
    buffer.save(&personal).expect("saves");

    let cited = answered(&mut server, "who_cites", json!({"id": place}));
    assert_eq!(cited["total"], json!(1));
    assert_eq!(cited["cited_by"][0]["name"], json!("chesbon"));
    assert!(
        !cited["cited_by"][0]["refs"]
            .as_array()
            .expect("refs")
            .is_empty(),
        "the refs that answer come back"
    );
    assert_eq!(cited["cited_by"][0]["cached_only"], json!(false));

    // And somewhere nobody's writing touches says so, rather than inventing.
    let quiet = answered(
        &mut server,
        "who_cites",
        json!({"id": first_of("rambam-on-mishnah-berakhot")}),
    );
    assert_eq!(quiet["total"], json!(0));
}
