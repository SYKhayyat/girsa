# YOU ARE THE BUILDER

**Your job: build Girsa from nothing to the whole of `spec.md`.** Work the tiers in
order. Everything you need is here or named here, with exact paths.

Start by reading, in this order:

1. **`spec.md` §2 (Ground truth) and §3 (The landmine)** — do not skip. §2 is
   verified fact about the real data, not documentation; §3 is the one decision
   that cannot be retrofitted. Every work order below is shaped by them.
2. **`spec.md` §16 (Decisions settled)** — 19 rows. These are closed. Do not
   reopen them, and do not "improve" them silently.
3. **This document, in full, before you touch anything.**

**You are not being asked to make a corpus load. You are being asked to build a
system whose addresses survive being edited.** Those are different jobs. A library
app that reads text is a weekend; a library app where fixing a typo doesn't
silently corrupt ten thousand commentary links is the actual project.

---

## 0. Rules that bind every work order

1. **Test-first.** Write the failing test, *run it, watch it fail*, then fix. A
   test you did not watch fail is not a test.
2. **Fix the whole family.** Each order has a **Siblings** line. Patching only the
   reported site hides a bug rather than fixing it. When you report, name the
   siblings you checked **including the ones you cleared, and why**.
3. **No legacy.** When a thing is replaced, the old thing is deleted in the same
   change — config keys, docs, tests and migrations included.
4. **Verify.** `cargo build --all-targets` → `cargo test` → `cargo clippy
   --all-targets --all-features -- -D warnings` → `cargo fmt -- --check`.
   Unverified is not done.
5. **Commit per work order**, with a message saying what changed and what it does
   *not* yet do.
6. **Never guess at a citation, a link, or a ref.** Ambiguity is surfaced to the
   user as a choice. This is a product rule, not a style preference — a wrong ref
   is worse than no ref, everywhere in this system.

### 0.1 STOP AND ASK — do not decide these alone

| Topic | Why it needs a ruling |
|---|---|
| **Any change to a `spec.md` §16 decision** | They were argued through and closed. Reopening one invalidates work downstream of it. |
| **The segment-ID scheme (W6)** | Everything anchors to it forever. Get the shape ruled on before writing the importer, not after. |
| **Source Packet field changes (W4)** | It is a cross-repo contract. A field change is a semver break in `sefer-crates` and a coordinated release. |
| **Anything that makes search widen a query silently** | §9 exists because Sefaria's search does this. If you think a case warrants it, ask. |
| **Adding a network dependency at runtime** | Offline is the product (§14). Corpus updates are the only sanctioned network use. |
| **Taking code from Zayit, HebMorph, or Sefaria-ElasticSearch** | See §0.2. This can poison the license irreversibly. |

Everything else is internal correctness. **Build it without asking.**

### 0.2 Traps — verified, and they will bite you

These are not hypotheticals. Each was confirmed by reading the real files.

**T1 · Line numbers are not addresses.** Otzaria links are `file + line_index`.
Insert or delete one line and every link below it in that file points at the
**wrong text** — silently, no error. This is why W6 exists. If at any point you
find yourself storing a line number as a durable reference, you have reintroduced
the central defect of the corpus you are replacing.

**T2 · `"Conection Type"` is misspelled** — in Otzaria's JSON *and* in Sefaria's
`links*.csv`. Match the typo exactly when parsing. Do not "correct" it on read
without a normalization layer, or you will silently drop every type.

**T3 · Line indices are floats, sometimes strings.** `913.0`, `"913.0"`. Parse as
`substringBefore('.').toInt()` equivalent. A naive integer parse throws on real
data.

**T4 · `path_2` is stale.** Otzaria folders were renamed after the links were
generated. **Resolve targets by filename, never by path.** Build a
`basename(no .txt) → absolute path` index over `אוצריא/` and route every lookup
through it. Verified independently in `OtzariaSonim/SPEC.md`.

**T5 · 74% of link types are blank**, and it originates upstream in Sefaria — not
an Otzaria conversion bug. Re-importing from Sefaria does not fix it. Sampled
Abudraham: 420 links → 311 blank, 61 `commentary`, 34 `quotation`, 10 `related`,
4 `reference`.

