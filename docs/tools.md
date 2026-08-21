# Every command this repository can be told to run

Girsa is a window, and behind it are **16**<!--=bins--> binaries and
**17**<!--=examples--> examples: the ones that build the corpus, the ones that
read the same data on a terminal so a feature can be checked without a window,
and the ones that measure a claim somebody made in prose.

Two numbers rather than one, and neither of them the word *thirty* this sentence
used to open with. Thirty was wrong by one and unfalsifiable — a number spelled
as a word cannot be marked, which is the argument `README.md` already makes
about *"a window and fifty commands"*.

**And then this page wrote them as words anyway, and was wrong for three
weeks.** *Sixteen and fifteen* replaced *thirty*, which is two unfalsifiable
numbers in place of one; the README marked `bins` and nothing anywhere counted
the examples. Two arrived — `measure-branch-citations` and `mishnah-table` —
each in a commit that edited this very page to give the new one a runnable
line, and neither touched the sentence above. They are digits with markers on
them now, re-measured on every push by
`the_numbers_in_the_readme_are_measurements`, and
[`tools/readme-numbers.sh`](../tools/readme-numbers.sh) is what writes them
back.

This page exists because they were not written down. Fifteen of them had never
been named in a runnable line in any document — including `girsa-companions`,
which the links panel used to tell the reader, in Hebrew, to go and run.
`crates/girsa-app/tests/the_documentation_names_things_that_exist.rs` now checks
both directions: every command a document names exists, **and** every command
that exists is named by a document.

Every one of them takes the same shape and answers `--help`. Where a command
takes `[corpus] [personal]`, both default and you can usually leave them off.

---

## Building the shelf

**All of it, in one command, downloads included:**

```sh
node tools/build-a-shelf.mjs <corpus> --download-otzaria [--otzarlib] [--dry-run]
```

It runs the six below in order, fetches what they need, skips any step whose
output already exists, and stops the whole run on the first failure rather than
leaving a shelf with a hole in it. `--help` for the options.

The rest of this section is what it does, for when you want to do it by hand.

In this order, once, before anything else. `docs/start-here.md` is the
walkthrough; this is the list.

```sh
cargo run -p girsa-corpus --bin girsa-fetch       corpus/sefaria         # the seforim
cargo run -p girsa-corpus --bin girsa-import      corpus <library>...    # onto permanent ids
cargo run -p girsa-link   --bin girsa-link-import corpus <library>...    # the links between them
cargo run -p girsa-link   --bin girsa-link-types  corpus personal        # the caches that read them backwards
cargo run --release -p girsa-app --bin girsa-companions corpus           # which seforim open beside which
cargo run -p girsa-search --bin girsa-index build index corpus personal  # the search index
```

`girsa-companions` walks the whole edge graph once and writes
`corpus/links/companions.jsonl` — *which seforim are worth opening beside
which*. Skip it and the shelf offers only the commentaries a **schema**
declares, which for a `.txt` library is none of them, because nothing in a
`.txt` declares a base text. Nothing refuses without it; the מפרשים list is
simply short.

`<library>` is a `.txt` tree with an `אוצריא/` directory in it — the Otzaria
copy you downloaded yourself, which is step 2 of the seven in the README and the
one nothing here fetches for you. The two tools that take it refuse without it.

**[`the-libraries.md`](the-libraries.md) says where to get one**, for each part
of a shelf, with the download link and the terms.

A folder of seforim is not yet a library: `girsa-import` shelves a sefer in the
folder it finds it in, so where the files sit is where a reader will look for
them. For OtzarLib, which arrives under its own categories with three `.docx`
in it, that work is a tool:

```sh
node tools/lay-out-otzarlib.mjs <checkout> <destination> [--dry-run]
```

It maps every sefer onto the categories Otzaria's library already uses,
converts the `.docx`, drops a byte-identical duplicate, flattens the `links/`
sidecars and writes the `library.json`. It **refuses to place a file it has no
rule for** — the unplaced are listed and the run exits non-zero, because that is
a decision for a person.

