# Architecture

How Girsa fits together, for somebody about to change it.

This is the *map*. It says where things are and why the seams are where they are.
It does not say why each individual decision was made — that is
[`the-record.md`](the-record.md), which is the same information with the defect
that caused each decision attached.

Read [`../spec.md`](../spec.md) §2 and §3 first if you have not. Section 1 below
is §3 restated, and everything else is downstream of it.

---

## 1 · The decision everything rests on

**Line numbers are not addresses.**

Otzaria — one of the two corpora — addresses every link in the library as *file +
line number*. That works exactly until somebody fixes a typo in a way that splits
or joins a line. Then every link below it in that file points at the wrong text.
Not a broken link that errors. **The wrong text, silently.**

Girsa's two headline features are *a library with links* and *you can fix typos*,
and they collide precisely here.

So: **every segment gets a permanent ID at import.** Assigned once, never
re-derived from file position.

- Corrections, notes, links, highlights, and every citation sitting in a Ksav
  document anchor to that ID.
- Editing text cannot move an anchor.
- Splitting a segment mints a **child** ID rather than shifting seventeen
  thousand others.
- When upstream re-segments a text, a redirect table absorbs it.

Each record in `segments.jsonl` **carries its own id**, so the file can be
sorted, reordered, appended to or diffed and every anchor still names the same
words. A file whose ids were its line numbers would have reintroduced the exact
defect the project exists to leave behind.

**If you find yourself storing a line number as a durable reference, stop.** That
is the one mistake here that cannot be fixed later, and it is why the segment-ID
scheme is in the *do not decide this alone* table in
[`../CONTRIBUTING.md`](../CONTRIBUTING.md).

Tests that hold this: `crates/girsa-corpus/tests/anchors_survive_editing.rs`,
`a_reimport_keeps_every_name.rs`, and
`crates/girsa-app/tests/an_anchor_survives_a_split_at_import.rs`.

---

## 2 · The two roots

At run time there are two directories, and they are not the same kind of thing.

```
corpus/     the download. Rebuildable, replaceable, never yours to edit
personal/   yours: how you arranged the shelf, the seforim you added,
            and everything you wrote — notes, marks, queries, folders
```

`girsa-import` **rewrites the whole of `corpus/works/index.jsonl` on every run.**
That is the contract, and it is what makes the corpus safely replaceable: nothing
of yours can be in it, so re-downloading loses nothing.

Everything you cannot re-download is under `personal/`. This is the one directory
with no backup upstream, which is why it never migrates format and why its store
is the way it is (§4).

Where they are found is answered in one place — `girsa_corpus::roots` — reading
`GIRSA_CORPUS` and `GIRSA_PERSONAL`, then falling back beside the session file in
the app's data directory. That question used to be answered separately in the
window, which is how a corpus could be found by one half of the application and
not the other.

---

## 3 · The crates, and which way the arrows point

Arrows point from a crate to what it depends on — read it bottom-up.

```
   app/src            the window — TypeScript, no framework
      ↓
   app/src-tauri      the shell — commands, and nothing that decides anything
      ↓
   girsa-mcp ────────────────────┐   the library, as tools a program can call
      ↓                          │
   girsa-desk  girsa-nearby  girsa-export  girsa-search
      └────────────┬───────────────┘          ↓
                   ↓                          │
              girsa-app                       │   the reading workspace
                   ↓                          ↓
   girsa-link  girsa-fix  girsa-note  girsa-scan  girsa-lane
                   └──────────┬──────────────┘
                              ↓
                       girsa-corpus        ingest, schemas, permanent ids
                              ↓
                    girsa-personal         the append-only log
                              ↓
                      girsa-plain          a command line, and one sentence
```

Two things in that picture are worth pausing on.

**`girsa-app` does not depend on `girsa-search`.** They are siblings, and
`girsa-mcp` is the crate that happens to use both. The reading workspace is *the
shelf, tabs and splits*; searching is a different subject, and keeping them peers
means a change to the relaxation ladder cannot rebuild the pane layout.

**The crates above `girsa-app` are there on purpose.** Its manifest used to carry
a BERT, three `candle` crates, a document format and `zip`, because three files
out of thirty needed them — so `cargo test -p girsa-app` built a neural network
forward pass in order to retest the taxonomy. The semantic lane is now
`girsa-lane` with `girsa-nearby` on top of it, the document format is
`girsa-export`, and the reading workspace compiles without any of them.

