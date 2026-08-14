# Corrections

*← [The Ksav loop](the-ksav-loop.md) · [The record](../the-record.md) · [Links, and repairing them](links.md) →*

---

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

### The overlay, taught to the indexer

Every surface a reader looks at went through the corrections overlay except the
one that finds anything. The pane drew the fix, a quote copied to Ksav carried
it, an export wrote it and said in its header that it had — and a search
answered out of the corpus files, which are the sefer as it was scanned. So a
typo you fixed this morning was findable **by the typo and not by the word**,
which is the one place a correction looked like it had never been made.

Rebuilding per correction is not the answer and neither is a second index. What
it wanted was the third thing: `girsa-index` reads the layer once at the start
of a build and hands tantivy the segment as you read it. `spec.md` §4.1 is
untouched — **never the text**. Nothing writes to `corpus/`; the base text on
disk is exactly what it was, and the index is a rebuildable cache over the
corpus *and* over what you have said about it.

Four decisions, and the tests in
`girsa-search/tests/a_typo_you_fixed_is_findable_by_the_word.rs` are one each:

- **`Showing::Fixed`, the pane's own default.** Scanning errors repaired; girsa
  variants noted and not applied. A variant is a claim about what the text
  *should* read and an index is about what it does read — and a search that
  found words the pane does not draw is a result a reader cannot see when they
  arrive at it. A segment carrying nothing but variants does not even reach the
  corrected path, so its index entry is bit-identical to one built with no layer
  at all.
- **Asked by `Standing`, not by segment id.** A correction made before upstream
  cut the se'if it was on is stored under a name the work no longer has.
  `Layer::on` takes exact equality and would have missed it in silence, which is
  the failure `Layer::at` exists downstream of. The two halves of the evidence —
  which names are still segments, and `redirects.jsonl` backwards — are gathered
  per work, and the inversion is now `girsa_corpus::standing::redirected_here`
  rather than a second copy of the window's.
- **A stale correction is counted and said.** Its words are not in the line any
  more, so nothing is applied and the corpus text goes in unchanged; the build's
  report names how many. Same reason the export header does: this is the moment
  a correction that stopped applying would otherwise vanish without a word.
- **It costs nothing.** `Corrections::touch` is one question per work, and a
  work nobody has corrected skips the standing derivation and the apply
  entirely. On a real shelf that is 7,189 works minus a handful.

The layers are a list rather than one, because a build is handed every root and
is not told which is personal — and could not work it out from the sefer
either, since a correction to a Sefaria work lives under `personal/` while the
work lives under `corpus/`.

What is still open is the other half, and it is `spec.md` §9.7's, not this
section's: a correction made **since** the last build is not in the index until
the next one. That is reported rather than silent — the results header counts
it, `girsa_note::since::Unindexed` composes the sentence — and closing it means
writing tantivy incrementally.

### What has not been checked

**The shelf panel has been driven in a browser, not in the shell.**
`cargo run -p girsa-app --example dev-fixtures` writes the real 7,189-work tree
to static JSON and the same page draws it — the counts above were read off that
page — but drag-to-rearrange and the file-drop event **only exist in the shell**
and were exercised through the Rust API and `girsa-shelf` instead.

Narrowed since, and it is worth saying exactly how far. **What a drop means** is
now a function rather than five lines inside a `drop` listener, and it is
tested: a sefer or a shelf onto another shelf moves; a drop with nothing held
moves nothing, which matters because a `drop` can arrive from outside the window
with no drag of ours behind it; a thing dropped on itself or back where it came
from is not an edit. A shelf dropped into its own child is deliberately *not*
refused there — `Arrangement` refuses it with the one walk of the tree that
knows the whole shape, and a second check in the window would be a second answer
to that question.

That was the one path in `shelf.ts` that rearranges a reader's shelf, and until
it was lifted out of the listener nothing anywhere had executed it: `app/test`
has no DOM, so a module's exported functions are reachable and its handlers are
not — and the browser build sets `row.draggable = false`, because dragging is
the shell's.

**What is still unexercised is the gesture.** A pointer press, a move and a
release over a native HTML5 drag cannot be synthesized through the debugging
protocol the eyes tool drives, and a file drop is an operating system event that
no browser can raise at all. So the logic behind the drag has been run and the
drag has not, and the difference is a real one: what remains untested is whether
the events fire, not what happens when they do.

`BUILDER.md` W9 carries a trap: *Tauri uses Edge's engine on Windows and
Safari's on macOS. Test Hebrew-with-nikud rendering on both — a screenshot from
one OS is not evidence.* **Only Windows has been looked at.** There is no Mac
here, and saying the rendering is fine on one would be exactly the claim the
trap warns about.

Narrowed, and it is worth being exact about by how much. CI now has a **macOS
job**, and what it settles is:

- the Rust half compiles and its tests pass there — a different target, a
  different linker, a case-insensitive filesystem, and `std::path` behaving
  differently in every place it does;
- the shell compiles against macOS's own WebKit bindings, which is where a Tauri
  feature that exists on one platform and not the other shows up.

What it settles about **rendering is nothing**. The eyes tool drives Chrome, and
Chrome on macOS is the same Blink it is on Windows — a second machine, not a
second engine. What W9 asks about is Safari's WebKit, which is what the shipped
window there actually uses, and no headless WebKit is reachable from this tool
without a WebDriver stack it does not speak.

So: the code is known to build and pass on macOS, and **nikud rendering on
WebKit is still unknown**. Unknown is the word, not fine.

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

One thing W10 leaves for the order that owns it. A PDF has pages and no words,
which is W26's to change; the index already carries them as `page` segments so
that §9.7's *"4 PDFs on this shelf aren't searchable yet"* is a count somebody
can take, rather than a silent gap.

---

*← [The Ksav loop](the-ksav-loop.md) · [The record](../the-record.md) · [Links, and repairing them](links.md) →*
