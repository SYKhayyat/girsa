//! What a program may ask the library, and what it gets back.
//!
//! Ten to read with, six to write with, and each is a thin call onto the engine
//! the window uses. The thinness is the design: a tool that reimplemented a
//! search would be the place where spec.md §9's guarantees quietly stopped
//! applying.
//!
//! # Deleting asks you to prove you have looked
//!
//! The three writes each have an undo now, and each takes an argument that
//! cannot be filled in without having read the thing: the note's own words, the
//! link's current type, the words the correction reads. A window asks *are you
//! sure* by **showing** you what you are about to lose, and this end has no
//! screen — so the question it asks instead is one only a caller that looked can
//! answer, and a wrong answer is refused with the thing left alone. The refusal
//! does not print the right answer, which would turn the check into a two-call
//! formality passed without ever reading anything.
//!
//! # Two refusals, encoded here
//!
//! **The engine never widens without being asked.** `search` runs Torat Emet
//! unless the caller names another mode, and a literal zero comes back with the
//! ladder *priced and unapplied* — the same offers the window draws (§9.6). A
//! caller that wants a widened count has to ask for `mode: "smart"` and is told
//! in the answer that it did.
//!
//! **Ambiguity is a choice, not a pick.** `resolve` returns every candidate the
//! shelf could not rule out. There is no `first()` anywhere in this file
//! (BUILDER.md rule 6), and the cost of that shows: `שו"ע או"ח א` comes back as
//! a list, and the caller has to decide, exactly as a person would.
//!
//! # And one promise about size
//!
//! Every tool that can return a long list takes a `limit`, defaults it, and
//! **says what it cut**. A list that silently stopped at ten reads to a caller —
//! and to whatever the caller is writing — as *these are all of them*.

use serde_json::{json, Value};

use girsa_corpus::segment::SegmentId;
use girsa_link::chain::{self, Direction, Found, Graph, Limits};
use girsa_search::bar::Answer;
use girsa_search::chips::Chips;
use girsa_search::index::Paging;
use girsa_search::Mode;

use crate::protocol::{Response, INVALID_PARAMS};
use crate::Server;

/// How many rows a tool returns when the caller does not say.
const DEFAULT_LIMIT: usize = 10;
/// The most any one call will return, however large a `limit` is asked for.
/// Reported when it bites, like every other cap in this project.
///
/// **Configurable, because 50 is a guess about the caller and not a fact about the
/// engine** (B24). `GIRSA_MCP_MAX_LIMIT` raises or lowers it: an agent summarising
/// a sugya wants more rows than a chat turn does, and the honest ceiling depends on
/// whose context window is on the other end. It is clamped to something sane at
/// both ends — a limit of zero is a tool that returns nothing, and a limit of a
/// million is a caller that has not thought about it.
///
/// What does **not** change is that the cap says what it cut. `search` came back
/// `total: 79 · hits: 50 · not_shown: 29` in the audit and that is the one thing
/// that matters when the caller is a program that cannot complain, so raising the
/// number must not quietly turn the reporting off.
fn max_limit() -> usize {
    std::env::var("GIRSA_MCP_MAX_LIMIT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(MAX_LIMIT_DEFAULT)
        .clamp(1, 5_000)
}

/// What the cap is when nothing says otherwise.
const MAX_LIMIT_DEFAULT: usize = 50;

/// The tools, as `tools/list` describes them.
///
/// `writable` adds the three that write into your own layer. They are **absent**
/// rather than listed-and-refused when it is off: a tool list is what a program
/// plans against, and one that advertises a door it cannot open gets an agent
/// halfway through a plan before the refusal arrives.
#[must_use]
pub fn catalogue(writable: bool) -> Value {
    let mut tools = json!([
        {
            "name": "search",
            "title": "Search the corpus",
            "description": "\
    Search ~7,200 seforim. Literal by default: what you pass is what is searched \
    for, with nikud and te'amim stripped on both sides and nothing else changed. \
    On zero results you are handed the relaxation ladder with counts already \
    computed — nothing is applied. Pass mode='smart' to let the engine widen, and \
    it will tell you that it did.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Hebrew, unpointed or pointed."},
                    "mode": {
                        "type": "string",
                        "enum": ["torat-emet", "smart", "regex"],
                        "description": "Default torat-emet: literal, nothing expanded or guessed."
                    },
                    "sefer": {"type": "string", "description": "A work slug, to search inside one sefer."},
                    "tag": {
                        "type": "string",
                        "description": "One of your own tags, to search only what you tagged with it.     Corpus seforim carry no tags; this narrows to your notes."
                    },
                    "limit": {"type": "integer", "minimum": 1, "maximum": max_limit()}
                },
                "required": ["query"]
            }
        },
        {
            "name": "read",
            "title": "Read a segment",
            "description": "\
    The text at a permanent segment id, with the lines around it. Ids look like \
    girsa:bavli/berakhot/2a:1#1 and survive corrections and re-segmentation. \
    `text` is the segment as the corpus stores it, markup and all; `counting` is \
    the same words with the markup taken out, and it is the string `correct`'s \
    character offsets are into. `corrections` is what your own layer has already \
    said about this line, and is where `uncorrect` gets its `says`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "around": {"type": "integer", "minimum": 0, "maximum": 20,
                               "description": "How many segments either side. Default 0."}
                },
                "required": ["id"]
            }
        },
        {
            "name": "resolve",
            "title": "Resolve a citation",
            "description": "\
    Turn a mareh makom as a person writes it — שו\"ע או\"ח נח:א, ברכות ב., Berakhot 2a \
    — into segment ids. A citation with more than one plausible target comes back \
    as every candidate, never as a pick. There is no reader standing here: pass \
    `sefer` when the citation is relative to a work you are already in, and read \
    `resolved_against` on the reply.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "citation": {"type": "string"},
                    "sefer": {"type": "string", "description": "A work slug to complete the citation against."}
                },
                "required": ["citation"]
            }
        },
        {
            "name": "where_from",
            "title": "Where is this phrase from",
            "description": "\
    Given a phrase, the places in the corpus it appears — the same engine that \
    answers 'who quotes this Gemara' from the other direction. Matched literally \
    first; if the ladder had to be climbed, the answer says which rung.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "phrase": {"type": "string"},
                    "except": {"type": "string", "description": "A work slug to leave out — the sefer you are already in."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": max_limit()}
                },
                "required": ["phrase"]
            }
        },
        {
            "name": "links",
            "title": "What links to this line",
            "description": "\
    Every edge touching a segment, in both directions, with its type and what the \
    corpus called it. Half of the graph says only `references` — that the two are \
    joined somehow — and those rows say so rather than being dressed as commentary.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": max_limit()}
                },
                "required": ["id"]
            }
        },
        {
            "name": "trace",
            "title": "Trace the transmission chain",
            "description": "\
    Walk from a segment along the axis of time — forward to how it became halacha, \
    back to where a ruling came from. Direction is when the seforim were written, \
    not which way the corpus happened to store the edge. Each chain says whether \
    every hop on it is a real claim; what was not followed is counted and returned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "direction": {"type": "string", "enum": ["forward", "back"]},
                    "depth": {"type": "integer", "minimum": 1, "maximum": 5},
                    "width": {"type": "integer", "minimum": 1, "maximum": 40}
                },
                "required": ["id", "direction"]
            }
        },
        {
            "name": "path",
            "title": "How two texts are connected",
            "description": "\
    The shortest chain of links between two segments, regardless of when either was \
    written. Answers `not_within` when it ran out of budget, which is not the same \
    statement as there being no path, and `none` only when everything reachable \
    from both ends was opened.",
            "inputSchema": {
                "type": "object",
                "properties": {"from": {"type": "string"}, "to": {"type": "string"}},
                "required": ["from", "to"]
            }
        },
        {
            "name": "fork",
            "title": "Where two readings were argued out later",
            "description": "\
    Pairs of later seforim that read the same line and are cited together by a \
    third. This is the shape a machlokes leaves behind, not a finding: the corpus \
    has no `disputes` edge anywhere in it and nothing here claims the two disagree.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "width": {"type": "integer", "minimum": 2, "maximum": 40},
                    "limit": {"type": "integer", "minimum": 1, "maximum": max_limit()}
                },
                "required": ["id"]
            }
        },
        {
            "name": "adjacent",
            "title": "Adjacent by meaning, not by these words",
            // The last sentence is `girsa_lane::MEASURED` rather than a copy
            // of it. It used to be written out here — and this was the *only*
            // place in the tree that said what the lane is known to be bad at,
            // so a robot was told and a reader was not. One string now, drawn
            // in the window under the results as well.
            "description": format!("\
    A separate lane, and it must be reported as one. Give it a line as you half \
    remember it and it returns passages that are ADJACENT — found by an embedding \
    model rather than by matching any word you passed. It is off unless the reader \
    turned it on and side-loaded a model, and it only covers what the reader chose \
    to embed: every answer carries a `coverage` sentence saying what is in the \
    index and what is not, and you must not present these results as the places a \
    phrase appears, or as complete. It is {}", girsa_lane::MEASURED),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "A line as you remember it — not a question."},
                    "sefer": {"type": "string", "description": "A work slug, to look in one sefer."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": max_limit()}
                },
                "required": ["text"]
            }
        },
        {
            "name": "seforim",
            "title": "Find a sefer",
            "description": "\
    Look a sefer up by name, Hebrew or English, and get its slug, its shelf, its \
    author and when it was written — which is what every other tool here wants.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": max_limit()}
                },
                "required": ["title"]
            }
        },
        {
            "name": "marks",
            "title": "Your marks",
            "description": "\
    The highlights, lns and bookmarks your own layer holds. An agent that can \
    see the layer it is standing in is the point: without this, everything \
    `write_note` writes has a twin nobody can read back. Filter by place, by \
    sefer, or ask for bookmarks alone.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "at": {"type": "string", "description": "A segment id — marks left on exactly that segment."},
                    "sefer": {"type": "string", "description": "A work slug — marks anywhere in one sefer."},
                    "bookmarks": {"type": "boolean", "description": "True: bookmarks only."},
                    "tag": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": max_limit()}
                }
            }
        },
        {
            "name": "folders",
            "title": "Your chaburah folders",
            "description": "\
    The folders your layer holds, each with its members in order — places, whole \
    seforim of yours, saved queries. Read-only here: a folder's shape is the \
    reader's business and an agent's order would be a guess.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "holding": {"type": "string", "description": "A segment id — only folders that hold this place."}
                }
            }
        },
        {
            "name": "queries",
            "title": "Your saved searches",
            "description": "\
    The queries you have saved, as typed — sigils and all — with what they say. \
    What `save_query` writes and `forget_query` takes back.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "who_cites",
            "title": "Who cites this place",
            "description": "\
    Which of YOUR OWN documents cite a place — notes in the drawer and documents \
    in the registry whose refs cover it. Not the corpus's link graph; `links` is \
    that. This is your own writing answering back.",
            "inputSchema": {
                "type": "object",
                "properties": {"id": {"type": "string", "description": "A segment id or girsa: ref."}},
                "required": ["id"]
            }
        }
    ]);
    if !writable {
        return tools;
    }
    let Some(list) = tools.as_array_mut() else {
        return tools;
    };
    list.extend(writing());
    tools
}

