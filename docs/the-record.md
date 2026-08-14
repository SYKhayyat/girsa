# The record

This was `README.md` until **14 August 2026**, and it is kept whole — split by
subject, because 3,300 lines in one file is an archive rather than something
anybody reads.

It is not a manual and it is not out of date by accident. It is the **argument**:
every decision in this repository written down beside the defect that caused it,
in the order the defects were found. The window's four hundred lines of pane
drawing, the ids that survive an edit, the twenty-one regular expressions that
matched English error prose, the build that lied and succeeded — the reasoning is
here and it is nowhere else.

| You want | Read |
|---|---|
| to use it, build it, or contribute | [`../README.md`](../README.md), then [`../CONTRIBUTING.md`](../CONTRIBUTING.md) |
| the map: what is where, and which test holds it there | [`architecture.md`](architecture.md) |
| to know **why** something is the way it is | the twelve pages below |

**Where this and the README disagree, the README is right.** The counts here were
measured on 14 August 2026 and nothing re-counts them; the README's are marked and
re-counted on every push by
`crates/girsa-app/tests/the_numbers_in_the_readme_are_measurements.rs`. That is why
the markers came off on the way in — a number under a check that does not read
this file is a number wearing the costume of a measurement.

---

## The twelve

### [The shape of it](record/the-shape-of-it.md)

Where everything lives, the fifteen crates and why the arrows run the way they do, the window that decides nothing, the wire format that was described four times, and the day a refusal stopped being a regular expression against English prose.

### [Building it, and checking it](record/building-and-checking.md)

The installer, the build that lied and succeeded, one command line read sixteen ways, forty-three tests that passed because they could not find their input, and the diagnosis every other page here is downstream of.

### [The shelf, and searching it](record/the-shelf-and-the-search.md)

Where 4.2 million links landed and where the rest went, what keeps two columns together, one taxonomy over two corpora, the five modes and what each promises, the chips, the facets, and the graph that had to be turned round first.

### [The Ksav loop](record/the-ksav-loop.md)

One Ctrl+C in three flavours, why the citation is not the string, what happens when both applications are running and there is no clipboard at all, and closing the loop from the other end.

### [Corrections](record/corrections.md)

Never the text, and what that buys measured rather than asserted; three seconds counted; why an offset is not a place; a typo and a girsa variant as one mechanism and two claims; and why the queue is worth more than the editor.

### [Links, and repairing them](record/links.md)

Saying the data is wrong without editing it, who comments on this line without opening seven thousand files, a repair that follows the edge rather than the row, and which words a link is actually about.

### [Scans, and reading them](record/scans.md)

The scan is the daf; the one number that would have been the same bug again; what a page cites as and what it never invents; the OCR engine question answered by measuring it; and never a silent gap.

### [Your own layer](record/your-own-layer.md)

A note is not a row beside the graph. Each paragraph carries its own name, everything of yours survives the corpus moving under it, what the panel is actually waiting for, and what a chaburah and a saved query are.

### [The chain](record/the-chain.md)

Direction is time, and the graph does not have any. The era code that could not make the hop the whole feature exists for, half of every link stored at the far end, and why a chain of *connected somehow* is not a chain.

### [The semantic lane](record/the-semantic-lane.md)

Something like this, but not the words: a licence that disagreed with itself, off meaning off, and 4.5 segments a second — which is why you choose rather than the machine choosing for you.

### [A document of yours, with its shape](record/a-document-of-yours.md)

Two thirds of a .ksav was not on the shelf. Which commands are structure, headings as the address, and a footnote that is its own line.

### [Answering a program](record/answering-a-program.md)

The library as MCP tools: the same engine and the same refusals a person gets, over stdio with nothing bound, and one guardrail bought expensively.

---

## Licence

MIT OR Apache-2.0 — see [`../LICENSE`](../LICENSE). Forced by crate-sharing with
Ksav. No corpus text is committed here; texts are downloaded at first run and each
carries its own source and licence.

What is bundled *into* the installer and is not ours is listed in
[`../THIRD-PARTY-NOTICES.md`](../THIRD-PARTY-NOTICES.md) — today that is pdf.js,
which draws a page of a scan and reads the words off one. Tesseract is **not** in
that list on purpose: it is found on the machine if it is installed and run as a
separate process, so nothing of it is linked into this program or shipped with it.
No AGPL or GPL code is used anywhere here: Zayit, HebMorph and
Sefaria-ElasticSearch were read as prior art and copied from nowhere
(`BUILDER.md` T7).
