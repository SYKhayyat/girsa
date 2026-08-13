# גִּרְסָא · Girsa

**A Torah library that assumes you are going to write something.**

Girsa (גִּרְסָא, "the text as received") is the page. **Ksav** (כְּתָב, "writing")
is the pen. The pairing is the idea.

**If you are here to use it, not to build it: [`docs/start-here.md`](docs/start-here.md).**
Every command in this repository, and what each is for: [`docs/tools.md`](docs/tools.md).
Five minutes, end to end, and it is the whole idea.

- **[`docs/`](docs/)** — for a reader: getting started, *coming from Otzar
  HaChochma* and *from Bar Ilan*, and the keyboard card (generated from the
  source, and diffed against it on every push by
  [`tools/check-card.sh`](tools/check-card.sh) — a generated file with nothing
  checking that anybody re-generated it is a hand-maintained file with a
  disclaimer).
- **[`spec.md`](spec.md)** — what Girsa is.
- **[`BUILDER.md`](BUILDER.md)** — what to do on day one: work orders, binding
  rules, the verified traps in the data, and what may not be decided alone.

Read `spec.md` §2 (ground truth), §3 (the landmine) and §16 (settled decisions)
first. They are what shape everything else.

## Where things are

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

**15**<!--=crates--> crates, and **16**<!--=bins--> command-line binaries
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

`app/` is the Tauri shell: a window and **110**<!--=commands--> commands, and
**nothing that decides anything**. Where a pane lands, what may
sit beside what, and what the nikud toggle takes off are all answered in
`girsa-app`, because those can be tested and a webview cannot. The window itself
is **29**<!--=window-modules--> TypeScript modules and one stylesheet of
**3,236**<!--=styles-lines--> lines — no framework, three runtime dependencies,
and no fossils.

That sentence used to say *supposed to be*, and it was measured: of the
shell's **4,455**<!--=shell-lines--> lines, about 150 — 23 commands — were
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

## Build

```sh
node tools/verify.mjs               # the gate: nine steps, three directories
node tools/verify.mjs --list        # what they are, without running them
node tools/verify.mjs --from 4      # pick up where a failure stopped it
```

It was this list, written out here and again in BUILDER.md rule 4, and it grew
from four commands to nine. A gate that lives in prose is a gate whose last two
steps stop being run — see `docs/the-second-sitting.md`, lesson 1. The runner is
the list now, and a test fails if rule 4 starts repeating it.

### Getting it

An installer is attached to every `v*` tag: the `bundle` job in
`.github/workflows/ci.yml` runs the real build on Windows and uploads the NSIS
`.exe` and the MSI to the release. `workflow_dispatch` runs the same job without
a tag and leaves the installers as artifacts. It is deliberately not on every
push — a release build of tantivy and candle on a Windows runner is tens of
minutes, and what has to be true on every push is that the code compiles, which
the other two jobs already say.

**The installer carries the application and the tools. It does not carry the
library.** Girsa is 11 GB of Torah and that is not in a 7 MB download, so a
fresh install has a window and no seforim. The road, which the first screen also
states:

| | | |
|---|---|---|
| 1 | Sefaria, ~2.2 GB | `girsa-fetch corpus\sefaria` |
| 2 | Otzaria | **you download this yourself** — nothing here fetches it |
| 3 | Build the shelf | `girsa-import corpus <otzaria>` |
| 4 | Search, ~3.6 GB | `girsa-index build index corpus personal` |

Those three tools are `girsa-tools-windows.zip` on the same release page. They
are **not** bundled into the installer, and that is deliberate rather than
lazy: Tauri validates `bundle.resources` when the shell *compiles*, so naming
three release binaries there breaks `cargo check` for anybody who has not built
them first — CI's own shell job included. A second download couples nothing.

Step 2 is manual and step 3 refuses without it — `girsa-import` needs an
`אוצריא/` directory and says so. If you already have a corpus, point the window
at it instead: with none it opens on a screen that says all of this and offers a
folder picker (`docs/the-second-sitting.md`, findings 19 and 26).

### Building the window

```sh
cd app && npx tauri build              # with an installer
cd app && npx tauri build --no-bundle  # just the executable
```

**Not `cargo build --release -p girsa-shell`, and it will now refuse.** That
command produced a binary which embeds no frontend and navigates to the Vite dev
server, so it opened a Chromium *this site can't be reached* page in a window
titled `גִּרְסָא · Girsa` on any machine not running `npm run dev`. It was the
only build this repository had ever produced, and it survived because the wrong
command **succeeded** — it printed `Finished`, wrote an executable, and the
executable looked like the product until you unplugged the thing it was leaning
on. `app/src-tauri/build.rs` panics on it now, naming the command that works.
Debug builds are untouched, because `cargo check`, `cargo clippy` and
`tauri dev` all want exactly that binary. `GIRSA_DEV_RELEASE=1` builds it anyway.
`docs/the-second-sitting.md` finding 16 is the whole story.

### Every command reads its command line the same way

All sixteen binaries take the same shape, and `--help` on any of them prints
what it reads:

```sh
girsa-shelf [corpus] [personal] [command]      # corpus and personal default
girsa-index find <index> <root> [how …] <query …>
```

An option that takes a value takes it **either way round** — `--depth 5` and
`--depth=5`. A wrong invocation exits **2**; a run that failed exits 1; asking
for `--help` exits 0.

That was five conventions and no shared line of code, and three of them cost
something rather than being untidy:

- **`girsa-chain` advertised a syntax it rejected.** Its usage said `[--depth
  N]`; its parser was `strip_prefix("--depth")?.strip_prefix('=')?`, so only
  `--depth=N` worked. Typing what the usage said left a bare `N` among the
  segment ids, and what came back was an error message about segment ids.
- **`girsa-notes` made every option take a value.** `split_flags` had `--x`
  unconditionally swallow the token after it, so a switch ate a positional and
  `--title=x` was stored under the key `title=x` while still eating the next
  word.
- **`girsa-link-orient` turned a typo into a path.** Its parser was `other =>
  root = PathBuf::from(other)`, so `--replce` silently became the corpus root.
  The run then read a directory of that name, found no links, and reported
  that it had finished.

Each is one shape: a parser that could not tell a switch from a value option,
because nothing had told it which was which. `girsa_corpus::argv::Argv::of`
is told — it takes both lists by name — and that is the whole of the fix.
Four binaries also used to exit 1 for a mistyped verb, through the same path
as *the shelf will not open*, so a script could not tell a typo from a broken
corpus.

### The tests do not need the corpus, and for a long time they pretended to

`cargo test` above is 816 tests and no download. Forty-three of them used to
open like this:

```rust
let root = corpus_or_skip!();   // 3.4 GB, not committed, absent in CI
```

`cargo test` captures stderr on a passing test, so what CI printed was

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s
```

for the acceptance tests of W7, W8, W9, W10, W27, W28, W32, W43, W44 and the
whole MCP surface. Eight green ticks, nothing asserted. `spec_counts.rs` — the
file that would have caught §3's permanent ids being renumbered by every
re-import — had not run since the day it was written.

`tools/check-ksav-fixture.sh:41` refuses this by name, twelve files away:

> *Not a skip. A check that passes because it could not find what it checks is
> the exact failure this script exists to end.*

The rule was written down correctly and forty-three tests in the same repository
broke it. The response that had already worked once is
`girsa-app/examples/fixture-packet.rs`: the Ksav fixture rotted because
regenerating it needed a corpus no gate has, *"so the corpus is the thing that
had to go."* That argument generalises, and `girsa-fixture` is it applied to the
other forty-three.

**It writes `merged.json`, not `segments.jsonl`.** A fixture that writes what the
importer *outputs* asserts itself back at itself: a test checking the walker put
daf 2a first would be checking that the fixture typed `2a`. So it writes at the
layer the download is written at — Sefaria `merged.json` and schemas, an Otzaria
`.txt` with headings, a `links0.csv` with the misspelled `Conection Type` column
intact — and the real importer, resolver and orienter read it. Twenty-eight
works, a link graph, both caches and a tantivy index, in about two seconds.

That distinction is load-bearing for one test. `the_meforshim_are_on_the_daf`
exists because Sefaria's export does not say which of its two citation columns is
the commentary, so half the commentary in the corpus was stored backwards and a
daf offered two aggadic works out of forty. The fixture writes eight of its
thirty-two rows base-first **on purpose**, exactly as the export does, and
`girsa_link::orient` has to undo them. Neuter `Orienting::apply` and the test
fails with five mefarshim unreachable — on synthetic data, with no download.

**What genuinely needs the download is `#[ignore]`d, not skipped.** *Orach Chayim
is 697 simanim of 4,171 se'ifim* is a fact about a Sefaria release and no fixture
can stand in for it. Ten such checks remain, and they read as `10 ignored` rather
than as ten green ticks:

```sh
cargo test -- --ignored      # on a machine that has run girsa-import
```

The line between the two halves is the one worth keeping: **the assertion was
never that Orach Chayim has 4,171 se'ifim, it was that the walker produces
exactly as many segments as the schema promised.** The first needs the corpus.
The second is a property of this code, is true of any shelf, and now runs
everywhere.

### The design lives in prose, and prose is not checkable

That is the diagnosis this repository was given, and it is the one finding every
other finding is downstream of: **98,488 insertions against 2,334 deletions in
59 commits over four days.** Each pass solved its problem correctly, in
isolation, and wrote down eloquently why its solution was right. Not one of them
went back to notice that six earlier passes had solved the same problem — so the
invariants exist, beautifully argued, next to callers that break them:

> `store.rs` — *"The importer calls this."* Three callers, all tests.
> `since.rs` — *"Shared, so a search panel that finds an index and one that does
> not cannot be two answers to one question."* Forty lines away, a second
> `find_index` with a different accept predicate.
> `beside.rs` — *"Built once per pair of open panes."* Once per **scroll event**,
> reading both works' shards.

Writing it down was mistaken for enforcing it. So
`crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs` is
**24**<!--=rules--> checks that read this repository's own source and fail when
a rule stated in a doc comment stops being true — a slug worked out twice, a
query prepared twice, a chip family read with a silent fallback, Ksav markup
composed outside the desk, a refusal Rust can send that the window has no
sentence for.

It is a grep, and a grep is a blunt instrument; every check in there says in its
own comment what it can and cannot catch. That is the trade. A rule nothing
checks is a rule that has already drifted at least once in this repository, and
a blunt check that fires is worth more than an elegant argument that does not.

The shell is a workspace member that is not built by default — it cannot compile
until the frontend has been built into `app/dist`, and the four commands above
have to stay quick without a node toolchain anywhere near them. That is what
`default-members` in the root manifest says. It used to say `exclude`, which
satisfied the same constraint and also cut the crate off from
`[workspace.lints]`, `[workspace.dependencies]` and the lockfile — so the 5,018
lines that own every byte of the interop were the one place a new workspace lint
could not reach:

```sh
npm --prefix app install
npm --prefix app run build          # tsc --noEmit && vite build
cd app/src-tauri && cargo build     # and `npm --prefix app run tauri dev` to run it
```

The shelf can also be walked without a window, which is how W10 is checked:

```sh
cargo run -p girsa-app --bin girsa-shelf -- corpus personal
cargo run -p girsa-app --bin girsa-shelf -- corpus personal add ~/חבורה.txt
cargo run -p girsa-app --bin girsa-shelf -- corpus personal move bavli/berakhot שלי
cargo run -p girsa-app --bin girsa-shelf -- corpus personal reset
```

The index is built and probed the same way — and it is a **rebuildable cache**,
so `build` throws the old one away rather than patching it. **The index
directory comes first and the corpus roots after it**; `rebuild` refuses a
directory that is not already an index, because that argument order has been
transposed here once and it cost the corpus:

```sh
cargo run --release -p girsa-link  --bin girsa-link-types -- corpus personal
cargo run --release -p girsa-search --bin girsa-index -- build index corpus personal
cargo run --release -p girsa-search --bin girsa-index -- stamp index
cargo run --release -p girsa-search --bin girsa-index -- find  index corpus יתגבר כארי
```

The transmission chain is four commands, and the library answers a program over
stdio:

```sh
cargo run --release -p girsa-app --bin girsa-chain -- corpus personal \
    back girsa:mishnah-berurah/58:1#1496 --depth=2
cargo run --release -p girsa-app --bin girsa-chain -- corpus personal \
    forward girsa:bavli/berakhot/2a:1#1
cargo run --release -p girsa-app --bin girsa-chain -- corpus personal \
    path girsa:bavli/berakhot/2a:1#1 girsa:mishnah-berurah/58:1#1496
cargo run --release -p girsa-app --bin girsa-chain -- corpus personal \
    fork girsa:bavli/berakhot/2a:1#1 --width=25

cargo run --release -p girsa-mcp -- corpus personal index
```

A scan is a sefer with pages instead of lines, and the once-per-sefer chore that
makes one citable can be done without a window too:

```sh
cargo run -p girsa-app --bin girsa-daf -- corpus personal add ~/ברכות.pdf
cargo run -p girsa-app --bin girsa-daf -- corpus personal map user/ברכות amud 5=ב. --of bavli/berakhot
cargo run -p girsa-app --bin girsa-daf -- corpus personal cite user/ברכות 47
cargo run -p girsa-app --bin girsa-daf -- corpus personal page user/ברכות "כג."
```

`5=ב.` says page 5 of the file is daf ב, amud alef, and the count runs on from
there. `43=-` says *from here these are not pages of the sefer* — the plates.

Your own layer is a terminal away too, and the second command below is the whole
of W27's claim: what you wrote comes back **in the list of links on the line**,
not in a list of its own.

```sh
cargo run -p girsa-desk --bin girsa-notes -- corpus personal \
    write mishnah-berakhot 1:1 "וצריך עיון מה שכתב הרמב\"ם כאן" --title מאימתי --tag ברכות
cargo run -p girsa-desk --bin girsa-notes -- corpus personal on mishnah-berakhot 1:1
cargo run -p girsa-desk --bin girsa-notes -- corpus personal after "girsa:note/מאימתי/2#2" "ובאמת"
cargo run -p girsa-desk --bin girsa-notes -- corpus personal mark mishnah-berakhot 1:1 0 6
cargo run -p girsa-desk --bin girsa-notes -- corpus personal folder thursday "חבורה יום ה" mishnah-berakhot 1:1
cargo run -p girsa-desk --bin girsa-notes -- corpus personal export /tmp/my-layer
```

In the window it is **Ctrl+N** to write one where you are standing, **Ctrl+M**
for the שלי drawer, **Ctrl+Shift+H** to highlight what is selected and **Ctrl+D**
to mark the place.

`girsa-link-types` reads the graph from the **segment's** side and has to run
before the index if the link facet is to have anything to count — see below. It
is a cache like the index, and an index built without it says so rather than
showing an empty column.

`find` searches in Torat Emet, the literal mode, and the chips of spec.md §9.5
are flags. Nothing else is ever applied:

```sh
girsa-index find index corpus --contains קדש          # המקדש · ויקדשהו
girsa-index find index corpus --letters  קדש          # קידוש too
girsa-index find index corpus --phrase   יתגבר כארי   # one after the other
girsa-index find index corpus --near 5   יתגבר כארי   # within five words, either order
```

The other four modes, and the scope chip the facets set:

```sh
girsa-index find index corpus --regex "מאימת."             # whole words, no hand-holding
girsa-index find index corpus "@ברכות ב."                  # a mareh makom — @ is the sigil
girsa-index find index corpus --instrument gematria 611    # every word that comes to it
girsa-index find index corpus --instrument rashei --in bavli/berakhot מקאש
girsa-index find index corpus --instrument dilug --skips 45-50 --in genesis תורה
girsa-index find index corpus --shelf תלמוד --not-shelf חסידות יתגבר כארי
```

In the window it is **Ctrl+F**, and the flags above are the chips under the
query bar.

## Status

**Tier 0 through Tier 8 are done — the corpus is on the shelf, the graph is on
top of it, there is a window, all five ways of searching it, the Ksav loop in
both directions, corrections as an overlay that never touches the text, a link
graph you can argue with, and scans that are citable and readable. Tier 9 is
nearly done: what you write is a sefer on your own shelf joined to the sugya by
the same kind of edge as Rashi; the transmission chain runs forward from a
Gemara to how it became halacha and back from a ruling to where it came from,
along the axis of *when the seforim were written* rather than which way the
corpus happened to store an edge; the whole library answers a program over MCP
with the same refusals it gives a person; and the semantic lane is in, off until
you turn it on, over a model you side-load and a corpus you choose. **The spec is
built.** All four verify commands green in all three repositories.

The work orders behind that, and what each one is asserted on, are in
[`BUILDER.md`](BUILDER.md) under *What holds, per work order* — twenty rows of
`W`-numbers, which are a builder's index and were living in a reader's document.
Everything in this file below here is the reasoning, and it does not need them.

The segments file is the load-bearing part and it is worth one line: each record
**carries its own id**, so the file can be sorted, reordered, appended to or
diffed and every anchor still names the same words. A file whose ids were its
line numbers would have quietly reintroduced the defect the whole project exists
to leave.

Where a link did **not** land, in full — because a rate without its remainder is
not a measurement:

| | rows | |
|---|---|---|
| became an edge | 4,182,344 | 81.9% |
| the sefer is not on the shelf | 594,660 | 11.6% — Sefaria catalogues it and has no Hebrew text for it |
| the address is not in the sefer | 323,817 | 6.3% |
| the citation resolved to nothing | 6,309 | 0.12% |
| an Otzaria line that is not a segment | 1,763 | a blank line, or past the end of the file |
| still ambiguous | **0** | and the queue for them is written down anyway |

