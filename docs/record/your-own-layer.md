# Your own layer

*← [Scans, and reading them](scans.md) · [The record](../the-record.md) · [The chain](the-chain.md) →*

---

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

### Six copies of nine lines, and none of them had heard of you

A sefer of yours could be opened by title and filed by title and **not cited by
name**. Typing its name into the citation bar answered *that is not a place on
this shelf*, about a sefer sitting on the shelf.

The resolver's vocabulary is two files: `corpus/lexicon.tsv`, every spelling of
every work Sefaria ships a schema for, and `corpus/lexicon-otzaria.tsv`, the 978
works Sefaria never had. Read the first, append the second. Nine lines — and
there were **six hand-written copies** of them:

```
app/src-tauri/src/lib.rs              read_lexicon, for linkify
crates/girsa-search/src/citation.rs   Citations::open, the citation bar
crates/girsa-link/src/bin/girsa-link-import.rs
crates/girsa-link/examples/why-dropped.rs
crates/girsa-app/examples/send.rs
crates/girsa-desk/examples/write.rs
```

Six copies of one paragraph is the shape this repository has diagnosed before —
*six correct solutions to one problem*, in the personal log — and they had
already drifted. Four joined the two files with a newline between them and two
concatenated them bare, so a `lexicon.tsv` not ending in a newline would have
glued its last title onto Otzaria's first and lost a work at each end.
`build-lexicon` does end its file with a newline, which makes those two correct
by luck; luck is not a property anything can check.

So it is one loader now, in `girsa-corpus`, and the seventh copy was not
written. It comes in two shapes, and which one a caller wants is a real
question rather than a default:

- **`Titles::of(corpus)`** — the corpus alone, for `girsa-link-import` and the
  `why-dropped` run that measures it. Those two resolve *Sefaria's own
  citations against Sefaria's own corpus*: no row in `links0.csv` names a sefer
  of yours, and a title of yours colliding with one of Sefaria's would turn a
  lookup that resolved into one that is ambiguous — which **drops the edge**.
  Your layer there is not an improvement; it is noise with a cost.
- **`Titles::across(corpus, personal)`** — both, for every caller where a
  person typed the title. The citation bar, linkify, and the two examples that
  send a mareh makom to the pen.

Everything in `personal/works/index.jsonl` goes in: a file you dropped, a
`.ksav` read onto the shelf, and a note — which this library holds to be a sefer
of yours and not a lesser thing, so it is one here too.

**A title of yours does not shadow a masechta.** Call a sefer ברכות and the
lexicon returns two works, and the bar draws a choice. That is not new
behaviour bolted on for this; it is what the resolver has always done for
או"ח, which is Orach Chayim in the Shulchan Arukh and in the Tur. The rule is
that ambiguity is shown as ambiguity, and nothing here was allowed to become
the exception.

The second half is the one that would have been easy to declare finished
without. A resolved citation is a slug, and the next thing any caller does is
read that work's segments — and your sefer's `segments.jsonl` is not under the
corpus root. Left there, the lexicon would have known the name and the shelf
would have answered *not on the shelf* about it, which is worse than never
knowing the name at all. `Titles` therefore reports which slugs came out of your
layer, and the resolver reads each work from the root that actually holds it —
from the catalogue that was read, not by trying one root and falling back,
because a fallback cannot tell *your sefer* from *a corpus sefer that failed to
load* and would report the second as the first.

From a terminal, which is where a claim like this gets checked:

```
girsa-index find index corpus --personal personal --citation "חבורה על הסוגיא ב"
```

`--personal` is a value option and not a second positional, deliberately:
`find <index> corpus personal יתגבר` once read `personal` as a word to search
for and answered zero in 5,000,847 segments.

### A mekor in a note of yours, and the crate that could not be reached

Linkify — prose full of citations becoming live refs, W19 — was written for the
Ksav loop and lived in `girsa-desk`. `girsa-desk` depends on `girsa-app`, and a
crate cannot depend back, so the reading pane had no way to call it. The pane is
where a note is *read*. So a note that said *ועיין ברכות ב.* said it in ink:
every other surface in the library will take you somewhere, and the one holding
your own words would not.

The function needs nothing from a desk. It is a lexicon and a string. `who_cites`
is the half that genuinely needs one — it scans your buffers and your registered
documents — and the two were together because they were written on the same
afternoon, not because they are the same shape. So `linkify` and `Linked` moved
down to `girsa-app`, and `girsa_desk::linkify` re-exports them: Ksav's loopback,
the `/linkify` errand and the window's own command all still call the name they
already called.