/// The tools that write, and everything they will not do.
///
/// Three, and they are the three the record named: a note, a link, a
/// correction. Every one of them writes into **`personal/`** and nothing here
/// can reach the corpus at all — the same wall the window is behind, and the
/// reason spec.md §4.1 can promise the download stays replaceable.
///
/// `readOnlyHint: false` on each, so a client that asks its user before a write
/// knows which calls to ask about. The hint is a claim about the tool and not a
/// promise about the client: this server does not know what the caller does
/// with it, which is why the flag that turns these on exists at all.
fn writing() -> Vec<Value> {
    // Read off `EdgeType::ALL` rather than typed here. The first draft of this
    // file listed `explains`, `sources` and `parallels`, none of which this
    // graph has ever had — a description that names types the parser will
    // refuse is a description that costs a program a round trip to find out
    // the truth, and the only reason it was caught is that a test called one
    // of them.
    let kinds: Vec<&str> = girsa_link::EdgeType::ALL
        .iter()
        .map(|kind| kind.as_str())
        .collect();
    let kinds = kinds.join(", ");
    vec![
        json!({
            "name": "write_note",
            "title": "Write a note",
            "description": "\
        Write a note anchored to a place in the library. A note is a sefer on your \
        own shelf (spec.md section 11) with the same kind of typed edge to the sugya \
        as any commentary, so what you write comes back in the links on that line \
        rather than in a list of its own. Written into `personal/` — nothing here \
        touches the corpus.",
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": false},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "at": {"type": "string", "description": "A segment id or a girsa: ref — the line it is about."},
                    "title": {"type": "string", "description": "What to call it. Defaults to the place."},
                    "text": {"type": "string", "description": "The note itself."},
                    "tags": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["at", "text"]
            }
        }),
        json!({
            "name": "draw_link",
            "title": "Draw a link",
            "description": "\
        Join two places with a typed edge. An override in your own layer, never an \
        edit to the shipped graph (spec.md section 8.3): the corpus's own links stay \
        exactly as they were, and yours sit beside them marked as yours. A link \
        drawn here is the same kind of object as the 4.2 million Sefaria seeded, so \
        it is walked by `trace` and shown by `links` like any other.",
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "A segment id or girsa: ref."},
                    "to": {"type": "string", "description": "The other end."},
                    "type": {
                        "type": "string",
                        "enum": girsa_link::EdgeType::ALL.iter().map(|kind| kind.as_str()).collect::<Vec<_>>(),
                        "description": format!("What it claims. One of: {kinds}.")
                    }
                },
                "required": ["from", "to", "type"]
            }
        }),
        json!({
            "name": "correct",
            "title": "Correct a word",
            "description": "\
        Record a correction to a stretch of a segment. An overlay, never an edit \
        (spec.md section 4.1): the base text on disk is untouched, so re-importing \
        the corpus keeps your correction and a download stays replaceable. `kind` \
        says what is being claimed — `ocr` is *the scanner got this wrong* and \
        `girsa` is *this edition reads differently*, which are the same machinery \
        and two very different statements. Character offsets are into `counting` — \
        the field `read` returns beside `text`, which is the same words with the \
        markup taken out. Not into `text`: a segment can carry markup, and counting \
        into the stored string would name letters nobody can see.",
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "at": {"type": "string", "description": "A segment id or girsa: ref."},
                    "from_char": {"type": "integer", "minimum": 0},
                    "to_char": {"type": "integer", "minimum": 1},
                    "says": {"type": "string", "description": "What those characters should read."},
                    "kind": {"type": "string", "enum": ["ocr", "girsa"]},
                    "note": {"type": "string", "description": "Why, in your own words."},
                    "source": {"type": "string", "description": "For a variant: the sefer that says so, as a ref."}
                },
                "required": ["at", "from_char", "to_char", "says"]
            }
        }),
        json!({
            "name": "forget_note",
            "title": "Throw a note away",
            "description": "\
        Delete a note: the file, the sefer on your shelf and the edges to the sugya. \
        `saying` must be exactly what the note says now — every paragraph, joined by \
        a blank line, which is what `search` and `read` give you. This end cannot \
        show you what you are about to delete, so it asks you to prove you have \
        looked; a mismatch is refused and the note is left alone. Nothing here \
        touches the corpus, and nothing here can undo this.",
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "note": {"type": "string", "description": "The note's name — what `write_note` returned as `wrote`."},
                    "saying": {"type": "string", "description": "What it says now, exactly. Read it first."}
                },
                "required": ["note", "saying"]
            }
        }),
        json!({
            "name": "undraw_link",
            "title": "Take back a link you drew",
            "description": format!("\
        Remove a link **you** drew. The shipped graph is not touched and cannot be: \
        this only takes back a `draw_link`, so an edge Sefaria seeded is refused \
        rather than deleted. `type` must be the type the link has now — `links` \
        reports it — because this end cannot show you the edge you are about to \
        remove. One of: {kinds}."),
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "A segment id or girsa: ref — the end it was drawn from."},
                    "to": {"type": "string", "description": "The other end, in the direction it was drawn."},
                    "type": {
                        "type": "string",
                        "enum": girsa_link::EdgeType::ALL.iter().map(|kind| kind.as_str()).collect::<Vec<_>>(),
                        "description": "The type it has now. Not what you meant to draw — what `links` says."
                    }
                },
                "required": ["from", "to", "type"]
            }
        }),
        json!({
            "name": "uncorrect",
            "title": "Take a correction back",
            "description": "\
        Remove a correction from your own layer. The base text was never edited, so \
        this restores nothing — it stops an overlay being applied. `says` must be \
        what the correction currently reads, which `read` returns in `corrections`; \
        a segment can carry more than one, and naming the words is how you say which. \
        Refused, and nothing removed, if no correction there says that.",
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "at": {"type": "string", "description": "A segment id or girsa: ref."},
                    "says": {"type": "string", "description": "What the correction reads now — `read`'s `corrections[].says`."}
                },
                "required": ["at", "says"]
            }
        }),
        json!({
            "name": "bookmark",
            "title": "Bookmark a place",
            "description": "\
        Put a bookmark in your own layer. A bookmark is a mark with no span — \
        the whole segment — and a name if you give one. It shows up wherever \
        your layer shows marks, and `marks` is where you take it back from.",
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "at": {"type": "string", "description": "A segment id or girsa: ref."},
                    "label": {"type": "string", "description": "What you called it."},
                    "colour": {"type": "string"},
                    "tag": {"type": "string"}
                },
                "required": ["at"]
            }
        }),
        json!({
            "name": "forget_mark",
            "title": "Take a mark back",
            "description": "\
        Remove a mark — bookmark or highlight — from your own layer, by the id \
        `marks` gave you. The id is the proof of having looked: there is no \
        other way to name one.",
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false},
            "inputSchema": {
                "type": "object",
                "properties": {"id": {"type": "string", "description": "`marks`' `id`."}},
                "required": ["id"]
            }
        }),
        json!({
            "name": "save_query",
            "title": "Save a search",
            "description": "\
        Keep a query as you wrote it, under a name, in your own layer. The same \
        object the window's saved-searches row holds, so it comes back there too.",
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "typed": {"type": "string", "description": "The query as it would be typed into the bar, sigils and all."}
                },
                "required": ["name", "typed"]
            }
        }),
        json!({
            "name": "forget_query",
            "title": "Throw a saved query away",
            "description": "\
        Remove a saved query from your own layer. `typed` must be what it says \
        now — `queries` gives you that — because this end cannot show you what \
        you are about to delete; a mismatch is refused and nothing is removed.",
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": false},
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "typed": {"type": "string", "description": "What it says now — `queries`' `typed`."}
                },
                "required": ["name", "typed"]
            }
        }),
    ]
}

