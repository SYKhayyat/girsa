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

   **And then the same again from `app/src-tauri`.** Those four run against
   `default-members`, which excludes the Tauri shell for a good reason (it cannot
   build before `app/dist` exists) — so they compile everything in this
   repository *except the 4,201 lines that own all the interop*.

   CI has a `shell` job and does catch it; the point of running it here is the
   three minutes and the push. It has caught it twice, and the second time is the
   one worth reading: on **9 August** the shell called
   `girsa_desk::refreshed_reporting`, which existed in `refreshing.rs` and was
   not in the `pub use` line of its own `lib.rs`. Red in CI from that commit
   onward, green on every local gate, and the fix sat uncommitted in a working
   tree for three days. A job nobody looks at is a job that reports to nobody.

   ```sh
   cd app/src-tauri && cargo clippy --all-targets -- -D warnings && cargo fmt -- --check
   ```

   And the window: `cd app && npx tsc --noEmit && node test/run.mjs`.

   The browser build is the cheapest way to look at what you changed —
   `cargo run -p girsa-app --example dev-fixtures -- corpus app/public/dev` then
   `npm run dev`. It found four defects in ten minutes that all of the above had
   passed over, three of them in code committed an hour earlier.
5. **Commit per work order**, with a message saying what changed and what it does
   *not* yet do.
6. **Never guess at a citation, a link, or a ref.** Ambiguity is surfaced to the
   user as a choice. This is a product rule, not a style preference — a wrong ref
   is worse than no ref, everywhere in this system.
7. **A test may not pass because it could not find what it checks.** No
   `if !present { return }`. If the input is missing the test either builds it —
   `girsa-fixture` is there for exactly this — or it is `#[ignore]`d so the run
   prints `ignored` and says so. This rule is not new; it was written down in
   `tools/check-ksav-fixture.sh` and then broken by forty-three test functions
   across ten files, which spent their whole existence printing
   `ok … finished in 0.00s` in CI. Among them was the one that would have caught
   §3's permanent ids being renumbered by every re-import. **Rule 1 says watch it
   fail; this is what happens when nobody can.**

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

**Amended 2026-08-07 — the packet carries a character range (sefer-crates 0.5.1).**
A ref names *places*. A reader who highlights half a se'if gets the words they
highlighted, and the packet said so in `text` and said nothing about **which
characters of the place** those were — so §10.2's *only the highlighted part
goes* and §7's *citations stay alive* contradicted each other at the
regeneration step: `/quote` handed back the whole se'if. `SourcePacket.range` is
`Range { from, to }`, optional, counted in characters of the text **as it was
shown**. Absent means *nobody recorded one* and regenerating whole is the only
honest answer; `Range::all()` means *the reader chose the whole place*.

Three things had to move together, and this is why the field change is on the
stop-and-ask list above:

1. `girsa-source` grew the field and `SourcePacket::part`.
2. `girsa-ksav`'s `mekor` writes it into the document as `תווים: "4-19"` —
   omitted for the whole place, so every document already on disk still reads —
   and `cited_in` reads it back. A range that stopped at the packet could never
   be asked about again.
3. Ksav's `typst/ksav.typ` learned the argument, and its own test compiles a
   partial quote with the **real Typst engine**. `girsa-ksav` can assert it
   wrote the string; only Ksav can assert Typst accepts it.

On this side: `send` puts the reader's highlight on the packet, `quote` takes a
range back and regenerates exactly it, and the desk's `/cite` and `/quote`
errands carry it. `quote` also reads `Ref::to()` now — it never had, so a
citation of three se'ifim regenerated as its first one, with no error to say the
rest had been dropped.

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

**Amended after the fact, because the acceptance above was passed by a design
that did not hold it.** All three scenarios were tested in memory, on a
`SegmentStore` that was never written down. `import::write` emitted `work.json`
and `segments.jsonl` and had no slot for a redirect table, so the third scenario
— *"and for an upstream re-segmentation via the redirect table"* — was green
against a table that could not survive the process exiting. Underneath it,
`SegmentStore::import` derived ordinals from enumeration position on **every**
run of `girsa-import`, so re-importing after Sefaria added one se'if renamed
4,170 segments: T1, at import granularity, inside the order that exists to
prevent T1.

So the acceptance is now: **run the importer twice**, with one se'if inserted
between the runs, and assert through `write`/`read_back` that every other name
still resolves to the same words. An in-memory assertion does not count here;
the disk is the whole point. `crates/girsa-corpus/tests/a_reimport_keeps_every_name.rs`,
and `--example measure-continuity` is the same check over the real corpus.

**Amended a second time, one level over, for the same reason.** The acceptance
says *"assert every one of those 501 links still resolves to the same text"* —
and that was asserted of the store and of the importer. The thing a reader
actually looks at was never in the room. `girsa_app::touching` matched a stored
anchor against the line you are standing on with `SegmentId::covers`, a prefix
test on the ordinal, which never opens `redirects.jsonl`. So it lost an edge to a
merge. Worse, it says yes to `#1.1` whether a cut carved it out of `#1` or
`mint_between` named a se'if upstream **inserted** after `#1` — both are spelled
the same — so it *gained* edges onto se'ifim that did not exist when the edge was
written. A missing link looks thin; an invented one is a claim nobody made, which
is rule 6 with the sign flipped. Notes, highlights, folders and hand-drawn links
each had their own copy of the same predicate and so the same two faults.

What tells a cut from an insertion needs no new file: **a cut deletes its parent
and an insertion does not**. So the acceptance is now also: re-import a work with
a se'if inserted, and one with a se'if folded away, and **ask the panel** rather
than the store. `crates/girsa-app/tests/a_link_survives_a_resegmentation.rs`, and
`--example measure-standing -p girsa-app` is the same question over the real
4.18M-edge graph. There is one predicate now — `girsa_corpus::standing::Standing`
— because six implementations of *does this anchor name these words* is five too
many, and the disagreement between them is invisible from the reader's chair.

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