`girsa-plain` at the bottom exists for the same reason, from the other end.
`argv` and `said` used to live in `girsa-corpus` — not because they are about a
corpus (between them they name a `PathBuf`, an `ExitCode` and a `Display` derive)
but because `girsa-corpus` was the crate everything could already `use`. A crate
everything depends on accumulates whatever has nowhere else to go, until somebody
makes a place on purpose.

**The rule to take from this:** before you add a dependency to a crate, ask
whether the crate is *about* the thing you are pulling in. If it is not, the code
that needs it probably belongs one layer out.

`crates/girsa-app/tests/manifests.rs` and
`the_reading_workspace_does_not_take_a_dependency_it_reads_nothing_from` hold
this.

---

## 4 · Your own layer is an append-only log

Everything you make — corrections, marks, saved questions, folders, judgments
about links — lands in a jsonl file under `personal/`, through one store:
`girsa_personal::Log`.

A record is a line. A later line for the same key wins. A line saying a key is
gone is a tombstone. **Writing one record appends one line.** Opening replays the
file, and rewrites it only when it has grown past twice what it holds.

Six crates once grew six copies of this store, and all six had the same defect:
they held records in a map and wrote the **whole map** on every mutation. That
reads well and it is quadratic — correcting *n* typos costs *n* full
serializations of an *n*-line file:

```text
18120 segments, no corrections yet:        75 ms
18120 segments, 1000 corrections already: 217 ms
```

The interaction budget is three seconds, and the OCR queue reaches **28,124
entries** on the real corpus with a pitch of *here are thousands of ranked
candidates, go through them*. A thousand was the last size at which it passed.

The format did not change when this landed, which matters here more than
anywhere else in the tree: a file with no repeated keys and no tombstones is its
own compaction, so a file written by the old stores replays to exactly what it
used to mean. **`personal/` is the one directory a reader cannot re-download**,
so it does not get migrations.

It also stays greppable and diffable, which is why it was jsonl to begin with.

`no_store_of_your_own_layer_rewrites_its_file_to_record_one_line` is the check.

---

## 5 · The window, and the wire between it and Rust

### The shell decides nothing

`app/src-tauri` is a bridge: a window and its Tauri commands, and **nothing that
decides anything.** Where a pane lands, what may sit beside what, and what the
nikud toggle takes off are all answered in `girsa-app`, because those can be
tested and a webview cannot.

This is a rule with tests behind it, not a description. The shell used to decide
these, and here is where each one went:

| What was decided in the window | Where it lives now |
|---|---|
| how many seforim stay in memory, and which one goes | `girsa_app::held` |
| who is writing, for the name on a patch | `girsa_personal::who` |
| how much of a thing is enough to show | `girsa_app::enough` |
| which font families are offered | `girsa_app::session::FONTS` |
| what makes a directory a corpus, and where to look | `girsa_corpus::roots` |
| what a chip key means, and what an unknown one means | `girsa_search::chips` |
| what order notes and corrections come back in | `girsa_app::view` |

Three of those were bugs rather than misplacements, and they are the argument for
the whole rule:

- **The sefer cache was a queue, not an LRU.** A hit never touched the order, so
  the sefer you had open all morning was evicted on its twelfth neighbour while a
  commentary you glanced at once outlived it.
- **"Who is writing" was two implementations that disagreed.** The terminal read
  `GIRSA_WHO` and the window had never heard of it — so the one variable this
  project offers for *call me something else* changed the name on your notes and
  not the name on your corrections.
- **Every chip family ended `_ => the default`.** A mistyped chip key came back
  as a search that ran, answered, and answered a different question than the one
  asked. Forty lines away, `link_repair` refused an unknown candidate by name.

`the_shell_decides_nothing_it_says_it_decides_nothing_about` and
`no_chip_family_is_read_with_a_silent_fallback` fail if one comes back.

### One wire format, in one place

The rows the window draws live in `crates/girsa-app/src/view.rs`. They were once
described **four** times: `girsa-app`'s own model types, fifty-two more structs
in the shell, fifty-nine hand-mirrored TypeScript interfaces, and
`examples/dev-fixtures.rs` — which could not import the shell's structs (a crate
cannot depend on the app) so it rebuilt every shape with `serde_json::json!`.