/// Serve one `tools/call`.
pub fn call(server: &mut Server, params: &Value) -> Response {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Response::error(INVALID_PARAMS, "no tool named");
    };
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let answered = match name {
        "search" => search(server, &args),
        "read" => read(server, &args),
        "resolve" => resolve(server, &args),
        "where_from" => where_from(server, &args),
        "links" => links(server, &args),
        "trace" => trace(server, &args),
        "path" => path(server, &args),
        "fork" => fork(server, &args),
        "seforim" => seforim(server, &args),
        "marks" => marks(server, &args),
        "folders" => folders(server, &args),
        "queries" => queries(server, &args),
        "who_cites" => who_cites(server, &args),
        "adjacent" => adjacent(server, &args),
        // The three that write. Guarded here as well as being absent from the
        // catalogue, because a tool list is a description and this is the
        // door: a client that remembered the tools from a writable session and
        // called one against a read-only server gets a refusal that names the
        // reason rather than a note appearing in somebody's layer.
        "write_note" | "draw_link" | "correct" | "forget_note" | "undraw_link" | "uncorrect"
        | "bookmark" | "forget_mark" | "save_query" | "forget_query"
            if !server.is_writable() =>
        {
            Err(format!(
                "{name} writes into your own layer, and this server was started without --writable"
            ))
        }
        "write_note" => write_note(server, &args),
        "draw_link" => draw_link(server, &args),
        "correct" => correct(server, &args),
        "forget_note" => forget_note(server, &args),
        "undraw_link" => undraw_link(server, &args),
        "uncorrect" => uncorrect(server, &args),
        "bookmark" => bookmark(server, &args),
        "forget_mark" => forget_mark(server, &args),
        "save_query" => save_query(server, &args),
        "forget_query" => forget_query(server, &args),
        other => Err(format!("no such tool: {other}")),
    };
    Response::ok(match answered {
        Ok(value) => content(&value, false),
        // A refusal is a *result* with `isError`, not a JSON-RPC error: the
        // protocol keeps transport failures and tool failures apart, and a
        // caller that cannot tell them apart cannot tell "I asked wrongly"
        // from "the server is broken".
        Err(why) => content(&json!({"refused": why}), true),
    })
}

/// A tool result, in both shapes: the text a reader sees and the object a
/// newer client parses.
fn content(value: &Value, is_error: bool) -> Value {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": value,
        "isError": is_error,
    })
}

fn text_arg(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("`{key}` is required"))
}

fn id_arg(args: &Value, key: &str) -> Result<SegmentId, String> {
    text_arg(args, key)?
        .parse()
        .map_err(|e| format!("`{key}` is not a segment id: {e}"))
}

fn limit_of(args: &Value) -> usize {
    args.get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |n| (n as usize).clamp(1, max_limit()))
}

/// The shape a segment is named in, everywhere in this file.
///
/// The wire shape is this file's; **what goes in it** is `girsa_app::Naming`,
/// the one place a title, an address and a date are worked out for a row. This
/// function used to do all three by hand, and was one of four that did — and
/// the four disagreed. The window's `HitRow` honoured the language the reader
/// set and this did not. The lane's `Near` carried no address and this carried
/// one. `girsa-chain`'s printer said `[no date]` where this said nothing at
/// all, so an agent reading a chain could tell an undated work from a dated one
/// and an agent reading a search result could not.
fn named(server: &Server, id: &SegmentId) -> Value {
    let at = server.names().of(id);
    json!({
        "id": at.id.to_string(),
        "sefer": at.work,
        "title": at.title,
        "address": at.address,
        "written": at.written,
        "era": at.era,
    })
}