**Amended after the fact: *your own documents* meant the toy editor's.**
`who_cites` delivered §10.4 by iterating `Buffer::list(personal)` —
`personal/ksav/*.ksav`, the directory of the four-hundred-line text box W17
built so the loop could be demonstrated without Ksav installed. So a `.ksav`
written in **the real Ksav, the application this entire pairing exists for**,
was never found: the reader's actual work, in the actual editor, answered
*nothing cites this*. And `buffer_to_ksav` saves into the personal layer *and*
posts, so there are two copies of one document with no owner between them, and
the answer came from the stale one.

There is nowhere to walk instead. A reader's documents live wherever they keep
documents — a shiur folder, a synced drive, a stick — and Girsa has no business
enumerating a disk. So Ksav says so: **`/document` on the desk**, and the path
lands in `personal/documents.jsonl`, a `girsa_personal::Log` like everything
else in the layer. This region argued for exactly that shape at length, for the
link graph (`links.rs`'s module note), and then wrote a directory walk.

Each row caches the refs it held and when it was read, and is re-read only when
the file has moved on — a modification time answers the question actually being
asked, which is `girsa_note::since`'s rule. A file that has gone is **not
forgotten**: it is reported as missing and still answered from, because a stick
that is not plugged in is not a document that was never written.

The desk earning itself as a **query** transport, which is the argument §10 is
really about: a push with a reply, which a clipboard cannot be.

**Still open, and said rather than left:** the window has no caller for
`who_cites` at all — it is a Tauri command nothing presses. `girsa-notes
documents`, `document <path>` and `cites <ref>` make the whole loop runnable
from a terminal, per §0.3, so what is missing is a button and not a mechanism.

---

## Tier 6 — Corrections

### W20 · The overlay

**Never mutate base text.** Patches are `segment ID + character span + provenance +
timestamp`. A `kind` field distinguishes an OCR error from a girsa variant — same
machinery, different claim, and it unifies with the `emends` edge type.

**The guardrail is a requirement, not a hope:** if correcting a typo is not a
**three-second interaction** from where you are reading, nobody does it. Measure
it.

**Amended after the fact, because *measure it* was satisfied by a measurement
that stopped just short of the failure.** `three_seconds.rs` measured at zero
corrections and at a thousand, printed 75 ms and 217 ms, and its own comment
named what it was watching for — *"an overlay that is fast when it is empty and
quadratic when it is not."* It was. `Layer::add` serialized **every** patch on
every call, so the 142 ms of difference was linear in what you already had and
the three-second line sat at about twenty thousand corrections — one order of
magnitude past the last size measured.

So the acceptance is now: **measure past where it would have failed.** The third
case corrects sixteen thousand lines of Mishnah Berurah, which is three typos a
day for sixteen years, and it is a case the old design could not have run at all
— getting there would have meant writing 128 million lines to make 16,000
corrections.

And the timing is not the guard. A wall-clock assertion fails on a loaded machine
and teaches nobody anything; what is asserted is the property underneath, in
`crates/girsa-fix/tests/a_correction_is_one_line_written.rs`: write a correction
that sorts **before** every one already held, and every byte that was in the file
is still in the file, in the same place. True of an append, false of a rewrite,
and false even when the rewrite lands a file of identical length.

### W21 · OCR-error detection

A word appearing once in the corpus, one edit-distance from a word appearing ten
thousand times, is almost certainly an error. Batch job → **ranked reviewable
queue**. This is worth more than the editor.

*The queue reaches **28,124 entries** on the real corpus, and going down that
list is the whole motion. Every decision used to rewrite all 28,124 lines; it now
appends the one that changed. See W20's amendment and `girsa-personal`.*

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

**Amended after the fact: one store, not six.** Marks, saved questions, folders,
corrections, link repairs and the spelling queue each grew their own copy of the
same file store, and each copy had the same defect — the whole collection
serialized on every mutation — plus its own hand-written
write-beside-and-rename. Six correct solutions to one problem, none of which
knew about the other five.

They are now one: `girsa-personal`'s `Log`. Same jsonl, read as an append-only
log — a record is a line, a later line for the same key wins, `{"gone":"…"}`
takes one back, and the file is rewritten only when it has grown past twice what
it holds. It is a leaf crate on purpose: `girsa-fix`, `girsa-note` and
`girsa-link` are siblings and none may depend on another, so a crate is the only
place the seventh copy could have *not* been written.

Nothing had to be migrated. A file with no repeated keys and no tombstones is its
own compaction, so every file any earlier version wrote replays to exactly what
it always meant — which is the property to insist on here and nowhere else,
`personal/` being the one directory a reader cannot re-download.

**And the leaf crate turns out to own more than the writing.** `girsa-note`'s
`since.rs` has to count how many corrections are newer than the search index.
Corrections are `girsa-fix`'s, and the two are siblings, so neither may name the
other's `Patch`. What it did instead:

```rust
if !body.contains("\"when\"") { … }
line.split("\"when\"").nth(1).trim_start_matches([':', ' ', '"'])
```

One crate parsing another's file by string surgery, with `serde_json` sitting
unused in its own manifest, purely because a type name was out of reach. It was
**correct** — a `"when"` inside a string value is escaped on disk, so the split
cannot land in one — and correct by luck rather than by construction, and it
would have gone on being silently correct right up until somebody added a field
called `whenever`.

The answer was never to reach for `Patch`. **Counting records in a log is a fact
about the log format**, and the format is this crate's — the same argument that
already put `is_tombstone` here. `girsa_personal::since` reads one field, `when`,
off any record any store in the layer writes, and reports how many are live and
how many are newer than a moment. A record it cannot date counts as newer, which
is the safe direction: over-reporting sends a reader to rebuild an index they
might not have needed to, under-reporting is the silent gap W26 exists to close,
and of the two only one is a lie.

`no_crate_reads_another_crates_file_by_string_surgery` holds it.

**Amended again: derived once is not derived.** A note lives twice — the `.md`
you can open in vim, and the `segments.jsonl` the shelf and the search index
read. The second was derived from the first exactly once, when the note was
written, and never again. `Notes::open` read only the `.md`; `Shelf::read` and
the index build read only `segments.jsonl`. So editing a note outside this
application — which "exportable as plain files" invites in as many words — left
two versions of it: the words you wrote, and the words the search box can find.

Worse, the machinery that exists to make a gap loud made this one silent.
`since.rs` stats the `.md`, sees it is newer than the index, and says *N notes
are not searchable yet*; you rebuild; the build reads the **stale**
`segments.jsonl`; the stamp is now newer than the `.md`; the gap reports zero. A
closed loop in which *"never a silent gap"* reports success over a gap of its own
making — and the reason it costs a rebuild to learn nothing is that neither half
of the loop ever compared the two files to each other.

`Notes::open` now does: two `stat` calls per note, and a re-shelve when the `.md`
is the newer of the two. Missing counts as stale, because a note with no shelf
entry is the same problem arrived at from the other side. `note.rs`'s
`a_note_edited_in_vim_is_what_the_search_box_finds` writes a note, sleeps past
the filesystem's timestamp granularity, rewrites the file the way an editor
would, reopens, and reads `segments.jsonl` — it fails on the old code, which is
the only reason to believe it.

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

*Built. `girsa-lane`, `girsa_nearby::adjacent`, the `adjacent` MCP tool, and the
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

### W31 · `touching.jsonl` needs the source work — **shipped, then reversed 6 August 2026**

> **Read this first.** W31 shipped and has since been **taken out**, and the
> reason is not that it was wrong when it was written. `touching.jsonl` no
> longer exists; the file beside the edges is `touching.bits`, one 16-bit mask
> per segment in reading order, 9.7 MB for the whole shelf against W31's 448.7
> MB over 6,268 files.
>
> W31's argument was that answering *which of my ticked mefarshim speak here*
> meant reading `inbound.jsonl` — 27.3 MB and 156,076 rows for Orach Chayim —
> and on a 192 MB heap that is the difference between opening the sefer and an
> `OutOfMemoryError`. True when written. **W28 then sorted `inbound.jsonl` by
> where its rows land and wrote `inbound.idx` beside it**, so that question is
> now a seek into *4,171 places rather than 159,273 rows*. The 12× file was
> paying for a read that had stopped happening one work order later, and nobody
> went back to notice — which is the pattern the 6 August lamdan report is
> about, arriving at the file that report used as its example.
>
> The order is kept, unedited below, because the measurement in it is still the
> best record of what the format cost and why. The **acceptance test it asks
> for is still the right one** and now runs against the masks.

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

**Amended after the fact: "one module that chooses" was six.** The guard above
holds for TypeScript — no `.ts` file reaches for `he_title` — and the leak went
the other way. Six Rust rows describe a segment for a surface, and each worked
out what to call it, where it sits and when it was written, on its own:

| | title | address | dated |
|---|---|---|---|
| `HitRow` (window search) | `Language::title_of` | `path().join(":")` | no |
| `Near` (semantic lane) | `he_title`, falling back to **the empty string** | *nothing at all* | no |
| `girsa_mcp::named` | `he_title` | `path().join(":")` | yes |
| `girsa-chain`'s printer | `he_title` | `path().join(":")` | yes, and `[no date]` |
| `PatchRow` | `Language::title_of` | `path().join(":")` | no |
| `SuspectRow` | `Language::title_of`, falling back to **`null`** | `path().join(":")` | no |

Read the columns, not the rows. **One** of the six honoured the language, so a
reader who set the window to English got English titles in the search column and
Hebrew ones in the lane column beside it. **One** had no address, so the shell
and `girsa-lane ask` each invented one — `58:1` in the window and the whole
permanent id on the terminal, for the same result. **Two** carried the date, and
they are the two a reader looks at least.

None of that was decided. `HitRow` got the language because it was built where a
`Session` was in scope; `mcp::named` got the years because it was built where a
`Timeline` was; `Near` got neither because `Adjacency::ask` took a `&Shelf`.
Six people answering one question from wherever they were standing.

`girsa_app::Naming` is the answer and `girsa_app::Names` is what it takes to
reach it — a shelf, a timeline and a language, passed **instead of** a bare
`&Shelf` so no arity grows and a caller with no dates says so once rather than
by leaving a column quietly blank. `SegmentId::address` is the address, spelled
once where seventeen sites spelled it and eleven more skipped it. `When::written`
is the years column, spelled once where three sites spelled it.

The window gained a `Timeline` in the doing: it had none, so every row it drew
was undated while the MCP server's were not.

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

---

### W43 · Choose your mefarshim, then click a line — the Otzaria model, kept beside the split

> *"there should be a way to have mefarshim also open like otzaria (click on
> text opens checked mefarshim)"* … *"but keep the split too — its also nice"*