**More than one is allowed**, and the order matters: a title an earlier library
supplied is not read again from a later one, the same precedence Sefaria has
over all of them. Name Otzaria's first, and give the same list in the same
order to both tools so that a filename two libraries share is looked for in the
one that supplied the text.

```sh
cargo run -p girsa-corpus --bin girsa-import corpus ~/Downloads/otzaria_latest ~/Downloads/otzarlib
```

Each library says where it came from, in a `library.json` at its root:

```json
{ "edition": "OtzarLib", "provenance": "https://github.com/gwngdwl/seforim" }
```

A tree with no such file has **no edition and no licence recorded** for the
seforim in it, which is the honest answer rather than a flattering one. Only
Otzaria's own library is recognised without one, by the `metadata.json` beside
its `אוצריא/`. Omit `license` when you do not know it; a wrong licence on a
sefer is worse than a blank.

One more, and it is a **retrofit rather than a step**:

```sh
cargo run -p girsa-link --bin girsa-link-orient corpus
```

`girsa-link-orient` turns the `comments-on` edges already on disk the right way
round. `girsa-link-import` has oriented as it writes for some time now, so a
shelf built by the list above does not need it — it exists for a store written
before that, where re-reading 672 MB of CSV to fix a resolved field would take
an hour to arrive at the same place. Rebuild `girsa-link-types` afterwards if
you do run it.

**`girsa-companions` used to be listed here beside it, and that was the
mistake.** These two are not the same kind of thing: one is a retrofit for an
old store, the other decides *which seforim are worth opening beside which* and
is needed on every shelf. Filed as optional, it last ran five days before the
seforim it was supposed to pair — and because nothing refuses without it, the
only symptom was a מפרשים list that was quietly short. It is step 6 of the list
above now.

It also writes, per pair, **how many of one sefer's simanim are joined to the
siman of the same number in the other** — which is what makes the Tur a
parallel of the Shulchan Arukh and the Mishneh Torah not one. See
`girsa_corpus::taxonomy::Keeping`. A `companions.jsonl` written before those
numbers existed still loads; it simply offers no parallel seforim until the
tool is run again.

Optional, and only if you want an agent to be able to ask:

```sh
cargo run --release -p girsa-mcp --bin girsa-mcp -- corpus personal index
```

`girsa-mcp` is its crate's only binary, so `-p girsa-mcp` alone runs it and that
is how the README writes it. The long form is here because it is the one that
cannot become ambiguous later.

## Reading the same data without a window

Each of these exists so that a feature can be seen, and tested, without a
running application. That is the whole reason they are here — a window is a bad
place to find out that a mapping is wrong.

```sh
cargo run -p girsa-app    --bin girsa-shelf  corpus personal   # the shelf
cargo run -p girsa-app    --bin girsa-daf    corpus            # the page→daf mapping
cargo run -p girsa-app    --bin girsa-chain  corpus forward <segment-id>  # the transmission chain
cargo run -p girsa-app    --bin girsa-read   corpus personal status   # words off a scan
cargo run -p girsa-desk   --bin girsa-notes  corpus personal   # your own layer
cargo run -p girsa-nearby --bin girsa-lane   corpus            # the semantic lane
cargo run -p girsa-search --bin girsa-suspects index personal  # the OCR queue
```

`girsa-read` is the one worth knowing: `words`, `ocr`, `show`, `status` and
`fix` over a scan you brought, all on a terminal.

## Measuring a claim

These print a number. Every one of them exists because a sentence somewhere
states that number, and a sentence is not a measurement.

```sh
cargo run --release -p girsa-corpus --example measure-continuity   # would a re-import keep every id?
cargo run --release -p girsa-corpus --example measure-oversized    # segments that name a volume, not a place
cargo run --release -p girsa-corpus --example measure-resolver  corpus/lexicon.tsv corpus/sefaria/links   # the resolver against real citations
cargo run --release -p girsa-corpus --example measure-ids          # what reading a work's ids costs
cargo run --release -p girsa-app    --example measure-standing     # what asking the shelf costs
cargo run --release -p girsa-app    --example measure-opening      # what opening a sefer costs the window
cargo run --release -p girsa-lane   --example measure  corpus personal <slug> <model-dir>   # the lane against a real model
cargo run --release -p girsa-link   --example why-the-panel-waits  # where the half-second goes
cargo run --release -p girsa-search --example measure-branch-citations  corpus   # can a branch work be reached by name?
cargo run --release -p girsa-app    --example mishnah-table  corpus   # the Mishnah Yomis table, counted rather than typed
```