fn search(server: &Server, args: &Value) -> Result<Value, String> {
    let query = text_arg(args, "query")?;
    let mode = match args.get("mode").and_then(Value::as_str) {
        None | Some("torat-emet") => Mode::ToratEmet,
        Some("smart") => Mode::Smart,
        Some("regex") => Mode::Regex,
        Some(other) => return Err(format!("no such mode: {other}")),
    };
    let limit = limit_of(args);
    let mut chips = Chips {
        mode,
        ..Chips::default()
    };
    // Narrowed through the same function a facet click goes through, so a program
    // cannot narrow by a rule the window does not have — and the two dimensions are
    // named by the same `Dimension`, so a chip that says `tag` and a flag that
    // applies it cannot drift.
    for (arg, dimension) in [
        ("sefer", girsa_search::facets::Dimension::Sefer),
        ("tag", girsa_search::facets::Dimension::Tag),
    ] {
        if let Some(key) = args.get(arg).and_then(Value::as_str) {
            chips.scope = girsa_search::facets::narrow(
                &chips.scope,
                server.bar.catalogue(),
                dimension,
                &girsa_search::facets::Row {
                    key: key.to_string(),
                    label: key.to_string(),
                    count: 0,
                    depth: 0,
                },
            );
        }
    }

    let paging = Paging {
        from: 0,
        size: limit,
    };
    // The declared scope is the citation context, because it is the only one
    // this surface has. No address travels with it — an agent is standing
    // nowhere — so partial spans complete against nothing, exactly as they do
    // for a window reader with no pane open; `resolve` says so on its reply.
    let context = girsa_ref::resolve::Context {
        work: Some(chips.scope.works().into_iter().collect::<Vec<String>>()),
        address: None,
    };
    match server.bar.ask(&query, &chips, paging, &context) {
        Answer::Segments {
            results,
            offers,
            note,
            rungs: _,
            // The place these words also name. An agent asking this tool is
            // asking for hits; the offer is the window's affordance and there
            // is nothing here to click.
            landing: _,
        } => {
            let hits: Vec<Value> = results
                .hits
                .iter()
                .map(|hit| {
                    let mut row = named(server, &hit.id);
                    row["text"] = json!(hit.text);
                    if hit.is_scanned() {
                        row["read_off_a_scan"] = json!(true);
                    }
                    row
                })
                .collect();
            Ok(json!({
                "mode": match mode {
                    Mode::ToratEmet => "torat-emet",
                    Mode::Smart => "smart",
                    _ => "regex",
                },
                "searched_for": results.header,
                "total": results.total,
                "showing": hits.len(),
                "not_shown": results.total.saturating_sub(hits.len()),
                // What this search could not see (B7). The window's results header
                // has said it since B7; a program is entitled to the same sentence,
                // and more so — a `total` of zero over an index that has never seen
                // your notes reads to an agent as *this is not in the library*, and
                // an agent cannot ask a follow-up question about it.
                //
                // Composed by `girsa_nearby::Unseen` rather than by calling
                // `Unindexed::said` here, which is what this line used to do: the
                // layer clauses were the only ones this answer carried, and the
                // one thing a caller could not learn from `did_not_search` was
                // that there is another thing it did not search.
                "did_not_search": girsa_nearby::Unseen::over_layer(server.unindexed(), None).said(),
                "hits": hits,
                "note": note,
                // §9.6: priced, and applied to nothing. The counts are computed
                // from the same query a click would run.
                "offered_and_not_applied": offers.offers.iter().map(|offer| json!({
                    "rung": offer.label,
                    "would_find": offer.count,
                })).collect::<Vec<Value>>(),
                "could_not_be_priced": offers.refused.iter().map(|r| json!(r.why)).collect::<Vec<Value>>(),
            }))
        }
        Answer::Cited(landing) => Ok(json!({
            "mode": "citation",
            "typed": landing.typed,
            "places": landing.places.iter().map(|place| named(server, &place.run.first)).collect::<Vec<Value>>(),
        })),
        Answer::Refused(why) => Err(why),
    }
}

fn read(server: &Server, args: &Value) -> Result<Value, String> {
    let id = id_arg(args, "id")?;
    let around = args
        .get("around")
        .and_then(Value::as_u64)
        .map_or(0, |n| (n as usize).min(20));
    let open = server
        .shelf
        .read(id.work())
        .map_err(|e| format!("cannot open {}: {e}", id.work()))?;
    let nth = open
        .position_of(&id)
        .ok_or_else(|| format!("{id} is not a segment of {}", id.work()))?;
    let from = nth.saturating_sub(around);
    let to = (nth + around + 1).min(open.segments.len());
    let segments: Vec<Value> = open.segments[from..to]
        .iter()
        .map(|segment| {
            let mut row = named(server, &segment.id);
            row["text"] = json!(segment.text);
            // The same words with the markup out, because that is the string
            // `correct` counts into and it was not being handed to anybody.
            //
            // `read` returned `<big><strong>מֵאֵימָתַי</strong></big> …` and the
            // tool that takes character offsets counted into `מֵאֵימָתַי …`, while
            // its own description said the two were the same string. A caller
            // that believed the description corrected different letters than it
            // named and got a success back — `from_char: 0, to_char: 4` reads as
            // `<big` in what it was given and landed on `מֵאֵ`.
            //
            // Built by the same `Shown` the correction path uses rather than by
            // stripping tags here, which would be a second opinion about what
            // markup is and would be wrong the first time the two disagreed.
            row["counting"] = json!(girsa_app::display::Shown::of(
                &segment.text,
                girsa_app::session::Pointing::Full
            )
            .text());
            // And what your layer has already said about the line, which is
            // what `uncorrect` asks you to name. Nothing here could learn it
            // otherwise, and an undo you cannot address is not an undo.
            let already = corrections_on(&open, &segment.id);
            if !already.is_empty() {
                row["corrections"] = json!(already);
            }
            row["asked_for"] = json!(segment.id == id);
            row
        })
        .collect();
    Ok(json!({"segments": segments}))
}

/// What your own layer has said about one line, in the shape `uncorrect` reads.
///
/// Applied and merely noted alike: a variant that is recorded and not shown is
/// still a correction you can take back, and leaving it out would make the one
/// kind of correction nothing displays also the one kind nothing can undo.
fn corrections_on(open: &girsa_app::shelf::Open, id: &SegmentId) -> Vec<Value> {
    let Some(corrected) = open.correction(id) else {
        return Vec::new();
    };
    corrected
        .applied
        .iter()
        .map(|a| (a, true))
        .chain(corrected.noted.iter().map(|a| (a, false)))
        .map(|(a, applied)| {
            json!({
                "was": a.was,
                "says": a.now,
                "kind": a.kind.as_str(),
                "who": a.who,
                "applied": applied,
                "note": a.note,
                "source": a.source,
            })
        })
        .collect()
}

fn resolve(server: &Server, args: &Value) -> Result<Value, String> {
    let citation = text_arg(args, "citation")?;
    // An optional sefer gives the citation something to complete against.
    // Without one there is no standing at all: a bare address ("הלכה ה")
    // refuses, and a partial span fills its missing sections against nothing
    // — which is exactly what a window does when the pane it was typed in
    // has no place yet, except there the reader can see where it landed.
    let scoped: Option<String> = args.get("sefer").and_then(Value::as_str).map(str::to_string);
    let context = girsa_ref::resolve::Context {
        work: scoped.clone().map(|slug| vec![slug]),
        address: None,
    };
    // Through the bar's citation mode, which is the path the query bar takes —
    // rather than the resolver underneath it, which would be a second way of
    // reading a mareh makom and a second place for it to disagree.
    let chips = Chips {
        mode: Mode::Citation,
        ..Chips::default()
    };
    let landing = match server.bar.ask(&citation, &chips, Paging::first(), &context) {
        Answer::Cited(landing) => landing,
        Answer::Refused(why) => return Err(why),
        Answer::Segments { .. } => return Err("that did not read as a citation".to_string()),
    };
    let places: Vec<Value> = landing
        .places
        .iter()
        .map(|place| {
            let mut row = named(server, &place.run.first);
            row["ref"] = json!(place.reference.to_string());
            if let Some(last) = &place.run.last {
                row["through"] = json!(last.to_string());
            }
            row
        })
        .collect();
    Ok(json!({
        "typed": landing.typed,
        // One candidate is an answer. More than one is a choice, and this
        // server does not make it (BUILDER.md rule 6).
        "settled": places.len() == 1,
        "places": places,
        // What the citation was completed against. Null means no standing —
        // this surface has no reader standing anywhere, so a relative or
        // partial mareh makom was resolved against nothing, and an agent
        // reading `settled` alone would never know.
        "resolved_against": scoped,
        "near_misses": landing.near.len(),
        "spellings_not_shown": landing.more_spellings,
    }))
}

