# Building it, and checking it

*← [The shape of it](the-shape-of-it.md) · [The record](../the-record.md) · [The shelf, and searching it](the-shelf-and-the-search.md) →*

---

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
**24** checks that read this repository's own source and fail when
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

---

*← [The shape of it](the-shape-of-it.md) · [The record](../the-record.md) · [The shelf, and searching it](the-shelf-and-the-search.md) →*
