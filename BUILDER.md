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

*Chain tracing and MCP shipped in W28. The lane was held for a ruling and is now
W30.*

---

### W30 · The semantic lane — **shipped**

**Ruled in `spec.md` §16 #20 — build to it, not around it.**

*Built. `girsa-lane`, `girsa_app::adjacent`, the `adjacent` MCP tool, and the
panel behind `לשון סמוכה` in the window. Two things changed on the way and both
are recorded rather than quietly absorbed:*

*· **BEREL's licence is `apache-2.0`**, checked three ways on 29 July 2026 — the
model card, its frontmatter and the Hub API. §9.4's "unrestricted" and the
README's warning disagreed; the API settles it, and Apache-2.0 is one of this
repo's own two licences, so side-loading it is clean.*

*· **A fetch button was ruled in, mid-order.** §16 #20 as first written said
Girsa fetches no model at all. It now says Girsa never *needs* the network: the
folder picker is the default path, and a `bring it in` button sits behind a
setting that is off in a fresh install. `spec.md` §9.9, §14 and §16 #20 were
amended to say so.*

*· **And the model was measured rather than assumed.** A half-remembered
statement finds its se'if in the top 16 of 240 every time, eight of ten at rank
1. A question about a se'if finds it one time in five. Mean-centring made it
worse and was not built. The lane is shaped around the first number, and the
side-loading is what makes a better model a setting rather than a release
(`girsa_lane::model`).*

**Goal.** "I remember a Rishon who says something like this but not the words"
gets an answer, without Girsa ever reaching the network and without a licence
that is not ours landing in the repo.

**Build.**

- **The model is side-loaded.** A setting takes a path to a model directory the
  user already has. Girsa downloads nothing, bundles nothing, and vendors no
  weights — §14 stays true and T7 stays clean. **Verify BEREL's actual licence
  before writing a line**: `spec.md` §9.4's candidate table calls it
  "unrestricted" and
  `README.md` warns it carries its own terms. Those disagree. Read the model
  card, and if it is not compatible with a side-loaded-by-the-user arrangement,
  stop and say so rather than shipping around it.
- **Off by default**, and off means literal search is byte-identical to what it
  was. No model configured is a stated absence in the search header, never a
  mode that returns nothing and looks like it worked.