Those six lines add up to 5,108,893 exactly. They have to: a row that is not in
one of them is a row nobody counted.

### The ambiguous ones, and why there are none left

There were 5,520, dropped rather than picked — a rate without a remainder, and
worse, a question thrown away. All of them turn out to be **one word**:

```
Meilah          bavli/meilah      addressed 2a:1, 3b:4 — dafim
                mishnah-meilah    addressed 1:1, 3:2  — perek and mishnah
```

A masechta of Gemara and a masechta of Mishnah with the same name, and 5,532
link endpoints that say `Meilah` and nothing else. The resolver is right to
refuse: `או"ח` means two seforim and so does this.

But the **address settles it**, and reading the address is not guessing. `Meilah
9b:3` is a place in the Bavli and is not a place in the Mishnah — the two are
addressed in different units, so almost every citation names exactly one of
them. 5,391 endpoints resolved that way; the rest name a place neither has, and
are now counted as a missing address rather than as a question.

The rule, stated so it can be argued with: **a candidate is eliminated only when
the shelf can refute it** — the work is here and the address is not in it.
A candidate whose work is *not* on the shelf is never eliminated, because
nothing here knows what is inside a sefer it does not have, and one of those
surviving keeps the whole thing a choice. Refuting needs evidence; an absent
sefer is not evidence about its contents. It also inherits the address lookup's
limits: where the lookup cannot find a real address, this reads it as a
refutation, which is why the 5,391 is **reported next to the import rate rather
than folded into it**.

And what nothing settles is no longer only counted. `corpus/links/unsettled.jsonl`
gets one line per citation, with every candidate and how often it came up — the
queue W23's repair UI reads. Today it is empty, which is the right outcome and
not a reason to delete the file: the next corpus update will not be.

Two more silent picks, found by re-running the import rather than by reading it:

- **The importer appended.** A run is many flushes, so each shard was opened in
  append mode — and so a *second* run added its edges to the first one's. Twice
  the graph, every commentary showing twice, no error. A shard is now replaced
  the first time a run touches it and appended to after that, which is what "a
  command someone else can run" has to mean.
- **A filename that names two seforim kept the first.** T4 resolves an Otzaria
  link target by filename, and `TitleIndex` held one work per key and let the
  rest fall out — so a collision sent every link in that file into whichever
  sefer the work index happened to list first. It now returns all of them and
  the caller declines to choose. On today's corpus this changed **no rows**:
  there are no collisions. It is fixed because the next import is not promised
  the same.

### What keeps two columns together

W9's acceptance is *scrolling the Gemara moves the Rashi column to the matching
ref*, and the whole of it is one question asked over and over: **given a segment
of one sefer, which segments of the other one sit against it?** There are three
answers and the second is the one that matters.

```
At(ids)     there, and exactly where
NoPlace     these two are related, and this line has nothing beside it
Unrelated   nothing joins these two seforim; the column does not move
```

Rashi does not comment on every line. A column that slid to the *nearest*
comment would show a reader Rashi on a different line — with the header still
naming the line they are on, and nothing anywhere saying it had moved. That is
rule 6 in the one place a reader would never think to check, so `NoPlace` exists
and the column stays put.

Two seforim follow each other only when something in the corpus says they are
related, and **neither thing is a resemblance**:

- **the corpus declares it.** Sefaria's schema for *Rashi on Berakhot* carries
  `base_text_titles: [Berakhot]`, and 5,150 works on this shelf say something
  like it about themselves. Once it is declared the addresses line up by
  construction — `Rashi on Berakhot 2a:1:3` is the third comment on
  `Berakhot 2a:1`, the base text's address with a level added — and reading that
  off is reading, not guessing;
- **or W8 imported an edge** between two of their segments.

Anything else and the panes are left alone, even though half the corpus is
addressed `1:1` and would line up beautifully. Guessing here is cheap to do and
invisible when it is wrong.

That declaration was not on the shelf before this order: `work.json` recorded a
title, categories, author and era, and not the sefer a commentary is *on*.
`girsa-import --metadata-only` re-reads the schemas and rewrites every
`work.json` without touching the five million segments — because a shelf that
has to be re-imported for one new field is a shelf nobody will ever add a field
to again.

### Two things the window found that the tests had not

- **A segment id serialized two ways.** `SegmentId` derived its `Serialize`, so
  it went to the window as `{"work":…,"path":…,"ordinal":…}` while every id
  already on the page was the string `girsa:bavli/berakhot/2a:1#1`. Nothing
  errored; the Rashi column simply never moved, because nothing could match the
  two shapes. It is now written and read as the text it travels as, everywhere,
  and the hand-rolled adapter that did this for one struct is gone.
- **The corpus's text is not plain text.** Berakhot alone carries 43,890 `</i>`
  and 747 `<b>`, and shown raw the first line of Shas reads
  `<big><strong>מאימתי</strong></big> קורין את שמע`. Stripping the tags is the
  other easy answer and it costs the dibur hamatchil, which is how you see where
  one Rashi ends and the next begins. So a segment is split into **runs** — text
  and how it is set — and the window builds elements from them. Corpus text is
  never put into the page as markup.

### The shelf: one taxonomy over two corpora

The corpus does not have a taxonomy. It has two. Sefaria files an acharon on the
Gemara under `Talmud/Bavli/Acharonim on Talmud`, in English; Otzaria files one
under `תלמוד בבלי/אחרונים`, in Hebrew. Both are right about their own download
and **neither is a shelf** — side by side they make a reader know which of two
corpora his sefer came from, which is the one thing the union was built to stop
mattering. So there is one shipped taxonomy, in Hebrew, and both vocabularies
are mapped onto it by three rules:

- **a prefix table** takes the first category, sometimes the first two, onto a
  top shelf: `Talmud/Bavli` and `תלמוד בבלי` both become `תלמוד/בבלי`;
- **`X on Y` loses its `on Y`** where `Y` names the shelf it is already under —
  `Acharonim on Talmud` is *the acharonim*, said twice, and the second saying is
  the whole of what kept it off Otzaria's `אחרונים`;
- **a term table** translates what is left, and **anything not in it is carried
  through exactly as the corpus wrote it.**

That last rule is why the shelf has `חסידות/Early Works` and
`תוספתא/Lieberman Edition` on it. `Early Works` there means the first
generations of chasidus, and `ראשונים` would file the Maggid of Mezritch with
the Rishonim; a category nobody has a Hebrew name for is shown in the corpus's
words rather than in a guess at them, and since any shelf can be renamed with a
double-click, a bad default costs one drag rather than a wrong label forever.

`cargo run -p girsa-app --bin girsa-shelf -- corpus personal` prints the whole
of it and the line that matters is the last one:

```
 תלמוד                           2141
   בבלי                            1624
   ירושלמי                          517
 …
 אחר                                2

15 shelves · 7189 seforim counted of 7189 on the shelf
```

**7,189 counted of 7,189.** A sefer on no shelf is a sefer that is on the shelf
and cannot be browsed to, and nothing anywhere would have said so — so the sum
is asserted, against the real corpus, in a test and in the tool.

`תלמוד/בבלי/אחרונים` holds **717** seforim, from both corpora, which is the
merge working. And `אחר` holds exactly **2**: `הודעה חשובה` and
`עריכת ספר באוצריא` — Otzaria ships its own about-box and a notice as works, and
W7 imported them as seforim because at import they are two more `.txt` files.
They are not deleted. They are on a shelf a reader can see, which is what `אחר`
is for.

`spec.md` §5 names seven shelves — *Tanach / Shas / Halacha / Machshava /
Chassidus / Responsa / yours*. The corpus does not fit in seven: משנה, תוספתא,
מדרש, מוסר, קבלה, תפילה and בית שני are each hundreds of seforim that would
otherwise be filed under something they are not. Sixteen ship, and the last two
are `שלי` and `אחר`.

### The arrangement is a file of yours

*The shipped taxonomy is a default, not a fact* (§5), and the whole of what
makes that true is `personal/shelf.json`. Move a sefer, move a shelf, rename
one, pin one to the front, make one: every edit writes that file and **nothing
writes to the corpus** — the same rule as corrections (§7.1) and link
judgments (§8.3), for the same reason, and a test fingerprints every byte under
`corpus/` before and after a pile of edits to keep it true.

Two things it is keyed to, and neither is a position:

- a **work** by its slug, so that `girsa-import` rewriting all 7,189 catalogue
  records leaves your filing where you put it;
- a **shelf** by the key the taxonomy derived for it — `תלמוד/בבלי` — which it
  **keeps wherever it is dragged to.** A key that moved with the shelf would
  break every other edit that named it. Titles are display, keys are identity,
  and the two are allowed to disagree: that is what renaming a shelf means.

An edit naming a sefer the shelf does not have is **kept**, not dropped. It
costs a line of JSON and it is the difference between a shelf that survives a
corpus update and one that quietly forgets what you did to it.

Three refusals worth naming, because each of them is a way a shelf could lose a
sefer without saying so:

- **a shelf cannot be put inside itself**, or inside its own child. Refused, not
  repaired — the reader has hold of one end of it and knows what they meant.
- **a hand-edited loop does not take the seforim with it.** `shelf.json` is a
  text file and can be made to say `a` hangs under `b` and `b` under `a`;
  neither would be reachable from any root and everything on them would be gone
  from the tree. A shelf in a loop is stood at the top instead.
- **an arrangement file that will not parse is moved aside, never overwritten.**
  It is the only copy of somebody's filing; the shipped shelf is shown and the
  window says what happened and where the file went.

### Your own material

A `.txt`, `.docx` or `.pdf` dropped on the window becomes a sefer — spec.md §5's
*not an onboarding step, not a second-class attachment*. It goes through the
same door as Shas: parsed into segments, **every one given a permanent id**, and
written as the same `work.json` + `segments.jsonl` every other work is. It is on
`שלי`, it can be filed anywhere, it opens in a pane, and the picker finds it.

It is catalogued in `personal/works/index.jsonl` and **not** in the corpus's,
for one reason: the importer truncates the file it owns, so a sefer of yours
filed in it would be gone at the next corpus update with nothing to say so.

Three places it is deliberately not clever:

- **a scan has no words.** A PDF becomes one segment per page and **no text at
  all** until it is OCR'd (W26). A parser that does not know the font's encoding
  would put invented Hebrew into a sefer, permanently, under a real segment id.
  §9.7 already says what to do instead: the page is addressable and citable, and
  search says *not searchable yet* rather than quietly returning nothing.
- **a heading is one Word was told about.** `w:pStyle`, and nothing reads a line
  and decides it looks like a heading.
- **a byte the code page does not define stays visible.** A Hebrew `.txt` off a
  Windows machine is usually windows-1255 and is not a UTF-8 string at all, so
  it is decoded with the code page written out — and an undefined byte becomes
  `U+FFFD` rather than a plausible letter. The work records which encoding was
  used, because a reader looking at a mangled word deserves to see what it was
  read as.

Two seforim of yours with one name are two seforim: the second is minted a new
slug rather than landing on top of the first, whose ids are permanent and
already anchored to.

### One index, and what is deliberately not in it

Five million segments, indexed by `girsa-hebrew` wearing tantivy's tokenizer
trait. Not *"the same rules as"* the query bar — the same function. Two
implementations of what a Hebrew word is would fail the way this system fails
worst: the reader is told the sefer does not contain a line that is printed in
front of them.

```
$ girsa-index build index corpus
  works              7189
  segments           5000545
  of which headings  356638
  wordless           1241   (empty headings, and scans not yet OCR'd)
  in the index       5000545
  took               248s  (20203 segments/s)
  on disk            3.6 GB
```

`in the index` is checked against `segments` and the run exits non-zero if they
differ. An index one sefer short is indistinguishable, from a search box, from a
corpus that does not contain the passage.

Nikud comes off here and in every mode, with no toggle (spec.md §9.1) — so a
bare query finds the pointed page, and the highlight still lands on the pointed
word:

```
$ girsa-index phrase index משעה שהכהנים נכנסים לאכול בתרומתן
girsa:bavli/berakhot/2a:1#1  [text]
  <big><strong>מֵאֵימָתַי</strong></big> קוֹרִין אֶת שְׁמַע בָּעֲרָבִין? [מִשָּׁעָה] [שֶׁהַכֹּהֲנִים]
  [נִכְנָסִים] [לֶאֱכוֹל] [בִּתְרוּמָתָן]. עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה…
```

**And nothing else was done to the words.** No peeled prefixes, no expanded
abbreviations, no roots: `שבת` does not find `ובשבת`, and a test asserts it.
That is not a limitation to be outgrown, it is what makes the rest possible —
if widening were baked in at import there would be no literal index left for
Torat Emet to default to (spec.md §9.3), and §9.6's *[try other forms — 7]*
could not show the count before the click, because the widened and unwidened
result sets would be the same set. The widening is W13's, applied by a reader
who asked for it.

The index is a **rebuildable cache** and it says what rules it was built under:

```
$ cat index/girsa-cache.json
{"schema_version":1,"normalizer_version":1,"ref_scheme":"girsa"}

$ girsa-index words index מאימתי         # after editing that 1 to a 0
the index at index cannot be trusted: built under schema 1 / normalizer 0 /
refs girsa; this build wants schema 1 / normalizer 1 / refs girsa
```

That refusal is the whole reason the file exists. A stale index does not
error — it silently returns less, which looks like an answer. Rebuilding costs
four minutes; reading it anyway costs the search box's credibility.

### What you typed is what was searched for

Torat Emet is the default mode, and its promise is that one sentence. The
operators are the ones that get used in learning, and each is a thing you turn
on — never something that happens to your query while you are not looking:

| | on the whole shelf |
|---|---|
| `קדש` | **31,483** segments — the word |
| `--contains קדש` | **301,910** — `המקדש`, `ויקדשהו` |
| `--letters קדש` | **577,637** — `קידוש` as well: ק then ד then ש |
| `--phrase יתגבר כארי` | **63** — one after the other |
| `--near 5 יתגבר כארי` | **69** — within five words, in either order |

Every query carries a **plan**, and the plan is the acceptance of W12: for any
input, `plan.words` is what was typed with the nikud off, and in the plain case
`plan.patterns` is the same list again — no `.*`, no alternation, nothing that
could reach a different word. The result header prints it, so what a reader is
told they searched for is read out of the thing that was actually run:

```
$ girsa-index find index --near 5 יתגבר כארי
searched for: the words יתגבר כארי, within 5 words of each other
69 in 5000545 segments · showing 69
```

Two places it refuses rather than approximates, both for the same reason —
a partial answer here is indistinguishable from a complete one:

- **within X words, in any order** is the union over orderings, one exact query
  each. Past five words that is more orderings than is reasonable, so it says
  so and points at the in-order chip instead of quietly checking some of them.
- **`--contains` inside a phrase** expands to every word matching the pattern,
  and there is a ceiling. Past it: *"those letters match more than 16384
  different words — narrow them, or drop the proximity"*. Not the first 16,384.

Order-free proximity is worth one more line, because the obvious implementation
is wrong. Tantivy's slop is a budget that lets terms reorder at a cost, so a
single query with slop 2 matches *"two words apart in order"* **and**
*"reversed and adjacent"* — a window the reader did not ask for. Asking each
ordering separately, at exactly the distance requested, and taking the union is
the same thing said precisely.

### Five modes, and what each one promises

spec.md §9.3 names five and the promises are not the same promise, which is the
point of having five rather than a setting:

```
$ girsa-index find index corpus יתגבר כארי
searched for: the words יתגבר כארי, anywhere in a segment
79 in 5000545 segments · showing 79
```

| mode | and what it will not do |
|---|---|
| **Torat Emet** | what you typed. On a zero it **offers** the ladder and applies nothing |
| **Smart** | widens, and says what it widened, with the literal query as the undo |
| **Regex** | whole words, no hand-holding — and **nothing** offered on a zero |
| **Citation** | a mareh makom, and never a near-miss presented as a place |
| **Instruments** | gematria · rashei tevot · sofei tevot · atbash · dilug |

Regex refuses three patterns rather than running them, and each one would
otherwise return nothing for ever while looking like an honest empty result —
in the mode whose whole contract is that an empty result means the corpus does
not say it. A pattern carrying **nikud**, one carrying a **final letter**, and
one that is **anchored**:

```
$ girsa-index find index corpus --regex "^קדש$"
`^קדש$` is anchored, and a pattern here is matched against the whole of a word,
so `^` is already implied — write `קדש`
```

The third is the interesting one. `^…$` means nothing here — a pattern is
already matched against the whole of a word — and tantivy answers it with a
parser error about empty match operators. Stripping the anchors would change no
result at all, and it is still not done: it would be the engine editing a
pattern somebody wrote, in the one mode that promises it does not.

Citation has three answers and only one of them is a jump:

```
$ girsa-index find index corpus "@Meilah"
Meilah could be 2 places
  girsa:bavli/meilah      →  קׇדְשֵׁי קָדָשִׁים שֶׁשְּׁחָטָן בַּדָּרוֹם – מוֹעֲלִין בָּהֶן…
  girsa:mishnah-meilah    →  קָדְשֵׁי קָדָשִׁים שֶׁשְּׁחָטָן בַּדָּרוֹם, מוֹעֲלִים בָּהֶן…

$ girsa-index find index corpus "@ברכות צט."
ברכות צט. is not a place on this shelf
  [bavli/berakhot] is on the shelf and has no 99a — open the sefer?
```