**T6 · The org tree is a trap.** `Downloads/seforim` has 22,556 `.org` files to the
txt tree's 6,618 and looks like 3.4× the corpus. It is not. 8,106 of its 8,376
apparently-new titles are in `Other` — Ben-Yehuda piyut-per-file poetry, volume
splits of works already counted, and artifacts (`999504`,
`00007_קש''ק כריתותconverted`, `_1`/`_2` suffixes). Across every real shelf it adds
**41 titles**. **Do not ingest it.** (`spec.md` §2.3a.)

**T7 · License contamination.** Girsa is MIT OR Apache-2.0 because it shares crates
with Ksav. **Zayit is AGPL-3.0 §7b. HebMorph is AGPL. Sefaria-ElasticSearch is
GPL-3.0.** Read all three as prior art; copy from none. Taking so much as a
prefix-stripping table verbatim is a problem — reimplement from the described
behavior.

**T8 · Data grime.** Empty `<h2></h2>` headings, titles with leading spaces
(`" דברי חמודות על ברכות"`), a leftover `"Unnamed: 9"` column in Otzaria's
`metadata.json`, inconsistent nikud (Berakhot is fully menukad, Mishnah Berurah
has none). Normalize on import; assert on the counts in §2 so a bad import is
loud.

### 0.3 Definition of done

A work order is done when **all** hold:

- The test you wrote fails on the pre-fix tree and passes on the post-fix tree.
  Paste both runs.
- Siblings named, and addressed or cleared with a reason.
- The four verify commands are green.
- **An independent reproduction exists** — a command someone else can run to see
  the behavior, stated in the commit message. Not "tests pass".

---

## Tier 0 — Scaffolding

### W1 · Three repos, wired

**Goal.** `girsa`, `ksav` (exists), `sefer-crates`, each standalone, with the
shared contract usable from both.

**Build.**

```
Videos/girsa/          this repo — spec.md, BUILDER.md live here today
Videos/Ksav/           exists, github.com/SYKhayyat/ksav
Videos/sefer-crates/   girsa-source · girsa-ref · girsa-hebrew · girsa-cite
```

- `git init` in `Videos/Girsa`. `Videos/sefer-crates` as a new Cargo workspace.
- Both apps depend on the shared crates by **exact pinned version**.
- Local dev uses a Cargo `[patch]` pointing at the sibling checkout, so working
  across all three feels like a workspace without being one.
- Dual-license both repos MIT OR Apache-2.0. Copy Ksav's `LICENSE-MIT` and
  `LICENSE-APACHE` verbatim.

**Test first.** A CI job in `sefer-crates` that builds **both** dependents against
the proposed change. Prove it works by making a deliberate breaking change to a
shared type and watching the Girsa and Ksav builds go red in the `sefer-crates`
PR. Revert it. That red run is the artifact — paste it.

**Traps.** T7. Do not vendor anything into these repos without checking its
license first.

**Acceptance.** `cargo build` green in all three from a clean clone. A breaking
change to `girsa-source` fails `sefer-crates` CI before it can reach either app.

---

## Tier 1 — Foundations everything depends on

Nothing above Tier 1 can be trusted until these land. Build them first even though
none of them draw a pixel.

### W2 · `girsa-hebrew` — the normalizer

**Goal.** One shared normalizer for query and corpus. Rules, not models
(`spec.md` §9.2, §9.4).

**Build.** In order of how often each silently breaks a search:

| Transformation | Example |
|---|---|
| Strip nikud + te'amim (`U+0591–U+05C7`) | `וּבַשַּׁבָּת` → `ובשבת` |
| Peel stacked prefixes ו ה ב כ ל מ ש ד | `ובשבת` → `שבת` |
| Ktiv male ↔ chaser equivalence | `כהן` ≡ `כוהן` |
| Fold geresh/gershayim `׳ ״ ' " " " ' '` | `שו"ע` ≡ `שו״ע` ≡ `שו”ע` |
| Fold final letters ך ם ן ף ץ | |
| Abbreviation expansion, bidirectional | `שו"ע` ↔ `שולחן ערוך` |
| Rabbinic acronym expansion | `רמב"ם` ↔ `רבינו משה בן מיימון` |

