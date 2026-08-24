# Links, and repairing them

*← [Corrections](corrections.md) · [The record](../the-record.md) · [Scans, and reading them](scans.md) →*

---

### The data is wrong, and you can say so — without editing it

spec.md §8.3, decision 9. 40% of the link graph carries no type at all and it
originates upstream (T5), so a re-import does not fix it. The four things a
reader can do — **reanchor, retype, reject or confirm, draw one by hand** — are
stored as overrides in `personal/links.jsonl`, and the shipped shards are never
written to. Same rule as corrections, same three reasons: the importer replaces
every shard it owns on every run, your judgement and the corpus's must stay
distinguishable, and a thing you said should be undoable.

**Everything shows its work**, because that is the difference between a repair
tool and a rumour. Each row carries what it is, what it *was*, how it was found,
how much to believe it, which of the four actions changed it, and who said so:

```
מפרש   ← רמב"ם על משנה ברכות 1:1:1    90% · sefaria-seed · "commentary"
קשור   ← ספר האסופות סימן כב           90% · sefaria-seed · ""      ← not curated
פוסק   ← שולחן ערוך, אורח חיים נח:א    100% · by-hand · drawn · you
```

The second row is the point of §8.3's last sentence: **a blank-typed link is
never presented as curated fact.** It is shown, greyed, and it stays that way
until somebody looks at it — confirming is a claim a person made, and it is
worth nothing if an unconfirmed link looks the same.

### Who comments on this line, without opening seven thousand files

An edge is stored once, in the shard of the work it points *from* (§8.2), so the
outgoing half of the question is one file. The incoming half — *who comments on
this se'if* — is the reverse direction, and the honest way to answer it would be
to open every shard in the corpus.

It doesn't, because `girsa-companions` already recorded which works share edges
with which. The panel reads **only the shards of works known to link here** — a
few dozen for the first mishnah of Berakhot, which has 333 links on it, in 2.5
seconds on a debug build. When that cache has never been built the panel says
so, rather than showing the outgoing half and letting a reader believe that is
all there is.

That number is also the honest limit of this design: a reverse index would make
it instant, and there isn't one. The tripwire in
`crates/girsa-app/tests/the_links_on_this_line.rs` exists to catch the day
somebody makes it read the whole graph instead.

### A repair follows the edge, not the row

Found by the real corpus, in the test: there are several links between the first
mishnah of Berakhot and the Rambam on it, and the panel sorts by confidence — so
confirming one moves it to the top and rejecting it moves it to the bottom. A
test that re-found "the first Rambam row" after each action was confirming one
link and rejecting another while believing it did both to one. Every repair is
keyed to the edge's **shipped name** — its two segment ids — which is also why a
reanchored edge is still found by the record that moved it.

### Which way the arrow points, said where it is shown

The importer orients every edge — `comments-on` edges point from the commentary
to its base, flipped where the CSV had it backwards — and prices the cases where
nobody declared anything into confidence as a 0.15 dent. The 23 August audit's
finding was that the fact itself was then dropped at every surface: chain hops,
path links and link rows carried type, label and confidence but not direction,
so an arrow that was the **order of two CSV columns** drew exactly like a
declared one. It now travels with the edge everywhere it goes — `Step` and path
`Link` carry it, the MCP `links`/`trace`/`path` replies name it, the window's
rows say *direction declared* or *direction undeclared* in both languages —
because a reader deciding whether to follow a link is entitled to know whether
anybody ever said which way it points.

And when a corpus update drops an edge your layer has said something about,
that record stays — it is a thing you said — and `girsa-link-types` now reports
it by name at the cache build, the one moment both halves of the question are
in hand. The promise in `Repairs::orphans` finally has a keeper.

### Deleting what a folder holds

Deleting a note used to orphan its seat in every chaburah folder — bare slugs
for works that no longer existed, drawn each time the folder opened. The delete
now takes the member out (`Collections::without_work`, and `without_query` for
saved queries) and says which folders changed; the MCP `forget_note` reply names
them as `folders_tidied`. Members naming **places** inside the deleted work stay:
a place survives re-segmentation by its name. Link repairs naming the work stay
too, deliberately — they are statements you made, and the report above covers
them.

### Which words a link is about, when anything says

spec.md §8.4: *links attach to specific words, not whole segments — selecting a
phrase highlights only the links touching it.* Nothing in the shipped data says
which words: Sefaria's links address a segment and so do Otzaria's. So a span
comes from one of exactly two places, and never from a guess:

1. **The dibur hamatchil** — the commentary says which words it is on, in the
   text. And the corpus writes that two ways: `<b>…</b>` in some volumes, and a
   dash in others. Rashi on Berakhot, in the copy on this shelf, is entirely the
   second — a reader of `<b>` alone finds **nothing** in the whole masechta.
2. **You said so** — a link you drew from a highlight, or pinned onto one.

Measured on the real text:

```
255 of 501 diburim landed on their words
girsa:bavli/rashi-on-berakhot/2a:1:1#1 — on: מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין? מִשָּׁעָה שֶׁהַכֹּהֲנִים…
girsa:bavli/rashi-on-berakhot/2a:1:2#2 — on: עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה
```

That is not a rate to optimise. The half that does not land is **refused on
purpose**: the words are not in that line, or they are there twice. A dibur
hamatchil that appears twice gives two candidate spans and no way to choose, so
it gives none — a highlight on the wrong half of a line looks exactly like a
highlight on the right one, which is rule 6 in the place a reader would never
check. Matching is through the normalizer throughout, because Berakhot ships
menukad and Rashi on it does not.

One of those refusals is worth reading twice: Rashi quotes `בערבין` where the
mishnah in front of him reads `בערבית`. That is a girsa and not a typo, the
whole-phrase candidate correctly finds nothing, and the shorter phrase he also
quotes is what lands.

Asking the narrower question drops **only what is known to be elsewhere**: a
link with no span stays, because it is on the whole segment and the segment
includes what was highlighted. Answering "which links are on these words" with
"the ones whose words I happen to know" would be a shorter list wearing the face
of a complete one.

### Lenses are saved filters, not five lists

spec.md §8.5. Halacha / Lomdus / Peshat / Girsa / Mine ship as five rows of
`personal/lenses.json` — each a filter over **type, era and strength** — and
every one of them is yours to change, add to or delete. Whether the Tur belongs
under Halacha is a question about how you learn, not about this program.

Strength is where W23 pays: a confirmed link and one you drew score 1.0, an
untyped seed scores what its method scores (0.9 citation-addressed, 0.7
line-indexed), and a rejected one scores nothing. So *"only what somebody has
actually checked"* is a lens with `at_least: 1.0` and no code behind it.

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

---

*← [Corrections](corrections.md) · [The record](../the-record.md) · [Scans, and reading them](scans.md) →*
