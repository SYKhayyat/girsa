# גִּרְסָא · Girsa

**A Torah library that assumes you are going to write something.**

Girsa (גִּרְסָא, "the text as received / the version you learn") is the page.
Ksav (כְּתָב, "writing") is the pen. The pairing is the idea; the idea should
survive a rename.

> **Building this?** Read **[`BUILDER.md`](BUILDER.md)** — work orders, binding
> rules, the verified traps in the data, and what you may not decide alone.
> This document is *what Girsa is*; that one is *what to do on day one*.
> Read §2, §3 and §16 here first — they are what shape it.

---

## 1. Thesis

Every Jewish-text application that exists is a **reader**. Sefaria, Otzaria,
Zayit, Torat Emet, Bar-Ilan — they differ in corpus, in search quality, in
license, in polish, but they all end at the same place: the passage is on your
screen. What happens next is your problem.

What happens next is almost always **writing**. A shiur, a chaburah, a chiddush,
a teshuvah, a sefer, a sheet for a chavrusa. The reading was never the goal; the
reading was procurement. And the seam between "I found the source" and "the
source is in my document, quoted accurately, cited correctly, and typeset like a
sefer" is the most painful part of the workflow — and the part *no one owns*,
because library apps and typesetting apps are built by different people.

They are not different people here. **Ksav already exists** — a real Typst
compiler with a Hebrew-first editor, MIT/Apache, already building installers for
Windows, macOS and Linux. So Girsa is not "another library app with an export
button." Girsa is the **intake half of a two-app system whose output is a printed
sefer.**

Two smaller theses, both load-bearing:

**The best Hebrew search is behind a paywall and the most usable one is 20-year-old
freeware.** Bar-Ilan has the power; Torat Emet has the feel. Nobody has both.

**The corpus you can download is not clean.** Every existing app treats the shipped
text as read-only fact. It has typos, bad commentary anchors, and 74% of its links
are untyped. Girsa lets you fix it, and keeps your fixes.

---

## 2. Ground truth — what the data actually is

Everything below was verified by reading the real files, not from documentation.

### 2.1 Otzaria (`Downloads/otzaria_latest/`)

```
<root>/
  אוצריא/          category folders → .txt books (6,618 files, ~4.0 GB)
  links/           <BookTitle>_links.json (5,819 files, ~2.3 GB)
  metadata.json    7,041 entries
```

**Text format.** UTF-8 `.txt`, **one segment per line**. Structure is inline HTML:
`<h1>` book, `<h2>` chapter or daf, `<h3>` siman. Inline `<big><strong>` marks the
dibbur hamaschil. Mishnah Berurah is 18,120 lines with 701 headings. Nikud
coverage is inconsistent — Berakhot ships fully menukad, Mishnah Berurah has none.

**Links format.** Per-sefer JSON arrays:

```jsonc
{ "line_index_1": 913.0,
  "heRef_2": "סליחות נוסח אשכנז ליטא, ליום ראשון,  ג, יא,",
  "path_2": "אוצריא\\סדר התפילה\\...\\סליחות נוסח אשכנז ליטא.txt",
  "line_index_2": 22.0,
  "Conection Type": "reference" }
```

**Known defects, all confirmed:**

- Line indices stored as **floats** (`913.0`), sometimes as strings.
- The key is misspelled **`"Conection Type"`**.
- **`path_2` is stale.** Folders were renamed after generation. Targets must be
  resolved by *filename*, not path. (Verified independently in `OtzariaSonim/SPEC.md`.)
- **74% of links are untyped.** Sampling Abudraham: 420 links → 311 blank,
  61 `commentary`, 34 `quotation`, 10 `related`, 4 `reference`.
- Grime: empty `<h2></h2>` headings, titles with leading spaces, a leftover
  `"Unnamed: 9"` spreadsheet column in the metadata.

**The meforshim lookup does work.** Verified end-to-end in the Sonim work: Mishnah
Berakhot line 3 → Rambam line 5, correct.

### 2.2 Sefaria (`gs://sefaria-export`, ~26 GB, ~85K files, no auth)

```
json/{categories}/{title}/{language}/{versionTitle}.json
txt/{categories}/{title}/{language}/{versionTitle}.txt
schemas/{title}.json
links/links0.csv … links16.csv, links_by_book.csv
table_of_contents.json
```

**Size.** `books.json` has 19,705 rows = 6,456 distinct titles. Hebrew is 13,546
rows / **6,211 distinct titles**; English 6,159 rows. Hebrew titles by category:

| Halakhah | Talmud | Mishnah | Tanakh | Tosefta | Midrash | Thought | Responsa | Chasidut | Kabbalah | Liturgy | Musar |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2,066 | 1,719 | 956 | 602 | 226 | 127 | 111 | 97 | 96 | 70 | 63 | 41 |

Comparable to Otzaria's ~6,618 files. **Sefaria is not mainly "more books."**

**The schemas are the prize.** `schemas/Shulchan_Arukh,_Orach_Chayim.json`, 86 KB:

```jsonc
{ "nodeType": "JaggedArrayNode",
  "depth": 2,
  "addressTypes":   ["Siman", "Seif"],
  "sectionNames":   ["Siman", "Seif"],
  "heSectionNames": ["סימן", "סעיף"],
  "lengths": [697, 4171],
  "match_templates": [ { "term_slugs": ["shulchan-arukh","orach-chayim"] }, … ],
  "titles": [ … 44 entries … ] }
```