That second line is the whole mode. `ברכות צט.` parses perfectly, resolves
perfectly, and there is no daf 99 — so it opens nothing and offers the sefer. A
near-miss here does not look like an error: it resolves, it opens a page, and it
is the wrong page, and if it is copied into a Ksav document it is wrong in a
printed sefer.

**A candidate is eliminated only when the shelf can refute it.** W8 settled that
rule for the link graph and this is the same rule in the same words: `או"ח`
naming a sefer we do not have is not refuted by our not having it, so it stays a
choice. Picking the one that happens to be downloaded would be choosing by
what is on the disk rather than by what was written.

### Two of the instruments are not index questions, and say so

Gematria and atbash are. Gematria adds up **every distinct word in the index**
once and searches for the ones that came to the number, which is a different
thing from a list somebody wrote:

```
$ girsa-index find index corpus --instrument gematria 611
searched for: words that come to 611
1407 words of the corpus: אאגרות אאתרוג אבולוציונית אבזרתא … and 1395 more
285191 in 5000545 segments
```

Notarikon and dilug are not, and they are **refused by name** rather than
approximated with something an inverted index happens to be able to do:

- a **dilug** runs through the letters of a sefer and pays no attention to where
  words or segments end;
- a **notarikon** looks like an index question and is not. `מקאש` is four
  one-letter patterns — `מ.*`, `ק.*`, `א.*`, `ש.*` — and on this corpus each of
  them matches more distinct words than a phrase query will hold, so the index
  answers it with a refusal about postings lists. True, and useless.

Both are read off the text instead, and both are bounded by **the scope chip**
rather than by a ceiling nobody chose. Over the whole shelf they say which sefer
they need; over one, they read it:

```
$ girsa-index find index corpus --instrument rashei --in bavli/berakhot מקאש
searched for: words whose first letters spell מקאש
read through 1 sefer of text, not the index
4 in 5000545 segments

girsa:bavli/berakhot/2a:1#1
  <big><strong>[מֵאֵימָתַי]</strong></big> [קוֹרִין] [אֶת] [שְׁמַע] בָּעֲרָבִין?…
```

That first line is there because of one thing the scan has to know: **a tag is
not a word.** The corpus stores it as
`<big><strong>מֵאֵימָתַי</strong></big> קוֹרִין`, so tokenized as it stands there
are two words called `strong` and `big` standing between the first word of Shas
and the second, and the notarikon a reader can plainly see is not found. Only
words written in Hebrew letters count as words here; on the page the tags are
invisible and those four words do stand together, which is what the instrument
is about.

### The chips, and the sigils that teach them

spec.md §9.5: *nobody should ever have to learn a syntax* — and *typing a sigil
flips the matching chip, so the power syntax teaches itself*. Both halves, and
the acceptance is that they are **the same search**:

| typed | the chip it flips |
|---|---|
| `"יתגבר כארי"` | one after the other |
| `*קדש*` | the word contains these letters |
| `~קדש` | these letters, in this order |
| `~5` | within 5 words of each other |
| `/מאימת./` | Regex |
| `@ברכות ב.` | Citation |
| `=613` | Instruments — gematria |

A sigil is taken **off** what is searched for and put **on** a chip, so what is
on the screen is what was searched for. The chip then shows the sigil beside the
setting, which is how the syntax actually teaches itself: you click it once and
see what you could have typed. And a sigil never touches a chip it did not name
— a reader who narrowed to the Bavli by clicking a facet does not lose it by
typing a quotation mark.

### The facets, and the promise on every row

§9.8 wants five, with counts, each one click to narrow or exclude. The counts
are taken over the **whole** result set, from the same built query the hits came
from — not over the page, which would change as a reader scrolled and would be a
measurement of nothing:

```
$ girsa-index find index corpus --size 3 יתגבר כארי
79 in 5000545 segments · showing 3 · page 1 of 27
…
narrow by:
  shelf      חסידות 26 · הלכה 22 ·   שולחן ערוך 10 · מוסר 9 ·   אחרונים 9
             … and 54 more
  era        אחרונים 49 · no era recorded 26 · אמוראים 2 · מחברי זמננו 1 · ראשונים 1
  author     אליעזר פאפו 6 · נתן שטרנהרץ 5 · חיים דוד אזולאי 4 · צדוק הכהן רבינוביץ 4
  sefer      פלא יועץ 6 · ליקוטי הלכות 5 · כף החיים על שולחן ערוך אורח חיים 3
  link type  references 29 · comments-on 25 · quotes 2
```

**The number on the row is the number clicking it gives you** — the ladder's
promise, one section on, and asserted for every row of every dimension rather
than for a sample:

```
$ girsa-index find index corpus --shelf חסידות     יתגבר כארי   →  26
$ girsa-index find index corpus --linked comments-on יתגבר כארי →  25
$ girsa-index find index corpus --not-shelf חסידות  יתגבר כארי  →  53
```

Four things the column is careful about, each of which is a way a facet could
lie quietly:

- **two clicks narrow twice.** A scope is one clause per click and a hit has to
  satisfy all of them. The first shape of this was a set of slugs that each
  click added to — so narrowing to `תלמוד` and then to `ראשונים` gave *either*,
  which is a **widening** with a narrowing's label on it. The test caught it and
  the type changed.
- **`no era recorded` is a row.** 2,377 of the 7,189 works have no era in either
  corpus, and a column listing only the five real eras would hide a third of the
  library behind something that looked complete.
- **shelf rows nest and say how deep they are.** `תלמוד` and `תלמוד/בבלי` are
  both rows, so the column does not add up to the total and is not meant to —
  flattening to top shelves answers *which shelf* and never *which part of it*.
- **hits in seforim the catalogue does not have are counted out loud**, because
  otherwise the three derived facets are short by that many and nothing says so.

Three of the five — shelf, era, author — are not facts about a segment at all
but about the sefer it is in, so they are added up through the catalogue rather
than indexed. That is why correcting an author's dates costs a `girsa-import
--metadata-only` and not a re-index of five million segments. And the shelf they
group by is **the same `girsa_corpus::taxonomy` the bookcase browses by**,
including the reader's own arrangement: a sefer on one shelf in the tree and
another in a result list would be two answers to one question.

### The link facet needed the graph turned round

The other two facets are columns of the index, and one of them did not exist.
spec.md §8.2 stores an edge **once, in the direction it was written**, and W8
put each one in the shard of the work it points *from* — so Berakhot's own shard
holds the handful of edges Berakhot makes, and the millions that land **on** it
are scattered across every shard in the corpus. Answering *what kind of link
touches this segment* per query would mean reading all 691 MB of the graph to
draw one row.

So the graph is walked once and each end of each edge is written into the file
of the work it lands in:

```
$ girsa-link-types corpus
  shards read        5790
  edges              4182344
  rows               3637528   (both ends of each)
  took               98s
```

4,182,337 — W8's number exactly, walked from the other side. What it costs sits
beside the edges as `touching.bits`: **one 16-bit mask per segment in reading
order**, 9.7 MB for the whole shelf and 8,370 bytes for Shulchan Arukh, Orach
Chayim's 4,171 se'ifim.

It was `touching.jsonl` until 6 August — one JSON row per `(endpoint, type)`
carrying the list of every sefer at the other end, **448.7 MB over 6,268
files** — and its one consumer destructured that list away to produce nine bits
per segment. Orach Chayim's was 3.95 MB to say 4,171 numbers. The `w` list
existed so a phone reader could ask *which of my mefarshim speak here* without
reading `inbound.jsonl`; W28's landing index has since made that a seek into
4,171 places rather than a walk over 159,273 rows, so the 472× file was paying
for a read that no longer happens.

It is a cache: delete it and run the tool again. What is **not** allowed is an
index reading its absence as a zero, so the index writes down whether it had it:

```
$ cat index/girsa-build.json
{"works":7189,"segments":5000545,"link_types":true}
```

Without that file the link facet says *not built* rather than showing an empty
column. *Nothing here is commented on* and *nobody worked out what is commented
on* are different statements, and a column of zeros says the first while meaning
the second — which is exactly the silent gap §9.7 forbids one facet over.

A mask is **positional**, which is a hazard the anchor-keyed file it replaced did
not have: a stale anchor file is merely short, and a stale mask lights up the
wrong lines. So every file names the segmentation it was built for — a count and
a fingerprint of the ids — and the index build **refuses** one that does not
match, per work, by name, with the command to run. That is
`girsa-lane/src/vectors.rs`'s rule borrowed one directory over:
*the same model at a different width is also another model.*

Adding the column bumped the index's schema to 2, which is what the stamp is
for: the old index was refused rather than read, and rebuilt. It cost time —
**1,215s against W11's 248s**, because the build now reads 964 MB of graph
alongside the text — and 3.5 GB on disk, and both are the price of a facet that
is a count rather than an estimate.

## The Ksav loop

*Moving a source into a document should feel like AirDrop between two of your
own devices* (spec.md §10). No export dialog, no file, no format decision, no
cleanup — and **the user does nothing different**: Ctrl+C is Ctrl+C.

### One Ctrl+C, three flavours

What changes is what lands on the clipboard beside the text:

| flavour | who takes it | what it has to survive |
|---|---|---|
| `text/plain` | WhatsApp, a terminal, anything | being read with no formatting at all |
| `text/html` | Word, an email, a browser | keeping its shape **and its direction** |
| `application/x-girsa-source+json` | Ksav | carrying the **ref**, so the citation stays alive |

```
$ cargo run -p girsa-app --example send -- corpus "שולחן ערוך, אורח חיים סימן א' סעיף ג'"
── the ref the document stores ──────────────────────────────
girsa:shulchan-arukh/orach-chayim/1:3

── text/plain — WhatsApp, a terminal, anything ──────────────
ראוי לכל ירא שמים שיהא מיצר ודואג על חורבן בית המקדש:
(שולחן ערוך, אורח חיים סימן א' סעיף ג')

── application/x-girsa-source+json — Ksav ───────────────────
{"schema":1,"ref":"girsa:shulchan-arukh/orach-chayim/1:3","display":"שולחן ערוך,
 אורח חיים סימן א' סעיף ג'","text":"ראוי לכל ירא שמים…","nikud":false,"lang":"he",
 "version":{"edition":"Maginei Eretz: Shulchan Aruch Orach Chaim, Lemberg, 1893",
 "provenance":"https://www.sefaria.org/Shulchan_Arukh,_Orach_Chayim"}}
```

The third flavour is **written natively, not from the webview**, and that is not
a detail. `navigator.clipboard.write` will take a custom type, but Chromium puts
it down as a *web custom format* — a private encoding another browser tab can
read and a native application cannot. Written from the window, Ksav would see
the plain text and nothing else, and the pairing would look like it worked.

That the packet is real is checked **in Ksav, against a packet Girsa really
sent**: `ksav/engine/tests/from_girsa.rs` reads the literal output of the command
above and asserts the words of the se'if and the mekor are *on the laid-out
page*, not merely that the document compiled.

### Only the highlighted part goes

`girsa_app::sending` is handed segment ids and **character offsets into the text
the window drew** — markup already turned into runs, nikud already applied. So
both ends slice the same string and neither has to describe a selection to the
other. Highlight four words of a se'if and four words travel; highlight nothing
and the line you are standing on travels, which is what Ctrl+C does everywhere
else.

A selection across three se'ifim keeps the head of the first and the tail of the
last, and its ref is a **span** — `girsa:…/1:1-1:3` — because a quote is a range
(§4.2). Dragged upwards, it is put back into reading order before anything else
looks at it.

### The citation is not the string

What the document stores is `girsa:shulchan-arukh/orach-chayim/1:3`. The printed
form is `girsa-cite`, the formatter **both applications compile**, and it can be
asked for another one at any time:

| style | |
|---|---|
| `HebrewFull` | `שולחן ערוך, אורח חיים סימן א' סעיף א'` |
| `HebrewShort` | `שולחן ערוך, אורח חיים א', א'` |
| `English` | `Shulchan Arukh, Orach Chayim 1:1` |

`סימן` and `סעיף` are not words this app chose. They are the schema's
`heSectionNames`, carried onto every work by `girsa-import --metadata-only`, and
where a schema does not say — 1,101 branch schemas, and all 978 Otzaria-only
works — a sefer is cited by number, which is an ordinary way to write a mekor.
Nothing is invented: **no abbreviation of a title is guessed at**, because
nothing in the data says which of a work's 44 title variants a citation should
use.

The rule the formatter is held to is that Girsa can read back what Girsa
printed. Writing that test found two real defects, both fixed in `sefer-crates`
0.3.0 rather than worked around here: the resolver knew nine of the corpus's 42
section words, so `ברכות דף ב. שורה א'` resolved to `2a:שורה:1` without
complaint; and a whole sefer could not be written down as a ref at all, because
`girsa:bavli/berakhot` means the work `bavli` at a section called `berakhot`.

### When both are running, there is no clipboard at all

spec.md §10.6. Girsa opens a **desk** on loopback — `127.0.0.1`, a port the
system picks, a token minted per run and published in a file only you can read
— and so does Ksav. Each asks the other whether it is there:

| | |
|---|---|
| `Live` | answering, and it says which version it is |
| `NotRunning` | there is no endpoint file — it has not been started |
| `Stale` | there is a file and nothing behind it, **with the reason** |

The window shows which of the three it is, and the send button only exists for
the first. That is the whole of *presence* (§10.6): an affordance is never
offered when it would fail, and a crashed Ksav is told apart from one that was
never started, because those are different things to a reader.

Ctrl+Shift+C sends the selection straight into the open document. What comes
back the other way is Ksav asking the library questions only the library can
answer:

| | |
|---|---|
| `POST /open` | *show me this place* — the window opens the sefer and lands on the segment |
| `POST /cite` | *print this ref in that style* |
| `POST /quote` | *the words again*, read out of the corpus as it stands now |
| `POST /refresh` | *this whole document again* — every citation in it, re-read |
| `POST /where-from` | *where is this phrase from?* — cite-on-selection |
| `POST /search` | *nothing fitted* — put the phrase in the search and open it |
| `POST /linkify` | *which of these are citations?* — only the certain ones |
| `POST /document` | *I have saved a document here* — so *where did I use this* is true |

`/cite` and `/quote` are what make a citation alive. Because a Ksav document
stores the **ref** and not the printed string, a whole sefer can be switched
from abbreviated to full-form citations, and every quote regenerated against a
corrected edition (§7) — but only if something knows the title, the words the
schema uses for a level, and the text. All three live in the library, so Ksav
asks rather than keeping a copy that nobody would remember to update.

`/refresh` is the one that makes the port worth having. Everything Girsa
*hands* Ksav, the operating system could carry: a source on the clipboard is
push, one direction, no reply, and Ctrl+V is the whole protocol. What a
clipboard cannot be is a **question**. §10.2's promise is stated about a
document — forty citations at once, some of which name a sefer this shelf does
not have — and one call comes back with a row for each, in the order the
document has them, a reason in the ones that failed and the other
thirty-nine still refreshed. The decision *one missing sefer is not a failed
document* is made once, in the library.

What comes back is rows, not a rewritten file. A correction somebody else made
silently changing the words in the sefer you are writing is the surprise §7.1
is built to avoid, so the writer sees what moved and says yes.

**Localhost is not private**, and the token is not decoration: every process on
the machine can reach a loopback port, and so can a web page. So it is required
on every path including `/health`, it travels in a header rather than a URL, and
the desk answers no preflight and sends no CORS header — a tab that guessed the
port and the token still cannot read a word of the reply.

### A citation is a link, and it was already one

`girsa://open?ref=…` opens a place. So does a bare `girsa:bavli/berakhot/2a:1` —
because **a ref is already a URI**. Nothing had to be generated: the string the
document has been storing all along is the link, which is why the citation in
the HTML clipboard flavour is `<a href="girsa:…">`. Paste a quote into Word,
print it to PDF, and the mekor in the PDF opens the page it names.

Anything that is not one of the two errands is refused rather than approximated.
A URL handler is an entry point every page on the machine can reach.

### A place to write, in the same window

spec.md §10.3. You are learning, you have a thought, and switching applications
to record one line is how the line does not get recorded. **Ctrl+E** opens a
drawer along the foot of the window — not a pane, because the sefer you are
writing about has to stay on the screen.

What it writes is **real Ksav markup from the first keystroke**:

```
#כותרת1[השכמת הבוקר]
#ציטוט[ראוי לכל ירא שמים שיהא מיצר ודואג על חורבן בית המקדש:]#מראה_מקום[שולחן ערוך, אורח חיים סימן א' סעיף ג']

וצריך עיון.
```

That is a `.ksav` file in your own layer, and the acceptance is checked **from
the other side**: `ksav/engine/tests/from_girsa.rs` takes a buffer this window
wrote, compiles it with the real Typst engine, and reads the words off the laid
out page — including that the mekor lands *below* the quote, where a footnote
belongs.

The markup is not written here. `#ציטוט[…]` comes from `girsa-ksav`, the crate
Ksav itself compiles, because *lightweight means the UI, not the format*: a
second writer in TypeScript would be two applications producing documents that
differ depending on which end wrote them. The window decides where the caret is
and nothing else.

**פתח ב־כְּתָב** hands the whole document to the real Ksav over the loopback —
offered only when presence says it is there. There is no conversion step, which
is the point: Ksav is opening a document it can already read.

