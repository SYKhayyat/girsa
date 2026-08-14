//! What a program may ask the library, and what it gets back.
//!
//! Nine tools, and each is a thin call onto the engine the window uses. The
//! thinness is the design: a tool that reimplemented a search would be the
//! place where spec.md §9's guarantees quietly stopped applying.
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
#[must_use]
pub fn catalogue() -> Value {
    json!([
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
    girsa:bavli/berakhot/2a:1#1 and survive corrections and re-segmentation.",
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
    as every candidate, never as a pick.",
            "inputSchema": {
                "type": "object",
                "properties": {"citation": {"type": "string"}},
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
                    "width": {"type": "integer", "minimum": 2, "maximum": 40}
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
        }
    ])
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
        "adjacent" => adjacent(server, &args),
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
    match server.bar.ask(
        &query,
        &chips,
        paging,
        &girsa_ref::resolve::Context::default(),
    ) {
        Answer::Segments {
            results,
            offers,
            note,
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
            row["asked_for"] = json!(segment.id == id);
            row
        })
        .collect();
    Ok(json!({"segments": segments}))
}

fn resolve(server: &Server, args: &Value) -> Result<Value, String> {
    let citation = text_arg(args, "citation")?;
    // Through the bar's citation mode, which is the path the query bar takes —
    // rather than the resolver underneath it, which would be a second way of
    // reading a mareh makom and a second place for it to disagree.
    let chips = Chips {
        mode: Mode::Citation,
        ..Chips::default()
    };
    let landing = match server.bar.ask(
        &citation,
        &chips,
        Paging::first(),
        &girsa_ref::resolve::Context::default(),
    ) {
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
    let mut graph = Graph::new(&server.root, &server.timeline, &repairs);
    let walked = chain::trace(&mut graph, &id, direction, limits);

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
    let mut graph = Graph::new(&server.root, &server.timeline, &repairs);
    let found = chain::path(&mut graph, &from, &to, limits_of(args));
    Ok(match found {
        Found::Path(links) => {
            let asserted = links.iter().filter(|l| l.edge_type.is_asserted()).count();
            json!({
                "found": true,
                "links": links.iter().map(|link| {
                    let mut row = named(server, &link.at.from);
                    row["type"] = json!(link.edge_type.as_str());
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
    let mut graph = Graph::new(&server.root, &server.timeline, &repairs);
    let (forks, left_out) = chain::forks(&mut graph, &id, limits_of(args));
    Ok(json!({
        "at": named(server, &id),
        "note": "nothing here says these disagree — the corpus has no `disputes` edge anywhere in it",
        "pairs": forks.iter().take(limit_of(args)).map(|pair| json!({
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
        "ranked_from_a_shortlist": answer.shortlisted,
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