fn where_from(server: &Server, args: &Value) -> Result<Value, String> {
    let phrase = text_arg(args, "phrase")?;
    let except = args.get("except").and_then(Value::as_str);
    let found = girsa_search::mekoros::where_from(&server.bar, &phrase, except, limit_of(args))?;
    Ok(serde_json::to_value(&found)
        .unwrap_or_else(|_| json!({"refused": "cannot write the answer"})))
}

fn links(server: &Server, args: &Value) -> Result<Value, String> {
    let id = id_arg(args, "id")?;
    let limit = limit_of(args);
    // Read the sefer to ask it what this place has been called: an edge is
    // stored under the name its endpoint had when the row was written, and only
    // the work on disk knows whether a corpus update has moved it since.
    let sefer = server
        .shelf
        .read(id.work())
        .map_err(|e| format!("{} will not open: {e}", id.work()))?;
    let at = sefer.standing(&id);
    let touching = girsa_app::links::touching(&server.shelf, server.shelf.repairs(), &at);
    let shown: Vec<Value> = touching
        .links
        .iter()
        .take(limit)
        .map(|link| {
            let mut row = named(server, &link.other.from);
            row["type"] = json!(link.repaired.edge.edge_type.as_str());
            row["the_corpus_said"] = json!(link.repaired.edge.source_label);
            row["asserts_something"] = json!(link.repaired.edge.edge_type.is_asserted());
            row["outgoing"] = json!(link.outgoing);
            row["confidence"] = json!(link.repaired.confidence());
            row["direction"] = json!(link.repaired.edge.direction.as_str());
            row
        })
        .collect();
    Ok(json!({
        "at": named(server, &id),
        "total": touching.links.len(),
        "showing": shown.len(),
        "not_shown": touching.links.len().saturating_sub(shown.len()),
        // A sidebar quietly short of half its links reads as a sefer nobody
        // comments on. So does a tool result.
        "incoming_half_unknown": touching.incoming_unknown,
        "links": shown,
    }))
}

fn limits_of(args: &Value) -> Limits {
    let base = Limits::default();
    Limits {
        depth: args
            .get("depth")
            .and_then(Value::as_u64)
            .map_or(base.depth, |n| (n as usize).clamp(1, 5)),
        width: args
            .get("width")
            .and_then(Value::as_u64)
            .map_or(base.width, |n| (n as usize).clamp(1, 40)),
        budget: base.budget,
    }
}

fn refused(refused: &chain::Refused) -> Value {
    json!({
        "other_way_in_time": refused.wrong_way,
        "written_at_the_same_time": refused.contemporary,
        "no_date_and_no_era": refused.undated,
        "dropped_by_width": refused.over_budget,
        "rejected_in_your_layer": refused.rejected,
        "works_whose_incoming_links_could_not_be_read":
            refused.incoming_unknown.iter().cloned().collect::<Vec<String>>(),
    })
}

fn trace(server: &mut Server, args: &Value) -> Result<Value, String> {
    let id = id_arg(args, "id")?;
    let direction = match text_arg(args, "direction")?.as_str() {
        "forward" => Direction::Forward,
        "back" => Direction::Back,
        other => return Err(format!("direction is `forward` or `back`, not `{other}`")),
    };
    let limits = limits_of(args);
    let repairs = server.shelf.repairs().clone();
    // Resuming, not fresh: the shards this walk reads stay on the server for
    // the next one. A graph per call re-read them every time.
    let mut graph = Graph::resuming(
        &server.root,
        &server.timeline,
        &repairs,
        std::mem::take(&mut server.walked),
    );
    let walked = chain::trace(&mut graph, &id, direction, limits);
    server.walked = graph.into_cache();

    let chains: Vec<Value> = walked
        .ends()
        .iter()
        .map(|end| {
            let hops: Vec<Value> = walked
                .chain(*end)
                .iter()
                .filter_map(|i| walked.steps.get(*i))
                .map(|step| {
                    let mut row = named(server, &step.at.from);
                    row["type"] = json!(step.edge_type.as_str());
                    row["the_corpus_said"] = json!(step.label);
                    // Whether the corpus said which way the arrow points, or
                    // whether it is the order of two CSV columns. Priced into
                    // confidence and shown to nobody until now.
                    row["direction"] = json!(step.direction.as_str());
                    row
                })
                .collect();
            json!({
                "hops": hops,
                // False the moment one hop only says the two are joined. 49% of
                // this graph says exactly that and no more.
                "every_hop_asserts_something": walked.is_transmission(*end),
            })
        })
        .collect();

    Ok(json!({
        "from": named(server, &id),
        "direction": direction.as_str(),
        "chains": chains,
        "not_followed": refused(&walked.refused),
    }))
}

fn path(server: &mut Server, args: &Value) -> Result<Value, String> {
    let from = id_arg(args, "from")?;
    let to = id_arg(args, "to")?;
    let repairs = server.shelf.repairs().clone();
    let mut graph = Graph::resuming(
        &server.root,
        &server.timeline,
        &repairs,
        std::mem::take(&mut server.walked),
    );
    let found = chain::path(&mut graph, &from, &to, limits_of(args));
    server.walked = graph.into_cache();
    Ok(match found {
        Found::Path(links) => {
            let asserted = links.iter().filter(|l| l.edge_type.is_asserted()).count();
            json!({
                "found": true,
                "links": links.iter().map(|link| {
                    let mut row = named(server, &link.at.from);
                    row["type"] = json!(link.edge_type.as_str());
                    row["direction"] = json!(link.direction.as_str());
                    row
                }).collect::<Vec<Value>>(),
                "unasserted_hops": links.len() - asserted,
            })
        }
        Found::NotWithin { opened, depth } => json!({
            "found": false,
            // Not the same statement as `no_path`, and the difference is the
            // whole reason there are two of them.
            "why": "not_within_budget",
            "opened": opened,
            "depth": depth,
        }),
        Found::None => json!({
            "found": false,
            "why": "no_path",
            "note": "everything reachable from both ends was opened and they never met",
        }),
    })
}

fn fork(server: &mut Server, args: &Value) -> Result<Value, String> {
    let id = id_arg(args, "id")?;
    let repairs = server.shelf.repairs().clone();
    let mut graph = Graph::resuming(
        &server.root,
        &server.timeline,
        &repairs,
        std::mem::take(&mut server.walked),
    );
    let (forks, left_out) = chain::forks(&mut graph, &id, limits_of(args));
    server.walked = graph.into_cache();
    let shown = forks.iter().take(limit_of(args));
    Ok(json!({
        "at": named(server, &id),
        "note": "nothing here says these disagree — the corpus has no `disputes` edge anywhere in it",
        "pairs": shown.map(|pair| json!({
            "a": named(server, &pair.a.from),
            "b": named(server, &pair.b.from),
            // Nearest first, and each says how far it is: a witness that
            // quotes both sides itself and one that reaches them through three
            // other seforim are different claims about the same pair, and a
            // list that flattened them would be handing a program the stronger
            // reading of the weaker fact.
            "cited_together_by": pair.witnesses.iter().take(4)
                .map(|w| json!({ "at": named(server, &w.at.from), "steps": w.steps }))
                .collect::<Vec<Value>>(),
            "witnesses": pair.witnesses.len(),
            "nearest_witness_steps": pair.witnesses.first().map(|w| w.steps),
            "a_link_joins_them_directly": pair.joined,
        })).collect::<Vec<Value>>(),
        // The schema takes `limit` now, so a longer list is askable rather
        // than something `total` quietly admits to.
        "showing": forks.len().min(limit_of(args)),
        "total": forks.len(),
        "not_followed": refused(&left_out),
    }))
}