- **You choose the corpus, at any granularity** — shelf, sefer, section, the
  personal layer, or all 5,000,545 — added to at any time. Background,
  resumable, never blocks reading (W26's rule, same reason).
- **The lane states its own coverage.** Every semantic result set says what is in
  the index and what is not, with a way to add the rest. A partial lane is
  expected; a partial lane that reads as complete is the §9 defect this project
  exists to not repeat.
- **Drawn as adjacent, always.** §14 — the lane assists retrieval, it does not
  pasken, and the UI may never blur that.

**Siblings.** Every search entry point: the five modes in `girsa-search`, the
zero-results relaxation ladder (§9.6 — the lane is *not* a rung on it; the ladder
must not silently widen into embeddings), facets, saved queries (W27), and the
MCP surface (W28), which must refuse and disclose partial coverage exactly as the
UI does.

**Done when.** A query that shares no words with its target finds it; the same
query with the lane off finds nothing and says why; coverage is stated in every
surface; and there is an independent reproduction someone else can run.

---

### W31 · `touching.jsonl` needs the source work — **filed from OtzariaSonim, 31 July 2026**

**Where this came from.** `Videos/OtzariaSonim` is the keypad-phone reader — the
"thin reader against the same data files" that `spec.md` §12 says is the answer if
Android ever matters. It is being migrated off Otzaria's raw JSON and onto this
corpus, and it hit this on day one. Filed as a work order rather than fixed
in-place because Girsa has its own builder.

**Goal.** Make `touching.jsonl` able to answer the question every reader actually
asks, so that consumers stop falling back to `inbound.jsonl` for it.

**The defect.** A line is `{"a":"girsa:shulchan-arukh/orach-chayim/100:1#636","t":"comments-on"}`
— segment plus edge *type*. No consumer asks "does this segment have a
`comments-on`." They ask **"does this segment have one from the mefarshim
currently selected"** — the per-book commentator filter, which is also `spec.md`
§8.5 lenses and the §8.4 gutter density map. That question needs the **source
work**, and it is the one field not stored.

Without it, `touching.jsonl` is a summary that cannot summarise, and the only way
to answer is the file it exists to avoid.

**Measured, Shulchan Arukh Orach Chayim:**

| File | Size | Answers "which segments light up under this filter?" |
|---|---|---|
| `touching.jsonl` | 1.15 MB | no — no work slug |
| `inbound.jsonl` | 27.3 MB | yes, by reading all 156,076 edges |

Corpus-wide the summary layer is 261 MB against 0.64 GB of `inbound`, and the
largest single `touching.jsonl` is 3.13 MB. On a phone with a 192 MB heap that
difference is the difference between opening שולחן ערוך and an `OutOfMemoryError`.

**Build.** Add the source work to each row:

```jsonc
{"a":"girsa:shulchan-arukh/orach-chayim/100:1#636","t":"comments-on","w":"mishnah-berurah"}
```

Then "which segments light up under this filter" is one streaming pass over ~1 MB
with no ref parsing, and `inbound.jsonl` is touched only when a user opens one
segment — which is the split the two files were presumably meant to be.

**Worth considering while it is open** (not required): `#2940` is already a
per-work ordinal, so within a work the ref string is 44 bytes carrying one
integer. `{"a":2940,"t":1,"w":88}` against a header table would put the same file
under 200 KB. Only worth it if a second consumer wants it — noting it so the
option is visible while the format is being touched anyway.

**Test first.** Assert that the set of segments returned by filtering
`touching.jsonl` to one work equals the set obtained by filtering `inbound.jsonl`
to the same work. Run it on `shulchan-arukh/orach-chayim` — 156,076 edges is
enough to catch an off-by-one in the dedup. It fails today because the first
filter cannot be expressed.

**Traps.** T1 — `w` is a work slug, not a position, so it stays durable. The
existing `(a, t)` dedup becomes `(a, t, w)`, which will grow the file; the SA O.C.
row count should be reported before and after so the growth is a measured number
rather than a surprise.

**Siblings.** Every consumer of `touching.jsonl`: the §8.4 gutter, §8.5 lenses,
the repair UI's "show me this commentator's anchors" (W23), and now OtzariaSonim's
packer. Whatever emits the file — check `girsa-link` and `girsa-corpus/src/index.rs`.

**Acceptance.** The equality test above, green. And a consumer can render a
filtered commentary indicator for the worst book in the corpus without opening
`inbound.jsonl`.

---

### W32 · W8's acceptance test does not hold on the shipped corpus — **filed from OtzariaSonim, 31 July 2026**

**Read this before W31.** W31 is a format improvement. This is a correctness
defect, and it is the reason the OtzariaSonim migration was halted.

**W8 says:** *"Acceptance. Reproduce the known-good lookup end-to-end: Mishnah
Berakhot segment 3 → Rambam segment 5."* That lookup returns **nothing** in the
corpus on disk today.

```
corpus/links/mishnah-berakhot/inbound.jsonl  — edges from rambam-on-mishnah-berakhot: 0
corpus/links/rambam-on-mishnah-berakhot/edges.jsonl — 6 rows, none onto mishnah-berakhot
corpus/works/rambam-on-mishnah-berakhot/     — 63 segments, ingested fine
```

The Rambam's peirush on the Mishnah is **in the corpus as text and absent from the
graph.** Not mistyped, not blank-typed — absent. Same for seven more on that one
masechta, every one of them ingested as a work:

| Missing from the graph | Otzaria has | Girsa has |
|---|---|---|
| יכין | 297 edges | 0, any type |
| תוספות יום טוב | 158 | 0 |
| משנת ארץ ישראל | 109 | 0 |
| רמב"ם | 63 | 0 |
| תוספות רבי עקיבא איגר | 51 | 0 |
| רש"ש | 25 | 0 |
| יש סדר למשנה | 24 | 0 |

Otzaria's graph carries 1,685 commentary edges on משנה ברכות from **14**
commentators. Girsa's carries 983 from **6**.

**It is not one masechta.** Distinct commentators per book, Otzaria vs Girsa,
`comments-on` against `commentary`+`targum`:

| | משנה ברכות | בראשית | שמות | ברכות | שבת | אסתר | תהילים | **12-book total** |
|---|---|---|---|---|---|---|---|---|
| Otzaria | 14 | 99 | 84 | 40 | 52 | 22 | 31 | **439** |
| Girsa | 6 | 27 | 26 | 1 | 10 | 2 | 17 | **145** |
| kept | 43% | 27% | 31% | **2%** | 19% | 9% | 55% | **33%** |

Bavli Berakhot resolves **one** commentator. Open a daf and there is no Rashi.

**The likely cause, and why it is a decision and not a bug.** `spec.md` §8.1 says
importing Sefaria's citation-addressed links is *"strictly better than repairing
Otzaria's degraded copies"*, and §16 #1 takes Sefaria's side for any shared work.
So for the 5,637 shared works, Otzaria's links are discarded — including every
edge Sefaria's `links*.csv` does not happen to carry. On this evidence that is
about two thirds of the commentary graph.

"Strictly better" is the claim these numbers refute. Sefaria's links are better
*addressed*; they are not more *complete*. The two graphs are complementary, and
choosing one wholesale silently drops the difference.

**Two measurements taken before proposing anything, because "more edges" is not
the same claim as "better edges".**

**(i) Where both graphs carry the same commentator, they agree on the anchor.**
משנה ברכות, the six works present in both, comparing which segment each commentary
attaches to:

| Commentator | Otzaria | Girsa | agree |
|---|---|---|---|
| ברטנורא | 56 | 56 | **100%** |
| מלאכת שלמה | 56 | 56 | **100%** |
| עיקר תוספות יום טוב | 50 | 50 | **100%** |
| הון עשיר | 48 | 48 | **100%** |
| בועז | 24 | 24 | **100%** |
| לחם שמים | 44 | 47 | 94% (Girsa +3) |

Two independently-derived graphs — one line-index converted, one citation-resolved
— landing on identical segments. That cross-validates both. **Otzaria's extra
commentators are not noise to be filtered out; they are the same quality of edge,
and there are simply more of them.** Note also that Girsa is not a strict subset
(+3 on לחם שמים), which is why the answer is a union and not "prefer Otzaria".

**(ii) The alignment §2.3b feared does not exist for links.** Otzaria's text lines
minus `<h1..6>` headings, against Girsa's segment count:

| | משנה ברכות | בראשית | תהילים | ישעיהו | ברכות | שבת | בראשית רבה | ש"ע או"ח | משנה ברורה | טור |
|---|---|---|---|---|---|---|---|---|---|---|
| Otzaria body | 57 | 1533 | 2527 | 1291 | 2749 | 3778 | 1036 | 4172 | 17419 | 6012 |
| Girsa segments | 57 | 1533 | 2527 | 1291 | 2749 | 3778 | 1036 | 4171 | 17418 | 6005 |
| delta | **0** | **0** | **0** | **0** | **0** | **0** | **0** | −1 | −1 | −7 |

Exact on 13 of 20 books tested; **−1 on six of the rest, and the one line is the
author's name** — Otzaria emits `יוסף קארו` / `רמב"ם` as a body line under the
`<h1>`, where Sefaria keeps it as schema metadata. Drop headings *and* that line
and the corpora align segment-for-segment, verified at both ends: Otzaria's first
body line of `רמבם על משנה ברכות` is `girsa:rambam-on-mishnah-berakhot/1:1:1#1`,
and the last is `9:5:3#63`.

So mapping Otzaria's `line_index` onto a segment ID is **arithmetic, not
alignment**. §2.3b's "line-by-line alignment across thousands of books would eat
the schedule" is true of *text* — you cannot interleave two editions — and §8.1
inherited that posture without re-deriving it for *links*. You can union two edge
sets. The address translation here is a subtraction.

**Build.** Do not simply re-run the importer — decide the merge first, then import:

- Union the graphs. Import Otzaria's JSON links for **every** work, not only the
  978 Otzaria-only ones, resolving targets by filename (T4) onto segment IDs via
  the body-line mapping above.
- **Quarantine, never guess.** The mapping is only valid where
  `body_lines == segment_count` after dropping headings and the author line.
  Where it does not hold — טור is −7 — the work is **skipped and reported**, not
  imported on a best guess. An off-by-one in a link anchor is precisely T1: the
  wrong text, silently. This guard is the whole difference between fixing the
  graph and corrupting it.
- Dedup against the Sefaria-seeded edges. `method` already distinguishes them
  (`sefaria-seed` / `otzaria-seed`) so provenance survives the merge and §8.3 can
  still show its work.
- Where the two disagree on the anchor, that is a repair-UI case (W23), not a
  silent pick. Rule 6.

**This touches §16 #1 and §8.1, so it is a §0.1 STOP AND ASK.** Bring the numbers,
not the proposal. Note that §5 already commits to *"the full union ships — nothing
is ever missing"*; that value was applied to works and not to edges, and the two
readings are hard to hold at once.

**Test first.** W8's own acceptance, as an actual test rather than a sentence:
assert `mishnah-berakhot/1:1#1` has an inbound `comments-on` from
`rambam-on-mishnah-berakhot`, and that its text begins
`מאימתי קורין את שמע בערבין וכו': כבר בארנו`. It fails on the current tree — run
it and watch it fail before touching the importer. Then widen it: for a fixed
sample of books present in both corpora, assert Girsa's distinct-commentator count
is **≥** Otzaria's. That is the regression that would have caught this at W8.

**Traps.** T4 (resolve by filename — the Otzaria side of the union depends on it
entirely), T5 (blank types are expected; they are not a reason to drop an edge —
which may be exactly how some of these were lost), T2, T3.

**Siblings.** Everything downstream reads a graph that is currently a third of its
size: §8.4 spans, §8.5 lenses, §8.6 chain tracing, W23 repair, W28 break analysis.
Chain tracing over a graph missing two thirds of its commentary edges will report
"no path exists" confidently and wrongly — check what W28 already concluded from
it.

**Acceptance.** The Rambam lookup, green. The sample-wide count assertion, green.
And a reported figure for edges gained by the union, by `method`.

**Reproduction.** Read-only, touches only `corpus/` and `Downloads/otzaria_latest/`:

```
python C:\Users\Administrator\Videos\OtzariaSonim\tools\girsa_coverage.py
python C:\Users\Administrator\Videos\OtzariaSonim\tools\girsa_coverage.py "משנה ברכות"
```

The first prints the 12-book table above. The second prints משנה ברכות commentator
by commentator, marking each as present, mistyped, or `NO EDGES (work ingested)`.

---

### W34 · Commentary anchors are indexed as words, so phrase search is broken — **filed from OtzariaSonim, 31 July 2026**

**`spec.md` §9.5's own worked example does not match the corpus.** The mockup in
that section searches `יתגבר כארי`. Here is Shulchan Arukh O.C. 1:1 as indexed:

```
יתגבר <i data-commentator="Ba'er Hetev" data-order="1"></i><i data-commentator="Sha'arei Teshuvah" data-order="1"></i>כארי לעמוד בבוקר
```

    "יתגבר כארי" contiguous in the segment text?  False
    ... after stripping the anchors?              True

**Why it reaches the index.** `girsa-search/src/index.rs:1792` adds
`&segment.text` — raw, anchors included. `sefer-crates/girsa-hebrew/src/normalize.rs:108`
keeps `c.is_ascii_alphanumeric()` as token characters. So the two anchors above do
not vanish; they tokenize to roughly `i · data · commentator · ba · er · hetev ·
data · order · 1 · i · …` and land **between** `יתגבר` and `כארי` in the position
list. The schema indexes `WithFreqsAndPositions` specifically so that phrases work
(index.rs:606-610), and those positions are now a dozen apart.

So a phrase query fails, and *"these words within X words of each other"* — §9.3's
headline Torat Emet operator — silently measures distance in markup.

**Scale.** 3,850 of 4,171 segments in Shulchan Arukh O.C. — **92%** — contain an
anchor with a Hebrew letter on both sides. Every one is a phrase the reader can see
on the page and the engine cannot match. Otzaria's `.txt` of the same sefer has the
identical defect (48% of its bytes, same phrase split), because it is a Sefaria
conversion too — so this is upstream of both corpora and not something either
project introduced.

**It is concentrated, not universal.** `mishnah-berakhot` and `bavli/berakhot` are
both 0%. The blast radius is Shulchan Arukh and its neighbours — which is the good
news for the fix and the bad news for the reader, since that is the shelf most
likely to be searched for an exact phrase.

**This is precisely the failure mode §9 was written to avoid**, arriving through
the corpus instead of through the analyzer: *"the reader is told the sefer does not
contain a line that is printed in front of them"* — `tokenizer.rs:10-12`, describing
the thing it was built to prevent. And §9.6's relaxation ladder cannot rescue it:
the ladder widens a query, it does not remove junk tokens from the index, so
`[try other forms — 7]` will be offered for a phrase that is right there.

**Build.** Strip the anchors, but strip them **at ingest, not at index time.**
Index-time stripping fixes the query path and leaves the stored text dirty for
snippets (`girsa-search/src/snippet.rs`), display, the Source Packet's `text`
field, and export (W22) — five places that each then need their own strip. One
pass at ingest serves all of them, and it is the same pass W33-A wants for §8.4
span anchoring: **mine the anchor to a span, then remove it from the text.**
The anchor's position is the span offset, so nothing is lost — it moves from being
noise in a string to being the data §8.4 asks for.

**Test first, and watch these fail:**

1. `girsa_hebrew::tokenize` on that segment yields `כארי` at the position
   immediately after `יתגבר`.
2. A Torat Emet phrase query for §9.5's own example `יתגבר כארי` returns
   `shulchan-arukh/orach-chayim/1:1#1`.
3. A proximity query for two words three apart on the page reports them three
   apart, not fifteen.

**Traps.** Do not strip with a naive `<[^>]*>` sweep — see W33-B, eight anchors in
that sefer are missing their opening quote, and note that `<b>`, `<i>` and `<small>`
carry real emphasis that display and export still want. Strip the empty
`data-commentator` anchors specifically.

**Siblings.** Anything that reads `segment.text` and assumes it is prose:
`girsa-search` (index, snippet, mekoros, the suspect miner in `girsa-fix` — an OCR
typo detector counting `commentator` as a corpus word will rank strangely),
`girsa-lane` (BEREL embedding markup alongside Hebrew), `girsa-app/src/display.rs`
`plain()`, `sending.rs` `plain_flavour`, and W22 export.

**Honest caveat.** The two file:line facts above are read from source, and the
`False`/`True` pair is measured. I did **not** run the indexer and watch a phrase
query miss — test 2 is the run that would prove it, and it should be written before
anything is changed.

**Reproduction.** `python C:\Users\Administrator\Videos\OtzariaSonim\tools\anchor_report.py`

---

### W33 · Two smaller findings from the same migration

**A. 62% of `segments.jsonl` for SA O.C. is unmined anchor markup.** The `text`
field carries Sefaria's inline `<i data-commentator="Turei Zahav" data-order="1"></i>`
anchors — 43,875 of them in that one sefer, **2.83 MB of the 4.55 MB**; the Hebrew
is 1.72 MB. That is `spec.md` §8.4 span anchoring already computed upstream and
sitting in the corpus unused, while every downstream reader has to strip it or
render empty italics. Mining it to span offsets and dropping it from `text` would
serve §8.4 and roughly halve that file.

**Scope, now measured rather than extrapolated — it is not corpus-wide:**

| | anchor bytes | segments with a mid-phrase anchor |
|---|---|---|
| `shulchan-arukh/orach-chayim` | **61%** | **92%** |
| `mishnah-berakhot` | 0% | 0% |
| `bavli/berakhot` | 0% | 0% |

So this is a heavily-commented-halacha phenomenon, not a flat tax on the corpus.
That makes it *cheaper* to fix than a 62%-everywhere number would suggest, and it
does not make W34 less severe — Shulchan Arukh is not a shelf anyone can afford to
have unsearchable. Run `anchor_report.py` across more shelves before sizing the work.

**B. A trap for whoever mines them.** 8 anchors in that sefer read
`data-commentator=Mishnah Berurah"` — **opening quote missing**, all of them
Mishnah Berurah. Upstream defect, same family as T2 and T4. A strict attribute
parser silently drops those anchors. Worth a row in §0.2 when A gets built.

---

### W35 · `segments.jsonl` is not a text file, and §4.1 says it should be

**The claim.** `spec.md` §4.1 is titled *"Text files on disk are the truth"* and
justifies itself with: *"The corpus stays greppable, diffable, backup-able, and
outlives the app."* `corpus/works/<slug>/segments.jsonl` does not deliver any of
those four, and it is one small change away from delivering all of them.

**Concretely, the same sefer in both shapes:**

| | Girsa `segments.jsonl` | OtzariaSonim packed `.txt` |
|---|---|---|
| read segment 2,940 | parse 2,940 JSON objects | it is line 2,940 |
| `grep` a phrase | hits return a JSON blob; the phrase may be split by markup | hits return the sentence |
| `git diff` a typo fix | one re-serialised line, id + kind + text | the sentence that changed |
| `sed`/`awk`/`wc` | no | yes |
| SA O.C. on disk | 4.35 MB | **1.71 MB** (anchors stripped) |
| survives editing | **yes — permanent IDs** | **no — line-addressed** |

The last row is the honest one and it goes the other way. My format reintroduces
exactly the defect T1 and §3 exist to prevent: it addresses by line number, so a
typo fix that splits a line silently repoints every link below it. I can live with
that because the phone never edits anything — it is a read-only reader against a
corpus packed on a PC. **Girsa cannot, and should not adopt it.**

**So the suggestion is not "use my format." It is: the two properties are not in
tension, and JSONL is buying the second at the cost of the first for no reason.**
Put the text in a `.txt`, one segment per line, and keep the identity in a sidecar:

```
works/<slug>/text.txt        one segment per line, plain, no anchors
works/<slug>/segments.jsonl  {"id": …, "kind": …, "line": N}   -- no "text" field
works/<slug>/work.json       unchanged
```

Line N of `text.txt` is the text of the segment whose sidecar row says `"line": N`.
IDs stay permanent and stay authoritative; the redirect table still absorbs
re-segmentation; nothing in §3 changes. What you gain is that §4.1 becomes true:
the corpus is greppable with `grep`, diffable with `git diff`, and a person with
`sed` can read it in fifty years without Girsa. And `girsa-fix`'s export-a-fixed-sefer
(W22) becomes a copy rather than a re-serialisation.

**Worth weighing against.** Two files per work instead of one, so an ingest that
half-fails can now half-fail asymmetrically — an assertion that `text.txt` line
count equals the sidecar row count, run at import, closes that. And a text file
cannot hold a segment containing a newline; if any `kind` ever needs one, that
`kind` stays in JSON and the rule becomes "prose lives in the text file."

**Not urgent, and not a defect** — unlike W32 and W34, nothing is currently wrong.
It is a cheap change while W34 is already rewriting how `text` is stored, and
expensive later once tools depend on the shape.

**Reproduction of the size column.**
`python C:\Users\Administrator\Videos\OtzariaSonim\tools\anchor_report.py`

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

---

# Seven things from opening the window — filed 31 July 2026

Reader feedback after the W32 orientation fix landed, in the reader's own order.
The evidence for each is *open it and look*, so these carry a mechanism and a
test rather than a measurement.

Read together they say one thing: **the link graph is now right and none of that
is reachable.** W36 and W37 are the two that matter; the rest are the reasons a
person gave up before getting to them.

---

### W36 · The mefarshim have no door

> *"i have no clue how to even open mefarshim."*

**The mechanism.** There is no affordance for commentary anywhere in the window.
A commentary is one row of the links drawer, typed `מפרש`, mixed in with every
`קשור` — and `references` outnumber `comments-on` more than two to one on a
typical daf (36,474 to 15,394 on Berakhot). So opening Rashi requires knowing
that the links drawer exists, that `מפרש` means commentary, and how to pick it
out of a list sorted by confidence.

Rust already has everything this needs and nothing consumes it:
`girsa_app::beside` reads `Work::commentary_on`, which is exactly *the seforim
printed on this page*, and it is what W9 was for. `laneview.ts`'s "adjacent" is
the semantic search lane (W30) and is unrelated.

**Build.** A **מפרשים** control on the pane's own header — not in a drawer —
listing the works that declare themselves commentaries on this sefer, each
opening beside the text in the split tree. The default set is the ones actually
linked to the line you are standing on; the full list is one click further.

**Test first, and watch these fail.**
1. Open `bavli/berakhot` at `2a:1`; the pane header offers `מפרשים`, and the list
   contains Rashi.
2. Clicking it opens a second pane, in the same split, scrolled to the Rashi on
   *that* line — not to the top of Rashi on Berakhot.
3. Open `genesis` at `1:1`; the list contains Ramban, Ibn Ezra, Radak, Sforno,
   Rashbam and Onkelos. All six were unreachable before the orientation fix and
   all six are reachable now, so this is the test that says the fix arrived
   somewhere a person can see.

**Acceptance.** A reader who has never used Girsa opens a daf and gets Rashi
beside it without being told anything. And it is the *pane* that offers it, so
the affordance is where the sefer is.

---

### W37 · The links drawer is a repair workbench wearing a reader's clothes

> *"kishuri i cant tell what is going on. it is hard to read."*

**The mechanism.** `linksview.ts` gives every row a type chip, an address, up to
seven provenance facts joined by a middle dot — `87% · sefaria-seed · על מילים ·
"commentary" · היה: קשור` — and **five action controls**: confirm, reject, a type
`select`, reanchor, and pin-to-words. On Berakhot 2a:1 that is now thirty rows
and about two hundred controls in one 620px drawer.

Worse, what a reader actually wants is not there at all. The row shows
`link.said`, an *address*, with the sefer's name buried inside it, and **no
commentary text**. So the panel answers "how confident is this edge and by what
method" and never answers "which sefer is this and what does it say".

W23's *show your work* was right for the repair queue and wrong as the default.
Both readings are legitimate; they are not the same surface.

**Build.** Two states, one panel. Reading is the default: **sefer name**, the
dibur hamatchil or first words of the target, and nothing else. Provenance and
the five repair controls move behind a per-row disclosure, and a *check the
links* mode turns them all on at once for someone doing W23's work.

**Test first, and watch these fail.**
1. A row in the default state contains the target sefer's title as its own
   element, and the first words of the target text.
2. A row in the default state contains **zero** `button` elements other than the
   row itself and its disclosure.
3. Turning on *check the links* restores every control W23 needs, and
   confirm/reject/retype still round-trips.
4. Sorted so `comments-on` precedes `references`, because a mefaresh is what a
   person came for.

**Acceptance.** Berakhot 2a:1 is legible at a glance: which mefarshim are on
this line, in reading order. The repair affordances are all still reachable and
none of them are in the way.

---

### W38 · Eleven panels are painted over the text

> *"i opened Shmiras Halashon and it is weirdly over the text, so i cant see it
> or the text."*

**The mechanism.** Eleven panels are `position: fixed` with `z-index` 10–40 and
none of them reflow the reading surface:

`.links` `.suspects` `.fixbox` `.picker` `.shelf` `.find` `.find-chip-menu`
`.said` `.writing` `.yours` `.lane-panel`

`.links.is-open` is `width: min(620px, 50vw)`, `top: 0; bottom: 0; left: 0` —
half the window, opaque, pinned to the left edge, which in an RTL window is
where the lines *end*. The text is not narrowed, it is covered. Two of these
open at once and the sefer is behind both.

`layout.ts` already has the right answer and the panels do not use it: a flex
split tree with draggable dividers, where opening a thing makes room for it.

**Build.** A panel holding content a reader reads *alongside* the text —
`links`, `yours`, `lane-panel` — becomes a leaf in the split tree and gets a
divider. Transient things that take a choice and close (`picker`,
`find-chip-menu`, `said`) may stay floating, and should dim what is behind them
so it is clear they are modal.

**Test first, and watch these fail.**
1. With the links panel open, the reading pane's rendered width is strictly less
   than with it closed. (Today it is identical — that is the bug in one line.)
2. No two panels from the reflowing set can be open and overlapping.
3. Opening a sefer from the shelf closes the shelf.

**Acceptance.** Nothing a person is reading is ever behind something else.

---

### W39 · A search result does not show what matched

> *"the search result is not clear (the actual hit)."*

**The mechanism.** B16 built `girsa_search::snippet` — a window around the
**first mark**, with elisions, carrying the matched range as offsets — and
`search.ts:420` renders the row as `${hit.he_title} ${hit.address}`. The
snippet's marks are computed in Rust and thrown away in the window, so a result
is a citation with no words in it, and where words are shown the match is not
distinguished from its surroundings.

**Test first, and watch these fail.**
1. A result row contains the snippet text.
2. The matched range is inside an element of its own, so it can be styled and
   read by a screen reader as the match.
3. A hit whose match sits 400 characters into a long segment still shows the
   match, not the first 220 characters of the segment. This is the property
   `snippet()` was written for and nothing exercises it end to end.
4. Searching the phrase from spec.md §9.5 shows both words marked in the same
   row — which fails today for the separate reason in W34, and is the test that
   ties the two together.

**Acceptance.** A reader scanning results can tell which one they want without
opening any of them.

---

### W40 · A tab cannot be closed without opening it first

> *"needs a way to close tab without going in."*

**Build.** A close affordance on every tab, reachable without activating it, and
Ctrl+W for the focused one. It must be a real control with a name — the B14 rule
— not a bare glyph.

**Test first, and watch these fail.**
1. Clicking a background tab's close control closes that tab and does **not**
   make it active first.
2. The control has an accessible name that says which sefer it closes.
3. Closing the last tab leaves the window in a state that says what to do next,
   not an empty grey rectangle.

---

### W41 · There is no UI language, and the seforim are named in only one

> *"hebrew and english ui. all seforim names in hebrew ui should be heb, all in
> english ui should be english."*

**The mechanism.** There is no language setting at all — the window is Hebrew
throughout, and `en_title` is used in exactly one way: as a `title=` tooltip
(`pane.ts:81`, `scanview.ts:180`). The corpus carries both names for all 7,189
works, so the data has been there the whole time.

**Build.** One setting, and **every** sefer name in the window reads it — tabs,
shelf, picker, links rows, search results, citations, chain steps, companion
lists. The rule as stated: Hebrew UI names seforim in Hebrew, English UI in
English.

**Test first, and watch these fail.**
1. A source-reading guard: no `.ts` file outside one module reaches for
   `he_title` or `en_title` directly. This is the only shape that holds, because
   the defect is thirteen call sites and the fourteenth is written next week —
   the same reason B14's `controls.ts` made the accessible name a required
   argument.
2. With the language set to English, no element in the shelf contains a Hebrew
   letter.
3. Switching language does not reload, does not lose the open panes, and does
   not change the *text* of any sefer — only its name. The text is the text.

**Acceptance.** Both settings, every surface, one module that chooses.

---

### W42 · A level of the shelf mixes folders with seforim

> *"i dont like to have folders and files. all files should be put in an other
> folder if needed."*

**The mechanism.** The shelf renders Sefaria's category path directly, and those
levels are ragged: a category holds sub-categories *and* loose works at the same
level, so a level reads as a file manager rather than as a shelf.

**Build.** A level is either all folders or all seforim. Where a level has both,
the loose seforim go into a child — *other* — named the same way everywhere.
That is the reader's own rule and it is a good one, because it makes depth mean
something.

**Test first, and watch these fail.**
1. For every node of the rendered shelf: it has folder children, or work
   children, never both.
2. No work becomes unreachable by the change and none appears twice — the count
   of reachable leaves equals the count of works on the shelf before it.
3. A category whose only content is loose works does **not** grow a pointless
   *other* containing everything.

**Acceptance.** Walk the shelf to any sefer and every level was a choice between
things of one kind.