Otzaria has a line that *says* "סימן א". Sefaria knows what a siman **is** — that
this sefer has 697 of them containing 4,171 se'ifim, and that a commentary
attaches at the se'if.

**And the title tables are the resolver, pre-written.** Those 44 `titles` entries
for this one sefer include `שו"ע או"ח`, `שו״ע או״ח`, `שו”ע או”ח`, `שלחן ערוך או״ח`,
`או"ח`, `S.A. O.C.`, `SA OC`, `OC`, `O.Ch.` — every way the sefer is written, both
languages, machine-readable, across 6,000+ works. §4.3 is largely downstream of
this file.

**The links are the same graph, better addressed.** `links0.csv` header:

```
Citation 1,Citation 2,Conection Type,Text 1,Text 2,Category 1,Category 2
"A Dictionary of the Talmud, אֱגוֹד 1",Mishnah Peah 6:6,quotation,…
```

Same misspelled column — **Otzaria's link graph is Sefaria's, converted down to
line numbers.** But Sefaria addresses by **canonical citation** (`Sanhedrin 74b:9`),
not file-and-line. 19 files, 671 MB. Blank connection types originate here, so
re-importing does not fix them.

### 2.3 What this decides

**Measured overlap.** Comparing Sefaria's 6,598 Hebrew titles (from
`table_of_contents.json`) against Otzaria's 6,615 distinct filenames, normalized
for nikud, gershayim and punctuation:

| Shared | Otzaria-only | Sefaria-only | Union |
|---|---|---|---|
| 5,637 (85% of Otzaria) | **978** | 961 | ~7,576 |

**The 978 are not marginal — they are disproportionately the learning material.**
Sampling them: גליוני הש"ס · אבני נזר · קרן אורה על נדרים · חידושי הראב"ד ·
מהר"ם שיק · שו"ת ישועות מלכו · אמת ליעקב · תורת חיים על חולין · קובץ שיטות קמאי ·
מגיני שלמה · ראשון לציון · מגיד משנה-era acharonim throughout. This is exactly the
gap in Sefaria that makes it insufficient alone — the acharonim you actually need
at 11pm.

### 2.3a The org tree is not a third source

`otzaria-library` — locally an org-mode conversion at `Downloads/seforim` — is
22,556 `.org` files / 8.3 GB, against the txt tree's 6,618 / 3.4 GB. The 3.4×
file count does **not** mean 3.4× the corpus. Measured against Sefaria ∪
Otzaria-txt, its 8,376 apparently-new titles break down as:

| Shelf | New titles |
|---|---|
| **Other** (the dumping ground) | **8,106** |
| מחשבת ישראל · שו"ת · תנ"ך · תלמוד בבלי · הלכה · מדרש · קבלה | **41 combined** |

The inflation is Ben-Yehuda piyut-per-file poetry (`בְּעֵת יִקְצֹף`, `[אִישׁ לוֹ]`),
volume splits of works already counted (`שולחן ערוך אורח חיים - חלק ו'`,
`תלמוד בבלי - ביצה - תוספות`), and conversion artifacts (`999504`,
`00007_קש''ק כריתותconverted`, `_1`/`_2` suffixes). 15,398 of 22,556 files sit
under `Other`.

**Not ingested.** Separating real works from noise here is a curation project, not
an import. The txt tree remains the substrate — the same conclusion
`OtzariaSonim/SPEC.md` reached from the linking side, now confirmed from the
corpus side. Revisit as an optional later fill pass if the 41 turn out to matter.

### 2.3b The split

Sefaria supplies the **skeleton**: structure, canonical refs, links, title
variants, versions and provenance. Otzaria supplies **breadth Sefaria lacks** —
its conversions from Torat Emet, Orayta, Ben-Yehuda, Dicta and WikiSource.

**Decision: Sefaria spine, Otzaria fill.** For any sefer both have, take Sefaria's
*text as well as* its structure. Grafting one project's schema onto another's text
file is a line-by-line alignment problem across thousands of books and it would
eat the schedule. Otzaria-only works enter as-is, structured from their `<h1>/<h2>/<h3>`
headings, and earn real schemas later if they deserve them.

**Measured download scope.** Hebrew `json/` + `schemas/` + `links/` only — skip
English and both `cltk-*` formats:

| Part | Size |
|---|---|
| Hebrew `merged.json` × 6,211 titles (mean 254 KB, sampled n=40) | ~1.5 GB |
| `schemas/` × 6,456 | ~32 MB |
| `links/` × 19 CSVs | 671 MB |
| **Sefaria total** | **~2.2 GB** (of 26 GB) |
| Otzaria text tree, already local (6,618 files) | 3.4 GB |

Ingest pulls ~2.2 GB over the network. That is one evening, not a project.

---

## 3. The landmine, and the fix

**Line-number addressing and typo-fixing are directly incompatible.**

Otzaria addresses every link as *file + line number*. Fix a typo in a way that
splits or joins a line, and every link below it in that file now points at the
wrong text. Not a broken link that errors — **the wrong text, silently.** That is
the worst failure mode this system can have, and the two headline features collide
on it.