**What Otzaria does**, read out of its own source: `CommentatorsActivity` is a
checkable list of the commentators on the open sefer, persisted per book
(`Settings.selectedCommentators`, a string set keyed by title). `Ui.kt:82`
prepends a `◆` to any line carrying commentary from the checked set, and tapping
such a line opens those commentators' comments **on that line**. The marks come
from a bitmap per commentator in its `.idx` sidecar — one bit per line — so the
marker costs nothing to draw.

**What Girsa does today** is one interaction: press `מפרשים · N`, pick **one**
sefer, and it opens as a whole pane in the split. That is a good interaction and
it stays. It is a different question, though: *put Rashi beside the Gemara and
keep the two in step* is not *what do my six mefarshim say about this line*.

**Girsa needs no sidecar for this.** `corpus/links/<slug>/inbound.jsonl` is one
file per sefer, and reading Berakhot's — 3.4 MB, 21,065 rows — and indexing every
`comments-on` by target segment takes **0.07s** and yields 3,183 marked segments
with their commentator sets. Once per open sefer, in memory, is enough for both
the markers and the click.

    bavli/berakhot/10a:1#418  <-  rashi, rashba, tosafot-harosh, ben-yehoyada,
                                  chidushei-agadot, steinsaltz

**Build.**
- `girsa_app::mefarshim` — `Marks`, built from one `inbound::read_back`: which
  works comment on each segment. Plus the chosen set, persisted per sefer in
  `Session` beside `where_i_was`.
