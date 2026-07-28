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
  Ksav/           the writing app          github.com/SYKhayyat/ksav
  sefer-crates/   the shared contract      github.com/SYKhayyat/sefer-crates
```

| Crate | Purpose |
|---|---|
| `girsa-corpus` | Storage, ingest, schemas, permanent segment IDs |
| `girsa-search` | tantivy indices, the five modes, the relaxation ladder |
| `girsa-link` | The typed link graph, repair, later mining |

plus `girsa-source`, `girsa-ref`, `girsa-hebrew` and `girsa-cite` from
`sefer-crates`, pinned to an exact version and resolved from the sibling
checkout during development.

**The sibling checkout has to be present.** Until `sefer-crates` is published,
cloning Girsa alone will not build.

## Build

```sh
cargo build --all-targets
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
```

## Status

**Tier 0, Tier 1 and Tier 2 are done — the corpus is on the shelf and the graph
is on top of it.** All four verify commands green in all three repositories.

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

Nothing draws a pixel yet. The shell (W9), the shelf (W10) and search (W11–W14)
are next, and the Ksav loop (W15–W19) is the milestone that makes the project
itself — `BUILDER.md` says to pull it as early as Tier 2 allows.

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