**Every segment gets a permanent ID at import.** Assigned once, never re-derived
from file position. Corrections, notes, links, highlights and every citation
sitting in a Ksav document anchor to that ID. Editing text cannot move an anchor.
Splitting a segment mints a child ID rather than shifting seventeen thousand
others. When upstream re-segments a text, a redirect table absorbs it.

This costs one import pass to get right and is close to impossible to retrofit. It
is the single most important decision in this document.

---

## 4. Foundations

### 4.1 Storage format

**Text files on disk are the truth. The database is a rebuildable cache.**

- Ingest is cheap — both corpora already are text.
- The corpus stays greppable, diffable, backup-able, and outlives the app.
- SQLite (structure, graph, corrections, personal layer) and the tantivy index are
  **derived**. Corrupt them and you rebuild; you never lose text.
- **"Export a fixed sefer" falls out for free**: base text + your patches applied →
  a new file. Two lines here, awkward in an all-in-database design like Zayit's.

### 4.2 The address

A ref is a stable, resolvable pointer to a **span**, because a quote is a range:

```
girsa:shulchan-arukh/orach-chayim/1/1
girsa:bavli/berakhot/2a:1-2b:4
girsa:user/reb-shmuel-handout-2024#p12
```

Refs travel between apps and get stored inside Ksav documents. They must survive
corpus updates — that is the promise that makes the two-app system trustworthy,
and it is a permanent maintenance burden. Accept it now or don't build this.

### 4.3 The reference resolver

One component, used everywhere, that turns any way a human writes a citation into
a canonical ref — offline and local:

- Hebrew abbreviations with or without gershayim — `שו"ע או"ח א׳ א׳`, `שוע אוח`
- Rabbinic shorthand — `רמב"ם הל' תפילה פ"ד ה"א`, `פמ"ג יו"ד`
- Daf/amud in any notation — `ברכות ב.`, `ברכות ב ע"א`, `Berakhot 2a`
- English/Sefaria-style refs pasted from anywhere
- Partial refs resolved against current context ("see siman 5" while in O.C.)
- **Ambiguity is surfaced as a choice, never guessed silently**

Seeded from Sefaria's `titles` and `match_templates` (§2.2), which is most of the
lexicon for free. Needs a regression corpus of real citations from day one.

The resolver is the highest-leverage component in the system: link quality,
search-by-citation, linkify and Ksav round-tripping are all downstream of it, and
it quietly determines whether the app feels smart or stupid.

---

## 5. Selecting seforim — the shelf

Browsable the way seforim are actually organized — Tanach / Shas / Halacha /
Machshava / Chassidus / Responsa / yours — **with the arrangement editable.** The
shipped taxonomy is a default, not a fact.

- **The full union ships.** All ~7,576 works (Sefaria ∪ Otzaria-txt), ~5 GB after
  merge. Nothing is ever missing, and "is it in there?" stops being a question you
  have to think about. Packs remain in the model for *ordering* the download and
  for user-added material, but they are not a v1 gate.
- First run is therefore a real download. It must be resumable, backgroundable, and
  must let you start reading the shelves that have already landed.
- **Your own material, whenever.** PDF, DOCX and TXT dropped in at any time — not
  an onboarding step, not a second-class attachment. Searchable, ref'd, citable and
  linkable like anything shipped, subject to §6.3 for scans.
- Per-work metadata — author, era, place, composition date — comes from Sefaria's
  schemas and Otzaria's `metadata.json`, and drives the era filters in §9.

---

## 6. Reading

### 6.1 The workspace

Tabbed, splittable, RTL-native. A sugya open with its commentaries in adjacent
columns, each independently scrollable but **ref-synchronized** — move in the
Gemara and the Rashi column follows. Nikud on/off. Position memory per sefer.
Serious typography; the webview is the best Hebrew text renderer that exists, and
this is exactly the app category where that shows.

### 6.2 No tzuras hadaf for text

Reconstructing the traditional page from text is a typesetting project, not a
layout problem. Modern columns with ref-sync for all text seforim. Deliberate, not
deferred.

### 6.3 Tzuras hadaf comes from PDFs

The scan **is** the daf. No typesetting engine required. This makes the PDF layer a
real second reading mode rather than a side feature.

- **You bring your own scans.** Nothing shipped, nothing fetched.
- **OCR is optional and off during onboarding.** It runs in the background,
  resumable, never blocking reading.
- **OCR text anchors to coordinates on the page image**, so a search hit highlights
  the words on the scan.
- **The image stays ground truth**, which makes fixing OCR errors safe by
  construction — the original is always right there to check against.
- A **page → daf mapping** makes a scanned sefer citable. Small once-per-sefer
  chore; large payoff.

Consequence to state honestly: tzuras hadaf is a *capability*, not something that
works on install.

---

## 7. Fixing typos

### 7.1 Never mutate the base text

Corrections are an **overlay patch layer** — segment ID plus character span, with
provenance and timestamp. The shipped corpus stays pristine. This buys reverting,
"show as printed / show corrected", surviving corpus updates, and handing a patch
file to someone else. In-place editing throws all of that away permanently.

### 7.2 A typo and a girsa variant are the same machinery