- A marker on every line that has commentary **from the chosen set**, not from
  all of them — otherwise every line on the daf is marked and the marker says
  nothing.
- Clicking a marked line opens the chosen mefarshim on that line, and only that
  line.
- The checkable list is the same list `מפרשים · N` already opens; checking is
  what is new.

**Test first, and watch these fail.**
1. `Marks::of` on `bavli/berakhot` puts `bavli/rashi-on-berakhot` on
   `bavli/berakhot/10a:1#418` — a fact from the real corpus, so the test fails
   the day the graph regresses again.
2. With nothing chosen, no line is marked. With Rashi chosen, the marked set is
   exactly the segments Rashi comments on — not a superset.
3. A chosen mefaresh that comments on nothing in this sefer marks nothing and is
   not an error.
4. The chosen set survives closing and reopening the sefer, and is per sefer:
   choosing Rashi on Berakhot does not choose Rashi on Shabbat.
5. Clicking an unmarked line says so, rather than opening an empty panel.

**Acceptance.** Check six mefarshim on a daf, and the daf shows which lines they
speak about; click one and read them, on that line. The `מפרשים · N` split still
works and is untouched.

---

### W44 · Mefarshim keep their folders

> *"it would be nice if meforshim remained in their folders"*

**The mechanism.** The companion list is flat. The corpus already knows the
structure and it is the structure a person actually uses:

| sefer | its mefarshim group as |
|---|---|
| `bavli/berakhot` (34) | Rishonim on Talmud (16), Acharonim on Talmud (16), Modern (2) |
| `genesis` (67) | Acharonim on Tanakh (17+), Rishonim on Tanakh (5+), and per-author folders under each — Abarbanel, Chida, Avi Ezer, Mechokekei Yehudah |
| `shulchan-arukh/orach-chayim` (18) | Shulchan Arukh > Commentary (17) |

Rishonim and Acharonim is not a filing convention, it is the first thing a person
wants to know about a mefaresh. A flat list of 67 throws it away.

**Build.** Group the list by the same shelf `girsa_app::taxonomy` already
computes — `tree()` over the mefarshim subset rather than over the whole corpus,
so there is exactly one idea of which shelf a sefer is on (the reason
`taxonomy.rs` exists at all). The depth is not fixed: the grouping category is
whichever one says Rishonim / Acharonim / Commentary, which sits at index 2 for
Talmud and index 1 for Tanakh.

**Test first, and watch these fail.**
1. The mefarshim of `bavli/berakhot` come back in exactly three groups, and
   `bavli/rashi-on-berakhot` is under the Rishonim one.
2. Grouping loses nothing: the leaves summed equal the flat list's length.
3. A sefer whose mefarshim all share one folder does not get a tree one level
   deep containing everything — a group of one group is not a group.
4. The grouping uses `taxonomy::shelf_key_of`, so a shelf the reader has moved
   or renamed is respected here too. (A source-reading guard: this module does
   not read `categories` directly.)

**Acceptance.** Open the mefarshim on Genesis and the Rishonim are together,
under a heading, above the Acharonim.

---

### W45 · A mefaresh is a commentary on **this** sefer

> *"shabbos is put as a mefaresh on shulchan aruch which is absurd. rashi on
> berachos is put as a mefaresh on tur. this is crazy."*

Both true, and both mine. W43's `Marks::of` took **every** incoming `comments-on`
edge and called the far end a mefaresh. That is inferring a commentary
relationship from the presence of an edge, which is the thing BUILDER.md rule 6
exists to forbid, and I wrote it three commits after fixing the same class of
error in the orientation layer.

Why the test suite missed it — the number that should have been the tell:

| sefer | works with `comments-on` landing in it | declared commentaries on it |
|---|---|---|
| `bavli/berakhot` | 30 | **30** |
| `shulchan-arukh/orach-chayim` | 48 | 17 |
| `tur` | 40 | 4 |

`rashi_is_on_the_first_line_of_the_daf_in_the_real_corpus` reads Berakhot, where
the graph is clean, so it passed. One masechta is not the corpus. The oracle test
`the_meforshim_are_on_the_daf` has the same shape — it asks *is Rashi reachable*,
never *is anything reachable that should not be*, except for five hand-picked
negative controls that happened to be about Shulchan Arukh mefarshim rather than
about Talmud appearing on Shulchan Arukh.

**Not fixable by keeping only declared commentaries.** The biggest mefaresh on
Orach Chayim is the Kaf HaChayim, 29,956 edges, and Sefaria declares nothing for
it. Nor is the Biur Halacha, the Levushei Serad, or R' Akiva Eiger. Dropping the
undeclared drops those.

What separates them is the shelf, and Sefaria's own filing says it plainly:

    kaf-hachayim-on-...orach-chayim  ['Halakhah','Shulchan Arukh','Commentary','Kaf HaChayim']
    bavli/rashi-on-berakhot          ['Talmud','Bavli','Rishonim on Talmud','Rashi']
    bavli/shabbat                    ['Talmud','Bavli','Seder Moed']
    shulchan-arukh/orach-chayim      ['Halakhah','Shulchan Arukh']
    tur                              ['Halakhah','Tur']

**The rule.** A work is a mefaresh on a sefer when it has commentary edges
landing there **and** one of:

- it **declares** it — `commentary_on` names that sefer; or
- it stands on a **commentary shelf** (`Commentary`, `Rishonim…`, `Acharonim…`,
  `Modern Commentary…`, `Targum…`) whose shelf-above is the shelf the sefer
  itself stands on.

No slug parsing, no title matching. Both halves are the corpus's own statements.

Every case falls out:

| commentary | sefer | why |
|---|---|---|
| Kaf HaChayim | S.A. O.C. | commentary shelf `Halakhah/Shulchan Arukh/Commentary`, above it `Halakhah/Shulchan Arukh` = O.C.'s own shelf ✓ |
| Beit Yosef | Tur | `Halakhah/Tur/Commentary` over `Halakhah/Tur` ✓ — **gained**, 18,353 edges, and `declared` alone missed it |
| Rashi on Berakhot | Tur | declares Berakhot; its shelf-above is `Talmud/Bavli`, not `Halakhah/Tur` ✗ |
| Shabbat | S.A. O.C. | not on a commentary shelf at all ✗ |
| S.A. O.C. | Tur | not on a commentary shelf ✗ |
| Yerushalmi Ketubot | Tur | ✗ |

**Where it lives.** `girsa_corpus::taxonomy::is_commentary_on`, beside `shelf_of`,
because it is the same knowledge about the same field and a second reader of
`categories` is how a sefer ends up a commentary here and not there. `Marks::of`
takes the shelf so it can ask.

**Test first, and watch these fail.**
1. Shabbat is not a mefaresh on Shulchan Arukh, Orach Chayim.
2. Rashi on Berakhot is not a mefaresh on the Tur.
3. Shulchan Arukh, Orach Chayim is not a mefaresh on the Tur.
4. The Kaf HaChayim **is** one on Orach Chayim, though it declares nothing.
5. The Beit Yosef **is** one on the Tur, though it declares nothing.
6. Rashi is still on Berakhot, and all 30 of Berakhot's survive — the fix must
   not be a filter that quietly empties the daf.
7. Over the **whole corpus**, not one masechta: every work the tick-list offers
   on ten sample seforim from five different parts of the shelf passes the rule.
   This is the shape the old test lacked, and the reason it lacked it.

**Acceptance.** The tick-list on the Tur has the Beit Yosef, Prisha, Bach and
Drisha in it, and does not have Rashi on Berakhot.

**Amended after the fact: the rule had three callers and one of them asked it.**
`taxonomy::stands` says in its own doc comment that it is *"the question W43's
tick-list, and anything else that says these are the mefarshim on this sefer,
has to ask"*. Three things say it:

| | reads | asked |
|---|---|---|
| `Shelf::companions` — the picker | `companions.jsonl`, every edge type | nothing: the `commentary_on` field |
| `mefarshim::Marks::of` — the tick-list | `inbound.jsonl`, `comments-on` only | `stands`, then its own private threshold |
| `Beside::between` — the column | both works' shards, every edge type | nothing: the `commentary_on` field |

Three data sources, three rules, three on-disk caches with three generators, and
**no test that they agree**. So the Beit Yosef — this section's own example,
declared nowhere and a mefaresh on the Tur by its shelf — was a full mefaresh in
the tick-list, an *undeclared* counted link in the picker, and `Relation::Linked`
rather than `Declared` in the column, which meant the pane never fell back to
lining up by address. And `mefarshim.ts` filters the picker's `declared` flag to
count the button, so the button read **5** over a list of forty.

`taxonomy::settled` is `stands` with the one case it refuses to guess at settled
by an edge count, and all three ask it. The threshold that resolves that case
moved out of `girsa_app::mefarshim`, where it was private, to sit beside the rule
it belongs to.

Three silent gaps closed in the doing:

- **Two unfiled seforim were `Alongside` each other.** `canonical_path` answers a
  work with no categories with a *default top*, so two files a reader dropped on
  the window matched on that default — a claim that they keep the same order,
  which would line them up by address. Four `beside.rs` tests caught it the
  moment the picker and the column started asking the same question.
- **`companions.jsonl` recorded a truncated list as a complete one.**
  `girsa-companions` keeps the 200 thickest joins per work — Berakhot has about
  1,600 — and printed what it dropped to stdout at the end of the run. The row
  carries the total now, and `Shelf::joins` reports both.
- **No inbound cache read as *nobody comments on this sefer*.** `Marks::of`'s doc
  says a missing file is not an error and leaves the distinction to the caller;
  the caller did not make it. `Mefarshim.unbuilt` does.

`crates/girsa-app/tests/one_answer_to_which_seforim_relate.rs` holds it — five
tests, and it does **not** collapse the three. They are three questions, and a
generous offer, a strict list and a placement rule are all correct answers to
different ones. What it holds is that they use one predicate where they mean one
thing.

**Amended again: the list itself was woven in TypeScript.** `mefarshim.ts` held
the whole information architecture of the picker — four sections, three Hebrew
headings, an ordering rule and a no-sefer-twice rule, 277 lines — beside
`mefarshim.rs`, which carried twenty-five Rust tests about this same list and
could not see any of it. The giveaway was the shape the window had to be given:
`Mefarshim` arrived as **four parallel arrays** that only `listed()` knew how to
weave.

`girsa_app::mefarshim::listed` is the weave. The four arrays stay, because the
picker and the tick-list each want a different one; what moved is the decision.
The window draws what it is sent.

---

### W46 · A sefer that says it is a commentary is filed with the commentaries

> *"shulchan aruch should have folders for mefrshim and then for actual shulchan
> aruch. it is structured oddly now, and pri megadim is lumped with it."*

Upstream, in one line:

    peri-megadim-on-orach-chayim  ['Halakhah','Shulchan Arukh']
    peri-megadim-on-yoreh-deah    ['Halakhah','Shulchan Arukh','Commentary','Pri Megadim']