### Where is this from, and who quotes it

spec.md §10.4 says these are one feature asked from two directions, and they
are **one function**: the only difference is whether the sefer you are standing
in is left out of the answer.

```
$ girsa-index where-from index corpus "משעה שהכהנים נכנסים לאכול בתרומתן"
משעה שהכהנים נכנסים לאכול בתרומתן  —  ב־61 מקומות
  ברכות             מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים…
  רש"י על ברכות     מאימתי קורין את שמע בערבין. משעה שהכהנים נכנסים לאכול בתרומתן…
  הלכות גדולות      מאימתי קורין את שמע בערבין משעה שהכהנים נכנסים לאכול בתרומתן…

$ girsa-index where-from index corpus --except bavli/berakhot "משעה שהכהנים…"
משעה שהכהנים נכנסים לאכול בתרומתן  —  ב־59 מקומות
```

61 places, and 59 of them are not the Gemara — which is the answer to *who
quotes this*. In Ksav it is Ctrl+Shift+M on a highlighted phrase: the first
mekor appears, Tab cycles the rest, Enter inserts it as a `#מראה_מקום`, and if
none fits, **the last row opens Girsa's search with the phrase already in it**.
A citation nobody could settle is not a citation to guess at.

What the engine is careful about is not finding — a phrase search always finds
something. It is not *lying*:

```
$ girsa-index where-from index corpus "אמר רבי יוחנן"
אמר רבי יוחנן  —  ב־12347 מקומות — ביטוי, לא ציטוט
(not offered as a source: 12347 places)
```

12,347 places has no source; it has a language. The list is still shown — the
reader may recognise one — but **nothing is preselected and nothing is called
the mekor**. And a quotation that is not letter for letter says so: the literal
search runs first, the ladder is climbed only on a zero, and what comes back
carries the rung that was used, so a near match is never shown as an exact one.

### Closing the loop

Three things fall out of one fact, and the fact had to be fixed first.

**The ref is in the document now.** For three work orders it was not: the markup
carried `#מראה_מקום[שו"ע או"ח סימן א' סעיף ג']` and the ref went nowhere, which
made §10.2's promise quietly false. It is now

```
#מראה_מקום(מקור: "girsa:shulchan-arukh/orach-chayim/1:3")[שולחן ערוך, אורח חיים סימן א' סעיף ג']
```

— printed exactly as before, and **storing the place**. Everything below is
that one change, seen from three sides:

- **Auto mareh mekomos.** `#מראה_מקומות()` collects every citation that carried
  a ref into a list at the back. Cheap by construction: the refs are already
  there, so it is a sort and a print. Checked by the real Typst engine.
- **Where did I use this.** Standing on a passage, Girsa scans your own layer
  for refs that *cover* it — a citation of `2a:1-2a:4` answers a question about
  `2a:3`, and a citation of siman 1 answers one about se'if 3 of it. A scan,
  not a guess.
- **Your writing is a sefer.** A `.ksav` file goes on the shelf like anything
  else: the words are read out of the markup by the same crate that wrote it,
  so `#כותרת1[` is never indexed and never shown, and the segments carry
  permanent ids like every other sefer.

### Linkify, and how much it refuses

spec.md §10.5 and decision 12: **high-confidence patterns only, anything
ambiguous stays plain text.** Three rules, and each refuses more than it
accepts:

| | |
|---|---|
| the resolver must say **Exact** | `או"ח` is the Shulchan Arukh's volume *and* the Tur's; a citation naming two seforim is left alone |
| there must be an **address** | *the Shulchan Arukh writes at length* is a subject, not a mekor |
| every level of it is a **number or a daf** | else `ברכות ב. ועיין שו"ע` reads as Berakhot at a section called *ועיין שו"ע*, and swallows the next citation whole |

A leading prefix letter is peeled — `וכתב בשו"ע או"ח סימן א' סעיף ג'` is how a
citation is actually written — and that widens *where* one is found, never what
it is found to be.

What comes back is wrapped as `#מקור_חי(מקור: "girsa:…")[…]`: the words print
exactly as they were typed, the ref rides underneath, and in a compiled PDF the
citation is a **link that opens the page it names**.

## Corrections

### Never the text, and it is measurable what that buys

spec.md §7.1 and decision 8: a correction is a **patch** — a permanent segment
id, a span of characters, what was printed there, what it should read, who says
so and when — kept in your own layer at `personal/corrections.jsonl`. The
shipped corpus is never written to.

The argument for that is usually made in a paragraph. Here it is four tests,
each run twice: once against the overlay, once against the obvious alternative,
which is to open the file and fix the word.

| | overlay | fixing the file |
|---|---|---|
| show as printed | the printed words are still there | gone, and nothing knows they existed |
| take it back | one line removed | you would have to remember what it said |
| survive `girsa-import` running again | untouched | overwritten, silently |
| hand your corrections to somebody | a file of lines | a 3 GB corpus |

`crates/girsa-fix/tests/a_correction_is_not_an_edit.rs` is that table. The
in-place version is nine lines of the same test file and it is correct as
written; what it cannot do is any of the four.

### The three seconds are measured, not hoped for

spec.md §7.5 says that if correcting a typo is not a three-second interaction
from where you are reading, nobody does it. `crates/girsa-app/tests/three_seconds.rs`
measures the machine's share of that on a sefer the size of Mishnah Berurah —
18,120 segments — from opening the shelf to the corrected words being back on
the page, re-reading the whole sefer twice on the way:

```
18120 segments, no corrections yet:        123 ms
18120 segments, 1000 corrections already:  176 ms
18120 segments, 16000 corrections already: 509 ms
```

The second number is the one worth having. An overlay that is fast when it is
empty and quadratic when it is not fails a year in, when nobody is looking.

### It was, and the test stopped one size short of saying so

The third line is new, and so is the reason it can be measured at all.

The layer used to be **serialized in full on every mutation** — `Layer::add`
wrote every patch you had ever made, so the cost of correcting one typo was a
function of how many you had already fixed. The old numbers were 75 ms empty and
217 ms at a thousand: 142 ms of file, linearly, which puts the three-second line
at about twenty thousand corrections. The test measured to a thousand and
stopped, and its own comment named the failure it was stopping short of —
*"fast when it is empty and quadratic when it is not."*

That is how a guardrail goes green over the thing it guards, and it was five
other files' problem too. Marks, saved questions, folders, link repairs and the
spelling queue were all the same store written five more times, and the queue is
the one that hurt: **28,124 entries on the real corpus**, rewritten in full every
time you said yes or no to one of them, in a feature whose whole pitch is being
handed thousands of ranked candidates.

All six are now one thing — `girsa-personal`'s `Log`. The file is the same jsonl
it always was, read as an append-only log: a record is a line, a later line for
the same key wins, `{"gone":"…"}` takes one back, and the file is rewritten only
when it has grown past twice what it holds. Nothing had to be migrated, because
a file with no repeats and no tombstones is its own compaction — which matters
here more than anywhere else in the tree, `personal/` being the one directory you
cannot re-download.

What is left of the slope at 16,000 corrections is reading them in order to apply
them, which no design gets out of.

The guard is not the timing, which is a bad thing to assert on.
`crates/girsa-fix/tests/a_correction_is_one_line_written.rs` asserts the property
underneath: after writing a correction that sorts *before* every one already
held, the bytes that were in the file are still in the file, unchanged, in the
same places. That is true of an append and false of a rewrite — including a
rewrite that happens to produce a file of the same length.

In the window it is: highlight the word, **Ctrl+K**, the box opens on the word
with the word already in it, type it right, Enter. No dialog, no navigation, and
the line is redrawn where it stands rather than the sefer being rebuilt under
the reader.

### An offset is not a place, so a patch carries the words too

A patch stores the span **and** what was printed in it. That looks redundant and
it is the whole verification: an offset says *where* and the words say *what*,
and when upstream re-types the line they stop agreeing. Then:

- the words are still there **exactly once** → the correction is re-anchored to
  them and says that it moved;
- they are there twice, or not at all → nothing is applied, and the patch is
  reported stale.

Never applied by offset alone. A correction that lands on letters nobody pointed
at is BUILDER.md rule 6 in the place a reader would never think to check.

### Two coordinate systems, and neither of them is the file

The window counts a highlight in characters of **what it drew** — markup off,
nikud applied, corrections already in place. A patch names characters of the
segment on disk. In Berakhot those differ by most of the line.

So `girsa_app::display::Shown` records what the markup scan took out, and
`girsa_fix::Corrected::base_span` records what the corrections put in. The scan
that draws a line and the scan that maps a highlight back to the file are now
**one function** — `runs()` is built on it, and its existing tests are what
proves the two agree.

A highlight that runs across a correction already there has no answer in the
file, so it is refused with what that correction says, rather than the system
inventing a base text.

### A typo and a girsa variant are one mechanism and two claims

spec.md §7.2. The `kind` field distinguishes them, and it is what the reader
sees:

| | applied to the words | marked |
|---|---|---|
| `ocr` — the scanner misread a letter | yes | `✓` |
| `girsa` — somebody reads it differently | **no**, noted beside them | `≠` |

Silently replacing the text you are learning with somebody's emendation is a
claim made on your behalf, so *show corrected* (the default) repairs scanning
errors and only notes variants. **Ctrl+Shift+K** rounds the three settings —
corrected, as printed, with variants — and it is remembered like the nikud
toggle. A variant carries the ref of the sefer that says it, which is the
`emends` edge of spec.md §8.2 written from the other end.

### The queue is worth more than the editor, and the corpus said why

spec.md §7.3: *a word appearing exactly once in the corpus, one edit-distance
from a word appearing ten thousand times, is almost certainly an OCR error.*
`girsa-suspects` is that batch job. It reads the **index's term dictionary** —
tantivy has already counted every word of every segment, so a second pass over
five million of them would be an hour spent arriving at the same table.

```
2,402,768 words in the index, read in 5.6s
   28,124 candidates in 90.1s
    1,356 of them a known confusion of shapes
```

What makes it usable is what it refuses. Hebrew attaches its function words to
the front of the next one, so `ובשבת` is `בשבת` with a vav — one edit, and it
looks exactly like a scanner dropping a letter:

| refused | why |
|---|---|
| a letter added or dropped at the **front**, where it is ו ה ב כ ל מ ש ד | a prefix, not a scanner |
| a letter added or dropped at the **end**, where it is ו י ה כ מ נ ת | a pronoun or a plural |
| words shorter than four letters | every short Hebrew word is one edit from a dozen others |

**And the first ranking was wrong, which the real corpus is what said so.**
Ranked by how common the neighbour is, the queue opened with ten misspellings of
`הוא` — a word in 1,305,264 segments, so every four-letter near-miss of it
outranks every ד/ר in the library. Frequency is not evidence. What replaced it
weighs three things: how common the neighbour is *as a logarithm*, how long the
rare word is, and **what the scanner did** — a letter read as another is worth
twice a letter that merely appeared, and a pair that look alike in print is
worth twice again. The same run, rescored:

```
סשומ (1) → משומ (574,691) [מ/ס]   bavli/shita-mekubetzet-on-bava-metzia 12b:2
שאיג (1) → שאינ (556,837) [ג/נ]   torah-ohr bereshit:3:11
אפילז (1) → אפילו (315,809) [ו/ז]  bavli/penei-yehoshua-on-kiddushin 12a:5
יהודח (1) → יהודה (173,217) [ה/ח]  ein-yaakov sanhedrin:11:70
רכינו (1) → רבינו (189,148) [ב/כ]  tzafnat-paneach-on-torah leviticus:7:35
```

**Nothing in the queue corrects anything.** A candidate is a question: which
word, which word it looks like, how often each was seen, and where to go and
look. Opening one takes you to the place with the word marked and the correction
box on it — and the correction goes through the same path a correction made
while reading does. Ctrl+J opens the queue; *לא טעות* takes a candidate off it.

A decision survives the batch job running again, which is the difference between
a tool and a list: without that, the second run hands you the four thousand you
have already dismissed, and you stop running it.

### Exporting a fixed sefer, which did fall out for free

spec.md §7.4 says base text + applied patches → a clean `.txt`/`.docx`, and that
it falls out of §4.1 for nothing. It does: the text is already text, the
corrections are already an overlay, and a sefer read through `Shelf::read` is
already corrected — what was left was writing it down.

What "clean" means: the words as the page shows them, the corpus's inline markup
gone, nikud as you are reading it, headings still headings — and **a header
saying what this is**. Which sefer, from where, which edition and licence, and:

```
משנה ברורה
Mishnah Berurah
מקור: sefaria
הוחלו שני תיקונים · גרסה אחת שנרשמה ולא הוחלה · תיקון אחד שלא חל, משום שהטקסט שתוקן אינו שם עוד
```

That last clause is the reason the header exists. A corrected sefer that does
not say it was corrected is a text somebody will quote as the printed edition,
and **exporting is the moment a stale correction would otherwise vanish**: it
was not applied, the file is fine, and nobody would ever hear about it.

The `.docx` is written by hand — a zip and two XML parts, which `girsa-corpus`
already opens from the other side to read a Word file you dropped on the window.
The paragraphs carry `w:bidi` and the runs carry `w:rtl`, without which Word lays
a Hebrew line out left to right; headings declare `w:pStyle`, which is exactly
what the importer reads. So the test is a **round trip**: export the sefer,
re-import the file with the same reader a dropped Word file goes through, and
the corrected words and both headings come back.

### What corrections do not reach yet

**The search index is built from the printed text.** A typo you fixed this
morning is still findable by its typo and not yet by its correction, because
`girsa-index` reads the corpus and knows nothing about your layer. The reading
pane, a quote copied to Ksav and a citation regenerated from a ref all show the
corrected words; a search result shows what was scanned. Rebuilding the index
per correction is not the answer and neither is a second index — this wants the
overlay taught to the indexer, and it is not built.

### What has not been checked

**The shelf panel has been driven in a browser, not in the shell.**
`cargo run -p girsa-app --example dev-fixtures` writes the real 7,189-work tree
to static JSON and the same page draws it — the counts above were read off that
page — but drag-to-rearrange and the file-drop event **only exist in the shell**
and were exercised through the Rust API and `girsa-shelf` instead. The shell
starts, opens the shelf and serves the commands; nobody has dragged a sefer with
a mouse.

`BUILDER.md` W9 carries a trap: *Tauri uses Edge's engine on Windows and
Safari's on macOS. Test Hebrew-with-nikud rendering on both — a screenshot from
one OS is not evidence.* **Only Windows has been looked at.** There is no Mac
here, and saying the rendering is fine on one would be exactly the claim the
trap warns about.

Half of it is cheap and is wired up: `cargo run -p girsa-app --example
dev-fixtures -- corpus app/public/dev` writes the real Gemara to static JSON and
`npm --prefix app run dev` serves the same page, same CSS, to any browser on
hand. That catches two engines disagreeing about where a nikud point sits. It
does not stand in for WebKit.

**The search panel has not been driven with a mouse either.** Ctrl+F opens it
in the shell, and every part of what it draws — the chips, their options, the
facet rows, what clicking one narrows by — is decided in `girsa-search` and
tested there, over an index built in memory and over the real one from the
command line. The shell builds, the commands are registered and the panel is
wired to them; nobody has clicked a facet row with a pointer.

**And it does not draw in a browser.** `npm run dev` reads static JSON written
by `dev-fixtures`, and a search index is neither static nor small, so the panel
in a browser says so instead of showing an empty result list — which would read
as a corpus with nothing in it. The consequence is that the W9 trap stands for
this panel: its Hebrew has been looked at on one engine only.

**The clipboard has not been driven with a mouse.** W15's three flavours are
decided in `girsa-app`, tested there, and the packet is checked from the far
side by a test in Ksav that reads a packet this corpus really produced. What
has not happened is a person pressing Ctrl+C in the window and pasting into
Word: `clipboard-rs` puts the three formats down inside one clipboard open, and
that call has been compiled and not watched. The same goes for the selection —
the offsets are computed in the page from a real `Selection` and handed to Rust,
which is tested, but nobody has dragged a mouse across a se'if.

**Neither end of the pairing has been watched with two windows open.** The
transport is tested end to end through a real socket — in Ksav's suite, because
that is where both halves can be linked into one test binary — and what is not
tested is the two *processes*: Girsa's desk answers out of the Tauri shell,
which has no test harness, and the endpoint files are per-user, so two builds
running at once is the one thing a test cannot arrange for itself. Start both
and press Ctrl+Shift+C; that is the check nobody has run.

The rest of the Ksav loop — cite-on-selection (W18) and sending your own writing
back into the library (W19) — is **built**; the status table above is the one to
read. This paragraph said *"still to come"* a thousand lines below that table for
long enough that a 2026-07-30 audit found the contradiction (D-1).

Two things W10 leaves for the orders that own them. A sefer of yours is **not in
the resolver's lexicon**, so it is opened and filed by title and not yet cited
by one. W14 wired the resolver into the query bar and did not change that: the
lexicon is `corpus/lexicon.tsv` and the 978 Otzaria titles beside it, both
written by the import, and a sefer you dropped in this morning is in neither. It
is searchable like anything else and it is not citable by name. And a PDF has pages and no words,
which is W26's to change; the index already carries them as `page` segments so
that §9.7's *"4 PDFs on this shelf aren't searchable yet"* is a count somebody
can take, rather than a silent gap.

## Links, and repairing them

### The data is wrong, and you can say so — without editing it