/// The semantic lane (spec.md §9.9, W30).
///
/// Three things this answer does that `search`'s does not, and all three are the
/// point: it names itself **adjacent** in every reply, it carries the coverage
/// sentence whether or not it found anything, and a lane that is off or adrift
/// comes back as a refusal with the reason rather than as an empty list. An
/// agent that got `{"hits": []}` from this would reasonably write *the corpus
/// contains nothing like it*, which is the §9 defect one layer further out than
/// a person can check.
fn adjacent(server: &Server, args: &Value) -> Result<Value, String> {
    let text = text_arg(args, "text")?;
    let limit = limit_of(args);
    let scoped: Vec<String> = args
        .get("sefer")
        .and_then(Value::as_str)
        .map(|slug| vec![slug.to_string()])
        .unwrap_or_default();

    let answer = server.lane.ask(&server.names(), &text, &scoped, limit);
    let state = server.lane.state();
    let found: Vec<Value> = answer
        .near
        .iter()
        .map(|near| {
            let mut row = named(server, near.id());
            row["text"] = json!(near.text);
            row["nearness"] = json!(near.nearness);
            row
        })
        .collect();
    Ok(json!({
        // First key, and the same wording the window draws.
        "these_are": answer.label,
        "lane": match &state {
            girsa_lane::State::Off => "off",
            girsa_lane::State::Adrift(_) => "on, but no model will run",
            girsa_lane::State::On { .. } => "on",
        },
        "model": match &state {
            girsa_lane::State::On { model, .. } => json!(model),
            _ => Value::Null,
        },
        // Said whether or not anything was found. A partial lane that reads as a
        // complete one is what §9.9 exists to prevent.
        "coverage": answer.coverage,
        // The same field `search` carries, and for the same reason. An adjacency
        // answer was the one answer in this server that said what the *lane* had
        // not covered and nothing about what your own layer holds that no lane
        // has embedded — so a chaburah written this morning was invisible to
        // `adjacent` exactly the way it was invisible to `search`, and only
        // `search` said so. Carries the coverage clause too, which is what makes
        // this the whole sentence rather than a second subset of it.
        "did_not_search": girsa_nearby::Unseen::over_layer(
            server.unindexed(),
            Some(server.lane().coverage().clone()),
        )
        .said(),
        "refused": answer.refused,
        // Null when every store was read whole. An agent that ranks these and
        // reports the top one is owed the same disclaimer the window draws.
        // The specific caveat where the general one is not enough. An agent
        // that asked a question is exactly the caller most likely to read ten
        // plausible rows as an answer.
        "reads_as_a_question": answer.asking,
        "ranked_from_a_shortlist": answer.shortlisted,
        // Null when every hit resolved. An agent that reports "showing 6" as
        // the answer to a request for ten is making the same mistake the
        // silent drop invited.
        "could_not_open": answer.unresolved,
        "showing": found.len(),
        "adjacent": found,
        "not_the_places_these_words_appear": "for that, call `search` — it is literal",
    }))
}

fn seforim(server: &Server, args: &Value) -> Result<Value, String> {
    let title = text_arg(args, "title")?;
    let limit = limit_of(args);
    // Compared through the shared normalizer, like every other comparison of
    // two Hebrew strings in this project (W2's sibling rule).
    let wanted = girsa_hebrew::normalize(&title);
    let mut matched: Vec<&girsa_corpus::work::Work> = server
        .shelf
        .works()
        .iter()
        .filter(|work| {
            girsa_hebrew::normalize(&work.he_title).contains(&wanted)
                || work
                    .en_title
                    .to_lowercase()
                    .contains(&title.trim().to_lowercase())
        })
        .collect();
    matched.sort_by_key(|work| work.he_title.chars().count());
    Ok(json!({
        "total": matched.len(),
        "showing": matched.len().min(limit),
        "seforim": matched.iter().take(limit).map(|work| {
            let when = server.timeline.when(&work.slug);
            json!({
                "sefer": work.slug,
                "title": work.he_title,
                "en_title": work.en_title,
                "shelf": work.categories,
                "author": work.author,
                "written": when.written(),
                "era": when.era.map(|e| e.he()),
            })
        }).collect::<Vec<Value>>(),
    }))
}

/// Your own marks (F4).
///
/// The layer an agent is standing in, read back: highlights, bookmarks and
/// their labels. Without this, `bookmark` wrote into a drawer that opened
/// from no side but the window's.
fn marks(server: &Server, args: &Value) -> Result<Value, String> {
    let limit = limit_of(args);
    let tag = args.get("tag").and_then(Value::as_str);
    let bookmarks_only = args
        .get("bookmarks")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // A place filter needs a standing, and a standing needs the sefer open —
    // the same derivation `links` asks for, for the same reason.
    let standing = match args.get("at").and_then(Value::as_str) {
        Some(text) => {
            let id: SegmentId = text
                .parse()
                .map_err(|e| format!("`at` is not a segment id: {e}"))?;
            Some(
                server
                    .shelf
                    .read(id.work())
                    .map_err(|e| e.to_string())?
                    .standing(&id),
            )
        }
        None => None,
    };
    let marks = server.shelf.marks();
    let mut chosen: Vec<&girsa_note::Mark> = match &standing {
        Some(at) => marks.on(at),
        None => match args.get("sefer").and_then(Value::as_str) {
            Some(slug) => marks.in_work(slug).collect(),
            None => marks.all().collect(),
        },
    };
    chosen.retain(|mark| {
        (!bookmarks_only || matches!(mark.kind, girsa_note::Kind::Bookmark))
            && tag.is_none_or(|wanted| mark.has_tag(wanted))
    });
    let total = chosen.len();
    Ok(json!({
        "total": total,
        "showing": total.min(limit),
        "marks": chosen.iter().take(limit).map(|mark| {
            let mut row = json!({
                "id": mark.id.as_str(),
                "kind": mark.kind,
                "at": named(server, &mark.at),
                "label": mark.label,
                "colour": mark.colour,
                "tags": mark.tags,
                "by": mark.who,
                "when": mark.when,
            });
            if mark.kind == girsa_note::Kind::Highlight {
                row["was"] = json!(mark.was);
            }
            row
        }).collect::<Vec<Value>>(),
        "note": "a highlight's `was` is the words it was made on; offsets live in your layer, not on this wire",
    }))
}

/// Your own chaburah folders (F4). Read-only here: reordering somebody's
/// shiur is not an agent's call to make over the wire.
fn folders(server: &Server, args: &Value) -> Result<Value, String> {
    let holding = match args.get("holding").and_then(Value::as_str) {
        Some(text) => {
            let id: SegmentId = text
                .parse()
                .map_err(|e| format!("`holding` is not a segment id: {e}"))?;
            Some(
                server
                    .shelf
                    .read(id.work())
                    .map_err(|e| e.to_string())?
                    .standing(&id),
            )
        }
        None => None,
    };
    let collections = server.shelf.collections();
    let list: Vec<&girsa_note::Collection> = match &holding {
        Some(at) => collections.holding(at),
        None => collections.all().collect(),
    };
    Ok(json!({
        "total": list.len(),
        "folders": list.iter().map(|folder| {
            let members: Vec<Value> = folder.members.iter().map(|member| match member {
                girsa_note::Member::Place(id) => {
                    let mut row = named(server, id);
                    row["member"] = json!("place");
                    row
                }
                girsa_note::Member::Work(slug) => json!({ "member": "work", "slug": slug }),
                girsa_note::Member::Query(name) => json!({ "member": "query", "name": name }),
            }).collect();
            json!({
                "name": folder.name,
                "title": folder.title,
                "tags": folder.tags,
                "members": members,
            })
        }).collect::<Vec<Value>>(),
    }))
}

/// Your own saved searches (F4).
fn queries(server: &Server, _args: &Value) -> Result<Value, String> {
    let queries = server.shelf.queries();
    let all: Vec<&girsa_note::SavedQuery> = queries.all().collect();
    Ok(json!({
        "total": all.len(),
        "queries": all.iter().map(|query| json!({
            "name": query.name,
            "typed": query.typed,
            "said": query.said(),
            "chips": query.chips,
            "only": query.only,
            "without": query.without,
            "tags": query.tags,
        })).collect::<Vec<Value>>(),
    }))
}