Same author, same sefer, two chalakim, two different filings. So the Pri Megadim
on Orach Chayim stands on the Shulchan Arukh's own shelf, beside the four
chalakim, as though it were a fifth.

It is not a guess to move it: it **declares** `commentary_on:
shulchan-arukh/orach-chayim`. So — a work that declares itself a commentary and
whose categories put it on its base's own shelf is filed one level down, under
that shelf's commentary folder. A declaration outranks a category, which is the
same precedence the rest of this codebase already uses.

**Build.** In `girsa_corpus::taxonomy::shelf_of`, because the shelf has to be one
mapping — the search facets group by it too (the note at the top of
`girsa_app::taxonomy` is about exactly this).

**Test first.**
1. `peri-megadim-on-orach-chayim` lands under the commentary folder, not beside
   the chalakim.
2. `shulchan-arukh/orach-chayim` does **not** move: it declares nothing and it is
   the base text.
3. `shulchan-arukh/introduction` does not move either — it declares nothing, and
   an introduction to a sefer is part of it.
4. Every sefer still has exactly one shelf, and `every_sefer_has_a_shelf` still
   counts to the whole corpus.

**Acceptance.** Open Shulchan Arukh on the shelf: four chalakim and an
introduction, and a מפרשים folder holding the sixty-eight commentaries — the Pri
Megadim among them.

---

### W47 · Open something without closing what you are reading

> *"there should be a way to open while keeping madaf open."*

---

### W48 · A search result opens without closing the search

> *"same for search - be able to go there while keeping search open."*

A reader working through search results reads one, goes back, reads the next. If
the jump closes the panel, the second result costs the whole search again.

---

### W35 · The answer: not yet, and here is the condition

W35 asks whether `segments.jsonl` should become `text.txt` plus a sidecar, so that
spec.md §4.1's four claims — *greppable, diffable, backup-able, outlives the app* —
become true rather than aspirational. It is a fair question about a real gap between
a stated principle and a file format, and it was filed carefully: it says plainly
that its own line-addressed format is unacceptable here, and asks only for the half
that is not in tension with permanent ids.

**The answer is no, not now — because the split does not buy the thing it is for.**
Measured, on Shulchan Arukh, Orach Chayim:

```
segments:                              4,171
containing inline markup ('<'):        4,170   (100.0%)
containing a newline:                      0
grep -c "יתגבר כארי" segments.jsonl:       0
```

The last line is the whole argument. That phrase is in the sefer; `grep` cannot find
it — and **moving the text into a `.txt` would not change that by one hit**, because
what defeats the grep is not the JSON envelope. It is `<b>`, `<i>` and `<span>`
sitting between the words, in **every single segment**. A `text.txt` built today
would be one segment per line and still unsearchable by a person with `grep`, which
is the claim the change exists to make true.

So of the four gains:

| claim | delivered by the split alone? |
|---|---|
| greppable | **no** — measured above. The markup is the obstacle, not the JSON |
| diffable | half. A diff would show the sentence *and* its markup, which is better than a re-serialised line and is not clean |
| `sed` / `awk` / `wc` | yes |
| outlives the app | marginal. JSONL is already plain text a person can read in fifty years |

Two files per work, an asymmetric half-failure mode, and an import-time invariant to
maintain, in exchange for one and a half of four. That is not a good trade **today**.

**The condition under which it becomes the right change: after W34.** W34 already
has to take the commentary anchors out of the text and hold them beside it. The same
pass is what would take the presentational markup out — into the run/span sidecar the
reading pane already consumes (`display::runs` reconstructs it from a description
rather than from tags). Once the text of a segment is *the words and nothing else*,
`text.txt` is genuinely greppable, the diff is genuinely the sentence that changed,
and every one of the four claims lands. The split stops being a reshuffle and
becomes the last step of a change that was happening anyway.

**Two things W35 got right that are worth recording**, because they are load-bearing
for whoever does this later:

- **No segment in Orach Chayim contains a newline** — 0 of 4,171. The worry that a
  text file cannot hold a segment with a newline in it is real in principle and empty
  in this corpus, so the "prose lives in the text file, exotic kinds stay in JSON"
  fallback is a contingency and not a design constraint.
- **The line-count invariant is the right guard** and should be written the day the
  format changes, not after: `text.txt`'s line count equals the sidecar's row count,
  asserted at import, is what makes a half-failed ingest loud instead of silently
  off-by-one for every segment after the break.

**What is filed instead:** W34 gains a clause. When it strips the anchors, it strips
the presentational markup into the same sidecar, and states the segment-text
invariant *the text of a segment contains no `<`*. That is testable, it is the thing
that unblocks W35, and it is worth doing for its own sake — a search index built over
text with tags in it is an index whose tokens include `b` and `span`.

---

### W34a · Strip the markup with the anchors, and say the text has none

**A clause on W34, added when W35 was answered.** W34 already has to take the
commentary anchors out of a segment's text at ingest. This says: take the
presentational markup out in the same pass, and assert what is left.

**Why it belongs to W34 and not to W35.** Two reasons, and the second is the one that
matters:

1. It is the same walk over the same strings. `display::bits` already parses every
   tag and records where each character came from, and `display::runs` already
   rebuilds the styling from that description rather than from the tags — so the
   window does not need the tags in the text and has not for some time.
2. **A search index built over text with tags in it is an index whose tokens include
   `b` and `span`.** That is true today, in 100% of Orach Chayim's segments, and it
   is a defect in the searching rather than a matter of taste about file formats.

**The invariant, which is the deliverable:** the `text` of a segment contains no `<`.
Assert it at import over every work, and the assertion is what makes W35 possible
later — a `text.txt` of segments that are only words is greppable, and one full of
`<b>` is not, which is the measurement in W35's answer.

**Test first.**
1. A segment ingested from a Sefaria record with `<b>…</b>` in it has no `<` in its
   text, and the run description says which characters were bold.
2. The words are unchanged: strip the markup from the old text and it equals the new
   text, character for character, over a whole work.
