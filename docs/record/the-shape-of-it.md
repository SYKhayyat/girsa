# The shape of it

*[The record](../the-record.md) · [Building it, and checking it](building-and-checking.md) →*

---

```
Videos/
  Girsa/          this repository — the library app
    crates/       the model: corpus, links, the workspace
    app/          the Tauri shell — a window over the crates
  Ksav/           the writing app          github.com/SYKhayyat/ksav
  sefer-crates/   the shared contract      github.com/SYKhayyat/sefer-crates
```

Two roots at run time, and they are not the same kind of thing:

```
corpus/          the download. Rebuildable, replaceable, never yours to edit
personal/        yours: how you arranged the shelf, the seforim you added,
                 and everything you wrote — notes, marks, queries, folders
```

`girsa-import` rewrites the whole of `corpus/works/index.jsonl` on every run, so
nothing of yours is ever kept in it. The window looks for the corpus at
`GIRSA_CORPUS` and for your layer at `GIRSA_PERSONAL`, else beside the session
file in the app's data directory.

**15** crates, and **16** command-line binaries
across them. Every number in this file that is a fact about this repository is
marked like those two and re-counted by
`crates/girsa-app/tests/the_numbers_in_the_readme_are_measurements.rs` on every
push — because this file said *"a window and fifty commands"* while there were a
hundred of them, and nothing said so.

| Crate | Purpose |
|---|---|
| `girsa-personal` | The one file your own layer is written in: an append-only jsonl log with tombstones, shared by every store under `personal/` |
| `girsa-corpus` | Storage, ingest, schemas, permanent segment IDs |
| `girsa-search` | tantivy indices, the five modes, the ladder, the chips and the facets |
| `girsa-link` | The typed link graph, your repairs to it, later mining |
| `girsa-fix` | Corrections as an overlay, and the ranked OCR queue |
| `girsa-note` | Your own layer: notes as nodes, marks, tags, saved queries, chaburah folders |
| `girsa-scan` | Scans you brought: which page is which daf, and what a page cites as |
| `girsa-lane` | The semantic lane: a side-loaded BERT, the vector store, the resumable job |
| `girsa-app` | The reading workspace: the shelf, tabs and splits, and what keeps two columns together |
| `girsa-desk` | The desk: the buffer you type into, what a highlighted phrase becomes, and which of your documents cite a place |
| `girsa-nearby` | What else is near this, and what an answer could not see |
| `girsa-export` | Handing somebody a sefer with your corrections in it: a clean `.txt` or `.docx` |
| `girsa-mcp` | The library as tools an agent can call, over stdio |
| `girsa-fixture` | A synthetic shelf, built from source-shaped input through the real importer, so a test needs no corpus. Never published; a dev-dependency only |

plus `girsa-source`, `girsa-ref`, `girsa-hebrew`, `girsa-cite`, `girsa-post`
and `girsa-ksav` from `sefer-crates`, pinned by **commit** and fetched by cargo.

The last four rows are above `girsa-app` and not beside it, and the reason is
the line under each of their names. `girsa-app` is *the shelf, tabs and splits*
— and its manifest carried a BERT, three `candle` crates, a document format and
`zip`, because three files out of thirty needed them. `cargo test -p girsa-app`
built the forward pass in order to retest the taxonomy. Each of those
dependencies now stops at the edge of a crate that is *about* it, the arrow
runs one way, and the reading workspace compiles without any of them.

`app/` is the Tauri shell: a window and **110** commands, and
**nothing that decides anything**. Where a pane lands, what may
sit beside what, and what the nikud toggle takes off are all answered in
`girsa-app`, because those can be tested and a webview cannot. The window itself
is **29** TypeScript modules and one stylesheet of
**3,236** lines — no framework, three runtime dependencies,
and no fossils.

That sentence used to say *supposed to be*, and it was measured: of the
shell's **4,487** lines, about 150 — 23 commands — were
genuine pass-through, and the rest decided cache policy, sort orders,
truncation lengths, patch provenance, which fonts a Hebrew reader is offered,
what makes a directory a corpus, and what to do with a chip key it did not
recognise. Each of those is now in the crate whose subject it is, and two
checks in `the_rules_this_repository_wrote_down.rs` fail if one comes back:

| What was decided in the window | Where it lives |
| --- | --- |
| how many seforim stay in memory, and which one goes | `girsa_app::held` |
| who is writing, for the name on a patch | `girsa_personal::who` |
| how much of a thing is enough to show | `girsa_app::enough` |
| which font families are offered | `girsa_app::session::FONTS` |
| what makes a directory a corpus, and where to look | `girsa_corpus::roots` |
| what a chip key means, and what an unknown one means | `girsa_search::chips::Chips::choose` |
| what order notes and corrections come back in | `girsa_app::view` |