An OCR error (ד/ר, ב/כ, ה/ח, mangled gershayim, stray nikud) and a real textual
variant — a hagahah, a Gra emendation — are both *"this span should read
differently, and here is who says so."* One mechanism, one `kind` field
distinguishing a scanning error from a scholarly claim.

That field costs nothing now and is the whole difference between a cleanup tool
and an actual instrument. It also unifies with the `emends` edge type in §8.2.

### 7.3 Detection beats the editor

A word appearing exactly once in the corpus, one edit-distance from a word
appearing ten thousand times, is almost certainly an OCR error. That is a cheap
batch job over the whole library, and it produces a **ranked, reviewable queue**.
Fixing typos you trip over is nice; being handed 4,000 ranked candidates is a
different product.

### 7.4 Export a fixed sefer

Base text + applied patches → a clean `.txt`/`.docx`. Falls out of §4.1 for free.

### 7.5 The guardrail

**If correcting a typo is not a three-second interaction from where you are
reading, nobody does it — including you.** Editorial features are a tar pit; a
text-critical apparatus can absorb a year. This is a requirement, not a hope.

---

## 8. Linking mefarshim

### 8.1 What ships

Import Sefaria's links (§2.2) — **citation-addressed, not line-addressed** — and
resolve them onto permanent segment IDs. This is strictly better than repairing
Otzaria's degraded copies. For Otzaria-only works, import its JSON links,
resolving targets **by filename, not path**, per §2.1.

Getting "what does the Mishnah Berurah say on this se'if" to be one keystroke *and
correct* beats a rich but wrong graph. That is the v1 bar.

### 8.2 The edge

Directed, typed, span-anchored, evidenced:

```jsonc
{ "from": "girsa:beit-yosef/orach-chayim/1#s4",
  "to":   "girsa:rosh/berakhot/1/1",
  "type": "quotes",
  "from_span": [412, 468],
  "to_span":   [88, 141],
  "confidence": 0.94,
  "method": "sefaria-seed",
  "attributed": "הרא\"ש",
  "strength": "sustained" }
```

Types, directed, inverse derived and never stored twice: `comments-on` ·
`quotes` · `paraphrases` · `codifies` · `disputes` · `emends` · `parallel-to` ·
`translates` · `references` (weak catch-all — the fallback, not the default).

**Store the type field from day one; populate only what we have.** Schema changes
are expensive, filling in values is not. Sefaria's four labels map onto ours; the
74% blank stay `references` until something better assigns them.

`attributed` records whom the citing text *claims* to cite, separately from where
the resolver thinks the source actually is. Those diverge constantly —
misattributions, lost intermediaries, "in the name of." Storing both makes the
divergence searchable, which no existing app offers.

### 8.3 Repair, because the data is wrong

A visible, one-gesture repair UI:

- **Reanchor** — drag a commentary to the segment it actually belongs on.
- **Retype** — set a blank `Conection Type` to what it really is.
- **Reject / confirm** — kill a bad edge, bless a good one.
- **Draw** — hand-create an edge, including from your own notes.

Stored as **overrides in the personal layer**, never edits to shipped data. A user
who reads carefully makes their own copy of the graph better. Everything shows its
work: matched text on both sides, confidence, method. A mined or blank-typed link
is never presented as curated fact.

### 8.4 Span anchoring

Links attach to **specific words**, not whole segments. Selecting a phrase
highlights only the links touching those words, so density distributes spatially
instead of dumping into a 200-item sidebar. A subtle gutter shows which lines the
world has argued about most.

### 8.5 Lenses

The sidebar is never one flat list. It is ranked by an active lens — **Halacha**
(poskim first), **Lomdus** (rishonim/acharonim, machlokes-weighted), **Peshat**,
**Girsa** (variants, hagahos), **Mine** (your notes and seforim first). Lenses are
saved filters over type/era/strength, not hardcoded lists.

### 8.6 Later: the transmission chain

With directed typed edges this falls out: trace forward from a Gemara to how it
became halacha; trace backward from a ruling to where the posek got it; find the
path between two texts, or report honestly that none exists; and **break
analysis** — where two rishonim read one Gemara into incompatible halachos. That
fork is usually the chiddush.

Post-v1, and it depends on §8.2 being right now.

---

## 9. Searching

**Governing rule: nobody should ever have to learn a syntax — and the engine must
never silently widen a query without saying so.**

Sefaria's own analyzer plugins are a *naive lemmatizer* (GPL-3.0, Java, ES-only,
last touched 2023). Naive lemmatization over-stems and doesn't report it, which is
exactly why Sefaria search feels non-deterministic next to Torat Emet's. **Named
failure mode. Do not reproduce it.**

### 9.1 Nikud and te'amim are always stripped

Every mode. No toggle, no setting. Nobody searches with them on. One normalized
index — simpler and faster.

Consequence accepted knowingly: you can never search *for* a nikud difference.
Comparing girsaos is a separate tool (§8.5 Girsa lens), not a search option.

### 9.2 Normalization — the unglamorous foundation

Every item below is a case where naive search silently fails to find a passage
that is right there on the page:

| You type | The page says | What must happen |
|---|---|---|
| `שבת` | `וּבַשַּׁבָּת` | strip nikud, peel stacked prefixes ו ה ב כ ל מ ש ד |
| `כהן` | `כוהן` | ktiv male ↔ chaser equivalence |
| `שו"ע` | `שולחן ערוך` | abbreviation expansion, bidirectional |
| `רמב"ם` | `רבינו משה בן מיימון` | rabbinic acronym expansion |
| `ארץ` | `אָרֶץ׳` | geresh/gershayim `׳ ״ ' "` folded; final letters folded |

Rules, not models. Invisible when it works, which is the point. Read
`Sefaria-ElasticSearch` for *which* prefixes it strips; reimplement in Rust — the
GPL forbids taking the code into a codebase sharing crates with Ksav.

### 9.3 Five explicit modes — Torat Emet is the default

Mode is always a visible selector, and the other modes are one click away, not
buried. The default is the literal one.

1. **Torat Emet — the default.** Completely literal, and the operators are the ones
   that actually get used: the word *contains* these letters (`קדש` → `המקדש`,
   `ויקדשהו`); these letters in this order with others between; these words within
   X words of each other. Nothing is stemmed, expanded or guessed. **What you typed
   is what was searched for.**
2. **Smart** — type words, and prefixes, male/chaser and abbreviations are handled
   for you (§9.2). Opt-in, because it widens your query.
3. **Regex** — full power, no hand-holding.
4. **Citation** — type a mareh makom, jump (§4.3).
5. **Instruments** — gematria, notarikon, atbash, dilug.

**Why literal is the default.** Predictability is the feature. A search you can
predict is one you can *aim* — you know why you got a result and why you didn't,
so a bad result tells you how to fix the query. The moment the engine helps
without saying so, every empty result becomes ambiguous: did the text not say
this, or did the engine mangle what I asked? Torat Emet is beloved for exactly
this and it is not a limitation to be outgrown.

Nikud and te'amim are still always stripped (§9.1). That is not a widening — it
removes marks nobody types, and it never causes a match you would not want.

**Torat Emet and Smart must feel different in kind.** Blurring them is what makes
a search box untrustworthy.

### 9.4 Morphology — deliberately deferred

Finding every form of a word (`כתב` → `נכתב`, `כותב`, `כתבו`) is the one hard case.
Investigated and rejected for v1:

| Candidate | Verdict |
|---|---|
| HebMorph | AGPL, Java/.NET, modern Hebrew. Wrong on all three. |
| DictaBERT-morph | CC-BY-4.0, good, but explicitly modern Hebrew — doesn't know Gemara. |
| Sefaria-ElasticSearch | GPL-3.0, Java, ES-only, naive. See §9. |
| BEREL | Right register (~220M words rabbinic, unrestricted) — but *embeddings*, not morphology. For §9.7. |

**There is no off-the-shelf rabbinic-Hebrew-and-Aramaic analyzer.** So: rules-based
80% (§9.2) plus a hand-built root table for the few hundred words that matter in
learning. This takes the biggest technical unknown off the critical path.

When morphology does arrive it lands **inside Smart mode**, on by default there
and always reporting itself: *"43 results — 12 match other forms of כתב"* with
one-click **[exact form only]**. From the literal default it appears only as an
offer with a count (§9.6). Power expressed as what it did for you — never as
something that happened to your query while you weren't looking.

### 9.5 Controls are objects, not incantations

Chips in the search bar — visible, clickable, discoverable:

```
┌────────────────────────────────────────────────────────────┐
│  יתגבר כארי                                          🔍    │
│  [torat emet ▾] [whole shelf ▾] [words near each other ▾]  │
└────────────────────────────────────────────────────────────┘
```

Typing a sigil flips the matching chip, so the power syntax teaches itself and a
power user can always type instead of click.

### 9.6 Never a bare zero — offered, not applied

Zero results is a bug in the interface, not an answer. But since the default mode
is literal (§9.3), the engine **offers** the next step instead of taking it:

> *No results for `טרף`.*
> *[try other forms — 7] [expand abbreviations — 2] [widen to same passage — 19]*

Counts are computed and shown **before** you click, so the offer is informative on
its own — you learn there are seven other forms without leaving literal mode. One
click applies it, and the result header then says what was changed, reversibly.

| Mode | Behavior on zero results |
|---|---|
| Torat Emet (default) | Offer the ladder with counts. Never auto-apply. |
| Smart | Auto-relax in order, announce what changed, one-click undo. |
| Regex | Nothing. You wrote a pattern; it matched nothing. |
| Citation | Offer near-miss refs — this is the resolver's ambiguity path (§4.3). |

The ladder in order: drop nikud → other forms → root → expand abbreviations →
widen proximity.

**The rule underneath both columns: the engine never changes your query without
you knowing.** Auto-applying is acceptable in Smart because widening is the mode's
declared purpose and it always reports itself. In the default mode it is not.

### 9.7 PDFs and text in one index

One index, two location types. A text hit is sefer + segment ID. A PDF hit is
sefer + page + box. Same result row; only the highlight differs — reflowed text
versus a rectangle on the scan.

Since OCR is off at onboarding, PDFs are absent from the index until run.
**Never a silent gap:** the results header says *"4 PDFs on this shelf aren't
searchable yet — [OCR now]"*. Scanned hits carry a badge, because OCR text is
dirtier and you should know which kind of result you are reading. Badge them,
don't demote them.