spec.md §8.3, decision 9. 40% of the link graph carries no type at all and it
originates upstream (T5), so a re-import does not fix it. The four things a
reader can do — **reanchor, retype, reject or confirm, draw one by hand** — are
stored as overrides in `personal/links.jsonl`, and the shipped shards are never
written to. Same rule as corrections, same three reasons: the importer replaces
every shard it owns on every run, your judgement and the corpus's must stay
distinguishable, and a thing you said should be undoable.

**Everything shows its work**, because that is the difference between a repair
tool and a rumour. Each row carries what it is, what it *was*, how it was found,
how much to believe it, which of the four actions changed it, and who said so:

```
מפרש   ← רמב"ם על משנה ברכות 1:1:1    90% · sefaria-seed · "commentary"
קשור   ← ספר האסופות סימן כב           90% · sefaria-seed · ""      ← not curated
פוסק   ← שולחן ערוך, אורח חיים נח:א    100% · by-hand · drawn · you
```

The second row is the point of §8.3's last sentence: **a blank-typed link is
never presented as curated fact.** It is shown, greyed, and it stays that way
until somebody looks at it — confirming is a claim a person made, and it is
worth nothing if an unconfirmed link looks the same.

### Who comments on this line, without opening seven thousand files

An edge is stored once, in the shard of the work it points *from* (§8.2), so the
outgoing half of the question is one file. The incoming half — *who comments on
this se'if* — is the reverse direction, and the honest way to answer it would be
to open every shard in the corpus.

It doesn't, because `girsa-companions` already recorded which works share edges
with which. The panel reads **only the shards of works known to link here** — a
few dozen for the first mishnah of Berakhot, which has 333 links on it, in 2.5
seconds on a debug build. When that cache has never been built the panel says
so, rather than showing the outgoing half and letting a reader believe that is
all there is.

That number is also the honest limit of this design: a reverse index would make
it instant, and there isn't one. The tripwire in
`crates/girsa-app/tests/the_links_on_this_line.rs` exists to catch the day
somebody makes it read the whole graph instead.

### A repair follows the edge, not the row

Found by the real corpus, in the test: there are several links between the first
mishnah of Berakhot and the Rambam on it, and the panel sorts by confidence — so
confirming one moves it to the top and rejecting it moves it to the bottom. A
test that re-found "the first Rambam row" after each action was confirming one
link and rejecting another while believing it did both to one. Every repair is
keyed to the edge's **shipped name** — its two segment ids — which is also why a
reanchored edge is still found by the record that moved it.

### Which words a link is about, when anything says

spec.md §8.4: *links attach to specific words, not whole segments — selecting a
phrase highlights only the links touching it.* Nothing in the shipped data says
which words: Sefaria's links address a segment and so do Otzaria's. So a span
comes from one of exactly two places, and never from a guess:

1. **The dibur hamatchil** — the commentary says which words it is on, in the
   text. And the corpus writes that two ways: `<b>…</b>` in some volumes, and a
   dash in others. Rashi on Berakhot, in the copy on this shelf, is entirely the
   second — a reader of `<b>` alone finds **nothing** in the whole masechta.
2. **You said so** — a link you drew from a highlight, or pinned onto one.

Measured on the real text:

```
255 of 501 diburim landed on their words
girsa:bavli/rashi-on-berakhot/2a:1:1#1 — on: מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין? מִשָּׁעָה שֶׁהַכֹּהֲנִים…
girsa:bavli/rashi-on-berakhot/2a:1:2#2 — on: עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה
```

That is not a rate to optimise. The half that does not land is **refused on
purpose**: the words are not in that line, or they are there twice. A dibur
hamatchil that appears twice gives two candidate spans and no way to choose, so
it gives none — a highlight on the wrong half of a line looks exactly like a
highlight on the right one, which is rule 6 in the place a reader would never
check. Matching is through the normalizer throughout, because Berakhot ships
menukad and Rashi on it does not.

One of those refusals is worth reading twice: Rashi quotes `בערבין` where the
mishnah in front of him reads `בערבית`. That is a girsa and not a typo, the
whole-phrase candidate correctly finds nothing, and the shorter phrase he also
quotes is what lands.

Asking the narrower question drops **only what is known to be elsewhere**: a
link with no span stays, because it is on the whole segment and the segment
includes what was highlighted. Answering "which links are on these words" with
"the ones whose words I happen to know" would be a shorter list wearing the face
of a complete one.

### Lenses are saved filters, not five lists

spec.md §8.5. Halacha / Lomdus / Peshat / Girsa / Mine ship as five rows of
`personal/lenses.json` — each a filter over **type, era and strength** — and
every one of them is yours to change, add to or delete. Whether the Tur belongs
under Halacha is a question about how you learn, not about this program.

Strength is where W23 pays: a confirmed link and one you drew score 1.0, an
untyped seed scores what its method scores (0.9 citation-addressed, 0.7
line-indexed), and a rejected one scores nothing. So *"only what somebody has
actually checked"* is a lens with `at_least: 1.0` and no code behind it.

### Measured against `spec.md` §2

Every number §2 states, checked. `girsa-import` prints this table at the end of
a run and exits non-zero if a row is wrong, so a change that quietly loses a
se'if is loud. Disagreements are **reported rather than coded around**, per
`BUILDER.md` Appendix B.5.

| | spec.md | measured |
|---|---|---|
| Sefaria download | ~2.2 GB | **3.4 GB** |
| schemas | 6,456 | **6,595** |
| Hebrew `merged.json` | 6,211 | 6,211 ✓ |
| link CSVs | 19 | 19 ✓ |
| Otzaria-only works | 978 | 978 ✓ |
| Mishnah Berurah | 18,120 / 701 | 18,120 / 701 ✓ |
| Shulchan Arukh O.C. | 697 / 4,171 | 697 / 4,171 ✓ |
| **works in the union** | ~7,576 | **7,189** |
| **links with a blank type** | 74% | **40%** |

The two counts that matter are exact, so the spec's method was sound; the size
was under-sampled (40 titles) and the schema count has drifted up since.

**The union is 7,189, not ~7,576.** §2.3 built it from `table_of_contents.json`'s
6,598 Hebrew titles, but the export ships Hebrew *text* for 6,211 of them —
which is the figure §2.2 itself states. 6,211 + 978 = 7,189, and the shared and
Sefaria-only halves come to 5,640 + 571 = 6,211 exactly. The missing 387 are
titles Sefaria catalogues and has no Hebrew for: there is nothing to read, so
they are not works. They are still in the resolver's lexicon, which is why a
link into one of them resolves cleanly and lands nowhere.

**Blank link types are 40%, not 74%.** §2.1 measured 74% by sampling one sefer
(Abudraham, 420 links). Across all 5,037,106 rows of Sefaria's CSVs it is 40%.
The finding underneath it is unchanged and still the one that matters — the
blanks originate upstream in Sefaria, so re-importing does not fix them.

`spec.md` §9.1 also says to strip `U+0591–U+05C7`. Four code points in that
range are *punctuation that separates words* — maqaf, paseq, sof pasuq, nun
hafukha. Deleting maqaf glues `אֶת־הַשָּׁמַיִם` into one token and the second verse of
the Torah stops being findable by either word in it. They become spaces.

## Scans

### The scan is the daf

spec.md §6.2 and §6.3 are one decision taken twice. A text sefer gets modern
columns and **no tzuras hadaf**, because rebuilding the traditional page out of
a string of words is a typesetting project. A scan needs no engine at all: the
photograph *is* the daf, with the Rashi in its column and the Tosfos in its. So
the PDF layer is a second reading mode rather than an attachment, and the whole
of what this work order adds is the one thing a photograph does not come with —
**a mekor**.

```
page 47 of the file  ──[ the mapping ]──►  ברכות כג.  ──►  girsa:bavli/berakhot/23a
```

Both directions, because both are asked. Forward is what the header says and
what Ctrl+C copies. Backward is *where is daf כג* — a search hit, a link, a
mekor clicked in a Ksav document — and it is what makes a scan open on the right
page instead of at the beginning.

### One number would have been the same bug again

The obvious mapping is an offset: *the daf is the page plus three*. It is two
lines of arithmetic and it is right until the first plate — and a scan of
anything old has one, bound in somewhere around daf כ, after which the number
that was right for four hundred pages is one daf out for the rest of the sefer.

The only repair for one number is to change it, and changing it **moves every
citation in the sefer**, including the four hundred pages that were already
right, silently, with nothing anywhere saying that a mekor written last month
now points a daf away. That is BUILDER.md T1 wearing a different hat: the page
number is being used as the address.

So the mapping is a **list of anchors**, and a page's daf is counted from the
nearest anchor *behind* it. Declaring a new one cannot move a page in front of
it, because no page's address is ever computed from an anchor after it. An
anchor may also say **nothing** — `43=-`, *from here these are not pages of the
sefer* — which is how the plates themselves stop being cited as dafim printed
elsewhere in the masechta.

```
5=ב.    43=-    45=כא.
page 42 → כ:     pages 43, 44 → not the sefer     page 45 → כא.
```

`crates/girsa-scan/tests/which_page_is_which_daf.rs` is written against the
one-offset version and seven of its fifteen cases fail there — including the
title page coming out as **daf 0a**, and the whole scan moving when the reader
declares the plates.

Three schemes, because a scan is not always a masechta: one **amud** to the page
(nearly every Shas PDF), one **daf** to the page (a photograph of the open
sefer, so the page is a *span* — which is what a ref has been since W3), or one
**number** to the page. A sefer with four simanim to the page is not describable
this way, and nothing here pretends otherwise: interpolating to the siman that
starts nearest is how a mekor names a place the reader was not looking at.

### What a page cites as, and what it never invents

A scan of Berakhot cites as **ברכות** — the mekor everybody else writes,
resolving to the same place in the library — once the reader says what it is a
scan of. Standing on its own it cites as itself, which is still a real ref to a
real sefer on a real shelf. And the property `girsa-cite` asserts about every
other citation in this system holds here too: **what a page cites as reads back
as the page it came from**, over every page of the scan, in all three styles.

Three things it will not do:

- **The front matter gets no daf.** There is no daf א in any masechta — the
  first leaf is the title page — so a mapping that extrapolated backwards would
  hand the reader a mekor to a place that has never been printed. The window
  says *עמוד 3 בקובץ*, which describes where they are without pretending it is
  citable.
- **A daf the scan does not carry is not the nearest page it does.** The same
  refusal to round as everywhere else here.
- **A mapping that would put two pages on one daf is refused**, naming both. A
  duplicated page happens; what may not happen is `page_of` quietly ceasing to
  be a function and one of the two pages becoming unreachable.

A page has no words — the importer will not invent Hebrew it cannot read — so
Ctrl+C on one puts down a **mareh makom**: the citation and the ref, and no
quote. `girsa-ksav` writes that as `#מראה_מקום(…)` alone rather than as
`#ציטוט[]`, which is the one change this work order made in the shared crates:
an empty quote block in the middle of somebody's chaburah reads as a paste that
failed.

### The defect a real PDF found and the tests had not

Running `girsa-daf` against a real 302-page sefer, with its printed numbering
declared from page 7:

```
$ girsa-daf … cite user/berachos-combined 47
berachos_combined מ"א
girsa:user/berachos-combined/41
girsa:user/berachos-combined/47#47
the ref opens page 47 — the page it was copied from
```

That last line is there because it once said something else. A scan's segments
are addressed by the **file's** page — page 47 is `47` — and a sefer numbered by
page has its own numbers, so `girsa:user/…/41` meant *printed 41* to the viewer
and *file 41* to everything that resolves a ref. Seven pages apart, both plain
numbers, and nothing anywhere saying which was meant.

Once a reader declares what the pages are called, **that is what an address of
that sefer means**, here and everywhere. A page the mapping does not cover is
then not reachable by a ref at all, which is the honest answer — the reader has
said the sefer starts on page 7, and the shaar blatt is not a place in it. It is
still reachable, still noteable and still linkable by its **permanent id**,
which no mapping ever moves. That is the whole of W6 said again about pages, and
`the_scan_is_the_daf.rs` asserts it: re-declaring the anchor moves every
citation the scan prints and not one of the 120 ids.

### The scan beside the Gemara

W9's acceptance in the second reading mode: move the Gemara and the column
beside it turns to the daf. It follows **only because the reader said this is a
scan of Berakhot** — a scan and a text that merely share an address shape line
up beautifully and mean nothing, and a column that moved on a resemblance shows
a reader one place while the header names another. A daf the scan does not carry
is `אין כאן`, and the pane stays where it is.

### What draws the page

pdf.js, bundled — Apache-2.0, which is one half of this project's own licence —
and **loaded the first time a scan is opened and not before**: it is half a
megabyte of renderer, and most readings of most seforim never touch a PDF. The
alternative was the webview's own PDF viewer, which is Edge's on Windows and
WebKit's on macOS: two behaviours, neither of them ours, and neither able to say
which page is on the screen, which is the one thing this pane exists to know.

The file itself is read off the disk through Tauri's asset protocol, scoped at
startup to `personal/files` and nothing else. A scan is hundreds of megabytes
and cannot travel over the IPC channel a page at a time.

## Reading a scan

### The engine question, answered by measuring it

`spec.md` §17 left one thing open here: *Hebrew OCR on old print is genuinely
hard and Tesseract is mediocre at it. An afternoon of evaluation decides whether
"optional OCR" is a good feature or a disappointing one.*

The afternoon happened. Five pages of a real sefer on this shelf — a Berachos
with the mishnah in square script under full nikud and the commentary beneath it
in **Rashi script** — rendered at 300 dpi and given to tesseract 5.4.0 with the
`tessdata_best` Hebrew model. The file carries its own text layer, so every word
on every page has a known right answer to score against, which is a luxury this
evaluation had and a Vilna Shas would not.

| page | what is on it | recall | precision |
|---|---|---|---|
| 151 | square script, unvocalized | **99%** | **99%** |
| 301 | square script, unvocalized, heavily abbreviated | 83% | 76% |
| 7 | square + nikud, Rashi script, footnote figures | **27%** | **23%** |
| 8 | the same | 28% | 23% |
| 51 | the same | 18% | 15% |
| | **all five** | 50% | 44% |

Tesseract can read a modern Hebrew paperback and cannot read a mefaresh. Which
is the answer §17 was worried about, and it decided three things — none of them
"find a better engine".

**The precision column is the one that matters.** On the Rashi-script pages
tesseract produced roughly four words that are not on the page for every one
that is. A word that is not there is not a gap in the index; it is a **hit that
does not exist**, and a reader sent to a daf that does not contain what they
searched for has been lied to by the search box in the one place they cannot
check without reading the whole page.

**And you cannot threshold your way out of it**, which is the finding that
surprised. The obvious repair is to throw away the low-confidence words. It does
not work, because tesseract is *confidently* wrong on a script it has never
seen — on page 7, raising the floor from 0 to 90 costs three quarters of the
recall and buys fifteen points of precision:

```
min conf   recall  precision
       0     27%       23%
      50     18%       25%
      70     11%       25%
      90      7%       38%
```

So no confidence knob ships. Every word's confidence is recorded, for the repair
screen; nothing is silently dropped on the strength of it, and the honest signal
to the reader is the badge and the photograph beside it.

### The engine that works is the one that does not run

**A PDF that was typeset rather than photographed carries its own text.** The
831 words this evaluation scored *against* came out of it — exact, instant, no
model, and incapable of inventing a word. So the default for any PDF is to ask
the file, and OCR is what happens to the pages that have nothing to ask.

On the same five pages, the same score, the same way:

| | recall | precision |
|---|---|---|
| the file's own text | **87%** | **94%** |
| tesseract | 50% | 44% |

Which sounds obvious and is not, because a PDF does not have words. It has
drawing instructions, and a Hebrew sefer typeset properly positions **every
letter and every nikud mark separately** so the marks sit where the typesetter
wanted them. Ask such a file what its text is and it answers

```
ֵמ ֵא יָמ ַת י
```

— a space between the halves of every letter, because the extractor puts one
wherever the pen jumped, and half of those jumps are inside a word.

So the words are worked out from the geometry: glyphs sorted onto lines, right
to left, cut wherever the gap between two of them is wider than **0.28 of their
height**. That number is measured rather than chosen. Over five pages of this
sefer there are 5,500 gaps between adjacent glyphs, and they fall into two piles
with a valley between them:

```
gap ÷ glyph height
+0.05..+0.10 ############################################ 1795   inside a word
+0.10..+0.15 ########################################     1620
+0.15..+0.20 ###                                           124
+0.20..+0.25                                                 8   ← the valley
+0.25..+0.30                                                19
+0.30..+0.35                                                12
+0.35..+0.40 ######                                        267   between words
+0.40..+0.45 ##                                             81
```

Thirty-nine gaps out of 5,500 land in the ambiguous band. The spaces the file
itself supplies are ignored entirely — which is what makes this the same code
for a text layer and for an engine that hands back loose glyphs.

### What the file will not spell is left out, not guessed at

The other half of that page is the encoding trap `girsa-corpus`'s importer
refused to walk into when it declined to read a PDF's text into a sefer. A font
that positions its own nikud very often has **no `ToUnicode` entry for the mark
glyphs**, and sometimes none for the pre-composed letter-plus-mark glyphs
either, so they come back as control codes: `U+000E`, `U+0010`.

A mark drawn on its own that the file will not name is dropped and costs
nothing — it is the nikud, and the index strips nikud in every mode (`spec.md`
§9.1). But a *letter* the file will not name is different, and the line it is on
comes out like this:

```
יַת5? ים דִס ֹוף   ‹— fragments of four real words
```

Those are not slightly-wrong words. They are strings that will be found by a
search for something that is not printed on the page, which is rule 6 again. So
**a line holding a letter the file would not name is refused whole** and
counted. On this sefer that is 3,605 words of 60,455 — the vocalized mishnah
lines — and the commentary beneath them, which is most of the page, reads
perfectly.

```
$ node app/tools/glyphs.mjs personal/files/user-berachos-combined.pdf \
    | girsa-read corpus personal words user/berachos-combined
273 of 302 pages carry their own text; 29 have none and want OCR
273 pages, 56850 words
4296 code points the file would not name; 3605 words left out for it
```

The 29 pages with no text turned out to be genuinely blank — this sefer needs no
OCR at all. A page that is read and found blank is written down as such, so it
does not come round the queue again forever.

### A correction is anchored to the ink

This is the load-bearing decision, and it is `spec.md` §6.3 taken literally:
*the image stays ground truth, which makes fixing OCR errors safe by
construction.*

W20 stores a correction to a text sefer as `segment id + character span`, and
that is right there: the base text is a file on disk that does not change under
it. It is **wrong here**, because a page's words are an engine's current opinion
and the whole premise of this work order is that a better engine replaces them.
Re-read a page and there are more words, or fewer, spelled differently — so
every offset now points somewhere else, silently, which is `BUILDER.md` T1 for
the third time.

So what is written down is a **rectangle on the photograph**, in fractions of
the page rather than pixels of whatever anybody rendered at. On the real sefer,
with the page OCR'd from a 300-dpi raster, corrected, and then OCR'd again from
a 200-dpi one — different pixels, different boxes, and a different reading of
the first word on the page:

```
$ girsa-read … ocr user/berachos-combined /tmp/pages300 151
page 151: 267 words
$ girsa-read … fix user/berachos-combined 151 20 מצווה
page 151, word 20: אפשר → מצווה
anchored to the ink at 0.551,0.196–0.611,0.206 of the page
$ girsa-read … ocr user/berachos-combined /tmp/pages200 151
page 151: 267 words
$ girsa-read … show user/berachos-combined 151
פרק ראשון ב. יש להעיר … (בהקטרה ובאכילה). מצווה לבאר שההיתר תלוי במצווה
```

The correction is on the same word. `girsa-scan/tests/the_image_is_ground_truth.rs`
is that property against every way a re-read can move an offset in one page — a
word split, two words merged, one misread, a speck of dust read as a letter that
is not there — and it fails on the offset-anchored implementation, which is kept
in the same file as a test rather than as a paragraph.

And a correction whose ink the new reading has no word under is **handed back**,
not dropped. The reader marked something and this engine found nothing there;
losing it quietly means they make the same correction again next year and never
know why the first one went.

### Two words with one rectangle are refused rather than resolved

An honest complication. The same PDF gives a vocalized page as 707 separately
positioned glyphs and an unvocalized one as **35 items, each a whole line with
its spaces in it**. On that page the file has said *which* words are on the line
and not *where* they are.

So the line is split into its words — the index needs that — and every one of
them carries the **line's** rectangle, which is what is actually known.
Apportioning the box across the letters would put a word break wherever the
arithmetic fell, and Hebrew letters run from a yud to a shin in width. A
highlight two letters off looks exactly like one that landed right, which is the
refusal W24 made about a dibur hamatchil made again about a rectangle. A
correction pointed at ink that two words share is refused, naming neither.

### One index, two location types

`spec.md` §9.7. A page with words on it is a row of the same result list as a
line of the corpus, found by the same query, ranked by the same rule:

```
$ girsa-index find index personal קפנדריא
searched for: the words קפנדריא, anywhere in a segment
3 in 302 segments · showing 3

girsa:user/berachos-combined/301#301  [page]  [read off the file]
  … מסתבר שגם [קפנדריא] אסור בהר הבית בזמן הזה, שהוא אף אסור בבית הכנסת …
```

**Badge them, don't demote them.** Nothing anywhere subtracts from a row's score
for having come off a photograph; what the row carries is a word for where its
words came from. Two badges and not one, because *the file said so* and *a
machine guessed at a picture* are the two rows of the table at the top of this
section and they are forty points of precision apart.

The rectangle is **not** in the index. A query cannot be asked about a
rectangle, and copying one into five million documents would buy nothing — so
the box is looked up from the reading when a row is opened, and the words to
mark come from the search's own marker rather than from what the reader typed.
Searching the drawn text for the typed string would find nothing on a menukad
page, which is most of them.

### Never a silent gap

Since OCR is off at onboarding, a shelf with scans on it has holes in its index
by design. The one thing that may not happen is for those holes to be silent:

```
$ girsa-read corpus personal status
1 PDF on this shelf isn't searchable yet — 23 pages
  user/berachos-combined — 279 of 302 pages read
```

That sentence is composed once, so the results header, this command, the MCP
server's `did_not_search` and the test cannot drift into disagreeing about a
count. A reader given forty hits over a shelf holding four unread scans has been
told *these are the forty places this appears*, and the forty-first is on a page
nobody has read. Search that quietly omits a shelf is worse than search that has
not been run, because it looks like an answer.

**"Composed once" was three times, and they drifted.** Three modules said part
of *what this answer could not see* — `girsa_note::since::Unindexed` (notes and
corrections newer than the index), `girsa_app::reading::Gap` (unread scans), and
`girsa_lane::Coverage` (what the semantic lane covers) — and each carried a doc
comment naming itself the only implementation so its surfaces could not drift.
Each was right about its own clause and none of the three could see the other
two. What drifted was everything between them: `Coverage` joined its clauses
with `; ` and knew a five-figure number wants a comma in it, the other two
joined with `·` and printed the bare integer, and `Gap` joined an already-joined
string into its own join.

The worse half was not punctuation. An `adjacent` answer carried the lane
sentence and said nothing about the chaburah written this morning; a `search`
answer said exactly that and nothing about the lane; the window's header said
scans and layer and nothing about either. **Three subsets of one truth,
depending on which surface you asked, each wearing a sentence that claimed to be
complete.**

Now: each module still words its own clause, because it is the only one that
knows the fact. `girsa_corpus::said::Clauses` does the joining — one separator,
one thousands separator, one plural rule, and `and()` flattens rather than
nests. `girsa_nearby::Unseen` decides which clauses belong to one answer, which is
the decision none of the three was in a position to make. The rule is checked by
`one_sentence_says_what_an_answer_could_not_see`: a module that words a clause
hands it over and spells no separator of its own.

A scan half read is neither searchable nor absent, and both numbers are
reported. *"3 PDFs aren't searchable yet"* over a sefer that is two-thirds done
would send a reader to run a job that is nearly finished; *"searchable"* over
the same sefer would be a lie about a hundred pages.

**And the fourth hole, found on 6 August 2026.** `girsa_note::since` had a table
of what the index cannot see — an un-OCR'd scan, a note written since the build,
a correction made since the build — and the fourth row was missing: **a word
corrected on a scan.** The index build *does* apply scan corrections; it reads
each page through `Words::page`, which re-finds every fix by its ink. But an
index is a snapshot, so a fix made after it holds the misreading — and the
reader who fixed a word could not find what they fixed and could still find what
they unfixed, with nothing saying so. It says so now, in the same sentence as
the other three:

```
words you corrected on 1 scan are still findable by the misreading
and not by the correction
```

Counted in scans rather than in words, deliberately: counting the words would
mean `girsa-note` opening `girsa-scan`'s file, and a modification time answers
the question the reader is asking. `pages.jsonl` is **not** counted here, because
a page OCR'd since the build is already reported — the index holds it as a page
with no words, so *"not searchable yet"* is exactly true of it, and saying it
twice would be two sentences about one silence.

### The job is one page at a time, and that is the promise

`spec.md` §6.3 asks for OCR that is *optional, off during onboarding,
background, resumable, never blocking reading.* Four of those are shape rather
than intention:

- **Resumable** with nothing to keep in step, because **the work product is the
  progress record**. The pages written down are the pages that are done; there
  is no separate counter that can survive a crash while disagreeing with what
  was actually read. Stopped at page 40 of 302, it starts again at 41.
- **Never blocking**, because the loop owns nothing between iterations: one
  page, then back to the window. The reader can turn the page, search, and copy
  a mekor while it runs.
- **Optional**, and *no engine installed* is a state with a name rather than a
  button that does nothing. Tesseract is **found, not bundled and not fetched** —
  nothing here downloads a model, because offline is the product (`spec.md` §14)
  and a runtime network dependency is not a decision this work order gets to
  take (`BUILDER.md` §0.1).
- And it looks for the Hebrew model in `personal/tessdata` as well as
  tesseract's own directory. That is not a convenience: tesseract installs into
  `C:\Program Files`, which takes an administrator to write to, and the Hebrew
  model is a separate download that does not come with it. This work order found
  that out by hitting it.

The window is the only thing here that opens a PDF — pdf.js, the same renderer
W25 chose for the same reason. It hands over glyphs, or a picture, and
everything after that is decided in `girsa-scan`, where it can be tested without
a webview.

**What this does not yet do.** OCR text does not reach the OCR-error queue of
W21, so a word tesseract got wrong is not ranked beside a word Otzaria's
scanner got wrong — the machinery is the same shape and the two have not been
joined. A page's words cannot be linked to at a finer grain than the page; W24's
span anchoring is about segments and a page is one segment. And nothing has been
run against a real photographed sefer: every measurement above is against a
born-digital PDF, which is the only kind on this shelf, so the numbers for
tesseract are its numbers on **clean 300-dpi print** and a photograph of a Vilna
Shas will do worse.

## Your own layer

### A note is not a row beside the graph

`spec.md` §11's claim is one sentence and it is the whole work order:

> **Your notes are nodes.** A note has the same typed edges as anything else, so
> *"what have I already written that touches this sugya?"* is the same query as
> *"who quotes this Rishon?"*

The cheap way to build notes is a table of `(segment id, text)` and a panel that
reads it. It works, it is a day, and it produces a library where your own
writing is the one kind of material in it that cannot be linked to, cited,
searched beside a Rishon, or asked about from the other end.

So a note here is two things it did not have to be:

- **a sefer on your shelf** — a `Work` with `Source::Mine`, whose paragraphs are
  segments with permanent ids, catalogued in `personal/works/index.jsonl` by the
  same code a dropped `.txt` goes through. It opens in a pane, it is citable,
  and the next index build finds its words;
- **joined to the corpus by a `girsa_link::Edge`** — the same directed, typed,
  evidenced edge as the 4,182,344 W8 imported.

Which means the claim is not a feature, it is an absence. There is no
`notes_on(line)` command, no notes panel, no second sort. Standing on the first
mishnah of Berakhot, **one call** answers both questions:

```
$ girsa-notes corpus personal on mishnah-berakhot 1:1
girsa:mishnah-berakhot/1:1#1
  שלי  comments-on  100%  מאימתי 1
       comments-on   90%  בועז על משנה ברכות 1:1
       comments-on   90%  ברטנורא על משנה ברכות 1:1:1
       …
334 links, 1 of them yours
```

333 of those rows are Sefaria's and one is mine, and the code that put them in
one list does not know which is which — it sorted them by the same rule, and
mine is first because a thing you wrote yourself is the strongest claim on the
line there is. It goes through the repair layer like everything else, too: W23
can retype or reject a note's edge, and W24's *Mine* lens was already the filter
that finds it.

Writing one is 3 ms, from the words to a sefer on the shelf. W20 put the
three-second guardrail on the clock; this inherits it, because a note that takes
a dialog and a *which notebook* is a note that does not get written.

### The file is the truth, and each paragraph carries its own name

A note is one plain text file. *Exportable as plain files* is not a feature on
the side — it is where the note lives, and the export is a copy, which is the
evidence rather than the shortcut: a format that needs an exporter is a format
you do not have.

```
girsa note
title: מאימתי
who: shaul
when: 1785334287
next: 4
tag: ברכות
on: girsa:mishnah-berakhot/1:1#1

girsa:note/מאימתי/2#2
וצריך עיון מה שכתב הרמב"ם כאן, דמשמע דהוי חיוב גמור

girsa:note/מאימתי/2.1#2.1
ובאמת כבר עמדו בזה

girsa:note/מאימתי/3#3
ועוד יש לדקדק בלשונו
```

Delete `personal/links.jsonl` and every note is still anchored where you put it,
because `on:` is in the note rather than only in the graph.

**Each paragraph's id is a line of the file**, exactly as in a `segments.jsonl`
and for exactly the same reason: a paragraph whose name was its position would
move every anchor below it the first time you inserted a line. That is T1, in
your own writing, where it would cost the thing the system exists to accumulate.

Look at `#2.1` above. It was written *between* `#2` and `#3`, and it is a
**child ordinal** — W6's trick reused rather than a second mechanism, because a
child sorts after its parent and before the parent's next sibling. So:

```
$ girsa-notes corpus personal after "girsa:note/מאימתי/2#2" "ובאמת כבר עמדו בזה"
girsa:note/מאימתי/2.1#2.1
2 paragraphs were already named, and 0 of them changed
```

That second line is the measurement. Under a store that named a paragraph by its
position, *the third paragraph* would now mean different words than it did a
moment ago; here `#3` is the words it always was. A paragraph you delete does
not give its ordinal back either — `next:` is on the file for that reason, and
an ordinal handed out twice would point two things at one permanent name.

The window edits a note **one box per paragraph, each carrying its id**, and
that is not a UI preference: a single textarea over the whole note would hand
back a wall of text to be re-split, and re-splitting is where ids get re-derived
from where the newlines fell.

### A highlight is an offset, and an offset is not a place

A highlight is a character range, and a range is a fact about the text as it
stood when you dragged over it. Correct a typo above it and the range names
different letters — silently, because a highlight looks the same wherever it is.

So a mark carries **the words as well as the offsets** and is placed through
`girsa_corpus::span::locate`, which is now one function with one caller-set:
corrections (W20) and highlights (W27). Offsets first, because that is where the
mark was made; then the words, **and only if they are there exactly once**. When
neither holds, the mark is reported stale rather than drawn:

| | what the panel says |
|---|---|
| the offsets still hold the words | drawn, silently |
| the line moved and the words are there once | drawn, and *השורה זזה, והסימון נמצא מחדש לפי המילים* |
| the words are gone, or are there twice | **not drawn**, and *המילים שסומנו אינן בשורה* |

The third row is BUILDER rule 6 in the one place a reader would never check. It
is not deleted either — it is a thing you did, and only you can put it right.

A highlight and a bookmark are one record with one difference: whether there is
a span. Two tables would have meant two files, two panels and two answers to
*what have I marked in this sefer*, for a distinction that is an `Option`.

### Everything of yours survives the corpus moving under it

Notes, marks and folders all anchor to permanent ids and all use `covers`, so a
correction that **splits** the line they are on does not orphan any of them —
the test in `crates/girsa-app/tests/a_note_is_a_node.rs` splits the first
mishnah of Berakhot in two and asserts the note, the highlight and the chaburah
folder are on both halves. That is W6's 501-link test, asked about the one kind
of anchor that is yours rather than the corpus's.

### And the second time you run the importer

Everything above is about a correction *you* make. The other direction is the
corpus itself changing under all of it, and for a long time the answer to that
was a promise rather than a mechanism.

`spec.md` §3 calls permanent ids *"the single most important decision in this
document"* and *"close to impossible to retrofit"*. Three doc comments —
`store.rs`, `segment.rs`, `BUILDER.md` W6 — promised a **redirect table** that
absorbs an upstream re-segmentation. `SegmentStore` really did have one. It was
in memory, `import::write` emitted `work.json` and `segments.jsonl` and nothing
else, and a store round-tripped through disk lost every row it held.

Which meant the thing underneath it was worse. `SegmentStore::import` handed out
`Ordinal::root(i + 1)` from enumeration position, with a doc comment saying *"it
happens once in the life of a work"* — a claim about the world, not about the
code. `girsa-import` runs over the whole catalogue on every invocation and
`write` is an unconditional overwrite. So:

> Sefaria adds one se'if to siman 1 of Orach Chayim. You re-run `girsa-import`.
> **4,170 segments renumber by one.**

Not a broken link. The wrong text, silently — which is T1 verbatim, at import
granularity instead of line granularity, in a tool called `girsa-import`. The
permanence held exactly as long as you never re-imported. `--metadata-only`
exists because somebody expected the importer to be re-run; somebody noticed
re-importing was expensive and nobody noticed it was also destructive.

**A name is now matched on the words, not on the address.** An anchor names
words, so that is the evidence that two records are the same place:

| what upstream did | what happens |
|---|---|
| inserted a se'if at 1:3 | every other text is unchanged, so every other name is kept; one name is minted **between** its neighbours — `#2.1`, not `#3` |
| re-sectioned the whole work | every address changed and no words did: **nothing is renamed** |
| fixed a typo | that text changed and its neighbours did not, so they pin the gap and the address settles it inside — corroborated by the opening word, because an address alone is how `1:3` ends up being a *different* se'if wearing the old one's name |
| folded se'if 3 into se'if 2 | `#3` is redirected at the record that absorbed its words; anchors on it still resolve |
| deleted a se'if | `#4` redirects to **nothing**, and says so. Its name is never handed to different words |

Unique texts anchor the alignment, the longest run of those that goes forward on
both sides is kept so the matching cannot cross itself, and addresses settle
what is left inside each gap. `crates/girsa-corpus/src/import/continuity.rs`.