**Test first.** A regression corpus of real Hebrew strings — build it from the
corpus itself, not by hand. Every row is `input → expected normal form`. Start at
200 rows spanning all seven transformations, including stacked prefixes
(`ובשבת`, `וכשהמלך`) and the gershayim variants, which are the ones that look
done and aren't.

**Traps.** T7 — read `Sefaria-ElasticSearch` for *which* prefixes it strips, then
write your own. T8 — nikud coverage is inconsistent across the corpus, so the
normalizer must be idempotent and safe on already-bare text.

**Siblings.** Any place that compares two Hebrew strings anywhere in the codebase
must route through this crate. Grep for direct `==` on Hebrew at the end of every
later tier.

**Acceptance.** Searching `שבת` finds `וּבַשַּׁבָּת`. Searching `כהן` finds `כוהן`.
Searching `שו"ע` finds `שולחן ערוך`. All three from a cold index, in a test.

### W3 · `girsa-ref` — refs and the resolver

**Goal.** Turn any way a human writes a citation into a canonical ref, offline
(`spec.md` §4.2, §4.3).

**Build.**

- The ref type. **Spans, not points** — a quote is a range and must be addressable
  as one: `girsa:bavli/berakhot/2a:1-2b:4`.
- The resolver, **seeded from Sefaria's `schemas/*.json`** — each schema's
  `titles[]` and `match_templates[]`. This is the lexicon and it already exists:
  `Shulchan_Arukh,_Orach_Chayim.json` alone carries 44 title variants across both
  languages (`שו"ע או"ח`, `שו״ע או״ח`, `או"ח`, `S.A. O.C.`, `SA OC`, `OC`).
- The redirect table, from day one. When upstream re-segments a text, refs must
  survive.
- Must handle: gershayim-or-not, rabbinic shorthand (`רמב"ם הל' תפילה פ"ד ה"א`),
  daf/amud in any notation (`ברכות ב.`, `ברכות ב ע"א`, `Berakhot 2a`), pasted
  Sefaria-style refs, and partials against context ("see siman 5" while in O.C.).

**Test first.** A regression corpus of real citations harvested from the corpus —
mine the actual seforim for citation strings rather than inventing them. Every row
is `citation text → expected ref, or Ambiguous(candidates)`.

**Traps.** Rule 6 — **ambiguity resolves to a choice, never a guess.** Assert this:
a citation with two plausible targets must return `Ambiguous`, and a test must
fail if it ever silently picks one.

**Siblings.** The resolver is used by the query bar, the paste handler, linkify
(W18), the link importer (W8) and Ksav round-tripping. Every one of them must call
this crate, not reimplement a subset.

**Acceptance.** ≥95% exact resolution on the harvested regression corpus, with
every miss classified as `Ambiguous` rather than wrong. Report the number.

### W4 · `girsa-source` — the Source Packet

**Goal.** The cross-app contract (`spec.md` §10.1).

**Build.** The struct as specified, **plus a schema version field** — three repos
means a mismatched pair must fail loudly at the handshake rather than quietly
mis-render a citation. Serde round-trip, and a stable JSON representation.

**Test first.** Round-trip property test. Then a version-mismatch test: an older
consumer given a newer packet must error clearly, not partially deserialize.

**Acceptance.** A packet built in Girsa deserializes in Ksav in a test that lives
in `sefer-crates` and runs against both.

---

## Tier 2 — Ingest

The expensive tier. `spec.md` §2 has the measured numbers; assert against them so
a partial import is loud rather than silent.

### W5 · Fetch Sefaria

**Goal.** ~2.2 GB, resumable.

**Build.** Hebrew `merged.json` × 6,211 titles (~1.5 GB), `schemas/` × 6,456
(~32 MB), `links/` × 19 CSVs (671 MB). **Skip English and both `cltk-*` formats.**
Public bucket, no auth:

```
https://storage.googleapis.com/sefaria-export/{json,schemas,links}/…
https://raw.githubusercontent.com/Sefaria/Sefaria-Export/master/books.json
```

Resumable, backgroundable, and it must let you start reading shelves that have
already landed (`spec.md` §5).

**Acceptance.** Kill it at 40% and restart; it resumes without refetching.

### W6 · Permanent segment IDs — **the load-bearing one**

**Goal.** Every segment gets an ID assigned once, never re-derived from file
position (`spec.md` §3).

