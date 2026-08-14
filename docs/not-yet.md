# What Girsa does not do yet

Every tier in [`../spec.md`](../spec.md) is built and each one is asserted on
something — [`../BUILDER.md`](../BUILDER.md), *What holds, per work order*, has
the twenty rows. That is a true sentence and on its own it is a misleading one,
because seven of the twelve pages in [`the-record.md`](the-record.md) carry a
section saying what the thing just described still cannot do, and those sections
are the only honest account of where this stands.

They were written where the work was argued, which is the right place to argue
them and the wrong place to find them. This page is the same eight lists in one
place, each linking back to the section that makes the case. Nothing here is
new; if this page and the record ever disagree, **the record is right and this
page has rotted.**

---

## Built, and the window cannot reach it

**Empty, and worth leaving here for that reason.** The pattern this section
watched for is the one that costs the most and shows the least: the crate works,
the tests are green, and the only way to it is a terminal. A reader does not have
a terminal, so a tier in that state is a tier that does not exist for them.

The last entry was the transmission chain, which had a library, a terminal tool
and no door for as long as `spec.md` §8 had been "built". It has a panel now.
The heading stays because the next thing built crate-first will belong under it,
and a category with nothing in it is easier to notice filling up than one that
has to be invented again.

## Built on both sides of a seam, and not joined across it

The larger group, and the more interesting one. In each case two pieces of
machinery exist, they are the same shape, and nobody has connected them —
which is a different and generally harder problem than either piece was.

| | Where it is argued |
|---|---|
| **The results header counts a note the index already has.** Your writing now goes into the index as you write it, but the sentence that says *what the index has not seen* compares file times against when the index was **built**, and absorbing one work does not make the index newly built. Left over-reporting on purpose — re-stamping would clear a pending correction's warning while the index still held the old words. | [your own layer](record/your-own-layer.md#what-this-does-not-do) |

## Built narrower than the name suggests

Not gaps so much as the honest width of a thing, written down where the feature
is described so that nobody has to discover it by being disappointed.

| | Where it is argued |
|---|---|
| **Two readings of a line are found at one hop from it.** A sefer that reads this sugya only by way of another sefer counts as a witness to a fork and never as a side of one. Whether that is a limit or the right definition is a question about how a sugya travels. | [the chain](record/the-chain.md#what-the-chain-does-not-do-yet) |
| **The semantic lane answers a half-remembered line, not a question.** BEREL is a masked-language model, not a sentence encoder. Over 240 se'ifim of Hilchos Tefillah: asked as a statement you half recall, the right se'if is in the top ten for **ten of ten** pairs and worst-case sixteenth; asked as a question about the se'if, **one of twelve** reaches the top ten and the worst is 97th. The lane's box asks for a line as you remember it, and does not pretend to answer questions. | [the semantic lane](record/the-semantic-lane.md#what-it-does-and-what-it-does-not) |
| **A scan is highlightable finer than the page; a *link* is not.** A highlight on a page is anchored to the ink and survives a re-read. Pinning a link onto words (spec.md §8.4) still takes a character span, which a page has nothing to count into — and a personal scan has no edges to pin, so nothing has needed it yet. | [scans](record/scans.md#a-highlight-on-a-photograph-is-on-the-ink-and-one-rectangle-per-line) |
| **The MCP end is one end, and its writes cannot be undone over the wire.** A program can write a note, draw a link and record a correction with `--writable`; nothing there deletes one, because deleting is a decision and this end cannot show you what you are about to delete. A search is still capped at 50 rows whatever `limit` says, and says so. Ksav's server is Ksav's repository. | [answering a program](record/answering-a-program.md#what-the-mcp-end-does-not-do) |
| **A `.ksav` on the shelf is read, not written.** Its nesting is in the address now, so a sub-point is inside its point. What the shelf still does not do is *edit* one: the file is the truth and Ksav is what writes it, and Girsa's own writing pane is the door that exists. | [a document of yours](record/a-document-of-yours.md#what-a-document-does-not-carry-yet) |

## Built, and nobody has exercised it

The most uncomfortable group, because everything in it may well work. *Not
known to be broken* is not the same claim as *known to work*, and this section
exists so the two do not get written down as one.

| | Where it is argued |
|---|---|
| **Nobody has dragged a sefer with a mouse — the gesture, not the logic.** What a drop *means* is a tested function now, refusals and all. What no machine here can raise is the gesture itself: a native HTML5 drag is not synthesizable through the debugging protocol the eyes tool drives, and a file drop is an OS event no browser can fire. So what is untested is whether the events arrive, not what happens when they do. | [corrections](record/corrections.md#what-has-not-been-checked) |
| **Nikud on WebKit is unknown — the code builds on macOS, the rendering is unseen.** CI has a macOS job now: the Rust half passes there and the shell compiles against macOS's WebKit bindings. What it does not settle is rendering, because the eyes tool drives Chrome and Chrome on macOS is the same Blink it is on Windows — a second machine, not a second engine. W9 asks about Safari's WebKit, which is what the shipped window there uses. | [corrections](record/corrections.md#what-has-not-been-checked) |
| **Nothing has been run against a photographed sefer.** Every OCR measurement is against born-digital PDFs, which is the only kind on this shelf. Those are tesseract's numbers on clean 300-dpi print; a photograph of a Vilna Shas will do worse by an unknown amount. | [scans](record/scans.md#the-job-is-one-page-at-a-time-and-that-is-the-promise) |

---

## And the one that outranks all of it

**Nobody has learned a sugya in it.**

Everything above is a list of things a person who built this can tell you about
the thing they built. None of it is the finding that a zman of real use would
produce, and the two documents in this repository that come closest — the
[five-minute report](the-five-minute-report.md) and
[the second sitting](the-second-sitting.md) — are between them eighteen
complaints and an hour, from somebody who opened it once. Both found things no
test had. That is the shape of the evidence still missing, and no amount of
work on the lists above substitutes for it.

---

## Keeping this page honest

The rule is the one this repository applies to every other copy: **the record
is the source and this is the copy, and a copy nothing regenerates is a copy
that rots.** Nothing regenerates this one. So when you close one of these,
close it in the record's own section first — that is where the argument lives
and where the next person will read it — and then strike the line here.

When you open a new one, write it in the record where the work is argued, and
add it here. A gap that is only on this page is a gap nobody had to think hard
enough about to write down next to the code.

---

| | |
|---|---|
| Why any of this is the way it is | [`the-record.md`](the-record.md) |
| What each work order is asserted on | [`../BUILDER.md`](../BUILDER.md) |
| What it does today, for a reader | [`start-here.md`](start-here.md) |
