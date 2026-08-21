# Where the seforim come from

**Girsa ships no seforim.** The installer is 7 MB; a full shelf is about 15 GB.
This page is where each part of that comes from, what it costs, and what its
terms are — because "download it yourself" without a *from where* is not an
instruction.

## The whole thing, in one command

```sh
node tools/build-a-shelf.mjs corpus --download-otzaria
```

Sefaria fetched, Otzaria's library downloaded and unpacked, both imported onto
permanent ids, the link graph built, the caches that read it backwards, and the
search index. About two hours, mostly the index, and **~15 GB** at the end.

Add `--otzarlib` to clone [OtzarLib](#otzarlib-and-the-tool-that-lays-it-out)
and lay it out as well — it prints that collection's terms when you pass it.
`--dry-run` prints every command and runs none of them. `--skip-search` stops
before the long step.

**Every step skips itself if its output is already there**, so an interrupted
run is resumed by running it again, and any step that fails ends the run rather
than leaving a shelf with a hole in the middle.

```sh
node tools/build-a-shelf.mjs --help
```

The rest of this page is what that command is doing, source by source, for when
you want to do it yourself or when something goes wrong.

---

You do not need all of it. Sefaria alone is a working library. Each row below
adds to the shelf and none of them is required by another.

| | What it is | Size | Fetched for you? |
|---|---|---|---|
| [Sefaria](#1-sefaria) | Tanach, Shas, Mishnah, Rambam, Shulchan Arukh, and the commentary Sefaria has digitised | 3.4 GB | **yes** |
| [Otzaria's library](#2-otzarias-library) | ~6,600 `.txt` seforim, heavy on acharonim | 1.3 GB zipped | no |
| [Other `.txt` libraries](#3-other-txt-libraries) | anything else laid out the same way | varies | no |
| [Your own](#4-your-own) | your PDFs, your notes, your typing | — | n/a |

---

## 1. Sefaria

```sh
girsa-fetch corpus\sefaria
```

Reads `gs://sefaria-export` — Sefaria's own public export bucket — and takes the
Hebrew `merged.json` of every text, every schema, and the link CSVs. English and
the two `cltk-*` formats are skipped, which is most of what makes it 3.4 GB
rather than very much more. **Resumable**: interrupt it and run it again.

The schemas are the part that matters and the part nothing else has. Otzaria has
a line that *says* `סימן א`; Sefaria's schema knows what a siman **is** — how
many this sefer has, what is inside one, and at which level a commentary
attaches. That is why Sefaria is the spine and everything else fills in around
it.

**Terms:** Sefaria publishes per-text licences in the metadata, most commonly
CC-BY or public domain, and Girsa records what each text says about itself.
Sefaria's own terms are at <https://www.sefaria.org/terms>.

## 2. Otzaria's library

The big one, and the reason Girsa reads `.txt` trees at all.

**Download:**
<https://github.com/Sivan22/otzaria-library/releases/download/latest/otzaria_latest.zip>
— 1.28 GB zipped. Unzip it anywhere; the folder you point Girsa at is the one
containing `אוצריא/` and `metadata.json`.

```sh
girsa-import      corpus C:\Downloads\otzaria_latest
girsa-link-import corpus C:\Downloads\otzaria_latest
```

It is the library behind the [Otzaria app](https://github.com/Sivan22/otzaria),
and it carries roughly a thousand seforim Sefaria has no text for at all — the
acharonim layer, which is disproportionately what you actually reach for at
11pm.

**Terms: it does not state any.** `Sivan22/otzaria-library` has no `LICENSE`
file, no SPDX licence, and no terms in its README. The Unlicense you may have
seen belongs to `Sivan22/otzaria`, the *application*, and does not travel to the
seforim. Girsa records the edition and where it came from, and **records no
licence**, because inventing one would be worse than leaving the question open.

> Girsa recognises this library without being told — by the `metadata.json`
> sitting beside its `אוצריא/`, which nothing else has. Every other tree
> declares itself; see below.

## 3. Other `.txt` libraries

`girsa-import` takes **as many libraries as you name**, in order:

```sh
girsa-import      corpus <otzaria> <another> <a-third>
girsa-link-import corpus <otzaria> <another> <a-third>
```

A title an earlier library supplied is not read again from a later one, and
Sefaria outranks all of them — so a second library adds what it alone has and
quietly declines to argue about the rest. Give both commands the same list in
the same order.

**A library is any directory holding an `אוצריא/` folder of `.txt` files.**
Inside it, Girsa reads two shapes, and picks per file by what the file carries:

- **headings** — `<h1>` the sefer, `<h2>` a perek or a daf, `<h3>` a siman;
- **an address written in words** at the head of a line —
  `שו"ת מהר"י בן לב חלק א סימן א` — which is how Bar-Ilan and DBS exports state
  their structure.

If a `links/` folder sits beside `אוצריא/` with `<sefer>_links.json` files in
it, those are imported too.

### Saying where it came from

Put a `library.json` at the root:

```json
{
  "edition": "OtzarLib",
  "provenance": "https://github.com/gwngdwl/seforim",
  "license": "CC-BY-4.0"
}
```

`edition` is required; the other two are not. **Leave `license` out when you do
not know it.** A blank is a thing a reader can act on and a confident wrong
licence is not — and a tree with no `library.json` at all simply has nothing
recorded for it, which is a perfectly good answer.

### OtzarLib, and the tool that lays it out

**OtzarLib** — <https://github.com/gwngdwl/seforim> — is ~130 files
including the whole Encyclopedia Talmudit with its footnote apparatus, Yabia
Omer, Minchas Yitzchak, Shevet HaLevi, Minchas Shlomo, the Rashba's teshuvos,
Piskei Ri"d, Ravyah, and the Brisker chiddushim. Most of it is in neither
library above.

> **That link is a mirror, and it is the one that exists.** The collection calls
> itself OtzarLib and its own README points at `YairDaniel123/OtzarLib`, which
> is a 404 — the upstream is gone, renamed or private. This repository recorded
> that dead URL as the provenance of 122 seforim for exactly as long as it took
> to run the clone and watch it fail, which is the argument for running a thing
> rather than reading it.

**Read its README before you do anything with it.** It states that parts of its
contents are subject to copyright and are *forbidden for public distribution,
copying or commercial use*, that the files are for private use only, and that
uploading them waives nothing. The repository carries no licence.

Girsa neither fetches it nor ships it, and nothing in Girsa will. Putting it on
your own shelf on your own machine is between you and the terms it came with;
this page tells you the terms exist rather than pretending they do not.

**Getting it is the easy half.** Its files sit under `ספרים/<its own
categories>/`, three are `.docx`, one is a byte-identical duplicate of its
neighbour under a slipped name, and nothing in it says where it came from. And
`girsa-import` shelves a sefer **in the folder it finds it in**, so a heap
imports as a heap.

So there is a tool, and it does the whole of that:

```sh
git clone --depth 1 https://github.com/gwngdwl/seforim.git otzarlib
node tools/lay-out-otzarlib.mjs otzarlib otzarlib-shelf --dry-run   # see the plan
node tools/lay-out-otzarlib.mjs otzarlib otzarlib-shelf             # write it

girsa-import      corpus <otzaria> otzarlib-shelf
girsa-link-import corpus <otzaria> otzarlib-shelf
```

It maps each sefer onto the categories Otzaria's library already uses — teshuvos
into `שות/{גאונים,ראשונים,אחרונים,מחברי זמננו}`, the Encyclopedia into
`ספרות עזר/אנציקלופדיות` beside שדי חמד, rishonim on masechtos into
`תלמוד בבלי/ראשונים`, commentaries on the Rambam into
`הלכה/משנה תורה/מפרשים` — converts the `.docx`, drops the duplicate, flattens
the `links/` sidecars, and writes a `library.json` with **no licence field**.

Two things about it worth knowing:

- **The mapping table at the top of the file is judgement, not mechanism.**
  Knowing that פסקי הרי"ד is a rishon on Shas and that מאמרי המשגיח is mussar
  rather than the "acharonim" drawer it arrived in cannot be derived from a
  filename. It is written down so it can be argued with and re-run, rather than
  living in somebody's shell history.
- **It never places a file it has no rule for.** Anything the table does not
  cover is listed, nothing is written for it, and the run exits non-zero. An
  unplaced sefer is a decision for a person; the one thing this must not do is
  make it quietly.

`--dry-run` prints exactly what it would write and touches nothing.

## 4. Your own

Not a download and not an onboarding step. Drop a PDF, a `.docx` or a `.txt`
into your personal root and it is a sefer on your shelf — citable, searchable,
and linkable to the sugya like anything else. See
[`your-own-layer.md`](record/your-own-layer.md).

---

## After the seforim: the four steps that make them usable

Steps 1 and 2 above put text on the shelf. These make it behave like a library:

```sh
girsa-link-import corpus <library>...        # the links between seforim
girsa-link-types  corpus personal            # the caches that read them backwards
girsa-index build index corpus personal      # search, about 4 GB
```

**Skip the middle one and no mefaresh will ever appear**, on any daf — the
מפרשים button reads `לצד` on every sefer and the panel says *I have not been
told*, which is the honest sentence for a missing cache and is indistinguishable
from a sefer nobody wrote on. It is the step most people miss.

Skip the last and everything still opens; search says why it cannot search.

Then point Girsa at the result: set `GIRSA_CORPUS` to the folder the import
wrote to — the one with `works/index.jsonl` in it. Girsa looks there first, then
beside the executable, then two levels up.

Command reference: [`tools.md`](tools.md). When something refuses:
[`troubleshooting.md`](troubleshooting.md).