3. `grep` for a phrase that spans a tag boundary in the old text finds it in the new
   one. That is the acceptance criterion and it is the one W35 actually wants.
4. Nothing loses a dibur hamatchil: `run_opening` still marks the same characters,
   asserted against the same segments before and after.

**Not required for W35 to be reconsidered — required for it to be worth doing.**

---

# Still open from the 31 July report — carried here 6 August 2026

`BUILT-2026-07-31.md` was a report on a round of work, and its §6 and §14 were the
only part of it that described the future rather than the past. The report is gone
— it was a changelog entry wearing a filename, and the tree it measured is
preserved at the tag `built-2026-07-31`, which is a better record than a tracked
file that has to be read past. What was live in it is here, where live work orders
live, re-verified against the tree on 6 August.

These are stated at the size §6 stated them. None is blocked on anything but time,
except B33, which is blocked on hardware.

**B19 · The transmission chain has no panel.** `girsa-chain back/forward/path/fork`
all work; `grep -i chain app/src/*.ts` still returns nothing, so no reader can
reach any of it. Four Tauri commands and one panel — presentation, not model work.
The `--width` widening for a fork two hops apart is a second, smaller piece.

The 6 August lamdan report adds the part that changes the order of this: `chain.rs`
is 1,052 lines whose only callers are a developer CLI and `girsa-mcp`, and it is
why `inbound`'s cache holds whole edge rows rather than a summary and why
`Graph::work` is an eager whole-work load. **A feature no reader can reach set the
memory shape of the feature every reader uses.** Building the panel is what makes
that trade honest; not building it is what makes it a cost with no payer.

**B20 · Nothing rebuilds the index incrementally — half done.** `Writer` can now
`delete_term` a work and re-add it, so *a work* is replaceable. What is still a
four-minute full rebuild is a note or a correction: the corrections half needs the
overlay taught to the indexer, and the README rules out both easy answers. B7's
honest notice is what makes shipping this incrementally safe and is not a
substitute for it.

**B22 · The rest of the personal layer.** Four pieces: linkify over notes,
`girsa-notes merge` (there is no `merge` in that binary), a `comp_date` on a note
so it can be a hop in the chain, and your own sefer in the resolver's lexicon at
import. **The merge first** — it is the substitute for the sync that was ruled
against, and `girsa-personal::Log` now makes it nearly free: two append-only files
replayed in one pass, which is what `Layer::merge` already became.

**B23 · `.ksav` on the shelf.** The containment is flat and nothing writes back.
The second half needs a decision about what editing a `.ksav` from the shelf means
when Ksav may have it open, and that decision is the work. Related, and newly
sharper: `who_cites` answers over `personal/ksav/*.ksav` only, so a document
written in the real Ksav is invisible to it.

**B32 · Nothing drives a pointer.** The one job that closes five gaps at once —
§4 items 21–24 of the grade, plus the accessibility claim that has only ever been
asserted statically. It needs a CI job that *runs* both applications rather than
building them, with `Ksav/.github/scripts/wasm-smoke.mjs` as the precedent. The
`compile_error!` for a bare `cargo build --release` of the shell — the trap that
cost an audit an hour — belongs in the same change and is four lines.

**B33 · macOS.** No Mac, no runner. Both applications compile for it and neither
has been run on it. The only item here that needs hardware.

**B34 · Property and model-based tests.** Three sweeps landed inside other orders.
The three asked for are still missing, and there is no `proptest` in any manifest:
generated refs through `girsa-cite`'s round trip; fuzzing the Ksav markup for
panics (*nothing should panic, everything should be a named refusal with a
location*); and fuzzing `Ref` parsing against `is_well_formed`.

**B31 · The two god modules.** `main.ts` came down by five modules. Girsa's shell
`lib.rs` has gone the other way — 4,087 lines and 91 commands then, **4,590 lines
and 100 commands now**, because the settings commands went into it. Stated rather
than buried, twice now. The lamdan report says where the seam actually is and it
is not where B31 guessed: the shell's problem is not its length, it is that
**4% of it is pass-through** and the rest decides things — cache policy,
truncation lengths, sort orders, patch provenance — which is what the DTO move is
for.

**W49 · The chrome is Hebrew only.** 312 Hebrew string literals across 19 TS files.
W41 built the language setting and made every sefer *name* follow it, which is the
half that was asked for by name; the buttons, headings and messages did not
follow. A string table and a translation pass, not a plumbing change.

**And the line that matters more than any of the above: nobody has written a sefer
in Ksav.** Everything in every report in this repository is a promise tested by its
author.
---

# What holds, per work order

Moved out of `README.md` on 7 August 2026. Twenty rows of `W`-numbers is an
index into *this* document, and it was sitting in the one a reader opens first —
seventy lines of shorthand that mean nothing without the orders above them. The
README keeps the paragraph that says the spec is built; this is the table behind
it.