**STOP AND ASK before implementing** — see §0.1.

**Build.** Import assigns IDs. Corrections, notes, links, highlights and every
citation in a Ksav document anchor to them. Splitting a segment mints a **child
ID**; it does not renumber siblings. A redirect table absorbs upstream
re-segmentation.

**Test first.** This is the test that justifies the whole design, so write it
before the importer:

1. Import a sefer. Record the links landing on segment N and the 500 after it.
2. Apply a correction that **splits** segment N into two.
3. Assert every one of those 501 links still resolves to **the same text** it did
   before.

Then the same for a merge, and for an upstream re-segmentation via the redirect
table. On a line-index design all three fail — run it against a deliberately naive
implementation first and watch them fail. Paste that run; it is the proof.

**Traps.** T1 throughout.

**Siblings.** Every table with a foreign key to a segment. Grep for anything
storing a line number after this order lands.

**Acceptance.** The 501-link test above, green. And: no line number is persisted
anywhere as a durable reference — prove it with a grep in the commit message.

### W7 · Import texts

**Goal.** Sefaria spine + Otzaria fill, one model (`spec.md` §2.3b).

**Build.**

- **Sefaria first**, text *and* structure, for the 5,637 shared works. Schemas give
  real `addressTypes` (`["Siman","Seif"]`), `heSectionNames` (`["סימן","סעיף"]`),
  `depth` and `lengths` — use them; do not re-derive structure from headings when a
  schema exists.