`redirects.jsonl` sits beside `segments.jsonl` and is where the rows live:

```jsonl
{"from":"girsa:…/1:1#32","to":["girsa:…/1:1#32.1","girsa:…/1:1#32.2"],"why":"cut"}
{"from":"girsa:…/1:5#5","to":["girsa:…/1:4#4"],"why":"resegmented"}
{"from":"girsa:…/1:9#9","to":[],"why":"gone"}
```

Three events, one mechanism, because from an anchor's point of view they are the
same event: *what I named is over there now*. The `cut` rows are the oversized
cutter's own (B12) and they are what makes this file exercised by real data
rather than a slot nothing ever fills — they are also how the *next* import
knows those three records were one se'if. `gone` carries an empty `to` on
purpose: a place this edition does not have is a different answer from an id
nobody ever minted, and it is the difference between a reader being told *this
is not in the edition you have* and being shown somebody else's words.

The reader follows it. `Open::covered_by` resolves live → **cut out of** →
redirect, in that order, which is the order of how much is known — and never
picks the nearest surviving segment, which would resolve cleanly and be wrong.
That middle step used to be *descended from*, which is a different claim and a
wrong one; the next section is why.

What the importer prints after a re-import:

```
permanent ids across the re-import:
  re-imported        7189 works already on the shelf
  kept their id      5000545
  newly minted       0 (between their neighbours; nothing moved)
```

And what it would do before you run it, without a network or an Otzaria tree:

```sh
cargo run --release -p girsa-corpus --example measure-continuity -- corpus
```

```
  re-imported        7189 works already on the shelf
  kept their id      5000545
  newly minted       0 (between their neighbours; nothing moved)

no permanent id would change the words it names.
```

**The whole shelf. All 5,000,545.** Not a sample and not a synthetic fixture —
`spec.md` §2's number, re-imported against itself, with every permanent id
landing on the words it already named.

It did not start there. The first run over the real corpus lost 5,868 names
across 1,500 works, and both causes were things no synthetic fixture would ever
have contained:

- **Text on disk written before W34 mined the anchors out.** `tosefta-shabbat-lieberman`
  and about 1,500 others still carry `<i data-commentator…></i>` in their `text`,
  because they were imported before that landed and nothing has re-imported them
  since. A freshly mined text matches none of it — so the works most in need of a
  re-import would have been exactly the ones it renamed. `places_of` mines the
  previous run's text too; mining is idempotent and costs a substring scan.
- **18 segments in `tur` whose entire content is one anchor**, and so are empty
  once it is mined. Two texts with no words in them now agree: the failure being
  guarded against is an old name landing on new *words*, and a segment with no
  words cannot be wrong about which ones it has.

The extra cost is one read of each work's own `segments.jsonl` — the file the
import is about to overwrite anyway.

### The same question, asked by everything that is anchored

The reading pane followed that table. Nothing else did. The links panel, your
notes, your highlights and your folders each asked `SegmentId::covers` — six
characters of `starts_with` on the ordinal — which is a fact about the **name**,
and it answers a different question from the one being asked. It was wrong in
two directions:

| what upstream did | the anchor says | `covers` said | the truth |
|---|---|---|---|
| folded se'if 3 into se'if 2 | `#3` | nothing here | those are se'if 2's words now |
| inserted a se'if after 1 | `#1` | **this is your line** | it has never seen those words |

The first was named in the last commit. The second was not, and it is the worse
one. `Ordinal::child` has two callers that mean opposite things by it: the
oversized cutter carving `#1` into pieces, and `mint_between` naming a se'if
upstream inserted after `#1` — the only name that sorts between `#1` and `#2`.
Both are spelled `#1.1`. A prefix test says yes to both, so every comment ever
written on se'if 1 shows on a se'if that did not exist when they were written.
Not a missing link — an **invented** one, which is rule 6 with the sign flipped,
and the same defect reached notes and highlights through `Note::insert_after`,
where the anchors are yours and nobody else has a copy.

**What separates the two is that a cut deletes its parent.** `import::assemble`
says so where it does it — *"The parent id is not written to disk: it is not a
segment any more"* — and `mint_between` is handed a `low` that kept its name and
is still on the shelf. So the shelf already knew which event minted a name, and
it needed no new file to say it:

> An ancestor names a descendant's words only if the ancestor is **not itself
> live**. Walk up, and stop at the first name still on the shelf.

Stopping matters as much as walking. `#7` cut into `#7.1` and `#7.2`, then a
se'if inserted after `#7.2`, is named `#7.2.1`: its parent is live, so the walk
stops there and `#7` does not reach it either — correct, because those words
were never in `#7`.

`girsa_corpus::standing::Standing` is a place under every name its words have
carried: the ancestors it was carved out of, and the dead names `redirects.jsonl`
points here, walked **backwards** — *which old names lead to where I am*, rather
than the forward walk `covered_by` uses to find text. One set, built once per
question, and one membership test over it. The six consumers that each had their
own idea of coverage now ask it, and a bare live id is no longer something any of
them can hand to the ancestry-only test.

**And a second defect underneath it.** `Open`'s segment → position map was a
`HashMap`, and `SegmentId`'s `Hash` takes in the section path where its `Ord`
does not — because the path is descriptive and the ordinal is the durable name.
So an anchor written before upstream re-sectioned a work, which is the case §3
exists for, looked up as **absent**. That map was also what decided whether a
name was live, so the first version of this fix passed every synthetic test and
still leaked links onto inserted se'ifim: the parent looked absent, so the
insertion looked like a cut. It is a `BTreeMap` now, and what caught it was the
test that re-imports a real work over itself rather than asserting the rule
against a fixture built to agree with it.

Measured over `corpus/` — `cargo run --release --example measure-standing -p
girsa-app`, the four Shulchan Arukh volumes, 200 lines each:

| | Orach Chayim | Yoreh De'ah | Even HaEzer | Choshen Mishpat |
|---|---|---|---|---|
| edges tested | 759,000 | 740,600 | 432,400 | 1,018,000 |
| the old predicate | 13.6 ms | 20.1 ms | 6.8 ms | 18.9 ms |
| the new one | **8.9 ms** | **13.2 ms** | **4.2 ms** | **12.7 ms** |
| links found, old → new | 262 → 262 | 311 → 311 | 425 → 425 | 552 → 552 |

**The same answers, and about a third faster.** The same answers because nothing
on the shelf has been re-segmented yet: 0 inherited names across the 800 lines
sampled, which is the redirect table being empty, which is what the previous
section's `newly minted 0` already said. This is a fix for the next import, not
this one — and it is checked in *before* that import rather than after somebody
notices a comment on a se'if that did not exist.

Faster because `Anchor::covers` compared the work slug **twice** for every edge
it tested — once in `Anchor::covers` and again inside `SegmentId::covers` — and
the set lookup compares it once. Building the `Standing` is 229 µs for 200
lines, about a microsecond each.

### What the panel is actually waiting for

Measuring the above turned up something with nothing to do with it: opening the
links panel on a line of Orach Chayim costs **524 ms warm and 2.2 s cold**. The
first attribution was *"70% of it is inside `read_back`"*, which is true and
useless — `read_back` covers a 27 MB `read_to_string`, 159,273 JSON parses,
318,546 segment-id parses and a `Repaired` built for every row. Naming the
function is not naming the cost. Split it apart
(`--example why-the-panel-waits -p girsa-link`, cold, Orach Chayim's 159,273
inbound rows):

| | | |
|---|---|---|
| read off disk | 59 ms | **3%** |
| JSON → `Row` | 356 ms | 16% |
| `Row` → `Edge` | 835 ms | 38% |
| `repairs.apply` | 938 ms | **43%** |
| the filter | 15 ms | 1% → **63 rows kept of 159,273** |

**The disk is 3% of it.** The pipeline reads everything, parses everything,
decorates everything, and then keeps four hundredths of one percent. Two things
stand out:

- `repairs.apply` is the largest slice **on an empty repair layer** — a reader
  who has never judged a link. It builds `format!("{} → {}", from, to)` for every
  edge to look up a map with nothing in it (297 ms and 16.3 MB of throwaway key
  on its own), deep-clones every `Edge` to serve the rare case where a repair
  changed one, and allocates two `Vec`s per row it is about to discard.
- `Row` → `Edge` is 318,546 `SegmentId::from_str` calls, each allocating a work
  string, a path vector and an ordinal vector. Sixty-three of them are wanted.

So three things, in the order they cost:

**The repair layer stopped charging for repairs nobody made.** `Repairs::about`
built its `format!` key before discovering the map was empty, and `Repairs::over`
cloned every `Edge` to fill a field that stays `None` unless a repair applied.
Both now check first. A reader who has never judged a link — which is every
reader on their first day — pays neither.

**Nothing is built out of a row until the row might matter.**
`girsa_link::store::Landing` gates the raw text: the ordinal spelled as a row
spells it, `#7"` and `#7-`, which `#7.1` and `#17` cannot satisfy. It is
**deliberately generous** — it searches the whole line rather than picking out the
`to` field, because a Sefaria section name can carry an ASCII `"` and scanning to
a closing quote would stop early and drop rows. So the other end's ordinal can
admit a row too, and that row is parsed and then rejected on the merits. A false
positive costs one parse; a false negative loses a link, and only one of those is
recoverable.

**And the links you moved by hand still arrive.** This is the part that makes the
gate safe rather than fast: `Repair::Reanchored` puts an edge somewhere its
stored ends do not mention, so filtering on stored text alone would silently drop
exactly the links a reader placed themselves. Every re-anchored edge's filed name
is fed back into the gate. `a_link_you_moved_by_hand_is_not_lost_by_the_thing_that_skips_rows`
fails if that loop is removed — checked by removing it.

| | Orach Chayim | Yoreh De'ah | Even HaEzer | Choshen Mishpat |
|---|---|---|---|---|
| a line, before | 2667 ms | 1035 ms | 437 ms | 1114 ms |
| a line, now | **311 ms** | **125 ms** | **76 ms** | **153 ms** |
| | 8.6× | 8.3× | 5.7× | 7.3× |
| links found | unchanged | unchanged | unchanged | unchanged |

Absolute numbers on a loaded laptop, so the ratios are the reliable half; the
link counts are identical either side, which is the half that had to be.

**And then the file stopped being read whole.** Everything above was still paying
95 ms a line just to get Orach Chayim's 27 MB off the disk, and nothing done to
rows already in hand gets under that. So `inbound.jsonl` is now **sorted by where
its rows land** — runs first, then points in landing order — with a small index
beside it:

```jsonl
corpus/links/shulchan-arukh/orach-chayim/inbound.landing
{"runs":352104}
{"at":[1],"from":352104,"len":1871}
```

Sorting is what makes the index small. Rows landing on one segment become
contiguous, so there is one entry per **distinct landing place** — 4,171 against
159,273 rows — and a lookup is `binary_search` over a slice in memory rather than
a hand-rolled seek over a file, which is the kind of thing that goes subtly wrong
and loses links quietly. The runs sit in a block at the head because a run covers
what sorts between its ends and so lands on places it does not name; there is no
ordinal to file it under, so all 1.3% of them are read every time.

| a line | Orach Chayim | Yoreh De'ah | Even HaEzer | Choshen Mishpat |
|---|---|---|---|---|
| before | 1753 ms | 975 ms | 368 ms | 1184 ms |
| after | **26 ms** | **36 ms** | **23 ms** | **74 ms** |
| | 68.7× | 26.8× | 16.1× | 16.1× |

816 links on Orach Chayim's twenty sampled lines, which is what every run before
it said too. Over the whole shelf the pass took **125 s for 5,317 works and
845,274 landing places**, and `find corpus/links -name inbound.jsonl | cat | wc -l`
reads 4,131,100 rows before and after — the sort refuses to write a file it would
have shortened, counted rather than trusted.

**Two read paths, and only one of them can be wrong about anything.** The index
is a cache of a cache (§4.1): missing, or disagreeing with itself, and the text
gate does the work instead. And the index knows where rows land *as stored*,
which a hand-re-anchored edge is precisely not — so a reader who has moved a link
takes the gate over the whole file, which finds it. Slower for them; the same
answers for everyone, because both paths hand what they find to the same `names`
test.

### A chaburah is a list, and the order is the chaburah

A folder holds **members, not copies**, and a member is one of the three things
the library already has names for — a place, a sefer, or a saved query:

```
thursday             חבורה יום ה              3
    משנה ברכות girsa:mishnah-berakhot/1:1#1
    מאימתי
    ? מאימתי
```

One string each, so the file is greppable: searching `collections.jsonl` for a
segment id finds the chaburos that line is in. There is deliberately **no note
member** — a note is a sefer, and giving it a second kind of membership would be
the first crack in the claim above.

The list is never sorted. The sequence a shiur goes in is the content of the
shiur.

### A saved query keeps the asking, not the answer

The corpus grows and your own seforim go on the shelf, so *every place the
Rambam is called on in Hilchos Tefillah* is a different list next year. What is
kept is the line you typed — sigils and all, since §9.5's sigils are half the
search — plus the chips as the `chip → key` pairs the row itself sends, plus the
seforim the scope came to. Recalling one sets the chips back through **the same
function a click goes through**, so a recalled query and a clicked chip cannot
come to mean different things.

Two honest edges: a scope narrowed by three facet clicks comes back as one
clause over the same seforim — it matches the same segments and no longer
remembers the three clicks; and the link-type scope of W14 is not saved at all.

### What this does not do

- **No sync.** `spec.md` §11 offers *optional, off by default, encrypted sync of
  the personal layer only*. Every word of that is a runtime network dependency,
  and `BUILDER.md` §0.1 says that is not a decision a work order takes on its
  own. **This one is for you to rule on.** What is built instead is the half
  that needs no ruling and that §11 names first: it is all plain files, and
  `girsa-notes export` puts them where you can copy them.
- **A note is not searchable until the index is rebuilt.** Being a sefer is
  enough for the indexer — pointed at a layer holding one note it reads it like
  anything else, and the search finds the paragraph that was written *between*
  two others:

  ```
  $ girsa-index find index personal "ובאמת כבר עמדו"
  1 in 4 segments · showing 1

  girsa:note/מאימתי/2.1#2.1  [text]
    [ובאמת] [כבר] [עמדו] בזה

  narrow by:
    shelf      שלי 1        author  shaul 1        sefer  מאימתי 1
  ```

  But **nothing rebuilds the index when you write one**, and a 5,000,545-segment
  rebuild is four minutes. Until tantivy is written to incrementally, your own
  writing is on the shelf and in the search only as of the last build, and that
  gap is real.
- **Tags are not yet a way in.** They are counted across the whole layer and
  shown, and clicking one does not narrow anything.
- **A note's own words are not linkified.** W19's linkify runs over Ksav
  documents; a citation typed into a note is text.
- **Nothing merges two people's layers.** Corrections have `girsa-fix merge`;
  notes, marks and folders do not, and two copies of `personal/` are two copies.

## The chain

### Direction is time, and the graph does not have any

`spec.md` §8.6 asks for four things, and every one of them has a direction in
it: *forward from a Gemara to how it became halacha; backward from a ruling to
where the posek got it; the path between two texts; and where two rishonim read
one Gemara into incompatible halachos.*

The graph has no direction. §8.2 stores an edge **once, in the shard of the work
it points from**, and which end that is was settled by whoever wrote the row.
Counted on the corpus here:

```
bavli/berakhot        → its commentaries    51,927 edges    earlier → later
mishnah-berurah       → shulchan-arukh      18,806 edges    later → earlier
shulchan-arukh o.c.   → turei-zahav          3,315 edges    earlier → later
shulchan-arukh o.c.   → tur                    719 edges    later → earlier
```

Two of those run one way and two the other, and the Shulchan Arukh does both.
Following arrows walks the first chain forwards, the second backwards, and calls
them the same thing. So a hop is forward when the sefer at the far end was
**written later** — [`girsa-corpus/src/era.rs`](crates/girsa-corpus/src/era.rs)
is the only thing that answers that, and it is the only place that is allowed to.

### The era code cannot make the hop the whole feature is for

Sefaria stamps an era on 4,812 of the 7,189 works — `T` `A` `GN` `RI` `AH` `CO`
— and it is too coarse by exactly one step. **The Shulchan Arukh and the Mishnah
Berurah are both `AH`.** On era codes alone, the most-asked hop in halacha is
two contemporaries and the chain stops before it starts.

`comp_date` is on 5,294 works and is a real year, so it carries the ordering the
era loses; it also reaches Tanach, which has dates and no era code at all. The
rule is **years first, era only where there are no years, and `Unknown` where
there is neither** — never an era stretched into a year range, because the
conventional span of *ראשונים* differs by a century between authorities and a hop
ordered on that is a claim nobody wrote down.

Six shapes of date in the corpus and all six are read, including the fifty
written `ה' תרלז - ה' תרלז (בקירוב)` — 5,637 anno mundi, 1877 CE. Those fifty are
Otzaria-side acharonim, which is the layer a halachic chain *ends* at, so
dropping them would shorten exactly the traces this is for.

**88.7% of the 4,182,337 edges point at a work that can be placed in time**
(78.2% on era codes alone). The other 11.3% are not walked, and are counted where
they were refused rather than quietly skipped — a chain that dropped what it
could not date would look shorter and surer than it is.

### Half of every link is stored at the far end