Nothing verified that any two of the four agreed, and by the time anyone looked,
three had drifted: a fixture carrying nine keys where the command sent fifteen,
under a comment naming five of the six missing ones; a card missing a field under
a doc comment reading *"the same fields the shell's command sends"*; and one
202-line file containing two hand-written copies of the same shape that
disagreed about the value under a key they both spelled identically.

Now: the rows are `girsa-app` types, so the fixture imports the real thing and
rustc holds that half. [`../app/test/wire.test.mjs`](../app/test/wire.test.mjs)
holds the other, comparing every `#[derive(Serialize)]` row against the interface
in [`../app/src/api.ts`](../app/src/api.ts) that declares it. Two structs stay in
the shell as visible exceptions rather than an invisible rule, because they carry
types that cannot cross.

**If you add or change a field the window reads, change it in `view.rs`.** The
TypeScript side is checked against it, not the other way round.

### A refusal carries a name

Errors crossing the wire are prefixed with a code:

```text
no-index: there is no index here
```

`girsa_app::trouble::Code` in front, prose behind it — the prose is still
English, still for whoever is reading a log, and no longer what decides the
sentence a reader sees.

Before this, `app/src/trouble.ts` turned an error into a Hebrew sentence by
matching **twenty-one regular expressions against the English `Display` output of
Rust errors**. That makes every error string in the repository load-bearing API:
reword one, and both halves stay green while the reader stops being told what to
run. Seven of the twenty-one match prose this project does not own — an `os error
2`, a `connection refused` — and those stay regexes, because matching somebody
else's words is the only thing available.

It is a prefix rather than a typed error because a hundred Tauri commands return
`Result<T, String>`, and a typed error across all of them is a change to a
hundred signatures for one question. When the wire grows a place for structured
errors, `trouble.rs` is the one file that has to move.

`every_refusal_this_codebase_names_has_a_sentence_in_the_window` fails if a code
Rust can send has no line in the window's table.

### Why the shell is not in the default build

`app/src-tauri` cannot compile until the frontend has been built into
`app/dist`, and `cargo build` at the root has to stay green on a machine with no
node toolchain. So it is a workspace member that is **not a default member**.

It used to be `exclude`d, which satisfied the same constraint and also cut the
crate off from `[workspace.lints]`, `[workspace.dependencies]` and the lockfile —
so the lines that own every byte of the interop were the one place a new
workspace lint could not reach, eleven path dependencies were spelled twice, and
a cold CI cache rebuilt tantivy and candle a second time.

The practical consequence for you: **`cargo test` at the root does not compile
the shell.** The gate has separate steps for it. That is why the gate matters
rather than running `cargo test` and calling it done.

---

## 6 · The link graph

The graph is Sefaria's and Otzaria's, merged onto permanent segment ids: about
4.2 million edges, 81.9% of the rows offered. Where the rest went is written down
in full in [`the-record.md`](the-record.md), because a rate without its remainder
is not a measurement.

Three things about it that will bite you if you do not know them:

**Orientation is not given.** Sefaria's export does not say which of its two
citation columns is the commentary. Half the commentary in the corpus was
therefore stored backwards, and a daf offered two aggadic works out of forty.
`girsa_link::orient` undoes it. The test fixture writes eight of its thirty-two
rows base-first **on purpose**, exactly as the real export does, so neutering
`Orienting::apply` fails a test on synthetic data with no download.

**The caches are separate from the graph, and ordered.** `girsa-link-types` reads
the graph from the *segment's* side and has to run before the search index if the
link facet is to have anything to count. `girsa-companions` walks the whole graph
once to answer *which seforim are worth opening beside which*.

**A cold cache is a third state, not a zero.** *Nothing links here* and *I have
not been told* are different answers and the code says so — `Touching` has three
variants precisely so that no caller can read a missing cache as an empty result.
That pattern is worth copying.

---

## 7 · Search

Five modes, and the flags on `girsa-index find` are the same thing as the chips
under the query bar in the window:

| Mode | What it is |
|---|---|
| Torat Emet | **the default.** Hebrew morphology — `קדש` finds `קידוש` |
| `--contains` | literal substring |
| `--phrase` | one word after another |
| `--near N` | within N words, either order |
| `--regex` | whole words, no hand-holding |