- **Otzaria for the 978 it alone has** — the acharonim layer (גליוני הש"ס,
  אבני נזר, קרן אורה, חידושי הראב"ד, מהר"ם שיק, שו"ת ישועות מלכו). Parse
  `<h1>/<h2>/<h3>` headings into whatever structure they support; these get real
  schemas later only if they earn it.
- **Never graft** a Sefaria schema onto an Otzaria text file for the same work.
  That is a line-by-line alignment problem across thousands of books and it will
  eat the schedule. Pick one source per work.

**Traps.** T6 (do not touch the org tree), T8 (grime — strip leading spaces, drop
`"Unnamed: 9"`, tolerate empty headings).

**Acceptance.** Import asserts the §2 counts: ~7,576 works in the union; Mishnah
Berurah at 18,120 segments with 701 headings; Shulchan Arukh O.C. at 697 simanim /
4,171 se'ifim from its schema. A count that drifts fails the import loudly.

### W8 · Import links onto segment IDs

**Goal.** The graph, addressed durably.

**Build.** Sefaria's `links*.csv` are **citation-addressed** (`Sanhedrin 74b:9`) —
resolve them through W3 onto segment IDs. This is strictly better than Otzaria's
line-indexed copies; use Otzaria's JSON only for its 978 exclusive works. Store the
type field from day one; populate what exists, leave the rest `references`.

**Traps.** T2 (match the typo), T3 (float indices), T4 (resolve by filename), T5
(blank types are expected — do not treat a blank as a parse failure).

**Acceptance.** Reproduce the known-good lookup end-to-end: **Mishnah Berakhot
segment 3 → Rambam segment 5**, correct text, verified independently in
`OtzariaSonim/SPEC.md`. Report resolution rate and the count of links dropped as
unresolvable — a silent drop is a defect.

---

## Tier 3 — Shell and reading

### W9 · The workspace

Tauri shell. Tabs, splits, **RTL-native**, ref-synchronized panes, nikud toggle,
per-sefer position memory.

**Traps.** Tauri uses Edge's engine on Windows and Safari's on macOS. **Test
Hebrew-with-nikud rendering on both** — they will not be pixel-identical, and this
is the app category where that shows. A screenshot from one OS is not evidence.

**Acceptance.** A sugya open with its commentary in an adjacent column; scrolling
the Gemara moves the Rashi column to the matching ref.

### W10 · The shelf

Browse by the real taxonomy (Tanach / Shas / Halacha / Machshava / Chassidus /
Responsa / yours), **with the arrangement editable** — the shipped taxonomy is a
default, not a fact. User PDFs/DOCX/TXT droppable at any time, first-class.

---

## Tier 4 — Search

Read `spec.md` §9 in full first. The governing constraint is not "make it
powerful" — it is **the engine never changes your query without you knowing**.

### W11 · Index

tantivy, over the W2 normalizer. **Nikud and te'amim stripped in every mode, no
toggle** — one normalized index.

### W12 · Torat Emet mode — **the default**

Completely literal. Operators: the word *contains* these letters (`קדש` →
`המקדש`, `ויקדשהו`); these letters in this order with others between; these words
within X words of each other. **Nothing stemmed, expanded or guessed.**

**Acceptance.** What you typed is what was searched for — assert that no
transformation beyond nikud-stripping is applied in this mode.

### W13 · Smart mode, and the relaxation ladder

Opt-in. Prefixes, male/chaser, abbreviations. Ladder order: drop nikud → other
forms → root → expand abbreviations → widen proximity.

**The behavior differs by mode and this is the point:**

| Mode | On zero results |
|---|---|
| Torat Emet (default) | **Offer** the ladder with counts computed up front. Never auto-apply. |
| Smart | Auto-relax in order, announce the change, one-click undo. |
| Regex | Nothing. |
| Citation | Offer near-miss refs (W3's ambiguity path). |

**Acceptance.** In the default mode, a zero-result query shows
`[try other forms — 7]` with the count **computed before the click**, and applies
nothing until clicked.

### W14 · Regex, Citation, Instruments; chips; facets

Modes 3–5, the chip row (`[torat emet ▾] [whole shelf ▾] [words near each other ▾]`),
and live facets by shelf/era/author/sefer/link-type with counts.

---

## Tier 5 — The Ksav loop

**This is the milestone that makes the project itself.** `spec.md` §10. Pull it as
early as Tier 2 allows — run it against a stub corpus rather than waiting for
ingest. The interop is the riskiest *design*, not the riskiest code, and design
risk should be retired first.

### W15 · Clipboard and send

Layered clipboard — one Ctrl+C puts down `text/plain` (works in WhatsApp),
`text/html` (RTL-correct, keeps shape in Word), and
`application/x-girsa-source+json`. Send-selection: highlight part of a passage,
only that goes. **The user does nothing different.**

### W16 · Transport

Loopback, token-gated, localhost only. `girsa://open?ref=…` and
`ksav://insert?packet=…`. Presence, so the affordance is never offered when it
would fail.

### W17 · The Ksav buffer in Girsa

Lightweight editor, with "open the real Ksav editor here" in the same pane —
`ksav serve` already hosts the editor SPA, so the embedded option is cheap.

**The constraint that matters: lightweight means the UI, not the format.** The
buffer writes real Ksav/Typst markup from the first keystroke. If it invents its
own note format, the handoff becomes lossy and the whole design is defeated.

**Acceptance.** Text written in the Girsa buffer opens in real Ksav with zero
conversion.

### W18 · Cite-on-selection

Highlight a phrase in Ksav → the first mekor appears → Tab cycles the rest → no
fit drops you into Girsa search.

**Note the reuse:** this is the same engine as corpus-wide quote detection. *"Where
is this phrase from?"* and *"who quotes this Gemara?"* are one feature asked from
two directions. Build it once.

### W19 · Closing the loop

Send text from Ksav **into the library** — your writing becomes a sefer on the
shelf, searchable and citable. Then "where did I use this?" and auto mareh mekomos
fall out (the refs are already in the document; it is a sort and a print).

**Linkify** is in scope but **high-confidence patterns only** — anything ambiguous
stays plain text. Rule 6.

**Not in scope:** quote drift check. Considered and dropped (`spec.md` §10.7).

---

## Tier 6 — Corrections

### W20 · The overlay

**Never mutate base text.** Patches are `segment ID + character span + provenance +
timestamp`. A `kind` field distinguishes an OCR error from a girsa variant — same
machinery, different claim, and it unifies with the `emends` edge type.

**The guardrail is a requirement, not a hope:** if correcting a typo is not a
**three-second interaction** from where you are reading, nobody does it. Measure
it.

### W21 · OCR-error detection

A word appearing once in the corpus, one edit-distance from a word appearing ten
thousand times, is almost certainly an error. Batch job → **ranked reviewable
queue**. This is worth more than the editor.

### W22 · Export a fixed sefer

Base text + patches → clean `.txt`/`.docx`. Falls out of the storage design for
free (`spec.md` §4.1).

---

## Tier 7 — Link repair

### W23 · Repair UI

Reanchor (drag a commentary to the right segment), retype (set a blank
`Conection Type`), reject/confirm, draw by hand. Stored as **overrides in the
personal layer**, never edits to shipped data. Everything shows its work — matched
text, confidence, method. A blank-typed link is never presented as curated fact.

### W24 · Span anchoring and lenses

Links attach to **specific words**, not whole segments — selecting a phrase
highlights only the links touching it. Lenses (Halacha / Lomdus / Peshat / Girsa /
Mine) are saved filters over type/era/strength, not hardcoded lists.

---

## Tier 8 — PDFs

### W25 · Viewer, page→daf, citation

**Tzuras hadaf comes from scans** — the scan *is* the daf, no typesetting engine.
BYO only; nothing shipped, nothing fetched. A page→daf mapping makes a scan
citable.

### W26 · OCR

Optional, **off during onboarding**, background, resumable, never blocking reading.
Text anchors to **coordinates on the page image** so a hit highlights on the scan.
The image stays ground truth, which makes fixing OCR errors safe by construction.

In search: one index, two location types. PDFs are absent until OCR'd, and
**never a silent gap** — the header says *"4 PDFs on this shelf aren't searchable
yet — [OCR now]"*. Scanned hits get a badge. Badge them, don't demote them.

**Open:** the OCR engine is undecided (`spec.md` §17). Tesseract is mediocre on old
Hebrew print. Evaluate before committing.

---

## Tier 9 — Personal layer and beyond

### W27 · Notes as nodes

Notes anchored to segment IDs, highlights, tags, saved queries, chaburah folders.
**A note has the same typed edges as anything else** — so *"what have I written
that touches this sugya?"* is the same query as *"who quotes this Rishon?"*

Local, exportable as plain files, no account. Optional encrypted sync of the
personal layer only — never the corpus, never telemetry.

### W28 · Chain tracing, semantic lane, MCP

Trace forward from a Gemara to how it became halacha; backward from a ruling to
where the posek got it; **break analysis** — where two rishonim read one Gemara
into incompatible halachos, which is usually the chiddush. Then the BEREL
embedding lane, **always visually separated** from literal results. Then MCP on
both ends.

---

## Appendix A — Environment

**Toolchain, verified present:** rustc/cargo 1.96.0 · node 26.4.0 · npm 11.17.0 ·
git 2.54.0 · gh 2.96.0.

**Local data:**

| What | Path | Size |
|---|---|---|
| Otzaria texts | `C:\Users\Administrator\Downloads\otzaria_latest\אוצריא` | 6,618 files, 3.4 GB |
| Otzaria links | `…\otzaria_latest\links` | 5,819 files |
| Otzaria metadata | `…\otzaria_latest\metadata.json` | 7,041 entries |
| **Org tree — DO NOT INGEST** (T6) | `C:\Users\Administrator\Downloads\seforim` | 22,556 files, 8.3 GB |
| Ksav | `C:\Users\Administrator\Videos\Ksav` | `github.com/SYKhayyat/ksav` |
| Verified Otzaria format notes | `C:\Users\Administrator\Videos\OtzariaSonim\SPEC.md` | |

**Remote data:** `gs://sefaria-export` — public, no auth. `json/` `schemas/`
`links/` `table_of_contents.json`, plus `books.json` on raw.githubusercontent.

**Prior art — read, do not copy (T7):** Zayit (AGPL-3.0 §7b), HebMorph (AGPL),
Sefaria-ElasticSearch (GPL-3.0). Otzaria is UNLICENSE and fine.

---

## Appendix B — How to report

Per work order, in the commit and in your final report:

1. **What changed**, and what it explicitly does **not** yet do.
2. **The pre-fix failing run and the post-fix passing run**, pasted.
3. **Siblings** checked — including cleared ones, with the reason.
4. **The independent reproduction** — a command someone else can run.
5. **Anything you hit that contradicts `spec.md`.** The spec is built on verified
   measurements, but measurements go stale. If the data no longer matches §2, say
   so loudly rather than coding around it — a silent workaround for changed data is
   how the corpus defects in §0.2 got there in the first place.
