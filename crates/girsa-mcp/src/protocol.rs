//! JSON-RPC 2.0, as MCP's stdio transport carries it.
//!
//! One request per line, one response per line, no framing headers. Written by
//! hand rather than taken from a crate: it is a hundred lines of `serde_json`,
//! and the alternative is a dependency whose licence and release cadence this
//! project would then be carrying for the sake of a message envelope (T7).
//!
//! # The version is agreed, not assumed
//!
//! A client sends the protocol version it speaks in `initialize`. If it is one
//! of the ones below, that is what comes back — echoing a version this server
//! has never heard of would be claiming compatibility it cannot have. Anything
//! else gets [`LATEST`], and the client decides whether to go on.

use serde_json::{json, Value};

/// The MCP revisions this server knows how to talk.
///
/// Newest first, which is the order [`initialize`] prefers them in.
pub const SPOKEN: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// What is offered when the client asks for something not in [`SPOKEN`].
pub const LATEST: &str = SPOKEN[0];

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;

/// One incoming call.
#[derive(Debug, Clone)]
pub struct Request {
    /// Absent for a notification, which gets no reply.
    pub id: Option<Value>,
    pub method: String,
    /// Always an object, `{}` where the caller sent none — so every reader is
    /// spared the same three lines of unwrapping.
    pub params: Value,
}

impl Request {
    /// Read one line.
    ///
    /// # Errors
    ///
    /// The response to send back, already shaped: a line that will not parse
    /// has no id to answer under, so the error carries a null one, which is
    /// what JSON-RPC says to do.
    pub fn parse(line: &str) -> Result<Self, Response> {
        let value: Value = serde_json::from_str(line)
            .map_err(|e| Response::error(PARSE_ERROR, &e.to_string()).with_id(Value::Null))?;
        let Some(method) = value.get("method").and_then(Value::as_str) else {
            return Err(Response::error(INVALID_REQUEST, "no `method`")
                .with_id(value.get("id").cloned().unwrap_or(Value::Null)));
        };
        Ok(Self {
            id: value.get("id").cloned().filter(|id| !id.is_null()),
            method: method.to_string(),
            params: value.get("params").cloned().unwrap_or_else(|| json!({})),
        })
    }
}

/// One outgoing answer, before it has been given the id it answers.
#[derive(Debug, Clone)]
pub struct Response {
    id: Value,
    body: Result<Value, (i64, String)>,
}

impl Response {
    #[must_use]
    pub const fn ok(result: Value) -> Self {
        Self {
            id: Value::Null,
            body: Ok(result),
        }
    }

    #[must_use]
    pub fn error(code: i64, message: &str) -> Self {
        Self {
            id: Value::Null,
            body: Err((code, message.to_string())),
        }
    }

    #[must_use]
    pub fn with_id(mut self, id: Value) -> Self {
        self.id = id;
        self
    }

    /// The line to write.
    #[must_use]
    pub fn write(&self) -> String {
        let value = match &self.body {
            Ok(result) => json!({"jsonrpc": "2.0", "id": self.id, "result": result}),
            Err((code, message)) => json!({
                "jsonrpc": "2.0",
                "id": self.id,
                "error": {"code": code, "message": message},
            }),
        };
        value.to_string()
    }
}

/// What the server tells a client about itself, once.
///
/// A raw string with its own line breaks rather than a continued literal:
/// `rustfmt` reindents a continued one and the indentation lands **inside the
/// string**, which would put four spaces down the middle of the first thing an
/// agent reads.
const INSTRUCTIONS: &str = r"The Girsa library: ~7,200 seforim, 5,000,545 permanently-named segments and a
link graph of 4.1 million edges, all on this machine and none of it fetched.

Two things about this engine are deliberate and will not be worked around:

1. Search is literal by default (`mode` is `torat-emet` unless you say otherwise).
   Nothing is stemmed, expanded or guessed. When a literal search finds nothing
   you are handed the relaxation ladder with its counts already computed, and
   nothing is applied until you ask for a rung by name.
2. A citation with more than one plausible target comes back as a list of
   candidates, never as a pick. Choose one, or ask the person you are working for.

Segment ids look like `girsa:bavli/berakhot/2a:1#1` and are permanent: they
survive a correction to the text and an upstream re-segmentation, which is what
makes them safe to write into a document.";

/// The answer to `initialize`.
#[must_use]
pub fn initialize(asked_for: Option<&Value>) -> Value {
    let version = asked_for
        .and_then(Value::as_str)
        .filter(|v| SPOKEN.contains(v))
        .unwrap_or(LATEST);
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "girsa", "version": env!("CARGO_PKG_VERSION")},
        "instructions": INSTRUCTIONS,
    })
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_notification_is_told_apart_from_a_call_by_having_no_id() {
        let call = Request::parse(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).expect("parses");
        assert_eq!(call.id, Some(json!(1)));
        let note = Request::parse(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .expect("parses");
        assert_eq!(note.id, None);
        // A null id is a notification too, and a server that answered it would
        // put a line on the pipe its client is not waiting for.
        let null =
            Request::parse(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#).expect("parses");
        assert_eq!(null.id, None);
    }

    #[test]
    fn a_version_this_server_has_not_heard_of_is_not_echoed_back() {
        // Echoing it would be claiming compatibility with a revision this code
        // was written before.
        let agreed = initialize(Some(&json!("2025-03-26")));
        assert_eq!(agreed["protocolVersion"], json!("2025-03-26"));
        let offered = initialize(Some(&json!("2099-01-01")));
        assert_eq!(offered["protocolVersion"], json!(LATEST));
        assert_eq!(initialize(None)["protocolVersion"], json!(LATEST));
    }

    #[test]
    fn a_line_that_will_not_parse_is_answered_under_a_null_id() {
        let Err(response) = Request::parse("{not json") else {
            panic!("that is not json");
        };
        let written: Value = serde_json::from_str(&response.write()).expect("writes json");
        assert_eq!(written["id"], Value::Null);
        assert_eq!(written["error"]["code"], json!(PARSE_ERROR));
    }

    #[test]
    fn params_are_an_object_even_when_the_caller_sent_none() {
        let call =
            Request::parse(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).expect("parses");
        assert!(call.params.is_object());
    }
}
