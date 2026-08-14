# Answering a program

*← [A document of yours, with its shape](a-document-of-yours.md) · [The record](../the-record.md)*

---

### The same engine, refusals included

`spec.md` §12 and W28 ask for **MCP on both ends**. Girsa's end is `girsa-mcp`:
ten tools over stdio — `search`, `read`, `resolve`, `where_from`, `links`,
`trace`, `path`, `fork`, `adjacent`, `seforim`. (`adjacent` — the semantic lane —
was added and this sentence said nine for a while, which is D-1's third row.)

Every one of them is a thin call onto the engine the window calls. That
thinness is the whole design. A second query path written for a caller that
cannot complain is exactly where §9's guarantees would quietly stop holding,
and it would stop holding where nobody is watching:

- **Torat Emet is the default here too.** `search` runs literally unless the
  caller names a mode, and the answer says which mode ran. A program cannot get
  a widened result by accident any more than a person can.
- **A zero offers the ladder, priced, and applies nothing.** The rungs come back
  as `offered_and_not_applied` with their counts. Asking for one is a second
  call.
- **A citation with two plausible targets comes back as two.** `resolve` returns
  `settled: false` and every candidate the shelf could not rule out. There is no
  `first()` in that file.
- **Every answer says what it cut** — `not_shown`, `not_followed`,
  `incoming_half_unknown`. A list that silently stopped at ten reads to an agent,
  and to whatever the agent is writing, as *these are all of them*.

The `initialize` reply says the first two out loud, because an agent that has to
discover a refusal by hitting it will work around it instead:

```
Two things about this engine are deliberate and will not be worked around:

1. Search is literal by default … nothing is applied until you ask for a rung by name.
2. A citation with more than one plausible target comes back as a list of
   candidates, never as a pick. Choose one, or ask the person you are working for.
```

### stdio, and nothing bound

A child process reading a pipe. No port, no socket, nothing dialled — §14 makes
offline the product, and W16's loopback transport for Ksav is token-gated
precisely because it *is* a socket. Here the program that can talk to Girsa is
the program that started it, which needs no gate.

The envelope is a hundred lines of `serde_json` rather than a protocol crate:
the alternative is carrying somebody else's licence and release cadence for the
sake of a message wrapper (T7). A version the server has not heard of is **not**
echoed back — that would be claiming compatibility with a revision this code was
written before.

```sh
cargo run --release -p girsa-mcp -- corpus personal index
```

### What the MCP end does not do

- **Ksav's end is Ksav's repo.** "Both ends" is two servers and this is one of
  them.
- **Read only.** Nothing here writes a note, draws a link or records a
  correction. Those are all *your layer*, and a tool that let an agent edit it
  without a person in the loop is a different decision from exposing the library.
- **No resources, no prompts, no sampling.** Tools only.
- **A search is capped at 50 rows** whatever `limit` says, and says so.

### And one guardrail, bought expensively

`girsa-index build` takes the index directory **first** and the corpus roots
after it. `SearchIndex::rebuild` deletes the directory it is handed before
creating an index there. Those two facts met, during this work order, in a
transposed command — `build corpus index` — and the corpus was deleted: 3.4 GB
of fetched export, 7,189 imported works and a 4.1-million-edge graph, with the
exit code of a missing file.

Everything was rebuildable, which is the design working (`spec.md` §4.1: the
files are the truth, everything else is a cache — and here even the files turned
out to be a cache of the export). Nothing authored was lost; the personal layer
lives in a different tree and was untouched. It still cost the whole of Tier 2
again.

So `rebuild` now refuses any directory that is not already an index or empty.
The check is a `stat` for tantivy's `meta.json` or Girsa's own stamp; the cost
of not having it was measured.

One number moved in the rebuild and it is worth saying rather than papering
over: the graph came back as **4,182,337 edges** against W8's 4,182,344.
Sefaria's export is a live bucket and a day passed between the two fetches;
7 rows in 5.1 million changed upstream. Every other measurement in this file
came back identical — 7,189 works, 5,375 of them datable, 4,812 with an era,
5,294 with a year. The W8 tables above are left at the numbers that run produced,
because rewriting a measurement to match a later one is how a measured number
turns into a documented one.

---

*← [A document of yours, with its shape](a-document-of-yours.md) · [The record](../the-record.md)*
