# גִּרְסָא · Girsa

**A Torah library that assumes you are going to write something.**

Girsa (גִּרְסָא, "the text as received") is the page. **Ksav** (כְּתָב, "writing")
is the pen. The pairing is the idea.

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
personal/        yours: how you arranged the shelf, and the seforim you added
```

`girsa-import` rewrites the whole of `corpus/works/index.jsonl` on every run, so
nothing of yours is ever kept in it. The window looks for the corpus at
`GIRSA_CORPUS` and for your layer at `GIRSA_PERSONAL`, else beside the session
file in the app's data directory.

| Crate | Purpose |
|---|---|
| `girsa-corpus` | Storage, ingest, schemas, permanent segment IDs |
| `girsa-search` | tantivy indices, the five modes, the ladder, the chips and the facets |
| `girsa-link` | The typed link graph, repair, later mining |
| `girsa-app` | The reading workspace: the shelf, tabs and splits, and what keeps two columns together |

plus `girsa-source`, `girsa-ref`, `girsa-hebrew` and `girsa-cite` from
`sefer-crates`, pinned to an exact version and resolved from the sibling
checkout during development.

`app/` is the Tauri shell: a window and thirty-seven commands, and **nothing
that decides anything**. Where a pane lands, what may sit beside what, and what the
nikud toggle takes off are all answered in `girsa-app`, because those can be
tested and a webview cannot.

**The sibling checkout has to be present.** Until `sefer-crates` is published,
cloning Girsa alone will not build.

## Build

```sh
cargo build --all-targets
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
```

The shell is its own cargo project — it cannot build until the frontend has
been built into `app/dist`, and the four commands above have to stay quick:

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
so `build` throws the old one away rather than patching it:

```sh
cargo run --release -p girsa-link  --bin girsa-link-types -- corpus
cargo run --release -p girsa-search --bin girsa-index -- build index corpus personal
cargo run --release -p girsa-search --bin girsa-index -- stamp index
cargo run --release -p girsa-search --bin girsa-index -- find  index corpus יתגבר כארי
```

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

**Tier 0 through Tier 4 are done — the corpus is on the shelf, the graph is on
top of it, there is a window, the shelf is one you can rearrange, and all five
ways of searching it are in that window with the counts to narrow by. Tier 5,
the Ksav loop, has started: one Ctrl+C puts down three flavours and the third
one is a source packet a real Ksav really renders.** All four verify commands
green in all three repositories.

| | What holds |
|---|---|
| **W1** · scaffolding | Three repos, pinned, dual-licensed. A breaking change to a shared crate fails in `sefer-crates` CI before it reaches either app — proven by breaking one. |
| **W2** · `girsa-hebrew` | The normalizer, and the line between what it will and will not do. 372-row regression corpus **harvested from 400 real seforim**, not written by hand. |
| **W3** · `girsa-ref` | The resolver. **100.00% exact on 2,970 real citations**, 0 wrong. Lexicon of 6,594 works and 24,731 spellings, built from Sefaria's schemas. |
| **W4** · `girsa-source` | The Source Packet. Ksav compiles it, and an arriving quote is put through the **real Typst compiler** rather than merely deserialized. |
| **W5** · fetch | 12,826 files, 3.4 GB on disk. Resumable — killed at 47%, resumed with nothing refetched. |
| **W6** · segment IDs | `girsa:mishnah-berurah/1:1#7`. One typo fix, 501 links: **line numbers moved 501, permanent ids moved 0.** |
| **W7** · import | Sefaria spine, Otzaria fill. **7,189 works · 5,000,545 segments**, each named once and never again. Mishnah Berurah 18,120/701 and Shulchan Arukh O.C. 697/4,171 — `spec.md` §2's numbers, exactly. |
| **W8** · links | The graph, on segment ids rather than line numbers. **4,182,344 edges** from 5,108,893 rows — 81.9%, and **92.6%** of the rows whose sefer is on the shelf at all. Every dropped row counted under why, and **nothing left ambiguous**. Mishnah Berakhot 1:1 → the Rambam on it, end to end. |
| **W9** · the workspace | Tabs, splits, RTL, nikud toggle, per-sefer position memory — and **a commentary column that follows the text**. Berakhot open with Rashi beside it: move the Gemara to 2a:6 and the Rashi column moves to 2a:6:1. **1,718 of Berakhot's 2,749 lines have a Rashi**; on the other 1,031 the column says *אין כאן* and stays where it is. |
| **W10** · the shelf | One taxonomy over two corpora's vocabularies: **15 shelves, 7,189 seforim, each on exactly one**. Editable — move, rename, reorder, make a shelf — as **one file in your own layer**, which a re-import cannot touch. A file you drop in is a sefer with permanent ids like any other. |
| **W11** · the index | **5,000,545 segments in 4m 8s**, one normalized index, built by the *same* code the query bar normalizes with. A bare `משעה שהכהנים נכנסים` finds the fully menukad first line of Shas, and the highlight lands on `שֶׁהַכֹּהֲנִים` — the word as printed. Nothing widened at import: `שבת` does not find `ובשבת`, and that is the point. |
| **W12** · Torat Emet | The literal mode, and the default. The three operators that get used — the word, the letters it **contains**, those letters **in order** with others between — plus **within X words of each other, in either order**. Every query carries a plan saying exactly what was asked of the index, and a test asserts that plan is the typed words with their nikud off and nothing else. On the shelf: `קדש` is 31,483 segments, `--contains קדש` is 301,910, and the difference is a thing the reader asked for. |
| **W13** · the ladder | Two columns of one table (§9.6), and the difference between them is the work order: the default mode **offers** the rungs with their counts and applies nothing; Smart climbs them and says so. The counts are computed from the query the click would run, so the promise and the result cannot disagree — checked both ways. Two rungs are named and not offered, because a missing chip reads as *there is nothing down that road*: nikud is already off in every mode, and the root rung is what §9.4 rejected every analyser for. |
| **W17** · the buffer | Ctrl+E, a drawer at the foot of the window, and **real Ksav markup from the first keystroke** — `girsa-ksav`, the writer Ksav compiles, not a second one in TypeScript. A buffer is a `.ksav` file in your own layer, and Ksav's suite compiles one this window wrote and reads the mekor off the page, below its quote. |
| **W16** · the pairing | A desk on loopback in each application, token-gated, presence asked rather than assumed — `Live`, `NotRunning` and `Stale` are three different things and the window says which. Ctrl+Shift+C sends into the open document with no clipboard at all; `/cite` and `/quote` let Ksav re-print a citation or re-read a quote from the corpus as it stands. Tested through a real socket, including the 401. And `girsa:…` **is** the deep link — the ref the document already stores. |
| **W15** · the clipboard | One Ctrl+C, three flavours — and the third is written natively, because a webview's custom format is a private encoding no other application can read. Only the highlighted part travels; the ref is a span when the quote is. The citation is `girsa-cite`, compiled into both apps, and **the test is that Girsa reads back what Girsa printed** — which found two defects in `girsa-ref` and fixed them there. Checked in Ksav against a packet Girsa really sent, asserted **on the laid-out page**. |
| **W14** · the rest of §9 | The other three modes, the chip row, and the five facets. **A facet row's count is the number clicking it gives you** — the ladder's promise, one section on, asserted for every row of every dimension. On the shelf: `יתגבר כארי` is 79 segments; the rail says `חסידות 26`, and clicking it gives 26. The two instruments the index cannot answer say so by name instead of approximating: a dilug reads letters and a notarikon is four patterns each matching half the vocabulary. |

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