It is a record of what each order was **accepted on**, not a list of what exists.
Where a claim here has since been overturned, the order above says so — see W31.
| | What holds |
|---|---|
| **W1** · scaffolding | Three repos, pinned, dual-licensed. A breaking change to a shared crate fails in `sefer-crates` CI before it reaches either app — proven by breaking one. |
| **W2** · `girsa-hebrew` | The normalizer, and the line between what it will and will not do. 372-row regression corpus **harvested from 400 real seforim**, not written by hand. |
| **W3** · `girsa-ref` | The resolver. **100.00% exact on 2,970 real citations**, 0 wrong. Lexicon of 6,594 works and 24,731 spellings, built from Sefaria's schemas. |
| **W4** · `girsa-source` | The Source Packet. Ksav compiles it, and an arriving quote is put through the **real Typst compiler** rather than merely deserialized. |
| **W5** · fetch | 12,826 files, 3.4 GB on disk. Resumable — killed at 47%, resumed with nothing refetched. |
| **W6** · segment IDs | `girsa:mishnah-berurah/1:1#7`. One typo fix, 501 links: **line numbers moved 501, permanent ids moved 0.** And across a **re-import**: one se'if added upstream to a 4,182-se'if sefer renames nothing and mints one name. Run over the whole shelf — **7,189 works, all 5,000,545 ids kept, none minted, none moved.** |
| **W7** · import | Sefaria spine, Otzaria fill. **7,189 works · 5,000,545 segments**, each named once and never again. Mishnah Berurah 18,120/701 and Shulchan Arukh O.C. 697/4,171 — `spec.md` §2's numbers, exactly. |
| **W8** · links | The graph, on segment ids rather than line numbers. **4,182,344 edges** from 5,108,893 rows — 81.9%, and **92.6%** of the rows whose sefer is on the shelf at all. Every dropped row counted under why, and **nothing left ambiguous**. Mishnah Berakhot 1:1 → the Rambam on it, end to end. |
| **W9** · the workspace | Tabs, splits, RTL, nikud toggle, per-sefer position memory — and **a commentary column that follows the text**. Berakhot open with Rashi beside it: move the Gemara to 2a:6 and the Rashi column moves to 2a:6:1. **1,718 of Berakhot's 2,749 lines have a Rashi**; on the other 1,031 the column says *אין כאן* and stays where it is. |
| **W10** · the shelf | One taxonomy over two corpora's vocabularies: **15 shelves, 7,189 seforim, each on exactly one**. Editable — move, rename, reorder, make a shelf — as **one file in your own layer**, which a re-import cannot touch. A file you drop in is a sefer with permanent ids like any other. |
| **W11** · the index | **5,000,545 segments in 4m 8s**, one normalized index, built by the *same* code the query bar normalizes with. A bare `משעה שהכהנים נכנסים` finds the fully menukad first line of Shas, and the highlight lands on `שֶׁהַכֹּהֲנִים` — the word as printed. Nothing widened at import: `שבת` does not find `ובשבת`, and that is the point. |
| **W12** · Torat Emet | The literal mode, and the default. The three operators that get used — the word, the letters it **contains**, those letters **in order** with others between — plus **within X words of each other, in either order**. Every query carries a plan saying exactly what was asked of the index, and a test asserts that plan is the typed words with their nikud off and nothing else. On the shelf: `קדש` is 31,483 segments, `--contains קדש` is 301,910, and the difference is a thing the reader asked for. |
| **W13** · the ladder | Two columns of one table (§9.6), and the difference between them is the work order: the default mode **offers** the rungs with their counts and applies nothing; Smart climbs them and says so. The counts are computed from the query the click would run, so the promise and the result cannot disagree — checked both ways. Two rungs are named and not offered, because a missing chip reads as *there is nothing down that road*: nikud is already off in every mode, and the root rung is what §9.4 rejected every analyser for. |
| **W19** · closing the loop | The ref is **in the document** — it was not, for three orders, and §10.2's promise was quietly false until now. Out of that one change: `#מראה_מקומות()` prints the sources at the back, *where did I use this* is a scan of your own layer, and a `.ksav` file is a sefer on the shelf whose words are read by the crate that wrote them. Linkify wraps only what is certain, and says so when nothing is. |
| **W18** · cite on selection | *Where is this from* and *who quotes this* are **one call** — 61 places for the Mishnah's own line, 59 when Berakhot is left out. Ctrl+Shift+M in Ksav; Tab cycles; no fit opens Girsa's search with the phrase in it. `אמר רבי יוחנן` is 12,347 places and is reported as a turn of speech rather than offered as a source. |
| **W17** · the buffer | Ctrl+E, a drawer at the foot of the window, and **real Ksav markup from the first keystroke** — `girsa-ksav`, the writer Ksav compiles, not a second one in TypeScript. A buffer is a `.ksav` file in your own layer, and Ksav's suite compiles one this window wrote and reads the mekor off the page, below its quote. |
| **W16** · the pairing | A desk on loopback in each application, token-gated, presence asked rather than assumed — `Live`, `NotRunning` and `Stale` are three different things and the window says which. Ctrl+Shift+C sends into the open document with no clipboard at all; `/cite` and `/quote` let Ksav re-print a citation or re-read a quote from the corpus as it stands. Tested through a real socket, including the 401. And `girsa:…` **is** the deep link — the ref the document already stores. |
| **W15** · the clipboard | One Ctrl+C, three flavours — and the third is written natively, because a webview's custom format is a private encoding no other application can read. Only the highlighted part travels; the ref is a span when the quote is. The citation is `girsa-cite`, compiled into both apps, and **the test is that Girsa reads back what Girsa printed** — which found two defects in `girsa-ref` and fixed them there. Checked in Ksav against a packet Girsa really sent, asserted **on the laid-out page**. |
| **W30** · the semantic lane | *I remember a Rishon who says something like this but not the words* — **measured, not hoped.** A half-remembered line finds its se'if **in the top 16 of 240 every time, and first 8 times in 10**; a *question* about a se'if finds it once in five, so the box asks for a line rather than a question. Mean-centring, the standard repair, made it worse and was not built. The model is side-loaded — a picker by default, a fetch button behind a setting that ships off — so the day a better encoder exists it is a setting and not a release. Off is off: the corpus tree is byte-identical before and after, and every answer carries *adjacent — found by meaning rather than by these words* and a sentence saying what is **not** in the index. 4.5 segments/second, resumable, and it never blocks reading. |
| **W14** · the rest of §9 | The other three modes, the chip row, and the five facets. **A facet row's count is the number clicking it gives you** — the ladder's promise, one section on, asserted for every row of every dimension. On the shelf: `יתגבר כארי` is 79 segments; the rail says `חסידות 26`, and clicking it gives 26. The two instruments the index cannot answer say so by name instead of approximating: a dilug reads letters and a notarikon is four patterns each matching half the vocabulary. |
