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
| `girsa-search` | tantivy indices, the five modes, the relaxation ladder |
| `girsa-link` | The typed link graph, repair, later mining |
| `girsa-app` | The reading workspace: the shelf, tabs and splits, and what keeps two columns together |

plus `girsa-source`, `girsa-ref`, `girsa-hebrew` and `girsa-cite` from
`sefer-crates`, pinned to an exact version and resolved from the sibling
checkout during development.

`app/` is the Tauri shell: a window and twenty-three commands, and **nothing
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
cargo run --release -p girsa-search --bin girsa-index -- build index corpus personal
cargo run --release -p girsa-search --bin girsa-index -- phrase index משעה שהכהנים נכנסים
cargo run --release -p girsa-search --bin girsa-index -- words  index יתגבר כארי
cargo run --release -p girsa-search --bin girsa-index -- stamp  index
```

`words` and `phrase` are the index's own probes, not the search bar. The bar has
five modes, a relaxation ladder and facets, and those are W12–W14.

## Status

**Tier 0 through Tier 3 are done, and Tier 4 has its index — the corpus is on
the shelf, the graph is on top of it, there is a window, the shelf is one you
can rearrange, and every word of it is findable.** All four verify commands
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

**The index has no window on it.** W11 is the index and its two probes;
`girsa-index words` and `girsa-index phrase` are a command line, not a search
bar. The five modes, the relaxation ladder with its counts, the chips and the
facets are W12–W14, and nothing in the shell searches yet. The Ksav loop
(W15–W19) is the milestone that makes the project itself — `BUILDER.md` says to
pull it as early as Tier 2 allows.

Two things W10 leaves for the orders that own them. A sefer of yours is **not in
the resolver's lexicon**, so it is opened and filed by title and not yet cited
by one — the lexicon is built from Sefaria's schemas and W14's citation mode is
what wires the resolver into the query bar. And a PDF has pages and no words,
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