**The rule is that the corpus does not get this.** A run of words carries a ref
only when the work is `Source::Mine` — a note, a `.ksav` read onto the shelf, a
file you dropped in. A sefer from Sefaria already has a link layer: 1.4 million
edges built from `links0.csv` by somebody with the whole corpus in front of
them, drawn in the links panel and repairable there. Linkify is three narrow
rules over a string. Laying its edges over the same words would put a weaker
answer beside a stronger one with nothing on the page to tell them apart. Your
own writing has no link layer and no prospect of one, which is precisely why the
words have to answer for themselves.

The offsets are the part that had to be got right. `Line::of` draws
`pointed(&segment.text, pointing)` — taking the nikud off **shortens the
string** — so linkify runs over the text the pane is about to draw and not over
the segment as it stands on disk. Reversed, a citation would land a few letters
to the left of the words it was about, which is the same defect `display::Shown`
exists to prevent for corrections, and
`the_link_lands_on_the_words_after_the_nikud_comes_off` is the guard.

A citation is a `<span class="run-cite">` in the flow of the sentence and not a
`<button>`: a button there breaks the line box, takes tab focus out of the
reading pane, and stops a reader dragging a selection across it to quote the
line. The click resolves through `post::landing`, the same function a `girsa://`
link from another application takes — one answer to *which segment does this
address name*, because two of them would eventually disagree. A citation of a
sefer that is not on this shelf says so out loud; you can write *עיין קצות סימן
ג'* whether or not you have the Ketzos, and a click that silently did nothing
would read as a broken window rather than as a sefer you have not imported.

### A tag was a tally, and is a route

The tags drawer counted every tag across your whole layer and told you the
truth about it: `ברכות` is on three notes, nine highlights and a folder. Then
you clicked it and nothing happened. A count you cannot follow is a report
about your layer rather than a way through it, which is the wrong half of the
work to have built.

**A tag is the one thing in here that crosses all four kinds.** A note, a
highlight, a saved question and a chaburah folder are four different files with
four different shapes, and the word you wrote on them is the same word. So
following one shows all four at once, and it is deliberately *not* a seventh
tab: a tab would have had to pick one drawer to filter, and picking is the one
thing the reader was not doing when they wrote the same tag on a note and on a
highlight.

The filtering is in the window and not in Rust, which is a decision and not
laziness. Your layer is the small side of this library — four files a person
wrote by hand, against five million segments — so there is nothing to push
down; and a `tagged` command would be a **fifth** answer to *what carries this
tag* standing beside the four lists that already exist, which is exactly how the
number in the drawer and the rows underneath it start disagreeing. The four
lists are asked for together rather than in turn, because they are four
independent reads and doing them one after another is three round trips of
waiting for nothing.

Every chip is the way in, not only the drawer: the tags on a note, on a mark, on
a query and — newly — on a folder, which carried them all along and was the one
kind of thing you own that is *already* a grouping and could not be reached from
a tag. A tab is how you get out, because a reader finished with `ברכות` wants a
drawer and not an empty one with the filter still on.

### Two people's layers, and the merge that refuses

`spec.md` §11 puts *optional, off by default, encrypted sync of the personal
layer only* on the table, and `BUILDER.md` §0.1 says a runtime network
dependency is not a decision a work order takes on its own. That ruling is still
yours to make. What did not need a ruling, and was missing anyway, is the thing
two people actually do: put a chaburah together. Corrections have had
`girsa-fix merge` since W20. Notes, marks, saved questions and folders had
nothing, so two copies of `personal/` were two copies and the only way to
combine them was to pick one.

```
$ girsa-notes corpus personal merge /media/stick/his-layer
note: 4 taken · 1 already had · 1 refused
mark: 12 taken · 0 already had · 0 refused
query: 1 taken · 0 already had · 1 refused
what was refused is yours, and is untouched
```

**The refusal is the feature.** Every store applies one rule: take a key you do
not hold, count a key you hold whose record is identical, and *refuse* a key you
hold whose record differs. Two people learning one sugya will both have a note
called מאימתי and a folder called ברכות, and last-one-wins — which is exactly
right *within* one layer, where the later line is the same person changing their
mind — is exactly wrong across two. It would replace a morning's writing with a
stranger's and leave nothing on the screen to say so, in the one kind of
material in this library that nobody else has a copy of. It is the same call
`girsa-fix` already makes when two corrections claim the same letters: the
system does not choose between two people.

The rule is written once, on `girsa_personal::Store` — the trait the six stores
already shared for *open, replay, hold, compact* — so marks, saved questions and
folders got a merge without one of them growing a method. Notes could not use
it, because a note is a file and not a line in a log; they have the same rule
over a directory instead, and a note that is taken goes through `Notes::write`,
so it arrives with its `work.json`, its `segments.jsonl` and its catalogue line.
A note of theirs is a sefer on your shelf exactly as one of yours is, and
`a_note_of_theirs_arrives_as_a_sefer_on_your_shelf` fails if a merge ever
becomes a file copy.