/// Which of your own documents cite a place (F4).
///
/// The desk's answer, not a second one: notes in the drawer and documents in
/// the registry, asked through [`girsa_desk::citing::who_cites`] exactly as
/// the window's panel asks.
fn who_cites(server: &Server, args: &Value) -> Result<Value, String> {
    let id = id_arg(args, "id")?;
    let place = id
        .to_string()
        .parse::<girsa_ref::Ref>()
        .map_err(|e| format!("{id} does not read as a ref: {e}"))?;
    let (documents, _) = girsa_desk::Documents::open(&server.personal);
    let citing = girsa_desk::citing::who_cites(&server.personal, &documents, &place);
    Ok(json!({
        "at": named(server, &id),
        "total": citing.len(),
        "cited_by": citing.iter().map(|one| json!({
            "name": one.name,
            "refs": one.refs,
            "path": one.path,
            "cached_only": one.away,
        })).collect::<Vec<Value>>(),
        "note": "your own writing only — for the corpus's link graph, call `links`",
    }))
}

/// Who a write is by, over the wire.
///
/// `girsa_app::who` is the same name the window and the command-line tools
/// stamp, read from the environment or the machine — so a note written by an
/// agent on your behalf is attributed to **you**, which is true: you ran the
/// server and pointed it at your layer. What it is not is anonymous, and a
/// personal layer with unattributed records in it is one nobody can audit.
fn writer(args: &Value) -> String {
    args.get("who")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map_or_else(girsa_app::who, str::to_string)
}

/// Write a note on a place (spec.md §11).
fn write_note(server: &mut Server, args: &Value) -> Result<Value, String> {
    let at = id_arg(args, "at")?;
    let text = text_arg(args, "text")?;
    let title = args.get("title").and_then(Value::as_str);
    let who = writer(args);
    let note = girsa_app::note_here(server.shelf_mut(), &at, title, &text, &who)
        .map_err(|e| e.to_string())?;
    let name = note.name().to_string();
    let slug = note.slug.clone();

    // Tags after the write, through the same door the window uses, so a tag
    // arriving over the wire is folded and compared exactly as a typed one is
    // (W2) rather than being pushed onto the vector raw.
    let tags: Vec<String> = args
        .get("tags")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if !tags.is_empty() {
        let mut held = server
            .shelf()
            .notes()
            .get(&name)
            .cloned()
            .ok_or("the note was written and cannot be read back")?;
        for tag in &tags {
            held.tag(tag);
        }
        server
            .shelf_mut()
            .write_note(held)
            .map_err(|e| e.to_string())?;
    }
    Ok(json!({
        "wrote": name,
        "sefer": slug,
        "at": named(server, &at),
        "tags": tags,
        "into": "personal",
        "note": "a note is a sefer on your own shelf, joined to that line by a typed edge — \
                 it comes back from `links` on that line, not from a list of its own",
    }))
}

/// Draw a link between two places (spec.md §8.3).
fn draw_link(server: &mut Server, args: &Value) -> Result<Value, String> {
    let from = id_arg(args, "from")?;
    let to = id_arg(args, "to")?;
    let asked = text_arg(args, "type")?;
    // The same parse the window's repair panel uses, so a type named over the
    // wire and a type picked in a dropdown cannot be two different vocabularies.
    let edge_type = girsa_link::touching::type_named(&asked).ok_or_else(|| {
        let names: Vec<&str> = girsa_link::EdgeType::ALL
            .iter()
            .map(|kind| kind.as_str())
            .collect();
        format!(
            "no such link type: {asked} — it is one of {}",
            names.join(", ")
        )
    })?;
    let who = writer(args);
    server
        .shelf_mut()
        .repairs_mut()
        .draw(
            girsa_link::Anchor::point(from.clone()),
            girsa_link::Anchor::point(to.clone()),
            edge_type,
            &who,
        )
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "drew": {"from": named(server, &from), "to": named(server, &to), "type": edge_type.as_str()},
        "into": "personal",
        "note": "an override in your own layer — the shipped graph is unchanged, and this edge \
                 is marked as yours wherever it is shown",
    }))
}

/// Record a correction to a stretch of a segment (spec.md §7).
fn correct(server: &mut Server, args: &Value) -> Result<Value, String> {
    let at = id_arg(args, "at")?;
    let says = text_arg(args, "says")?;
    let from_char = usize::try_from(
        args.get("from_char")
            .and_then(Value::as_u64)
            .ok_or("`from_char` is required")?,
    )
    .map_err(|_| "`from_char` is too large".to_string())?;
    let to_char = usize::try_from(
        args.get("to_char")
            .and_then(Value::as_u64)
            .ok_or("`to_char` is required")?,
    )
    .map_err(|_| "`to_char` is too large".to_string())?;
    if from_char >= to_char {
        return Err("`from_char` must be before `to_char`".to_string());
    }
    // `ocr` unless told otherwise, and the two are not interchangeable: one
    // says the scanner got a letter wrong and the other says this edition reads
    // differently. A default of `girsa` would let a program quietly file
    // emendations to the text of Shas.
    let kind = match args.get("kind").and_then(Value::as_str).unwrap_or("ocr") {
        "ocr" => girsa_fix::Kind::Ocr,
        "girsa" => girsa_fix::Kind::Girsa,
        other => return Err(format!("no such kind: {other} — it is `ocr` or `girsa`")),
    };
    let who = writer(args);

    let sefer = server.shelf().read(at.work()).map_err(|e| e.to_string())?;
    // `Pointing::Full`: every letter and every nekuda, and the markup out. That
    // is what `read` hands back as `counting`, and the two have to be the same
    // string or the offsets name letters the caller never saw.
    //
    // They were not. `read` returned the stored text with its markup and this
    // counted into the drawn text without it, and the description above claimed
    // they were one string — so `from_char: 0, to_char: 4` on Berakhot 2a:1#1
    // read as `<big` to the caller and landed on `מֵאֵ` here, successfully.
    // `read` now returns the string this counts into, built by this same
    // `Shown`.
    let mut patch = girsa_app::correction(
        &sefer,
        &at,
        from_char..to_char,
        &says,
        kind,
        &who,
        girsa_app::session::Pointing::Full,
    )
    .map_err(|e| e.to_string())?;
    if let Some(note) = args.get("note").and_then(Value::as_str) {
        patch = patch.with_note(note);
    }
    if let Some(source) = args.get("source").and_then(Value::as_str) {
        patch = patch.from_source(source);
    }
    let was = patch.was.clone();
    server.shelf_mut().fix(patch).map_err(|e| e.to_string())?;
    Ok(json!({
        "corrected": named(server, &at),
        "was": was,
        "says": says,
        "kind": kind.as_str(),
        "into": "personal",
        "note": "an overlay — the corpus text on disk is untouched, so re-importing keeps this",
    }))
}

/// Throw a note away (spec.md §11).
///
/// # The caller has to have read it
///
/// `saying` is the note's own words, and it is checked before anything is
/// removed. That is the one shape this end can enforce: a window asks *are you
/// sure* by **showing** you the thing, and this end has no screen — so the
/// question it asks instead is *what does it say*, which cannot be answered
/// without having looked.
///
/// A mismatch does not print what it does say. Handing the answer back would
/// turn the check into a two-call formality that an agent passes without ever
/// reading the note, which is precisely the thing being guarded against; the
/// refusal names the tool that will show it.
fn forget_note(server: &mut Server, args: &Value) -> Result<Value, String> {
    let name = text_arg(args, "note")?;
    let saying = text_arg(args, "saying")?;
    let held = server
        .shelf()
        .notes()
        .get(&name)
        .ok_or_else(|| format!("no note called {name}"))?;
    let title = held.title.clone();
    let slug = held.slug.clone();
    let on: Vec<Value> = held.on.clone().iter().map(|at| named(server, at)).collect();
    if held.words().trim() != saying.trim() {
        return Err(format!(
            "that is not what {name} says — read it first, and pass its words as `saying`"
        ));
    }
    let gone = server
        .shelf_mut()
        .forget_note(&name)
        .map_err(|e| e.to_string())?;
    if !gone {
        return Err(format!("{name} could not be thrown away"));
    }
    Ok(json!({
        "forgot": name,
        "sefer": slug,
        "title": title,
        "was_about": on,
        "into": "personal",
        "note": "the file, the sefer and its edges are gone — the corpus is untouched, \
                 and nothing here can put it back",
    }))
}

