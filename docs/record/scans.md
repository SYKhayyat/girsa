# Scans, and reading them

*← [Links, and repairing them](links.md) · [The record](../the-record.md) · [Your own layer](your-own-layer.md) →*

---

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

### The OCR queue was ranking these words all along, and could not open one

W21 built a queue of words that are almost certainly scanning errors: a word
seen once, one letter from a word seen ten thousand times, ranked by which pair
of letters a scanner confuses. This document used to say that OCR text does not
reach it — *a word tesseract got wrong is not ranked beside a word Otzaria's
scanner got wrong.* That sentence was wrong, and it was wrong in the direction
that costs the most: it described a missing feature, so nobody went looking for
a broken one.

`girsa-suspects` builds its vocabulary from **the index's term dictionary**, not
from the corpus files — and `add_page` has written a page's words into that same
dictionary since this work order. So tesseract's misreadings have been counted,
compared and ranked beside the corpus's from the day the two existed together.
`girsa-search/tests/what_tesseract_got_wrong_is_in_the_queue.rs` is that,
written down: three lines of Gemara saying `קורין`, one page where the engine
read the final nun as a vav, and the pair coming out of `hunt` with its counts.

What did not work was **opening** one. The queue row points at a segment id; the
window resolves it by looking for the word in that segment's text; and a page
segment's text is the empty string the importer minted, because the words of a
page are an engine's opinion and live in `personal/words/<slug>/pages.jsonl`.
Every candidate on a photograph answered *that word is not in that line any
more* — a sentence about a word that had never been looked for. And before even
that, the window gave up one line earlier: a scan opens into a `ScanView` and
the queue reached for a reading pane, which for a scan does not exist.

So a page takes the other branch from end to end, and that is the whole of the
join:

- `scanning::where_word_on_page` finds the word in the **reading**, tokenized by
  the same normalizer the index was built with. Tokenized and not compared whole,
  because a `Word` can hold an entire line when the file positions lines rather
  than words — see the section above, and then the word wanted is one token
  inside it.
- The reading is asked for with **the reader's own corrections already applied**,
  so a candidate they fixed an hour ago reports itself as gone rather than
  opening a box on a word that no longer says that.
- And the correction goes through `scan_fix`, by ink, which is the only
  correction a photograph can take. Sending it through `fix` would have written
  a character span into a text that does not exist, which is the anchor this
  work order spent its first section refusing.

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

### A highlight on a photograph is on the ink, and one rectangle per line

W24 attaches a highlight to **specific words** by storing a character span into
the segment's text. A page has no text — the importer gives a dropped PDF one
segment per page with an empty string in it, because it will not invent Hebrew
it cannot read — so the span had nothing to count into and a scan could be
marked whole and no finer.

The answer is the one this work order already settled two sections up, applied
to a highlight: **anchor to the ink.** A page's words are an engine's current
opinion and the whole premise here is that a better engine replaces them, so an
offset written down today points somewhere else after a re-read, silently. The
photograph does not move.

`girsa-app/tests/a_highlight_on_a_photograph_is_on_the_ink.rs` is that property:
the page is read again, a word the first pass missed appears *before* the run
and another is spelled differently — so every offset after the first change is
wrong — and the mark is still on the two places it was made on, reporting the
engine's new opinion of what is written there.

**One rectangle per line and not one round the run.** A highlight from the
middle of the top line to the middle of the third has a bounding box that also
covers the far ends of all three, including words the reader never touched;
redrawing from that box would grow the mark. The test asserts both halves — the
three rectangles cover exactly the words picked, and the box round them covers
more.

And the picking is **two clicks, not a drag**. There is no text over a
photograph to select, so a drag would have to hit-test its own path across the
boxes and guess what a diagonal across two columns of a daf means — and on a
page set in two columns the guess is wrong often enough to matter. Two clicks
say exactly which words; the first stays lit until the second lands. Clicking
one word twice is a run of one, which is the common case and needs no separate
path.

The mark carries what it was made on **and** what is under it now. On a page
nobody has re-read those are the same sentence; where they differ, the reader is
entitled to see that the engine has changed its mind about the words they
highlighted.

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

**What this does not yet do.** Nothing has been
run against a real photographed sefer: every measurement above is against a
born-digital PDF, which is the only kind on this shelf, so the numbers for
tesseract are its numbers on **clean 300-dpi print** and a photograph of a Vilna
Shas will do worse.

---

*← [Links, and repairing them](links.md) · [The record](../the-record.md) · [Your own layer](your-own-layer.md) →*