### 9.8 Refinement beats formulation

Results carry live facets — shelf section, era, author, sefer, link type — each
with counts, each one click to narrow or exclude. You get it right on the second
try instead of being punished for the first.

### 9.9 Later: the semantic lane

A local embedding index (BEREL) for "I remember a Rishon who says something like
this but not the words." **Always visually separated** from literal results, never
blended — you need to know whether the engine found the words or something
adjacent. Trivially disableable.

---

## 10. The Ksav loop

**Design target: moving a source into a document should feel like AirDrop between
two of your own devices.** No export dialog, no file, no format decision, no
cleanup. Both apps are ours; there is no reason to communicate through a scraped
lowest-common-denominator export.

### 10.1 The Source Packet

A JSON object defined in a crate **both apps compile in** — the contract, not a
convenience wrapper. Adding a field is a compile error on the side that ignores
it, rather than a silent production bug.

```jsonc
{ "ref":     "girsa:shulchan-arukh/orach-chayim/1/1",
  "display": "שו\"ע או\"ח סימן א' סעיף א'",
  "text":    "יתגבר כארי לעמוד בבוקר לעבודת בוראו...",
  "nikud":   false,
  "version": { "edition": "…", "license": "CC-BY", "provenance": "…" },
  "lang":    "he",
  "range":   { "from": "…", "to": "…" },
  "note":    "my margin note, if attached" }
```

### 10.2 Girsa → Ksav

- **Send a source.** Arrives as a proper quote block, citation formatted to the
  document's style, **the ref stored in the document — not just the printed
  string.**
- **Send a selection.** Highlight part of a passage; only that goes.
- **Layered clipboard.** One Ctrl+C puts three flavors down: `text/plain` (works in
  WhatsApp), `text/html` (RTL-correct, pastes into Word with its shape), and
  `application/x-girsa-source+json` (the full packet). Paste anywhere and get
  something sane; paste into Ksav and it silently takes the rich flavor. **The user
  does nothing different.**

Storing the ref rather than the string is what makes citations alive: switch a
whole sefer from abbreviated to full-form citations, or regenerate quotes against a
corrected edition (§7), without touching the prose. No paste-based workflow can
ever do that.

### 10.3 The Ksav buffer inside Girsa

You are learning, you have a thought, you write it without switching apps.

**Main path: a lightweight editor in Girsa**, with **"open the real Ksav editor
here"** offered in the same pane. `ksav serve` already runs as a local HTTP server
hosting the editor SPA, so the embedded option is cheap.

**Critical constraint: lightweight means the UI, not the format.** The buffer
writes real Ksav/Typst markup from the first keystroke while showing a simple
toolbar. If it invents its own note format, we get exactly the drift that
embedding was meant to prevent and the handoff becomes lossy.

### 10.4 Ksav → Girsa

- **Cite on selection.** Highlight a phrase, press cite, the first mekor appears,
  Tab cycles the rest, and if none fits you drop into full Girsa search.
  *This is the same engine as corpus-wide quote detection* — "where is this phrase
  from?" and "who quotes this Gemara?" are one feature asked from two directions.
  Build once, both fall out.
- **Send text into the library.** Your writing becomes a sefer on the shelf —
  searchable, citable, linkable. This closes the loop and is what makes the system
  compound over years instead of being a lookup tool.
- **Where did I use this?** Standing on any passage in Girsa, see which of your own
  documents cite it. Only possible because §10.4 put your writing in the library.
- **Auto mareh mekomos.** The document's refs compile to a source list at the back.
  Cheap — the refs are already there; it is a sort and a print.

### 10.5 Linkify — scoped

Paste prose full of citations and have them become live refs. Genuinely useful for
getting years of existing notes in without retyping. But citations are written too
many ways to guess safely:

**High-confidence patterns only. Anything ambiguous stays plain text.** Never
guess — a wrong link is worse than no link. Ambiguity is surfaced, not resolved.

### 10.6 Transport

- **Loopback.** Localhost, token-gated handshake, no network, no account, nothing
  leaves the machine — matching Ksav's existing posture.
- **Deep links.** `girsa://open?ref=…` and `ksav://insert?packet=…`. Click a citation
  anywhere, including inside a compiled PDF, and land on the exact line.
- **Presence.** Each app shows whether its sibling is live, so the affordance is
  never offered when it would fail.

### 10.7 Explicitly dropped

**Quote drift check** — verifying every quote in a document still matches the
corpus. Considered, rejected as not worth the surface.

---

## 11. The personal layer

- Notes anchored to segment IDs, surviving corpus updates
- Highlights, tags, bookmarks, saved queries
- **Your own seforim** — PDF/DOCX/TXT, first-class (§5)
- **Your corrections** (§7) and **your link judgments** (§8.3)
- Chaburah/shiur folders — just named collections
- **Your notes are nodes.** A note has the same typed edges as anything else, so
  *"what have I already written that touches this sugya?"* is the same query as
  *"who quotes this Rishon?"*

Everything local, everything exportable as plain files, no account. Optional and
off by default: encrypted sync of the personal layer only. Never the corpus, never
telemetry.

---

## 12. Architecture

**Rust core + tantivy + Tauri shell, TypeScript frontend.**