/// Take back a link you drew (spec.md §8.3).
///
/// Only a link **you drew**. An edge the corpus shipped is refused rather than
/// removed, and that is not a politeness: rejecting a shipped edge is a
/// different statement with its own record, and a tool that deleted one under
/// the name *undraw* would be the second way to change the graph that
/// `girsa_mcp`'s own header says will not exist.
fn undraw_link(server: &mut Server, args: &Value) -> Result<Value, String> {
    let from = id_arg(args, "from")?;
    let to = id_arg(args, "to")?;
    let asked = text_arg(args, "type")?;
    let (from_anchor, to_anchor) = (
        girsa_link::Anchor::point(from.clone()),
        girsa_link::Anchor::point(to.clone()),
    );
    // What is actually drawn between them, so the type can be checked against
    // the graph rather than against what the caller remembers.
    let drawn = server
        .shelf()
        .repairs()
        .drawn()
        .find(|link| link.edge.from == from_anchor && link.edge.to == to_anchor)
        .ok_or_else(|| {
            format!("you have not drawn a link from {from} to {to} — `links` shows what is there")
        })?;
    let held = drawn.edge.edge_type;
    if !girsa_link::touching::type_named(&asked).is_some_and(|named| named == held) {
        // Which type it *is* stays unsaid, for the reason `forget_note` gives.
        return Err(format!(
            "the link from {from} to {to} is not a {asked} — `links` says what it is"
        ));
    }
    let undrawn = server
        .shelf_mut()
        .repairs_mut()
        .undraw(&from_anchor, &to_anchor)
        .map_err(|e| e.to_string())?;
    if !undrawn {
        return Err(format!(
            "the link from {from} to {to} could not be taken back"
        ));
    }
    Ok(json!({
        "undrew": {"from": named(server, &from), "to": named(server, &to), "type": held.as_str()},
        "into": "personal",
        "note": "your layer only — the shipped graph never had this edge and still does not, \
                 and anything else you have said about this pair stands",
    }))
}

/// Take a correction back (spec.md §7).
///
/// Restores nothing, because nothing was edited: a correction is an overlay,
/// so removing it stops it being applied and the base text on disk is what it
/// always was. The answer says so, because *undo* over an overlay reads like a
/// revert and is not one.
fn uncorrect(server: &mut Server, args: &Value) -> Result<Value, String> {
    let at = id_arg(args, "at")?;
    let says = text_arg(args, "says")?;
    // Through the opened sefer, because a correction is held against the work
    // it is in — the shelf knows which works there are and `Open` knows what
    // your layer said about their lines.
    let open = server
        .shelf()
        .read(at.work())
        .map_err(|e| format!("cannot open {}: {e}", at.work()))?;
    let corrected = open
        .correction(&at)
        .ok_or_else(|| format!("nothing in your layer corrects {at}"))?;
    // Applied and noted alike — a variant that is recorded and not shown is
    // still one you can take back.
    let found = corrected
        .applied
        .iter()
        .chain(corrected.noted.iter())
        .find(|a| a.now.trim() == says.trim())
        .ok_or_else(|| {
            format!("no correction on {at} says that — `read` returns them in `corrections`")
        })?;
    let (id, was, now, kind) = (
        found.id.clone(),
        found.was.clone(),
        found.now.clone(),
        found.kind.as_str(),
    );
    let removed = server.shelf_mut().unfix(&id).map_err(|e| e.to_string())?;
    if !removed {
        return Err(format!("the correction on {at} could not be taken back"));
    }
    Ok(json!({
        "uncorrected": named(server, &at),
        "no_longer_says": now,
        "reads_again": was,
        "kind": kind,
        "into": "personal",
        "note": "an overlay was removed, not an edit reverted — the text on disk never changed",
    }))
}

/// Bookmark a place (F4).
fn bookmark(server: &mut Server, args: &Value) -> Result<Value, String> {
    let at = id_arg(args, "at")?;
    let who = writer(args);
    let mut mark = girsa_note::Mark::bookmark(at.clone(), who);
    if let Some(label) = args.get("label").and_then(Value::as_str) {
        mark = mark.called(label);
    }
    if let Some(colour) = args.get("colour").and_then(Value::as_str) {
        mark = mark.coloured(colour);
    }
    let tags: Vec<String> = args
        .get("tag")
        .and_then(Value::as_str)
        .map(|tag| vec![tag.to_string()])
        .unwrap_or_default();
    if !tags.is_empty() {
        mark = mark.tagged(tags);
    }
    let id = mark.id.as_str().to_string();
    server
        .shelf_mut()
        .marks_mut()
        .add(mark)
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "marked": id,
        "at": named(server, &at),
        "into": "personal",
        "note": "`marks` reads them back; `forget_mark` takes one back by this id",
    }))
}

/// Take a mark back (F4).
///
/// The id is the proof of having looked: `marks` is the only way to name one,
/// so a caller removing by id has read the list it came from.
fn forget_mark(server: &mut Server, args: &Value) -> Result<Value, String> {
    let id = text_arg(args, "id")?;
    // The type carries no constructor over the wire, and should not have to:
    // find the mark your layer actually holds, then take *that* back.
    let found = server
        .shelf
        .marks()
        .all()
        .find(|mark| mark.id.as_str() == id.trim())
        .map(|mark| mark.id.clone())
        .ok_or_else(|| {
            format!("no mark in your layer has id {id} — `marks` lists the ids that exist")
        })?;
    let gone = server
        .shelf_mut()
        .marks_mut()
        .remove(&found)
        .map_err(|e| e.to_string())?;
    if !gone {
        return Err(format!("the mark {id} could not be taken back"));
    }
    Ok(json!({
        "forgot": id,
        "into": "personal",
    }))
}

/// Save a search (F4).
fn save_query(server: &mut Server, args: &Value) -> Result<Value, String> {
    let name = text_arg(args, "name")?;
    let typed = text_arg(args, "typed")?;
    // Through the same constructor the window's row uses, so a query saved
    // over the wire and one typed into the bar are the same object — sigils
    // and all, chips empty until something sets them.
    let query = girsa_note::SavedQuery::new(name.clone(), typed);
    let said = query.said();
    server
        .shelf_mut()
        .queries_mut()
        .save(query)
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "saved": name,
        "said": said,
        "into": "personal",
        "note": "`queries` reads them back; the window's saved-searches row holds the same objects",
    }))
}

/// Throw a saved query away (F4).
///
/// `typed` must be what the query says now, which `queries` gives you — the
/// same proof `forget_note` asks for, for the same reason: this end cannot
/// show what is about to be deleted.
fn forget_query(server: &mut Server, args: &Value) -> Result<Value, String> {
    let name = text_arg(args, "name")?;
    let says = text_arg(args, "typed")?;
    let queries = server.shelf.queries();
    let held = queries
        .get(&name)
        .ok_or_else(|| format!("no saved query named {name} — `queries` lists the names"))?;
    if held.typed.trim() != says.trim() {
        return Err(format!(
            "{name} says something else now — `queries` returns its `typed`; a mismatch is refused"
        ));
    }
    let gone = server
        .shelf_mut()
        .queries_mut()
        .remove(&name)
        .map_err(|e| e.to_string())?;
    if !gone {
        return Err(format!("{name} could not be removed"));
    }
    Ok(json!({
        "forgot": name,
        "said": says,
        "into": "personal",
    }))
}
