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

### Writing, and the permission around it

The server answered nine questions and wrote nothing. That is a defensible place
to stop and it is not where `spec.md` §12 stops — *MCP on both ends* — and a
library a program can read but not add to is one an agent learns beside rather
than with.

Three tools, and they are the three this section used to name as missing: a
note, a link, a correction. Each is a thin call onto the same door the window
uses, for the reason every read tool here is: one that reimplemented a write
would be the place the layer's own rules quietly stopped applying. A note
arrives as a **sefer on your shelf** with its `work.json` and its segments,
because §11 says a note is a sefer and one written by a program is not a
second-class one. A link is an **override**, so the shipped graph is unchanged
and yours is marked as yours wherever it is drawn. A correction is an
**overlay**, so the corpus text on disk does not move and re-importing keeps it.

The permission is the part worth arguing, and it has four pieces:

- **Off unless asked.** `--writable`, and nothing else sets it. The corpus is a
  download and your own layer is not: nothing in it can be recovered by
  re-fetching, so the case where nobody has thought about whether an agent
  should be writing there is the case that has to be safe.
- **Absent from the catalogue when it is off**, rather than listed and refused.
  A tool list is what a program *plans against*, and one advertising a door it
  cannot open gets an agent halfway through a plan before the refusal lands.
- **And refused at the door anyway.** A client that remembered the tools from a
  writable session will call them, and the refusal names `--writable` rather
  than saying *no such tool*, which would be a lie about why.
- **`readOnlyHint: false` on each**, so a client that asks its user before a
  write knows which calls to ask about. That is a claim about the tool and not a
  promise about the client — this server does not know what the caller does with
  it, which is exactly why the flag exists.

`ocr` is the default kind for a correction and `girsa` has to be asked for. They
are the same machinery and two very different claims — *the scanner got a letter
wrong* against *this edition reads differently* — and a default of `girsa` would
let a program file emendations to the text of Shas without anybody choosing to.

One repair fell out of writing the tests. The `draw_link` description listed
`explains`, `sources` and `parallels`, none of which this graph has ever had:
the real nine are `comments-on, quotes, paraphrases, codifies, disputes, emends,
parallel-to, translates, references`. A description that names types the parser
will refuse costs a program a round trip to discover the truth, so the list is
read off `EdgeType::ALL` now rather than typed, and it reaches the schema as an
`enum` as well as as prose.

### What the MCP end does not do

- **Ksav's end is Ksav's repo.** "Both ends" is two servers and this is one of
  them.
- **No resources, no prompts, no sampling.** Tools only.
- **A search is capped at 50 rows** whatever `limit` says, and says so.

### Deleting, and what a screenless end can ask instead

This section used to say a write could not be undone over the wire, and the
reason it gave was sound: *deleting is a decision, and this end has no way to
show you what you are about to delete first.* What was wrong with it was the
conclusion. A window asks *are you sure* by **showing** you the thing; the
absence of a screen removes that one way of asking, not the requirement behind
it.

So each of the three writes has an undo, and each takes an argument that cannot
be filled in without having read what is about to go:

| | takes | which comes from |
|---|---|---|
| `forget_note` | `saying` — the note's own words | `read`, or `search` |
| `undraw_link` | `type` — the type the link has **now** | `links` |
| `uncorrect` | `says` — the words the correction reads | `read`, in `corrections` |

Three rules make that a check rather than a gesture:

1. **A mismatch is refused and the thing is left standing.** No partial delete
   and no confirmation flow to abandon halfway.
2. **The refusal does not print the right answer.** Handing back the words it
   just refused would turn this into a two-call formality an agent passes
   without ever reading anything — which is the entire failure being guarded
   against. It names the tool that will show them.
3. **Twice is not a silent success.** *Deleted* and *was not there* are
   different answers, and a caller that cannot tell them apart cannot tell
   whether its first call worked.

`undraw_link` carries one more wall: it takes back a link **you drew** and
refuses an edge the corpus shipped. Rejecting a shipped edge is a different
statement with its own record, and a tool that deleted one under the name
*undraw* would be the second way of changing the graph this server exists not to
have. Underneath, `Repairs::undraw` removes only the `Drawn` records for a pair
rather than everything said about it — a key is the pair of anchors, so a link
you drew between two places the corpus **also** joins shares its key with the
shipped edge, and the wide `undo` would have silently taken back your rejection
of the shipped one too.

`uncorrect` restores nothing, and says so in its answer. A correction is an
overlay; removing it stops the overlay being applied, and the text on disk is
what it always was.

### `read` was handing back a string nobody could count into

Found by starting the server and speaking to it, which is the only way this
could have been found: every test was green and the description was the thing
that was wrong.

`correct` takes character offsets. Its description said they were *into the
segment's text as the corpus stores it, which `read` returns* — and they were
not. `read` returned Berakhot 2a:1#1 as the corpus stores it, markup and all:

```
<big><strong>מֵאֵימָתַי</strong></big> קוֹרִין אֶת שְׁמַע בָּעֲרָבִין? …
```

while `correct` counted into the same words with the markup taken out, which is
what the window's `Shown` draws. So `from_char: 0, to_char: 4` reads as `<big`
in the string the caller was given, and landed on `מֵאֵ` — thirteen characters
further along, because `<big><strong>` is thirteen characters. It came back
**successful**, with nothing on the wire to tell it apart from a call that
corrected what it named.

The repair is not to move the offsets. Counting into the stored string would
have a program filing corrections against tags nobody can see, and the window
and the tool would then be two coordinate systems where there had been one.
`read` returns the string the offsets are into, as `counting`, built by the same
`Shown` the correction path uses — not by stripping tags a second time here,
which would be a second opinion about what markup is and would be wrong the
first time the two disagreed. `text` is still there, unchanged.

`the_offsets_a_correction_takes_are_the_ones_read_hands_back` asserts the
agreement rather than either string: it reads a line, takes the first four
characters of what it was handed, corrects `0..4`, and requires that those are
the characters that moved.

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