*Identical* is compared as the serialized line, and for notes as the text the
file would be written with. That is not a shortcut around `PartialEq` — the file
**is** what is being merged, and two records that write the same line are the
same record by the only definition this layer has.

`merge` reads exactly what `export` writes. The four names come from one
`where_it_goes`, which is why an export directory and a `personal/` root are the
same shape: handing somebody the directory and handing them an export arrive at
the same door, and neither needs a format.
`an_export_is_a_layer_a_merge_can_read` is the guard, and it is the sentence
*exportable as plain files* meaning something.

### What this does not do

- **No sync.** `spec.md` §11 offers *optional, off by default, encrypted sync of
  the personal layer only*. Every word of that is a runtime network dependency,
  and `BUILDER.md` §0.1 says that is not a decision a work order takes on its
  own. **This one is for you to rule on.** What is built instead is the half
  that needs no ruling and that §11 names first: it is all plain files, and
  `girsa-notes export` puts them where you can copy them.
- **The results header still counts a note the index already has.** See the
  section below: your writing goes into the index the moment you write it, but
  the sentence that says *what the index has not seen* answers by comparing file
  times against when the index was **built**, and absorbing one work does not
  make the index newly built. So a note written and indexed a second ago is
  still counted as outstanding until the next full rebuild.

  Deliberately left over-reporting rather than fixed by re-stamping the index.
  The stamp is what the *corrections* gap is measured against too, and a note's
  absorb re-stamping would clear a pending correction's sentence while the index
  still held the old words. Over-reporting says *there may be something you
  cannot see*; under-reporting says *you are seeing everything* and is the
  failure `spec.md` §9.7 exists to prevent. Closing it properly means recording
  per work what has been absorbed, which is a store this does not have yet.

### Your writing is in the index before you have finished reading it back

`spec.md` §11 says your notes are searchable, and they were — **as of the last
build**. Being a sefer was already enough for the indexer; pointed at a layer
holding one note it reads it like anything else, and finds the paragraph that
was written *between* two others:

```
$ girsa-index find index personal "ובאמת כבר עמדו"
1 in 4 segments · showing 1

girsa:note/מאימתי/2.1#2.1  [text]
  [ובאמת] [כבר] [עמדו] בזה

narrow by:
  shelf      שלי 1        author  shaul 1        sefer  מאימתי 1
```

The trouble was the word *build*. Nothing indexed a note when you wrote one, and
a 5,000,545-segment rebuild is four minutes — for one paragraph.

Nothing about tantivy required that, and this is the part worth writing down: **a
work has been the unit of replacement since W11.** The first time a writer is
handed a segment of some work it deletes every segment of that work already in
the index, because `girsa-import` rewrites `segments.jsonl` wholesale and an
append would have doubled every hit. That rule was built as a full rebuild's
safety net. Read from the other side it is an incremental update — one work in,
the old copy out, nothing else touched — and it had been sitting there since the
day the index was written.

What was missing was a caller, and before a caller could exist the body of the
build loop had to become a function. It is not three lines: a page of a scan is
indexed from its reading and not from its segment, your corrections are applied
over a `Standing`, the link-type masks are read per work and **refused** when
they were built against a different segmentation, and the wordless count is asked
of what actually went in. An `absorb` that reimplemented any of that would be a
second indexer, and the disagreement would be silent — a note indexed one way and
Shas the other, agreeing until the day they did not. So `girsa_search::building`
holds it once, and the build loop and the window both call it.

Three doors, all the same function underneath:

- `girsa-index update <index> <root> <slug…>`, for a shelf maintained from a
  terminal;
- the window, which absorbs a note when you write one, when you edit one, and a
  sefer when you drop one — and which **never fails a write because a cache
  would not update**. The note is on disk before the index is asked; if the
  index refuses, the reader has still written their note and what they lose is
  that it is findable until the next build, which is exactly where everything
  stood before;
- `forget`, which is the half `absorb` cannot cover. A note you threw away has
  no `segments.jsonl` to read back, so nothing is ever added under its name and
  the delete never fires — it would stay findable until the next full build, and
  a hit on it would open a sefer that is not on the shelf. That is worse than
  the gap this closes, so it is closed in the same breath.

`girsa-search/tests/a_note_is_searchable_before_the_next_rebuild.rs` asserts the
four things that would go quietly wrong: the note is findable and the corpus has
not moved, taking it twice leaves one copy rather than two (W8 shipped that
failure once already), an edit replaces rather than accumulates, and a deleted
note stops being found.

---

*← [Scans, and reading them](scans.md) · [The record](../the-record.md) · [The chain](the-chain.md) →*
