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

The pattern to watch for: the crate works, the tests are green, and the only
way to it is a terminal. A reader does not have a terminal.

| | Where it is argued |
|---|---|
| **The transmission chain is a command, not a panel.** `girsa-chain` prints all four traversals — forward to halacha, back to a source, the era ladder, the forks. Nothing in the window draws any of them. | [the chain](record/the-chain.md#what-the-chain-does-not-do-yet) |

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
| **A fork is one hop wide on each side.** Two readings joined through an intermediate sefer are not found, and the ones that are found are bounded by `--width` with the drop counted. | [the chain](record/the-chain.md#what-the-chain-does-not-do-yet) |
| **The semantic lane answers a half-remembered line, not a question.** BEREL is a masked-language model, not a sentence encoder. Over 240 se'ifim of Hilchos Tefillah: asked as a statement you half recall, the right se'if is in the top ten for **ten of ten** pairs and worst-case sixteenth; asked as a question about the se'if, **one of twelve** reaches the top ten and the worst is 97th. The lane's box asks for a line as you remember it, and does not pretend to answer questions. | [the semantic lane](record/the-semantic-lane.md#what-it-does-and-what-it-does-not) |
| **A scan is linkable at the page and no finer.** W24's span anchoring is about segments, and a page is one segment. | [scans](record/scans.md#the-job-is-one-page-at-a-time-and-that-is-the-promise) |
| **The MCP end is read-only, and it is one end.** Nothing over the wire writes a note, draws a link or records a correction; a search is capped at 50 rows whatever `limit` says, and says so. Ksav's server is Ksav's repository. | [answering a program](record/answering-a-program.md#what-the-mcp-end-does-not-do) |
| **A `.ksav` on the shelf is flat and read-only.** A list inside a list item is two items at two depths, not a tree — visible, not foldable — and reading a document onto the shelf does not make the shelf able to edit one. | [a document of yours](record/a-document-of-yours.md#what-a-document-does-not-carry-yet) |

## Built, and nobody has exercised it

The most uncomfortable group, because everything in it may well work. *Not
known to be broken* is not the same claim as *known to work*, and this section
exists so the two do not get written down as one.

| | Where it is argued |
|---|---|
| **Nobody has dragged a sefer with a mouse.** Drag-to-rearrange and the file-drop event exist only in the shell; the shelf panel was driven in the browser build and the Rust API. The shell starts, opens the shelf and serves the commands. | [corrections](record/corrections.md#what-has-not-been-checked) |
| **Only Windows has been looked at.** W9's trap says Tauri uses Edge's engine on Windows and Safari's on macOS, and that a screenshot from one OS is not evidence about the other. There is no Mac here, so nikud rendering on WebKit is unknown — not fine, unknown. | [corrections](record/corrections.md#what-has-not-been-checked) |
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