4,182,344 — W8's number exactly, walked from the other side. The 273 MB that
costs sits beside the edges as `touching.jsonl`, and it is a cache: delete it
and run the tool again. What is **not** allowed is an index reading its absence
as a zero, so the index writes down whether it had it:

```
$ cat index/girsa-build.json
{"works":7189,"segments":5000545,"link_types":true}
```

Without that file the link facet says *not built* rather than showing an empty
column. *Nothing here is commented on* and *nobody worked out what is commented
on* are different statements, and a column of zeros says the first while meaning
the second — which is exactly the silent gap §9.7 forbids one facet over.

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

The last two are what make a citation alive. Because a Ksav document stores the
**ref** and not the printed string, a whole sefer can be switched from
abbreviated to full-form citations, and every quote regenerated against a
corrected edition (§7) — but only if something knows the title, the words the
schema uses for a level, and the text. All three live in the library, so Ksav
asks rather than keeping a copy that nobody would remember to update.

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

**פתח בכסב** hands the whole document to the real Ksav over the loopback —
offered only when presence says it is there. There is no conversion step, which
is the point: Ksav is opening a document it can already read.

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

The rest of the Ksav loop (W18–W19) is still to come: cite-on-selection, and
sending your own writing back into the library.

Two things W10 leaves for the orders that own them. A sefer of yours is **not in
the resolver's lexicon**, so it is opened and filed by title and not yet cited
by one. W14 wired the resolver into the query bar and did not change that: the
lexicon is `corpus/lexicon.tsv` and the 978 Otzaria titles beside it, both
written by the import, and a sefer you dropped in this morning is in neither. It
is searchable like anything else and it is not citable by name. And a PDF has pages and no words,
which is W26's to change; the index already carries them as `page` segments so
that §9.7's *"4 PDFs on this shelf aren't searchable yet"* is a count somebody
can take, rather than a silent gap.

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

## Licence

MIT OR Apache-2.0 — see [`LICENSE`](LICENSE). Forced by crate-sharing with
Ksav. No corpus text is committed here; texts are downloaded at first run and
each carries its own source and licence.