Three of them were bugs rather than misplacements. The cache was a **queue**: a
hit never touched the order, so the sefer you had open all morning was evicted
on its twelfth neighbour while a commentary you glanced at once outlived it.
*Who is writing* was two implementations that disagreed — the terminal read
`GIRSA_WHO` first and the window had never heard of it, so the one variable
this project offers for *call me something else* changed the name on your notes
and not the name on your corrections. And the four chip families each ended
`_ => the default`, forty lines from a `link_repair` that refused an unknown
candidate by name: a mistyped chip key came back as a search that ran, answered,
and answered a different question than the one asked.

(The line above also said *fifty* commands while there were 100 of them, which
is the other half of the same problem: nothing checked a number in this file.
Something does now.)

### A refusal carries a name, not a sentence to be pattern-matched

`app/src/trouble.ts` turned an error into a Hebrew sentence by matching
**twenty-one regular expressions against the English prose of Rust's `Display`
impls** — `/no search index/i`, `/no sefer here called/i`, `/state is
poisoned/i`. Which makes every error string in the repository load-bearing API,
and the only test asserting any of them was on the TypeScript side, against a
hand-typed copy. Reword `"there is no index here"` and both halves stay green
while the reader stops being told what to run.

Seven of the twenty-one match prose this project does not own — an `os error 2`,
a `connection refused`, a `serde_json` message. Those stay regexes, and that is
honest: matching somebody else's words is the only thing available.

The other fourteen are this codebase refusing on purpose, and they carry a name
now:

```text
no-index: there is no index here
```

`girsa_app::trouble::Code`, in front of the prose, which is still English and
still for whoever is reading a log — and no longer what decides the sentence a
reader sees. A prefix rather than a typed error because a hundred Tauri commands
return `Result<T, String>` and a typed error across all of them is a change to a
hundred signatures for one question; when the wire grows a place for structured
errors, `trouble.rs` is the one place that has to move.

`every_refusal_this_codebase_names_has_a_sentence_in_the_window` fails if a code
Rust can send has no line in the window's table.

### The wire format was described four times, and one copy could not be checked

The rows the window draws lived in the shell — 52 structs, 936 lines — and were
mirrored by hand into 59 TypeScript interfaces in `app/src/api.ts`, with
**nothing verifying that the two agreed**. The fourth copy was the sharp one:
`crates/girsa-app/examples/dev-fixtures.rs` emits the same JSON as static files
for the browser build, and it **could not import** the shell's structs, because
`girsa-app` cannot depend on `app/`. So it rebuilt every shape with
`serde_json::json!`.

It had already drifted three ways:

- `state.json` carried nine keys where the command sends fifteen — and the
  comment above it named five of the six that were missing, so the comment
  documenting the drift had itself drifted;
- `card()` was missing `scan`, under a doc comment reading *"the same fields the
  shell's command sends"*;
- and the text fixture built a **second** inline copy of a card, missing
  `source` and `scan`, emitting `"era": work.era` — the raw code — where
  `card()` seventy lines below emitted `display::era_said(code)`. Two
  hand-written copies of one shape inside one 202-line file, disagreeing about
  the value under a key they both spelled the same way.

The two shapes the example got right for free were `Branch` and `Companion` —
the only two commands whose return type was a `girsa-app` type. That is the
argument, made by the example's own behaviour.

The rows now live in `crates/girsa-app/src/view.rs`, so the fixture imports the
real types and rustc holds that half; `app/test/wire.test.mjs` holds the other,
comparing every `#[derive(Serialize)]` row against the interface that declares
it. Two structs stayed in the shell and are the visible exceptions rather than
the invisible rule: `FoundPage` carries `girsa_search` types and `Copied`
carries a clipboard handle. `HitRow` moved and its constructor did not — the
shape of a result row is `girsa-app`'s, and filling it from a
`girsa_search::index::Hit` is the shell's, because the hit is.

The gate found three more the day it was written: `api.ts` declared neither
`cite` nor `pairing` although Rust has sent both since the desk existed;
`CiteStyle` was typed as `string`, hiding that Rust *sends* `hebrew_full` and
*takes* `hebrew-full`; and `export interface Landing` was declared **twice** in
one file, which TypeScript merges rather than refuses, so `Landing` was silently
the union of a citation landing and a `girsa://` link.

**Cloning Girsa alone builds.** That is new, and it was not true until
2026-08-09: the shared crates were `path = "../sefer-crates/crates/…"` — a
sibling of *this checkout's root* — so `git clone girsa && cargo build` failed
at `cargo metadata`, before a compiler ran, with `os error 3` naming a directory
the reader had never heard of. Nothing in this file said so, and every CI job
carried a second `actions/checkout` to fake the desk layout, which is what a
load-bearing workaround looks like.

They are pinned by `git` + `rev` now (see the note above them in `Cargo.toml`),
with the exact version kept beside the rev so a commit whose manifests say
something else is a resolution error rather than a surprise.

For the days you are working on both halves at once, `.cargo/config.toml`
carries the `paths` override to write, and
`sefer-crates/tools/check-dependents.sh` installs exactly that override itself —
so a change over there is checked against *this tree* rather than against the
last commit this repository pinned.

---

*[The record](../the-record.md) · [Building it, and checking it](building-and-checking.md) →*
