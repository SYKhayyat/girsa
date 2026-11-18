//! A program reading a daf pays for one parse of the sefer, not one per word.
//!
//! Issue #53. `read` called `Shelf::read` on every call, and `Shelf::read`
//! parses the work's whole `segments.jsonl` — tens of megabytes for a masechta —
//! to serve at most forty-one segments. An agent reading a daf a segment at a
//! time re-parsed the sefer a segment at a time.
//!
//! The window fixed this by holding `Arc<Open>`; `girsa_app::held::Held` is that
//! answer, and it is what the server holds now. So the property is stated the
//! way it is true — **the second read is the same object, not a second parse** —
//! and the second half is that a write makes it a different object, because a
//! cached sefer is a sefer a correction has not reached.
//!
//! # Why an `Arc` and not a stopwatch
//!
//! A test that timed two reads would report the machine (see the argument in
//! `girsa-app`'s `three_seconds`). `Arc::ptr_eq` is the property itself: two
//! reads that parsed twice cannot be the same allocation, and two that share
//! one cannot have parsed twice.

// A panic in a test is a failure report. The workspace denies these in library
// code, where a panic would take the reader's window with it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use serde_json::{json, Value};

use girsa_mcp::Server;

/// A sefer on the fixture's shelf, and another one for the eviction test.
const SEFER: &str = "mishnah-berurah";
const OTHER: &str = "shulchan-arukh/orach-chayim";

fn shelf() -> &'static girsa_fixture::Shelf {
    girsa_fixture::indexed()
}

/// A server over that shelf, with writes allowed.
fn server() -> Server {
    Server::open(shelf().root(), shelf().personal(), shelf().index())
        .expect("the fixture server opens")
        .writable()
}

fn ask(server: &mut Server, method: &str, params: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": 7, "method": method, "params": params});
    let answer = server
        .serve(&line.to_string())
        .expect("a call with an id is answered");
    serde_json::from_str(&answer).expect("the answer is json")
}

fn tool(server: &mut Server, name: &str, arguments: Value) -> Value {
    let answered = ask(
        server,
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
        "initialize",
        json!({"protocolVersion": "2025-06-18", "capabilities": {}}),
    );
    assert_eq!(hello["result"]["protocolVersion"], json!("2025-06-18"));
}

/// The permanent id of the first segment of a sefer, off the shelf.
fn first_of(slug: &str) -> String {
    girsa_corpus::import::read_back(shelf().root(), slug)
        .unwrap_or_else(|e| panic!("{slug}: {e}"))
        .segments
        .first()
        .map(|segment| segment.id.to_string())
        .unwrap_or_else(|| panic!("{slug} has no segments"))
}

#[test]
fn the_second_read_is_the_first_read_and_not_a_second_parse() {
    let server = server();
    let once = server.sefer(SEFER).expect("it opens");
    let again = server.sefer(SEFER).expect("it opens again");
    assert!(
        std::sync::Arc::ptr_eq(&once, &again),
        "two reads of one sefer parsed it twice, which is what #53 is"
    );
}

#[test]
fn two_seforim_are_two_entries_and_not_one() {
    // The cheap version of the cache is a single slot, and a single slot is
    // right up until a reader has the Gemara and its Rashi open — which is the
    // ordinary case, not an edge one.
    let server = server();
    let first = server.sefer(SEFER).expect("it opens");
    let second = server.sefer(OTHER).expect("the other opens");
    assert!(!std::sync::Arc::ptr_eq(&first, &second));
    assert!(
        std::sync::Arc::ptr_eq(&first, &server.sefer(SEFER).expect("still here")),
        "opening a second sefer evicted the first"
    );
}

#[test]
fn a_correction_is_not_hidden_by_the_copy_that_was_already_read() {
    // The window knows this failure by name: it re-reads the sefer after a fix.
    // A cache that outlived the fix would hand a program the words it had just
    // corrected out of the text, and it would look like the correction had not
    // been taken.
    let mut server = server();
    handshake(&mut server);
    let at = first_of(SEFER);

    let before = server.sefer(SEFER).expect("it opens");
    let read = tool(&mut server, "read", json!({"id": at}));
    let counting = read["segments"][0]["counting"]
        .as_str()
        .expect("a string to count into")
        .to_string();
    assert!(
        counting.chars().count() > 2,
        "the fixture's first line is shorter than this test needs: {counting}"
    );
    // Two letters that are not the two letters already there.
    let says = if counting.starts_with("אא") {
        "בב"
    } else {
        "אא"
    };

    tool(
        &mut server,
        "correct",
        json!({"at": at, "says": says, "from_char": 0, "to_char": 2, "kind": "ocr"}),
    );

    let after = server.sefer(SEFER).expect("it opens again");
    assert!(
        !std::sync::Arc::ptr_eq(&before, &after),
        "the sefer parsed before the correction is not the sefer after it"
    );
    let corrected = tool(&mut server, "read", json!({"id": at}));
    assert!(
        corrected["segments"][0]["counting"]
            .as_str()
            .expect("a string")
            .starts_with(says),
        "a read after a correction has to see the correction: {}",
        corrected["segments"][0]["counting"]
    );

    // And taking it back is the same rule in the other direction.
    tool(&mut server, "uncorrect", json!({"at": at, "says": says}));
    let again = tool(&mut server, "read", json!({"id": at}));
    assert!(
        again["segments"][0]["counting"]
            .as_str()
            .expect("a string")
            .starts_with(
                &counting[..counting
                    .char_indices()
                    .nth(2)
                    .map_or(counting.len(), |(byte, _)| byte)]
            ),
        "the correction was taken back and the read still has it: {}",
        again["segments"][0]["counting"]
    );
}