`what does this se'if answer to` is a question about edges that are **not** in
the Shulchan Arukh's own file: the Mishnah Berurah's shard holds all 18,806 of
them. Until now the sidebar answered it by reading the shards of every work the
companions cache listed — a few dozen files, and quietly capped, since
`girsa-companions` keeps the top 200 works per sefer and Berakhot is joined to
1,600.

So the graph is walked once more and each edge is written a second time, into the
file of the work its far end lands in:

```
$ cargo run --release -p girsa-link --bin girsa-link-types -- corpus personal
two caches written beside the edges:
  shards read        5790
  edges              4182337
  type rows          3637524   (both ends of each, deduplicated)
  inbound rows       4131100   (51237 skipped — both ends in one work, whose own shard holds them)
  took               139s
```

Identical rows to `edges.jsonl`, read back by the same reader, so the two halves
of a segment's links cannot come to mean different things. An edge whose two ends
are in one sefer is **not** written here — its own shard has it, and a caller
reading both files wants their union to be each edge once.

`personal` is there for the second cache. An edge's type is what the corpus
shipped **plus what you have said about it** — that is what `girsa_link::Repairs`
means in the link panel, which shows your type the moment you set it. The masks
were built from the shipped label alone, so a reader who retyped an edge saw the
new type in the sidebar and searched by the old one: one question, two answers,
and the facet was the one that could not be argued with. Leave `personal` off and
the masks are the shipped answer, which is still true — the run says so in a
sentence rather than leaving you to notice.

A hop is then two file reads, cached for the life of a walk. A three-deep trace
out of the first mishnah of Berakhot reads 8 works and takes 1.6 seconds; the
same walk over the companions scan would open several hundred files, some of
them 16 MB. The links panel now reads the same cache and has lost its 200-work
cap with it.

### A chain of *connected somehow* is not a chain

Every edge type present in the graph, counted:

```
2,123,215  comments-on   50.8%
2,048,326  references    49.0%   ← "these two are joined", and nothing further
    7,812  paraphrases    0.2%
    2,984  quotes         0.1%
```

There is no `codifies` and **there is no `disputes` anywhere in it.** So half of
any long chain is built out of links that say only that two places are connected,
and a walk that drew those the same as a commentary would be manufacturing
scholarship. Each chain carries its weakest hop, and the answer says so out loud:

```
$ girsa-chain corpus personal back girsa:mishnah-berurah/58:1#1496 --depth=2

back from משנה ברורה 58:1  [1875–1905]
  (א) זמן ק"ש - וברכות ק"ש לפניה ג"כ אין לומר קודם הזמן …

  └ שולחן ערוך, אורח חיים 58:1  [1563]   (comments-on, the corpus said `commentary`)
    └ טור orach_chayim:58:1  [1300–1340]   (comments-on, the corpus said `commentary`)
      └ ספר מצוות גדול positive_commandments:19:1  [1243–1247]   (references, the corpus said `ein mishpat / ner mitsvah`)

13 chains, 3 of them a transmission all the way — the rest pass through a
link that only says the two are connected somehow, which is 49% of this graph.
not followed:
       54  the other way in time, which is the bulk of any graph
        7  written at the same time, so neither came from the other
       26  no date and no era in either corpus, so which way the hop goes is not known
      315  dropped by --width, best first
```

The last paragraph prints every time, including when it is empty. *Twenty-six of
the seforim that read this line could not be dated* changes what the thirteen
chains above it mean, and it is part of the answer rather than a diagnostic.

`path` keeps the same distinction one level up. A search that runs out of budget
reports **`not found within N hops`**, which is not the same sentence as *there
is no path*; only a search that exhausted everything reachable from both ends
says the second. Two-sided, because a daf of Gemara has tens of thousands of
links and a one-sided walk spends its whole budget on the first two hops.

### Where two readings were argued out later

Break analysis is the one thing in §8.6 the corpus cannot actually do. Nothing in
4.1 million edges says two seforim disagree — there is no `disputes` edge. What
the data *can* say is that two of them read the same line and that a later sefer
had to deal with both, which is the shape a machlokes leaves behind:

```
$ girsa-chain corpus personal fork girsa:bavli/berakhot/2a:1#1 --width=25

  1 pair read this line and is later cited together. Nothing here says they
  disagree — the corpus has no `disputes` edge anywhere in it. This is where to look.

  רש"י על ברכות 2a:1:2  [1065–1115]
  תוספות על ברכות 2a:1:1  [1150–1350]
      both cited by רשימות שיעורים על ברכות 2a:69  [1909–2011]
```

Rashi and Tosafos on the first mishnah of Berakhos, and the sefer that takes them
both up. It is offered as a place to look and never as a finding, and a pair with
an edge joining the two directly is marked as such — one of them may simply be
answering the other, which is a different thing.

### What the chain does not do yet

- **It is a command, not a panel.** `girsa-chain` prints all four; nothing in the
  window draws them.
- **A fork is one hop wide on each side.** Two readings joined through an
  intermediate sefer are not found, and the ones that are found are bounded by
  `--width` with the drop counted.
- **Nothing walks into your own layer's dates.** A note has no `comp_date`, so it
  is `Unknown` against everything and is never a hop — which is the truthful
  answer, and not a useful one.
## Something like this, but not the words

`spec.md` §9.9 asks for one thing the literal index cannot do: *I remember a
Rishon who says something like this but not the words.* It was the last thing in
the spec still unbuilt, and it was unbuilt because every route to it crossed a
line `BUILDER.md` §0.1 says a work order may not cross alone — a model fetched at
runtime, a licence that is not this repository's, and 5,000,545 segments to
embed before it answers anything. **Ruled** (§16 #20) and now built.

### The licence disagreed with itself, so it was checked

§9.4's candidate table called BEREL *unrestricted*. This README warned it carries
its own terms. Those are not the same claim, and W30's first instruction was to
settle it before writing a line. Checked three ways on 29 July 2026 — the model
card, its YAML frontmatter, and the Hub API's metadata for `dicta-il/BEREL_2.0`,
which redirects to **BEREL 3.0**: **`apache-2.0`**, with a request to cite the
paper. That is one of this repository's own two licences.

### What it does, and what it does not

This is the part worth reading, because the honest answer is narrower than the
feature sounds. BEREL is a **masked-language model, not a sentence encoder** —
nothing in its training gave it a similarity objective — and it shows. 240
se'ifim of Hilchos Tefillah, embedded, asked 22 questions with a known right
answer, scored by where the right se'if landed:

| asked as | rank 1 | top 5 | top 10 | worst |
|---|---|---|---|---|
| **a half-remembered statement** (10 pairs) | 8 | 9 | 10 | **16 of 240** |
| **a question about the se'if** (12 pairs, 5 sharing no word) | 1 | 1 | 1 | 97 of 240 |

*I think it says the drunk may not daven because he has no kavanah* — none of
`נשתכר`, `ביין`, `יעמוד` or `דעתו` is in the se'if — comes back **first**, out of
240. *How late may one daven shacharis?* comes back twenty-fourth.

So the lane's box asks for **a line as you remember it**, which is §9.9's own
sentence, and does not pretend to answer questions. And the standard repair for
a raw BERT — subtract the mean of the space, since every sentence sits in a
narrow cone — was **tried and made it worse** (24→40, 97→123, 9→24). It is
measured in `examples/measure.rs` and it is not built. A plausible improvement
that does not survive measurement is exactly what §9 exists to refuse.

The measurement is why the side-loading matters more than it looks. Nothing in
`girsa-lane` is BEREL-specific: it reads a `config.json`, a `tokenizer.json` and
a `model.safetensors`, runs the forward pass in Rust on the CPU, and stamps the
store with a fingerprint of what made it. The day a contrastively trained
rabbinic-Hebrew encoder exists, that is **a setting and not a release** — point
the lane at it, re-embed, nothing to migrate. The same *make it reversible rather
than permanent* move W26 made for OCR.

### Off is off, and the numbers say what is missing

Four things hold, and each has a test that fails against a deliberately naive
version of it:

- **Off means off.** Not *a mode that returns nothing*: with the lane off no
  model is loaded, no vector is read, and the whole corpus tree is
  **byte-for-byte identical** before and after a run — asserted by comparing
  every file under `corpus/`. Everything the lane writes is in your own layer.
- **The absence has words.** The lane turned on with nothing to run says
  *"the semantic lane is on but cannot run: no semantic model is configured…"* in
  the search header. A reader who turned it on and got nothing is owed the reason
  rather than left to conclude the corpus has nothing like their query in it.
- **Every answer states its own coverage.** *"this lane covers משנה תורה, הלכות
  תפילה — 240 segments; 7,190 other seforim on this shelf aren't in it."* Found,
  empty, refused or off, the sentence is drawn — composed once in Rust so the
  window, the CLI, the MCP tool and the test cannot drift. A sefer half-embedded
  reports **both** numbers, for the reason a scan stopped at page 40 of 302 does.
- **And every answer states what the lane was measured to do** — added 6 August
  2026, and it is a correction rather than an addition. The sentence *"measured
  on a half-remembered statement, and it works poorly on a question. It does not
  pasken"* existed, in exactly one place: the MCP tool description. **A robot was
  told and the reader was not.** It is `girsa_lane::MEASURED` now, one string,
  drawn in the window under every answer and read by the MCP surface from the
  same constant.

  It gained a clause it never had: **over 240 se'ifim, not over the whole shelf.**
  Every number in the table above is at n=240. A 0.11 cosine margin — 0.74 for the
  right answer against 0.63 for unrelated se'ifim — is a different claim at 240
  candidates than at 5,000,545: at 240 the tail is empty, and at five million the
  tail *is* the answer set. Nothing measured says which way that goes. It may
  hold; nobody has looked. Re-running `examples/measure.rs` over ~50,000 segments
  would replace that clause with a number, and it is the one afternoon this
  feature still owes.
- **The offer of the whole shelf says what it costs.** `Chosen::everything()` is
  a first-class standing choice with a tested branch in the coverage sentence, so
  the thirteen days were being offered as an equal option to the 54 seconds the
  measurements came from — with the thirteen days written down in a module note
  the reader never opens. The sentence now spends the measured throughput:
  *"this lane covers the whole library — 1,200,000 of 5,000,545 segments so far,
  about 13 days of embedding left."*
- **It is never a rung on the ladder.** `girsa-search` does not depend on
  `girsa-lane`, so no relaxation rung can reach a vector even by accident — and
  adding a `Rung::Meaning` variant does not compile, which is the proof. Every
  rung is priced before the click; an embedding neighbourhood cannot be, and a
  chip with a made-up number on it is the one thing §9 forbids.

And two vectors from two models rank against each other perfectly happily, which
is the failure mode a reader could never spot from the results — so the store's
header records **which model made it** and a different one opens it empty, says
whose it was, and refuses to add to it until you ask for a restart.

### 4.5 segments a second, which is why you choose

Release build, one CPU, batches of sixteen: **54 seconds** for Hilchos Tefillah,
about **thirteen days** for all 5,000,545 segments. That is the whole argument for
§16 #20's *the corpus is yours to choose* — a lane that insisted on the library
before it answered anything would be a feature nobody ever switched on. So: a
sefer, a section, a shelf, your own notes, or all of it; added to whenever;
resumable, because the vectors on disk **are** the progress record; and on its own
thread sharing the one loaded model, so reading never waits on it.

### The button, and what it cost

The first form of §16 #20 said Girsa fetches no model at all — you point it at
one. Mid-order that was amended: the folder picker stays the default path, and a
**bring in BEREL** button sits behind a setting that is off in a fresh install.
With it off there is no code path from anywhere in the application to the
network, and `bring()` refuses even if something calls it. What §14 now promises
is *Girsa never **needs** the network*, which is the sentence that was worth
keeping; nothing is vendored either way, because the weights land in your own
layer beside your notes rather than in this repository (T7).

The download is resumable **inside** one file — `Range: bytes=N-`, appended to a
`.part`, length-checked at the end, renamed into place only when whole. The
corpus fetcher gets away with per-file atomicity because its files are a few
hundred kilobytes; one of these is 738 MB over a domestic line, and a fetcher
that started again from zero on every dropped connection would never finish.

## A document of yours, with its shape

### Two thirds of a .ksav was not on the shelf

W19 put your writing on the shelf, and it read the file for its **words**:
commands off, contents of the brackets kept. Run against Ksav's own sample
document that turns out to lose more than the formatting.

```
#כותרת1[מבוא]                      → "מבוא", as body text. Not a heading,
                                       so not a level of the address either.
#רשימה(פריט[א], פריט[ב])            → nothing at all
#טבלה(עמודות: 2, תא[א], תא[ב])      → nothing at all
סוף#הערה[הערת שוליים].              → "סוף הערת שוליים ." — the note spliced
                                       into the sentence, the full stop orphaned
```

A list's items and a table's cells live in the command's **arguments** — Ksav
writes `#רשימה(פריט[…], פריט[…])`, not `#רשימה[…]` — and the reader skipped
arguments because arguments are usually settings. So every list and every table
in a document of yours was absent from the shelf and absent from the search,
and it did not read as a loss: it read as a document that never had a table in
it. That is the silent gap this project refuses everywhere else.

### The reader knows which commands are structure

`girsa-ksav` now reads a document into blocks — heading, paragraph, quote, list
item, table row, footnote — and `to_text` is that reading rendered flat, so
there is one parser and not two. It is still a **reading and not an
evaluation**: Typst is the only thing that can say what a document *renders*
as, and putting the compiler inside the library to shelve a paragraph is not a
trade worth making.

Of Ksav's 104 commands it knows the forty that are structure. Everything else
is inline and its content is kept, so a new style command in Ksav needs no
change here and **cannot lose a word by being unknown** — which is the
behaviour that let the old reader lose the tables in the first place.

Ksav nests without limit; the engine ships an example 25 lists deep and a table
inside a footnote inside a table cell. The blocks come out **flat, in reading
order**, because a segment id is a path of levels and a citation is a range over
them — a faithful tree would be a tree nothing here can address. An item carries
its depth, a note carries the number left behind in the text, and a table inside
a footnote emits its rows after that footnote. What is lost is the shape of the
containment, and it is lost in one stated place rather than by omission.

### Headings are the address, and a footnote is its own line

On the shelf the blocks become segments, and two of them are new kinds:

```
heading  girsa:user/sample/מבוא:רשימות_עם_קינון_עמוק#5    רשימות עם קינון עמוק
item     …:רשימות_עם_קינון_עמוק:1#6                      פריט ראשון פשוט.
item     …:רשימות_עם_קינון_עמוק:4#9                       2. שלב ב, עם הדגשה והערת שוליים1
note     …:רשימות_עם_קינון_עמוק:5#10                     1. הערה בתוך פריט בתוך רשימה
row      …:טבלה_עם_הערה:1#17                             מונח  הסבר  מקור
row      …:טבלה_עם_הערה:2#18                             קינון  הכלה של מבנה  ראה הערה2
note     …:טבלה_עם_הערה:4#20                             2. טבלה בתוך הערת שוליים בתוך תא
row      …:טבלה_עם_הערה:5#21                             פנימי א  פנימי ב
```

**Headings are levels of the address**, the same way Otzaria's `<h1>/<h2>/<h3>`
are (W7) and for the same reason: a chaburah with three chapters should be cited
as `girsa:user/חבורה/ראיות:2#9` and not as line 47.

**A footnote is its own segment**, immediately after the line that carried it,
with the marker still in that line's words. That is the whole difference between
a footnote and an interruption — and it is what makes a note searchable, citable
and correctable on its own, like any other line in the library.

**An editor's note is not a line of the sefer at all.** `#הערת_עורך` is a remark
*about* the text and was never part of it — the same distinction W20 draws
between a correction and a girsa variant — and importing one would put a
note-to-self into the corpus as though the author had written it.

`SegmentKind` gained `note`, `item`, `row` and `quote` beside `text`, `heading`
and `page`. Nothing in the corpus has the new ones: Sefaria and Otzaria give
text and headings and nothing else, so **no sefer but yours changes shape**. The
page draws each as itself — a row's cells in columns, an item indented by its
depth, a note in small type, a quote against a rule.

### What a document does not carry yet

- **The containment is flat.** A list inside a list item is two items at two
  depths and not a tree; you can see the nesting and you cannot fold it.
- **Nothing writes back.** Reading a `.ksav` into segments does not make the
  shelf able to *edit* one; the file is still the truth and Ksav is still what
  writes it.
- **A table has no header row unless it says so.** The header is the run of
  `כותרת_תא` cells, and a table written entirely of `תא` has none — which is
  what the document said.
- **A note's own words are not linkified**, the same gap W27 has for notes.

This is a change to `girsa-ksav`, which both applications compile, so the shared
crates went to **0.4.0** and both pins moved with them — the coordinated release
W1 exists to make routine. Ksav's engine suite is green against it.

## Answering a program

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

## Licence

MIT OR Apache-2.0 — see [`LICENSE`](LICENSE). Forced by crate-sharing with
Ksav. No corpus text is committed here; texts are downloaded at first run and
each carries its own source and licence.

What is bundled *into* the installer and is not ours is listed in
[`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) — today that is pdf.js, which
draws a page of a scan and reads the words off one. Tesseract is **not** in that
list on purpose: it is found on the machine if it is installed and run as a
separate process, so nothing of it is linked into this program or shipped with
it. No AGPL or GPL code is used anywhere here: Zayit, HebMorph and
Sefaria-ElasticSearch were read as prior art and copied from nowhere
(`BUILDER.md` T7).
