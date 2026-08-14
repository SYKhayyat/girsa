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

### What a document does not carry yet

- **The containment is flat.** A list inside a list item is two items at two
  depths and not a tree; you can see the nesting and you cannot fold it.
- **Nothing writes back.** Reading a `.ksav` into segments does not make the
  shelf able to *edit* one; the file is still the truth and Ksav is still what
  writes it.
- **A table has no header row unless it says so.** The header is the run of
  `כותרת_תא` cells, and a table written entirely of `תא` has none — which is
  what the document said.

This is a change to `girsa-ksav`, which both applications compile, so the shared
crates went to **0.4.0** and both pins moved with them — the coordinated release
W1 exists to make routine. Ksav's engine suite is green against it.

---

*← [The semantic lane](the-semantic-lane.md) · [The record](../the-record.md) · [Answering a program](answering-a-program.md) →*