1. **Crate sharing with Ksav is the entire differentiator.** §10 works because the
   apps share code rather than agree on a protocol. One citation formatter compiled
   into both means the app that *produces* citations and the app that *prints* them
   cannot disagree — precisely the class of bug that would destroy trust.
2. **The webview is the best Hebrew renderer that exists.** Nikud, te'amim, bidi,
   mixed RTL/LTR, Hebrew-with-nikud inside an English sentence. Two decades of
   adversarial testing. Compose and Flutter are both weaker here.
3. **Tantivy is Rust**, and Otzaria already proved it on this corpus.
4. **Ksav de-risked the stack** — same build, packaging, signing problem, CI.

Rejected: Kotlin/Compose (Zayit's — good, but zero Ksav sharing), Flutter
(Otzaria's — same), C#/Avalonia (closest to your working day, weakest Hebrew
typography).

### Repos and crates

**Three repos.** Girsa and Ksav must each stand alone — neither should be
unbuildable or unreleasable because of the other.

```
girsa/          the library app
ksav/           the writing app (exists today)
sefer-crates/   the shared contract — depended on by both
```

| Crate | Repo | Purpose |
|---|---|---|
| `girsa-source` | shared | The Source Packet. The contract. |
| `girsa-ref` | shared | Refs, parsing, resolution, redirect table |
| `girsa-hebrew` | shared | Normalization, nikud, abbreviations, male/chaser |
| `girsa-cite` | shared | Citation formatting — one implementation, both apps |
| `girsa-corpus` | girsa | Storage, ingest, schemas, segment IDs |
| `girsa-search` | girsa | tantivy indices, modes, relaxation ladder |
| `girsa-link` | girsa | The typed graph, repair, later mining |

**The cost this buys, and how to pay it.** Standalone repos mean a breaking change
to a shared crate is no longer one atomic commit that compile-checks both apps —
which is exactly the drift §10 exists to prevent. Mitigations, and they are not
optional:

- **Semver on the shared crates, strictly.** Both apps pin an exact version; a
  bump is a deliberate act on each side.
- **CI in `sefer-crates` builds both dependents** against the proposed change. A
  break shows up in the shared repo's PR, not weeks later in an app.
- **The Source Packet gets a schema version field**, so a mismatched pair fails
  loudly at the handshake instead of silently mis-rendering a citation.

Local development uses a Cargo `[patch]` to point at a checkout, so day-to-day work
across all three feels like a workspace without them actually being one.

Storage: SQLite for structure/graph/personal layer, tantivy for indices, texts on
disk (§4.1). Offline by default; network only for optional corpus updates.

### Platforms

Windows, macOS and Linux — all three, desktop-first. A browser build is possible
later but is not the target. **Tauri uses Edge's engine on Windows and Safari's on
macOS, so Hebrew-with-nikud rendering must be tested on both** — they will not be
pixel-identical.

Android is out of scope. If it ever becomes first-class, that is the one argument
strong enough to reopen the stack decision; the answer then is a thin reader
against the same data files, not a port.

---

## 13. Licensing

Solo use and bochurim, so this mostly stops mattering. Two things worth doing
anyway because they are nearly free:

- **Carry each text's source and license in its metadata.** Costs nothing now, and
  it is the only thing preserving the option to distribute publicly later.
- **Keep the clean-room boundary.** Ksav is MIT/Apache. Zayit is AGPL-3.0 §7b;
  Sefaria-ElasticSearch is GPL-3.0. Read both as prior art, copy neither. Otzaria
  is UNLICENSE and fine.

---

## 14. Non-goals

- **Not a web app.** Offline desktop is the product.
- **Not a social network.** No public sheets, no feeds, no profiles.
- **Not a translation project.** Ship what exists.
- **Not a paskening machine.** The semantic lane assists retrieval. It does not
  answer she'eilos, and the UI must never let it look like it does.
- **Not a Sefaria/Otzaria competitor.** Both are upstream. Contribute fixes back.
- **No tzuras hadaf for text seforim** (§6.2).
- **No quote drift check** (§10.7).

---

## 15. Milestones

**The target is the whole spec — there is no reduced v1.** What follows is a
build *order*, not a scope cut. Nothing here is "phase two" in the sense of
maybe-never; every milestone ships. The ordering exists because some things are
load-bearing for others (M1 → everything, M2 → M5/M6) and because design risk
should be retired before code risk.

**M0 — Skeleton.** Tauri shell, RTL workspace, tabs, read the Otzaria corpus as-is.
Nothing clever. Proves the shell.

**M1 — Refs and resolver.** `girsa-ref` + `girsa-hebrew`, seeded from Sefaria's
`titles`/`match_templates`, with a real regression corpus. Everything downstream
depends on this.

**M2 — Ingest.** Sefaria spine + Otzaria fill (§2.3). **Permanent segment IDs**
(§3). Schemas, structure, links resolved onto IDs. The expensive one. Budget
honestly.

**M3 — Search that feels like Torat Emet.** Normalization, tantivy, the five modes,
chips, relaxation ladder with Pattern/Regex exempt. *No syntax anywhere in the UI.*

**M4 — The Ksav loop.** Source Packet crate, layered clipboard, send-selection,
deep links, loopback, the buffer, cite-on-selection. **This is the milestone that
makes the project itself** — do not let it slide behind polish.

**M5 — Corrections.** Overlay patches, the OCR-error detection queue, export a
fixed sefer.

**M6 — Link repair.** Reanchor/retype/reject/draw, span anchoring, lenses.

**M7 — PDFs.** Viewer, optional OCR, coordinate anchoring, page→daf, tzuras hadaf.

**M8 — Personal layer, notes-as-nodes, chain tracing, semantic lane, MCP.**

**M4 should move earlier and run against a stub corpus.** The interop is the
riskiest *design* — not the riskiest code — and design risk should be retired
early. A fake source packet from a hardcoded ref landing correctly in a Ksav
document is worth more than another month of ingest.

---

## 16. Decisions settled

| # | Decision |
|---|---|
| 1 | Sefaria spine + Otzaria fill; Sefaria's text where it has it (§2.3) |
| 2 | Permanent segment IDs at import; never address by line number (§3) |
| 3 | Text files are truth, database is a cache (§4.1) |
| 4 | Otzaria's structure need not be preserved — convert freely |
| 5 | Nikud/te'amim always stripped, every mode, no toggle (§9.1) |
| 6 | **Torat Emet literal search is the default mode**; Smart, Regex, Citation and Instruments are one click away (§9.3) |
| 6b | Zero results offers the relaxation ladder with counts; auto-applies only in Smart (§9.6) |
| 7 | Morphology deferred; rules + root table for v1 (§9.4) |
| 8 | Corrections are overlay patches, never edits; typo and girsa variant unified (§7) |
| 9 | Links imported + repair UI; type field stored from day one (§8) |
| 10 | PDFs BYO, OCR optional and not at onboarding; tzuras hadaf via scans (§6.3) |
| 11 | Ksav buffer lightweight by default, real Ksav offered; Ksav format from keystroke one (§10.3) |
| 12 | Linkify high-confidence only; quote drift check dropped (§10.5, §10.7) |
| 13 | Windows + macOS + Linux, desktop-first (§12) |
| 14 | Rust + tantivy + Tauri + TS, crates shared with Ksav (§12) |
| 15 | **Three repos** — `girsa`, `ksav`, and a shared-crates repo. Both apps stay standalone (§12) |
| 16 | Girsa is **MIT OR Apache-2.0**, matching Ksav — forced by crate sharing (§13) |
| 17 | **Full union ships** — all ~7,576 works, no reduced v1 corpus (§5) |
| 18 | **Build the whole spec**, not a spike or an MVP. Milestones are a build *order*, not a scope cut (§15) |
| 19 | The org tree (`otzaria-library`) is **not** ingested — 41 real new titles, 8,106 artifacts (§2.3a) |

## 17. Still to determine

Not blocking; each is decidable inside its milestone.

1. ~~**OCR engine** (M7). Hebrew OCR on old print is genuinely hard and Tesseract
   is mediocre at it. An afternoon of evaluation decides whether "optional OCR"
   is a good feature or a disappointing one.~~ **Measured in W26 and answered:**
   tesseract 5.4 with `tessdata_best` scores 99% of the words on modern square
   print and **27% at 23% precision on Rashi script with nikud**, and a
   confidence floor does not separate the two — it is confidently wrong. So the
   default reader for a PDF is the file's own text layer (87%/94% on the same
   pages), tesseract is *found rather than bundled* for pages that have none,
   and the decision is made reversible instead of permanent: a reading is an
   overlay in the personal layer and a correction is anchored to the ink, so
   re-reading a scan with something better is one pass with nothing to migrate.
   See the README's *Reading a scan* and `crates/girsa-scan/src/engine.rs`.