`mishnah-table` is the odd one out: it prints Rust rather than a number, and
what it prints is `luach::MISHNAYOS` — 63 masechtos and the mishnah count of
each of their perakim. Run it when the corpus changes and paste the output over
the table; `the_table_is_the_whole_shas` asserts the total is 4,192.

**It exists because that table cannot be written from memory.** Roughly 525
numbers, and one wrong one is a wrong limud for a day with nothing to catch it.
It also carries three traps in its header, each of which yields a plausible
wrong count rather than an error: `categories[0] == "Mishnah"` finds **948**
works, because every commentary on Mishnayos is filed under it; a `mishnah-*`
glob finds **62**, missing `pirkei-avot` and swallowing the Mishnah Berurah's
17,418 segments; and the order is neither alphabetical nor the Bavli's.

`measure-branch-citations` is the newest and answers a question a reader asked
by trying it: **can you type a mekor at a sefer that holds its chalakim inside
itself?** For every work whose schema names its sections it writes the address
of that section's first segment the way a person writes it — `טור אורח חיים סימן
א` — and reports whether it lands back on that segment. It landed on none of
them before `girsa_corpus::sections` learned the way back from a name to a slug.

Eight of the ten take their roots from where you are standing and the other two
do not, which is not a style this page can tidy away: `measure-resolver` scores
the resolver against Sefaria's own link CSVs and has to be told where they are,
and the lane's `measure` needs a **slug to measure and a model to measure it
with**. `<model-dir>` is the side-loaded BERT of spec.md §9.9, which is a setting
on a reader's machine and not a release, and `<slug>` is a work the lane has
actually embedded — one whose name is a path, not a title:

```sh
cargo run --release -p girsa-lane --example measure \
    corpus personal mishneh-torah/prayer-and-the-priestly-blessing ~/berel
```

Both of those lines, and the two below them, printed a usage line and did
nothing until they were given their arguments — the same failure the five
build-the-shelf commands had, one namespace over, because the check that caught
it there read `--bin` and not `--example`. It reads both now.

## Looking at one thing that went wrong

```sh
cargo run -p girsa-link --example why-dropped   corpus   # why a link did not become an edge
cargo run -p girsa-link --example sort-inbound  corpus   # sort every inbound.jsonl and index it
```

`why-dropped` prints the rows rather than the count, which is the difference
between *4,102 links were dropped* and *here is one, and here is why*.

## The Ksav loop, from this side

```sh
cargo run -p girsa-app  --example send             # a source, in its three flavours
cargo run -p girsa-app  --example fixture-packet   # the packet Ksav's test asserts against
cargo run -p girsa-desk --example write  corpus personal <buffer-name> <citation>   # a buffer, written the way the window writes it
```

`write` takes the citation you would have sent, because that is what a buffer is
made of — `cargo run -p girsa-desk --example write corpus personal chaburah "ברכות ב."`
writes one the way the window writes one.

## Rebuilding something generated

```sh
cargo run -p girsa-corpus --example build-lexicon  corpus/sefaria/schemas corpus/lexicon.tsv   # the resolver's lexicon, from Sefaria's schemas
```

It reads Sefaria's schemas and writes the `.tsv` — both named, because it
overwrites the second one and a generator that guessed its own output path is a
generator that overwrites a file you did not name.

Two more are generated and are **not** run by hand: `girsa-card` writes
`docs/shortcuts.md` (the page says so at the top), and `dev-fixtures` writes
`public/dev/*.json` for the browser build, which `npm run dev` runs for you.
They are the two entries in that test's `NOT_FOR_A_READER` list, each with the
reason written out beside it.