Plus citation lookup (`@ברכות ב.`) and the instruments — gematria, roshei teivot,
dilug. Scope is set by `--in`, `--shelf` and `--not-shelf`.

**Nothing widens a query silently.** That is a product rule with its own line in
the *do not decide this alone* table, and the relaxation ladder
(`girsa_search::ladder`) is the sanctioned exception: it is visible, it is
ordered, and the answer says which rung it came from.

Which is the general shape here: `girsa_nearby` exists so that *what this answer
could not see* is a sentence the system can say, joined the same way on all three
surfaces (`girsa_plain::said`). An answer that quietly omits something is the
failure mode this whole area is designed against.

The index is a **rebuildable cache**. `build` throws the old one away rather than
patching it; `rebuild` refuses a directory that is not already an index, because
that argument order has been transposed here once and it cost the corpus.

---

## 8 · Finding your way around

| To change… | Look in | Checked by |
|---|---|---|
| how text is imported, or ids assigned | `crates/girsa-corpus/src/import/`, `segment.rs` | `spec_counts.rs`, `anchors_survive_editing.rs` |
| how searching works | `crates/girsa-search/src/` | `chips.rs`, `ladder.rs` tests |
| the link graph, or repairs to it | `crates/girsa-link/src/` | `the_links_on_this_line.rs` |
| what the shelf looks like | `crates/girsa-app/src/shelf.rs`, `arrangement.rs` | `the_shelf_is_yours.rs`, `every_sefer_has_a_shelf.rs` |
| how panes split and stay in step | `crates/girsa-app/src/beside.rs`, `workspace.rs` | `two_columns_stay_together.rs` |
| what the window is sent | `crates/girsa-app/src/view.rs` | `wire.test.mjs` |
| notes, marks, folders | `crates/girsa-note/`, `crates/girsa-desk/` | `a_note_is_a_node.rs` |
| corrections and the OCR queue | `crates/girsa-fix/` | `a_correction_is_not_an_edit.rs` |
| scans and page→daf | `crates/girsa-scan/`, `girsa-app/src/scanning.rs` | `the_scan_is_the_daf.rs` |
| keyboard shortcuts | `crates/girsa-app/src/keys.rs` | `tools/check-card.sh` regenerates `shortcuts.md` |
| what a refusal says | `crates/girsa-app/src/trouble.rs` + `app/src/trouble.ts` | `every_refusal_…_has_a_sentence_in_the_window` |

The test names are the documentation here. `crates/*/tests/` is a directory of
sentences — `a_sefer_never_vanishes_quietly`, `the_queue_beats_the_editor`,
`never_a_silent_gap`, `provenance_reaches_the_pen` — and reading the list is a
fast way to learn what this system promises.

Every binary has a terminal twin of a window feature, on purpose: a window is a
bad place to find out that a mapping is wrong. [`tools.md`](tools.md) lists all
sixteen.

---

## 9 · What is checked, and what is not

Checked, and failing the gate if broken:

- The invariants stated in doc comments, by source scan —
  `crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs`. Blunt on
  purpose. Each check says in its own comment what it can and cannot catch.
- Every command the documentation names exists, and every command that exists is
  named by the documentation. Both directions.
- Every relative link in a document resolves inside this repository.
- Every marked number in `README.md` against the tree that measures it.
- The wire format, Rust against TypeScript.
- One generated file against its generator, twice: the shortcut card, and the
  packet Ksav's own test asserts against.

**Not checked:**

- **Prose.** Nothing here would have caught *"a window and fifty commands"* while
  there were a hundred. That is why numbers get markers.
- **Pixels**, almost. `npm run eyes` is the only check that has ever looked at
  one, and it exists because of two defects no string search can express — a
  mefaresh's comment drawn at `opacity: 0`, and a pane title measured at 0px in a
  three-way split.
- **Whether any of this is good to learn with.** Nobody has written a real sefer
  in it. Three separate audits call that the most important line in any of them.

---

## Where to next

| | |
|---|---|
| Make a change | [`your-first-change.md`](your-first-change.md) |
| The rules, the gate, how to send it | [`../CONTRIBUTING.md`](../CONTRIBUTING.md) |
| Why any single decision was made | [`the-record.md`](the-record.md) |
| What the system is supposed to do | [`../spec.md`](../spec.md) |
| The work orders and what each is asserted on | [`../BUILDER.md`](../BUILDER.md) |