2. **Root table scope** (M3). How many words earn a hand-written entry before the
   effort stops paying.

*Resolved by measurement — see §2.3: download is ~2.2 GB; overlap is 5,637 shared,
978 Otzaria-only, 961 Sefaria-only.*

---

## Sources

- [Sefaria-Export](https://github.com/Sefaria/Sefaria-Export) ·
  [developer docs](https://developers.sefaria.org/docs/) ·
  [Linker API](https://developers.sefaria.org/docs/linker-api) ·
  [Sefaria-ElasticSearch](https://github.com/Sefaria/Sefaria-ElasticSearch) (GPL-3.0)
- [Otzaria](https://github.com/Sivan22/otzaria) ·
  [otzaria-library](https://github.com/Sivan22/otzaria-library) ·
  [mcp-otzaria-server](https://github.com/Sivan22/mcp-otzaria-server)
- [Zayit](https://github.com/kdroidFilter/Zayit) (AGPL-3.0 §7b — read, do not copy)
- [HebMorph](https://github.com/synhershko/HebMorph) ·
  [DictaBERT-morph](https://huggingface.co/dicta-il/dictabert-morph) ·
  [BEREL](https://arxiv.org/pdf/2208.01875) ·
  [Nakdan](https://arxiv.org/pdf/2005.03312)
- [Bar-Ilan Responsa Project](https://www.lib.uchicago.edu/e/ets/Responsa.html) ·
  ["Search Engines and Hebrew — Revisited"](https://link.springer.com/chapter/10.1007/978-3-642-45321-2_16)
- Local: `Videos/OtzariaSonim/SPEC.md` (verified Otzaria format findings) ·
  `Videos/Ksav/` (the other half)
