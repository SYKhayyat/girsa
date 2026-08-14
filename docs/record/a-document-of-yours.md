# A document of yours, with its shape

*← [The semantic lane](the-semantic-lane.md) · [The record](../the-record.md) · [Answering a program](answering-a-program.md) →*

---

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

### A list inside a list item is inside it

A nested item used to be a **sibling** of the item it sits under: drawn with an
em-space so a reader could see the nesting, and addressed as though there were
none. That is the class of mistake W6 is about — what a reader can see was not
what the address said. `girsa:note/חבורה/ראיות:1` covered the point and none of
its sub-points, so a chaburah folder holding *point 1* did not hold them, a note
anchored to the point was not found from a sub-point, and nothing could fold
what the pane was already drawing indented.

So the depth goes into the **address**: point 1's second sub-point is
`…/ראיות:1:2`. Containment is then structural, which means it is answered by the
machinery that already answers it everywhere else — `SegmentId::covers`, and
`Standing` above it.

**A top-level item still takes a line number like any other block**, and that is
deliberate rather than incidental: a document with no nested lists in it has
exactly the addresses it had before, so nothing already anchored to a `.ksav` on
somebody's shelf moves. The nesting is spelled *below* the number rather than
instead of it. A list that opens already indented — a writer who started at the
second level — is padded rather than flattened, because the level they wrote is
the level they meant.

The em-space stays in the text. It is what a nested list looks like in a plain
rendering, the way a tab is what a table row looks like, and a surface that
draws lines rather than lists still needs it: **the address says what contains
what, and the text says what it looks like.** Both are asserted in
`your_own_seforim.rs`.

### What a document does not carry yet

- **Nothing writes back, and that is a boundary rather than a gap.** Reading a
  `.ksav` into segments does not make the shelf able to *edit* one. The file is
  the truth and Ksav is what writes it — Girsa's own writing pane produces a
  `.ksav` in your layer, which is the door that exists; turning the shelf into a
  second editor for the same format would be two applications writing one file.
- **A table has no header row unless it says so.** The header is the run of
  `כותרת_תא` cells, and a table written entirely of `תא` has none — which is
  what the document said.

This is a change to `girsa-ksav`, which both applications compile, so the shared
crates went to **0.4.0** and both pins moved with them — the coordinated release
W1 exists to make routine. Ksav's engine suite is green against it.

---

*← [The semantic lane](the-semantic-lane.md) · [The record](../the-record.md) · [Answering a program](answering-a-program.md) →*
