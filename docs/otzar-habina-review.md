# Otzar HaBina: a field review for Girsa

**Reviewed** 27–28 October 2026 · **Site** <https://otzarhabina.com> · **Snapshot**
`?v=night20261028` and the catalogues built 2026-10-19.

This is a review of what the site does, what is worth taking into Girsa, what
Girsa already does better, and what must not be copied. It is written to be
acted on: every numbered finding has a disposition — and, where it is work, a
size and a priority — and the umbrella issue lists all of them.

It is **not** a licence to take the corpus. The site's terms forbid automated
download of the library or large parts of it, and the software is closed. The
texts themselves come from Sefaria, Wikisource, Dikta, Torat Emet, Peninim and
the National Library, each under its own licence; that is the road in, and it is
the road Girsa already uses. Section *Outreach* is the alternative to scraping.

The umbrella issue is **#67**; the outreach draft is **#66**.

---

## The site in one page

Otzar HaBina is a free Hebrew Torah library that runs in a browser, as a desktop
program and on a phone. Its own description is exact: **three kinds of book** —
*עימוד חדש* (newly typeset pages, with commentaries around the text), *טקסט*
(full text that can be searched, marked and copied), and *PDF* (scans of old
editions, plus books uploaded with permission). It claims **more than 11,000
books** and **more than 7 million links between books**, full-text search, and a
set of study tools.

The first impression is justified: it is beautiful, fast enough, and unusually
complete. The typography is not a theme laid over text; it is a page engine. The
links are not a list of "related books"; they are per-line edges into exact
addresses. The search is not a single box; it has a grammar, scopes, and a
question mode. And there is a real attempt to make OCR trust visible, which is
rarer than any of the above.

### What is genuinely excellent

1. **The page-as-artifact.** A typeset daf with running heads, a perek opener,
   Rashi script, absolute commentary columns and a 285×403 mm sheet — and a
   print stylesheet that puts one page on one sheet. It is a generation ahead of
   "a column of text with comments underneath".
2. **The link graph at line level.** Roughly 2,100 edges for Bekhorot alone,
   each `Book|ref`, grouped by corpus and opened at the exact address. Seven
   million of them is a different order of magnitude from a "related seforim"
   shelf.
3. **Proof levels as a first-class status.** *Level 1* means the suspicious
   words were checked against the image; *level 2* means the whole page was read
   against it. A book carries ✓ / ✓✓ / a percentage in the library list. That is
   how a reader knows what the text is worth.
4. **The proofing workplace.** Scan on one side, OCR text on the other,
   suspicious words marked, a click removes a mark, drafts survive leaving the
   page, corrections wait for approval, trusted correctors' fixes go straight in.
5. **The catalogue's honesty about editions.** The same work can be a scan, a
   fresh typesetting, and a Sefaria text; the search result is one row and opens
   the best available form.
6. **The reading surface.** Paper tints including night, font choice including
   Rashi and Stam, width, size, two-page spread, reading ruler, page box, a
   per-page context menu, and a "my marks" list that is genuinely usable.
7. **The search grammar and the scopes.** Alternatives, exclusion, wildcards,
   phrase, ordered, near-within-N, prefixes, substring — and scope to the whole
   library, the open book, a category, favourites or a saved list.
8. **The study cycle.** Daily learning from a published calendar, zmanim,
   Hebrew calendar, parasha, siddur, tikkun koreh, shenayim mikra — each one
   click from the sefer it names.
9. **Small, humane details.** "Always open this book at this page", a reading
   ruler, a floating quick window, a page-turn by swipe, "make this commentary
   mine so it comes first", a hover peek at a linked passage, and a back button
   that returns to the passage with its links panel restored.
10. **Trust surfaces.** Per-page and per-commentary credit, a licence on the
    last page, a "report a problem on this page" that carries the page, and a
    privacy policy that says guest settings stay in the browser.

### Where Girsa is already ahead

This matters as much as the takes, because several of the site's best-looking
ideas are already solved here, and copying them would be a regression:

| The site | Girsa |
|---|---|
| A search box with chips and a scope | The same, plus five explicit modes, eight-rung widening with counts, live facets, a semantic lane and never a bare zero |
| Highlight a passage, open a commentary | Tick a mefaresh once and every line it wrote is marked; click the line and it opens there (start-here §2) |
| Links grouped by book | Links grouped by sefer with counts, type, provenance, lenses, repair, inbound caches and a transmission chain |
| "My marks" | An append-only personal layer of notes, marks, saved queries, chaburah folders, tags and corrections, with tombstones |
| Page image + OCR text | OCR anchors to coordinates, page→daf mapping, OCR badges in results, a degraded-OCR measurement, and a correction overlay that never mutates the base |
| A saved arrangement of tabs | Desks: tabs, panes, which pane follows which, widths — all kept |
| Daily learning tiles | A luach with two tracks and a published ruling on why the other three are not guessed |
| An account that syncs | Local-first, no account, nothing leaves the machine |

The site is a better *product* in scale and polish. Girsa is a better *library*
in provenance, correction, search control and ownership. The useful review is
therefore narrow: take the product ideas that do not cost the library its soul.

### The ten things I would take first

| | Finding | Why first |
|---|---|---|
| 1 | **I1–I2** Proof status and the proofing workplace | Girsa already detects suspects and applies corrections; the missing half is a *status a reader can see* and a page-level workflow. This is the highest-value, most defensible take. |
| 2 | **D2–D6** The links drilldown: classes → books → words, open-all, pin, peek, back | Girsa has the graph and the panel; this turns a list into a place to learn. |
| 3 | **A4–A5, I5** A book's own page: author, place, year, topics, edition, licence, OCR status | Cheap, and it is the trust story Girsa's spec already tells. |
| 4 | **B1, B8** A first-run/home surface that teaches the five-minute loop | Girsa has the documentation and no in-app door. |
| 5 | **C2–C3, C7–C8, C12** Reading comfort: paper, font, width, spread, ruler, "always here" | Small CSS and state; felt every day. |
| 6 | **F5, F4** Save a result set as a shelf list, and scope search to a list | The site's most useful search feature Girsa does not have. |
| 7 | **E3, E5** Group links by corpus, and show the linked words in the panel | Makes a 2,000-edge daf readable. |
| 8 | **G9, J7** A visible OCR queue: progress, pause, priority, resume | Girsa runs OCR in the background and says little about it. |
| 9 | **C9, C10, G12** Region select, the page context menu, copy page text | The scan reader becomes a working surface, not a picture. |
| 10 | **H1, H4** Daily learning and the parasha card, within Girsa's ruling | Retention without guessing calendars. |

### The one thing I would not do

**Do not bulk-download the library.** The terms forbid it, the software is
closed, the link graph is their work, and none of it is necessary: the texts are
upstream under open licences. If a work exists only there, the answer is an
email (see *Outreach*), not a crawler.

---

## Numbers

Measured from this machine on 28 October 2026, over the public site. They are a
snapshot, not a benchmark.

| | |
|---|---|
| Works counted | 11,170 (830 PDF + 2,743 typeset + 7,597 text; the site says 11,000+) |
| Links claimed | 7,000,000+ |
| `/api/books` (PDF catalogue) | 830 rows, 136 KB compressed |
| `flow/catalog.json` (typeset) | 2,743 rows, 1,066 KB raw / 107 KB compressed |
| `sefaria/catalog.json` (text) | 7,597 rows, 3.0 MB raw |
| Home HTML | 9.8 KB |
| Home CSS + JS | 37 files, 382 KB compressed |
| Font faces declared | 33 (10 families, all open-licence) |
| Search "ספק ספיקא" | 25,314 places / 2,603 books, server 264 ms, 0.70 s, 81 KB |
| Search "שלום" | 98,344 / 4,945, 188 ms, 0.63 s, 138 KB |
| Search "ברכת בורא פרי האדמה" | 705 / 248, 213 ms, 0.68 s, 8 KB |
| Search "מאימתי קורין את שמע" | 501 / 287, 70 ms, 0.55 s, 9 KB |
| Search "שבת" | 322,056 / 6,773, 485 ms, 0.78 s, 192 KB |
| `/api/ask` (question) | 1,778 places, 40 sources, 627 ms |
| Page image, 1000 px | 137 KB JPEG, 0.78 s |
| One masechta's link file | 2,142 line entries, 1.1 MB (Bekhorot) |
| Forced opening screen | 4 s minimum, 6 s maximum |

---

## How to read a finding

Each is a heading `### ID · Title`, then what the site does, what Girsa should
do, and a verdict:

- **TAKE · size · priority** — build it. Size S/M/L; priority P1 (next), P2
  (soon), P3 (later).
- **ALREADY** — Girsa does this, or does something stronger. No work.
- **NO** — an anti-pattern, or a violation of a Girsa non-goal. Do not build.
- **DECIDE** — it conflicts with a documented decision, or needs a ruling from
  the owner before it can become work.

---

## A. The library and its catalogue

### A1 · Three kinds of book, one library

**Site.** Every work is one of three representations: a fresh typesetting with
commentaries (*עימוד חדש*), a Sefaria-style text, or a PDF scan. The library
list, the search and the reader all know which one a row is, and colour the tab
by kind.

**Evidence.** About page ("שלושה סוגי ספרים"); `/api/books` has 830 PDF rows,
`flow/catalog.json` 2,743 typeset rows, `sefaria_catalog.json` 7,597 text rows.

**Girsa.** Girsa has text and scan; it deliberately has no typeset page. The
*model* is still right: a work has representations, and the reader should never
have to know which one a search hit came from until they open it.

**Verdict — ALREADY.** Girsa's `girsa-corpus` editions and scan layer cover the
text/scan halves; the typeset third is a non-goal (C1, L1). Keep the kind visible
in the shelf and the tab, as the site does.

### A2 · Breadth: eleven thousand against Girsa's shelf

**Site.** 11,170 works. The long tail is public-domain scans, self-published
seforim and school editions, not only Sefaria's 7,597. The catalogue includes
173 responsa, 167 Tanach commentators, 102 later commentators on Shas, 66
commentaries on Mishneh Torah, 63 Mishnah, 61 Tosefta, 47 responsa collections,
44 rishonim on halacha, 44 Ein Yaakov, 40 general seforim.

**Evidence.** `flow/catalog.json` category counts; `/api/books` rows.

**Girsa.** Spec §5 ships ~7,576 works (Sefaria ∪ Otzaria). The gap is real, but
it is a *coverage* problem, not a scraping problem. Build a diff: normalise
titles and authors, mark works present/absent, and resolve each absent work to an
upstream open source. Add only what is licensed; record the licence.

**Verdict — TAKE · L · P2.** A coverage report first, then a curated ingestion
list. Do not add the site's scans to the repo.

### A3 · One work, several representations, resolved as one

**Site.** `flTwinOf(id)` maps a text book to its typeset twin. Search merges the
two into one row, sums occurrences, and opens the typeset page at the passage
found; the same logic runs in the question results and in the links pane.

**Evidence.** `search.js` (`flTwinOf`, merged rows), `flow.js` (`flTwinOf`,
`flPageOf`), `flow_catalog.json` (`tx` = Sefaria id, `f` = file, `o` = order).

**Girsa.** Girsa has permanent segment ids and a redirect table; what it does not
have is an explicit "same work in another form" edge used at the UI. That edge is
worth having as data, not as a title heuristic: a representation points at a
work, and a work points at its representations. Then "open the best form" is a
query, not a string comparison.

**Verdict — TAKE · M · P2.** Add a representation/twin relation to the work
model; use it in search results and the shelf. Keep the permanent id as the
identity.

### A4 · A book's own page

**Site.** Right-click → *פרטי הספר* opens a small form: title, author, place of
printing, year, topics, a free note, then category, page count, file size, book
id, and whether it is scanned and searchable. A typeset book has a shorter form:
title, library path, type. The fields are editable; online, an edit becomes a
request for the manager.

**Evidence.** `tools.js` `bookInfo`; screenshot `book_info`.

**Girsa.** Girsa carries author/era/place from Sefaria and Otzaria in the corpus,
and licence/provenance in the print view, but there is no single "about this
sefer" surface a reader can open from the shelf or the tab. That surface is where
provenance stops being metadata and becomes trust.

**Verdict — TAKE · S · P1.** One panel: identity, edition, provenance, licence,
OCR/proof status, counts, and "where this came from". Read-only for corpus
fields; the reader's own note is editable.

### A5 · Credit on every page, licence on the last one

**Site.** A typeset Mikraot Gedolot page carries a credit line naming the edition
and licence of *every* commentary on it (Sefaria, Wikisource, Metsudah, public
domain, CC-BY, CC-BY-NC). The last page of a book carries its full licence and
credit. The About page repeats the rule: the credit is on the last page of every
book.

**Evidence.** `flow_gen.html` `.credit`; About page; `sfBookInfo`.

**Girsa.** Spec §13 already requires carrying source and licence. The site shows
the *reader-facing* form: not a metadata field, but a colophon on the page and a
panel at the end. Girsa's print view has `print-provenance`; extend that to the
reader and the shelf.

**Verdict — TAKE · S · P1.** A "Sources & credits" section in A4 and in the print
sheet, generated from the corpus record, never hand-written.

### A6 · The catalogue as a downloadable list

**Site.** The About page offers an Excel download of the whole catalogue, in
tabs: all books, typeset, text, PDF, alphabetical. It is a trust gesture: a
reader can see what is and is not there without asking.

**Evidence.** About page ("הורדת רשימת כל הספרים (Excel)").

**Girsa.** A local library can do better: export the shelf as CSV/JSON with
title, author, category, kind, licence, path and id. This also makes A2's
coverage diff reproducible.

**Verdict — TAKE · S · P3.** A `girsa-corpus` export, documented in `tools.md`.
Low urgency, high trust.

### A7 · Series and multi-volume sets are first-class shelves

**Site.** The library groups books whose titles share a series prefix
("תלמוד ירושלמי", "תוספתא (ליברמן)", "משנה ברורה") into a shelf row, rather than
making the reader scroll past forty volumes. Volumes are ordered by the series'
own order, and the text twin is named "* – טקסט".

**Evidence.** `core.js` `shelfSer` / series detection.

**Girsa.** The shelf has branches and loose works, and a multi-volume set can be
a branch. What is worth taking is the *automatic* series suggestion with a manual
override — and the honest suffix that says which form a row is.

**Verdict — TAKE · S · P2.** Suggest series in the shelf build; let the reader
rename/merge/split. Do not infer a series from a single shared word.

### A8 · Duplicate detection and book status

**Site.** Adding a book computes a normalised title and file size and warns when
a near-duplicate exists. Every row also carries a status: indexed/searchable, or
waiting for OCR; and a proof badge (✓ / ✓✓ / percentage). A book can be hidden
from the list without being removed.

**Evidence.** `siteFindSame`, `bookStatus`, `proofBadge`; screenshot `findbook`.

**Girsa.** The import path has permanent ids and continuity; it does not have a
reader-visible duplicate check or a per-book status row. Both are cheap and both
prevent the same class of support question.

**Verdict — TAKE · S · P2.** Duplicate warning on import (title + size + first
segment), and a status column: indexed / not indexed / OCR partial / corrected.

---

## B. Finding a sefer

### B1 · A search-first home

**Site.** The home screen is a search box, sixteen category tiles with counts,
the daily learning card, the luach and zmanim, a parasha card, and a rail of
tools. It answers "what is here?" and "what am I learning today?" before it asks
the reader to choose a shelf.

**Evidence.** `home.js`; screenshots `home`, `tools`.

**Girsa.** Girsa opens on the shelf. That is a defensible library choice, but
there is no surface that says *start here*: the five-minute loop lives in
`docs/start-here.md`, not in the window. A first-run/home pane can carry the
search box, "continue reading", the daf, and three one-click examples without
turning the app into a dashboard.

**Verdict — TAKE · M · P1.** A home/first-run surface with the loop in it:
search a sugya, open Rashi, send to Ksav. Dismissible; the shelf stays the
default for a reader who has used it.

### B2 · The library map

**Site.** *מפת הספרייה* is a modal tree: every category with its count, a search
inside the tree, sort by initial/author/name, favourite categories, recent books,
and a one-click "open". It is the same tree as the shelf, zoomed out.

**Evidence.** `screenshots/explorer`; `core.js` tree rendering.

**Girsa.** The shelf is a two-column tree with counts and drag/drop. A map view
is not a new model; it is a different projection of `Branch`. Useful when the
tree is deep or the window is narrow.

**Verdict — TAKE · S · P3.** A "map" toggle over the existing branch data, with
counts and a filter. No new taxonomy.

### B3 · The book pane beside the results

**Site.** Search results and the book list share one pane, split by a draggable
divider; double-click resets it. A result click does not throw away the list.

**Evidence.** `search.js` splitter; screenshot `search_final`.

**Girsa.** Girsa already docks search to a column and opens the daf beside it —
the stronger version of this idea. The site's extra is the *resizable split
between two lists*, which Girsa's dock does with panes.

**Verdict — ALREADY.** Do not add a second splitter.

### B4 · The book context menu

**Site.** Right-click a book: open, open in a new tab, open beside the open book,
open in a quick window, favourite, save to a list, pin to the quick bar, hide
from the list, details/licence, create a desktop shortcut, show in folder, scan
this book first, remove (manager only).

**Evidence.** `core.js` `bookMenu`.

**Girsa.** The shelf has open/rearrange, but the context menu is not the place
where a reader meets "open beside", "pin", "details" and "scan first". Those are
the verbs the site teaches by putting them one right-click away.

**Verdict — TAKE · S · P2.** A book context menu with the four Girsa-native
verbs: open here/beside/new tab, pin, details, scan/correct. Skip desktop
shortcuts and "show in folder" unless the shell already has them.

### B5 · Favorites, custom lists and quick buttons

**Site.** Favourites are a cross-shelf star; lists are named collections with
add/rename/delete; quick buttons pin a book or a list to the rail. A list can be
a search scope and a saved result set.

**Evidence.** `core.js` `saveToList`, `addQuick`; `search.js` scope; `site.js`.

**Girsa.** Girsa has chaburah folders, saved queries and shelf pins. The missing
verb is *save the books a search found as a folder/list*, which is B5 + F5. Do
not add a second "favourites" concept beside folders; use folders and pins.

**Verdict — TAKE · S · P2.** "Save these results as a folder" and "search this
folder"; reuse the personal layer, no new store.

### B6 · "Open beside" and the quick window

**Site.** From the book menu: open beside the book you are reading, or in a
floating quick window that survives switching tabs. The split has a "1|2" button
and the two halves can be independent.

**Evidence.** `core.js` `sideOpen`, `openQuickWindow`; `side.js`.

**Girsa.** Girsa has tabs, panes, docking and desks — the harder half. A floating
quick window is a shell feature; if the Tauri shell can own a second small
window, it is a nice one, but it is not core.

**Verdict — ALREADY (split) / DECIDE (quick window).** The split is done; the
floating window is a shell question, not a library one.

### B7 · Recent books and "continue reading"

**Site.** Recent books, reading history, a per-book start page, and a "go to
bookmark" button. The home screen shows the last place.

**Evidence.** `U.recentBooks`, `U.startPage`, `addHistory`.

**Girsa.** Position memory per sefer exists; a "continue" row and a "always open
here" action are small additions on top of it.

**Verdict — TAKE · S · P3.** "Continue" on the home pane; "always open at this
page" in the page menu (C12).

### B8 · Empty states that teach the next move

**Site.** An empty search says *"type a word or a question, results will appear
here"*; an empty library says what to do; a missing index says it is being built;
a book not yet scanned says so. The empty state is where a new reader learns the
product.

**Evidence.** `search.js` empty states; `noIndexNote`; screenshots.

**Girsa.** Girsa's empty states are unusually good in prose (the trouble codes),
but the first-run empty shelf is a command in a sentence. A first-run panel with
the command, a "choose a folder" button and the five-minute loop would do more
than another paragraph.

**Verdict — TAKE · S · P1.** Make the first-run state a door, not a diagnosis.

---

## C. Reading

### C1 · The page facsimile

**Site.** A typeset daf: 285×403 mm sheet, running head, page number, perek
opener, Rashi in Rashi script, absolute columns for each commentary, a credit
line, `@page` print CSS, and a print path that puts one page on one sheet. The
layout is measured in JS and rendered server-side; the page is the product.

**Evidence.** `flow_gen.html` (styles, `.page`, `.head`, `.credit`, `@page`);
`flow.js` (`flFit`, `flPrint`, print stamps).

**Girsa.** Spec §6.2 is explicit: **no tzuras hadaf for text seforim** — it is a
typesetting project, not a layout problem. Spec §6.3 says the scan *is* the daf.
Do not reopen this as a "nice to have". The page's *chrome* is separable and
worth taking: running head, page/amud box, perek selector, zoom, spread, print
range, credit. Those are C2, C6, C7, A5.

**Verdict — NO.** A page facsimile for text would contradict a deliberate
non-goal. If the owner wants it, it reopens §6.2 and should be argued there, not
smuggled in as a view.

### C2 · The text reader's chrome

**Site.** Top bar: title, volume/part select, perek/verse select, language
(he/en/both), paper colour, font, A−/A+, width, find-in-book, my marks, page box,
links toggle. The reading area is a page-width column with generous leading.

**Evidence.** `sefaria.js` `sfEnsureView`; screenshots `reader_page2`,
`sf_links`.

**Girsa.** Girsa has the toolbar, nikud/te'amim, fonts, leading and measure; the
site's useful additions are the *combined place selectors* (part + perek/verse)
and the page box. The languages toggle is a non-goal (L2).

**Verdict — TAKE (place selectors, page box) · S · P2.** One "go to" control that
knows the sefer's own divisions, instead of making the reader choose between a
TOC and a search.

### C3 · Paper, font and width

**Site.** Per-book paper tone (including night), font family (including Rashi
and Stam), size, and a width slider. The choices are remembered per reader, and
the PDF reader has its own paper tone that multiplies over the scan so black
letters stay black.

**Evidence.** `SF_PAPER`, `sfApplyLook`, `applyPdfPaper`; screenshots.

**Girsa.** Girsa has theme, Hebrew/Latin fonts, line height and measure. The
site's paper *tints* and the scan-preserving multiply are the missing comfort
layer; they are CSS, not a new engine.

**Verdict — TAKE · S · P2.** Paper tint (light/sepia/night) for text and scans,
per-book font choice, width presets. Respect the reader's theme; do not override
it silently.

### C4 · Page images on demand

**Site.** `/api/page?id=&p=&w=&n=` returns a JPEG resized to the requested width;
the client asks for a DPR-aware width, caches the aspect ratio per book, and
prefetches the current page of every other open tab. A 1000 px page is ~137 KB.

**Evidence.** `viewer.js` `pageSrc`, `imgWidth`, `bookAspect`, `prefetchTabs`;
measured 137 KB / 0.78 s.

**Girsa.** Girsa renders scans locally; the transferable parts are the *aspect
cache* (no layout jump), *prefetch of the other open tabs* (one at a time), and
*width quantisation* (do not re-render for every pixel).

**Verdict — TAKE · S · P2.** Prefetch the next page and the other tabs' current
pages; cache aspect per book; quantise render width.

### C5 · Text mode for a scan

**Site.** A "read as text" mode shows the OCR text with its own font and size,
side by side with the page, and the OCR is what search and copy use. The image
stays ground truth.

**Evidence.** `viewer.js` `toggleTextMode`; `marks.js` page text copy.

**Girsa.** Girsa has scan text and OCR anchors; the missing convenience is a
one-click "text beside image" with the same position, and a "copy page text"
that says when the page is not indexed.

**Verdict — TAKE · S · P2.** A split text/image toggle in the scan view, with the
OCR badge and the page's proof status.

### C6 · Page and amud navigation

**Site.** A right rail with the perek list and an amud grid (א/ב), a page box
that accepts a number or Hebrew numerals, prev/next, and a toast naming the page
it opened. For a typeset book the rail holds its parts and pages; for a scan, the
PDF pages; for a text, the chapters.

**Evidence.** `viewer.js` `renderPageList`, `gotoPage`; `flow.js` `flNav`;
screenshot `bavli`.

**Girsa.** Girsa has a TOC and a page/daf box in the toolbar; the site's amud
grid is a compact way to jump two pages at a time on a daf. Worth taking for
Gemara only, where the amud is the unit.

**Verdict — TAKE · S · P3.** A daf/amud grid in the TOC for works with a daf
paging scheme; keep the existing box.

### C7 · Two-page spread and focused reading

**Site.** A two-page spread, fit width/page, a focused-reading mode that hides
the rails, fullscreen with the list kept, and a page-fullscreen that takes both
halves of a split.

**Evidence.** `marks.js` `TOOLS.two`, `pagefull`, `full`; `viewer.js`.

**Girsa.** Girsa has focus/fullscreen and panes; a true two-page spread for scans
is missing and is a small layout state. It matters for a printed sefer and for a
wide monitor.

**Verdict — TAKE · S · P3.** Spread for scans; focused reading for text is
already there.

### C8 · The reading ruler

**Site.** A movable horizontal band that dims everything except the line being
read. One click on the rail; remembered.

**Evidence.** `marks.js` `TOOLS.readline`, `toggleReadLine`.

**Girsa.** No equivalent. It is a small, genuinely useful accessibility/comfort
feature for a dense daf, and it does not touch the text model.

**Verdict — TAKE · S · P3.** A ruler overlay with a keyboard toggle; remember per
reader, not per book.

### C9 · Select a region of a page

**Site.** "צילום קטע" lets the reader drag a rectangle on a scan; the app
extracts the OCR text of that region, shows it, copies it, and can save the crop
as a PNG. Selection inside the OCR text highlights the corresponding area on the
image.

**Evidence.** `marks.js` `startSelect`, `selectionDialog`; `/api/region`;
`proof.js` selection sync.

**Girsa.** Girsa has scan highlights anchored to ink and a correction overlay,
but not a "crop this rectangle and give me its text" action. That is the bridge
between a picture and a citation.

**Verdict — TAKE · M · P2.** Region select → OCR text + crop, with "copy",
"search", "correct", "save image". Reuse the scan anchor/box model.

### C10 · The page context menu

**Site.** Right-click a page: copy page text, copy citation, add note, highlight,
topic key, link to another page, named mark, bookmark at this line, open in a
quick window, "always open at this page", report a problem.

**Evidence.** `marks.js` page context menu; `tools.js`.

**Girsa.** Girsa's note/mark/link/correction machinery is stronger, but the verbs
are spread across panels. A context menu at the pointer is the site's best
discoverability trick.

**Verdict — TAKE · S · P2.** A context menu on the page and on a selection,
wired to the existing commands. Keep "report a problem" free of any account.

### C11 · Tabs that behave like documents

**Site.** Tabs are coloured by kind, draggable, closable by middle-click, have a
"close all others", an "all open books" menu when they overflow, and each keeps
its own query/hits/scroll. A tab click in "side" mode opens the book in the
other half.

**Evidence.** `core.js` `renderTabs`, `tabsMenu`, `makeSortable`.

**Girsa.** Girsa has tabs and per-tab state; drag-reorder, close-others and an
overflow menu are standard desktop affordances worth matching.

**Verdict — TAKE · S · P3.** The three missing tab verbs; no new model.

### C12 · "Always open at this page"

**Site.** The page menu can pin a book's start page, so a Gemara always opens at
the daf you are learning, not at page 1. A "go to bookmark" button does the same
for the current bookmark.

**Evidence.** `marks.js` `U.startPage[id]`.

**Girsa.** Position memory restores the last place; a *declared* start page is a
different thing (it survives finishing the sefer). Small state, real use.

**Verdict — TAKE · S · P3.** A per-sefer start page in the personal layer, with a
clear "use last position" default.

---

## D. Mefarshim

### D1 · Mefarshim in classes, with counts

**Site.** The mefarshim panel lists the commentaries on the masechta in folders
(פשט, תלמוד, הלכה, שו״ת, סידור, מלונים וספרי עזר) with counts, and "מפרשים 144"
on the toolbar. A click opens one beside the daf; the list stays.

**Evidence.** Screenshot `bavli_click`, `sf_links`; `mefarshim` panel.

**Girsa.** Girsa's mefarshim panel already has folders, counts, open-beside and
tick-to-mark-lines (start-here §2) — the stronger version. The site's extra is
the *class* vocabulary and the count on the toolbar button.

**Verdict — ALREADY.** Keep Girsa's folders; do not replace them with the site's
six classes.

### D2 · The three-level links drilldown

**Site.** With "מפרשים" on, a click on a passage opens a pane that drills:
**class → book → the linked words**. The first level is a count per class
(mefarshim, targumim, Tanach, Mishnah, Tosefta, Talmud, Midrash, Halacha,
responsa, Kabbalah, Chassidut, machshava, mussar, siddur); the second is the
books in that class with counts; the third is up to 200 passages, each with an
"open" button.

**Evidence.** `sefaria.js` `sfShowLinks`, `sfLinkGroup`, `SF_GROUP_ORDER`;
screenshot `sf_links`.

**Girsa.** Girsa's panel groups by **sefer** with counts, types, lenses and
inline words. The missing first level is the *corpus class*: on a daf with 2,000
edges, "which kinds of sefer are talking about this line?" is the question before
"which sefer?". Add a class grouping/filter, not a second panel.

**Verdict — TAKE · M · P1.** Group/filter the existing link rows by corpus class,
with counts; keep the sefer grouping inside each class.

### D3 · "Open all" in a group

**Site.** Every class/book group has "פתח את כולם": open all the linked books at
this passage at once.

**Evidence.** `flow.js` `linksPane` (`data-all`); screenshot `openall`.

**Girsa.** Girsa can open one link at a time; "open all in this class" is a
natural extension of panes/desks, but it can also produce twenty tabs. Cap it,
and open as columns/panes where possible.

**Verdict — TAKE · S · P2.** With a cap and a confirmation that names the count;
prefer panes to tabs.

### D4 · Pin a commentary

**Site.** In the links pane, a star beside a book says *"קבע כמפרש שלי — יופיע
ראשון"*: this commentary comes first on this base sefer from now on.

**Evidence.** `flow.js` `linksPane` (`data-fav`, `favKey`).

**Girsa.** Girsa has shelf pins and mefarshim order, but not a per-base-sefer
"my commentary" preference. This is a small, high-value personalisation: Rashi
first on Shas, the Gra on Shulchan Arukh, the sefer you are learning with.

**Verdict — TAKE · S · P2.** A per-work pinned mefaresh stored in the personal
layer; it affects order, not inclusion.

### D5 · Peek before you open

**Site.** Hovering a linked book shows a floating preview of the linked passage
(title, up to six lines, the ref), so the reader can decide without leaving the
page. Rashi and Bartenura are allowed to open themselves.

**Evidence.** `flow.js` `lpv` styles and hover handler.

**Girsa.** Girsa expands a group and fetches the words inline on open; a hover
peek is the same data, earlier, and cheaper than a tab. Do not auto-open anything
on hover (the site's Rashi/Bartenura exception is a surprise, not a feature).

**Verdict — TAKE · S · P3.** A peek card with the first lines and the ref; click
to open; no auto-open.

### D6 · The back stack

**Site.** Opening a commentary from the links pane puts a "חזרה ל<ref>" button at
the top; going back returns to the passage *with the links panel restored*, not
to the book's beginning.

**Evidence.** `sefaria.js` `sfOpenAt`, `sfGoBack`, `sfDrawBack`.

**Girsa.** Girsa has panes and docks; the missing thing is the *return context*:
passage + panel + scroll. If the reader follows a chain three deep, back should
be a stack, not a guess.

**Verdict — TAKE · M · P2.** A per-tab navigation stack of (work, segment, panel
state); back restores all three.

### D7 · The fixed Mikraot Gedolot desk

**Site.** The typeset Chumash page carries a fixed set of ~20 mefarshim (Rashi,
Onkelos, Ramban, Ibn Ezra, Sforno, Chizkuni, Baal HaTurim, Daat Zekenim,
Rabbeinu Bachya, Recanati, Ohr HaChaim, Kli Yakar, Siftei Chachamim, HaEmek Davar,
Malbim, and the rest), each with its own credit. The reader does not choose the
set; the page is the set.

**Evidence.** `flow_gen.html` credit line; screenshot `reader_page2`.

**Girsa.** `girsa-desk` is the machinery for "one base text, a chosen set of
companions"; the site's fixed set is a *default profile*, not a new feature. The
take is to ship per-corpus default profiles (Chumash, Shas, Mishneh Torah,
Shulchan Arukh) that a reader can accept in one click, then edit.

**Verdict — TAKE · S · P2.** Default companion profiles per corpus, with the
site's Chumash list as a starting point (checking each edition's licence).

---

## E. Links between seforim

### E1 · Seven million edges, per line

**Site.** Links are stored per line: `{"21b:5": ["Rashi on Bekhorot|21b:5:1",
"Mishneh Torah, Firstlings|4:12", ...]}`. One masechta has 2,142 entries; the
site claims more than seven million edges. The edge is a book + an exact ref, not
a title + a chapter.

**Evidence.** `links_bekhorot.json`; `sefaria/_links/*.json`; About page.

**Girsa.** Girsa has a link graph, inbound caches and repair, but its scale and
its *per-line density* are the site's real asset. The take is not the data; it is
the target: every line that somebody comments on should have its edges, and the
UI should be able to say how many.

**Verdict — TAKE · L · P2.** A coverage metric and a build report: lines with
links / lines without, by corpus. Use Sefaria's links and the imported libraries;
do not scrape the site.

### E2 · A count on the line

**Site.** A line with links is clickable when the toggle is on; the panel header
says "26 links"; the toolbar's mefarshim button carries a count. A reader can see
that *something* is here before clicking.

**Evidence.** `sfShowLinks` (`all.length`), screenshot `sf_links`.

**Girsa.** Link rows live in a panel per line; there is no count in the margin.
Girsa already marks lines that a *ticked* mefaresh wrote about (◆); a link count
would be a second, quieter mark, and it must be optional or it becomes noise.

**Verdict — TAKE · S · P3.** An optional count glyph in the margin for lines with
links; off by default.

### E3 · Grouping by corpus, not just by sefer

**Site.** The first level of the links pane is the corpus class, in a deliberate
order: mefarshim, targumim, Tanach, Mishnah, Tosefta, Talmud, Midrash, Halacha,
responsa, Kabbalah, Chassidut, machshava, mussar, siddur.

**Evidence.** `sfLinkGroup`, `SF_GROUP_ORDER`.

**Girsa.** Girsa groups by sefer and exposes link *types* (commentary, quotation,
reference, parallel, emends...). The class is orthogonal to the type: it says
*what kind of book is speaking*, not *how it is connected*. Add it as a facet.

**Verdict — TAKE · S · P1.** A corpus-class facet/group in the links panel,
derived from the work's taxonomy, not hand-tagged per edge.

### E4 · Opening at the exact address, with fallbacks

**Site.** A link ref is resolved against the target book's address map; if the
exact address is absent, the first sub-passage is used; for books whose leaves
are counted 1,2,3 (Meiri), `3:11` is converted to `2a:11`; if the target book is
not in the library, the row is dropped rather than shown as a dead link.

**Evidence.** `sfFindSeg`, `sfAddrMap`; `flow.js` `flOpenLinked`.

**Girsa.** Girsa's ref resolver is a dedicated crate with a regression corpus —
already stronger. The take is the *honest dead-link handling*: a link whose
target is absent should say so, not disappear.

**Verdict — ALREADY (resolver) / TAKE (absence is visible) · S · P3.** Show
"target not on this shelf" rather than filtering it out.

### E5 · Words in the panel

**Site.** Opening a book in the links pane fetches and shows the linked passages
inline (up to 200), each with a ref and an "open" button; the reader can read the
commentary without leaving the daf, then open it if it deserves a column.

**Evidence.** `sfShowLinks` `showTexts`; screenshot `sf_links`.

**Girsa.** Girsa fetches the words when a sefer group is expanded (`linkWords`),
so the idea is already there. The site's extra is the hard cap and the "open"
per passage; Girsa's expansion is better because it keeps the text in the panel.

**Verdict — ALREADY.** Do not add a second preview path.

### E6 · Links to books outside the corpus

**Site.** `ביאור חברותא` is not a Sefaria book; it is stored as per-address
paragraphs in `_extra`, with an author and a licence, and shown first among the
commentators. The link record carries the external book's identity and credit.

**Girsa.** Girsa's link model points at works in the corpus; personal/extra
sources are a separate layer. The site's pattern is worth copying *as data*: an
edge can point at a work that is not shipped, with provenance and licence, and
the UI can still show it.

**Verdict — TAKE · M · P2.** Allow link targets outside the shelf with a
provenance record; render them with a "not on this shelf" badge.

### E7 · The link graph is not a search result

**Site.** The links pane is a curated graph (Sefaria links plus their own), not
"search the phrase and show the hits". The one attempt to offer "sources on this
page" from search was removed because it found wrong things on scans.

**Evidence.** `marks.js` comment: *"'המקורות שבדף' was deleted — it found wrong
things in scanned pages"*.

**Girsa.** Girsa's `mekoros`/linkify is explicitly high-confidence only; the
spec's rule is "never guess — a wrong link is worse than no link". This is the
same lesson, independently learned. Keep it.

**Verdict — ALREADY.** No action; cite it when someone asks for auto-sources.

### E8 · No backlinks surface

**Site.** Forward links are excellent; there is no "who cites this?" view. The
reader can see what this line links to, not what links here.

**Evidence.** The links pane is one-directional; the data files are per source.

**Girsa.** `girsa-link` has inbound caches, a reverse index and repair — this is
a place Girsa is materially ahead. Do not import the site's one-way model.

**Verdict — ALREADY.** Guard it; a "cited by" surface is a Girsa advantage.

### E9 · No link repair

**Site.** Links are shipped data. A reader cannot say "this edge is wrong", retype
it, or undo it.

**Evidence.** No link-editing UI in the panel; only "open".

**Girsa.** Link drawing, retyping, provenance, undo and repair are built. Another
place Girsa is ahead.

**Verdict — ALREADY.** No action.

---

## F. Search

### F1 · Books, not hits

**Site.** Results are grouped by sefer: one row per book with an occurrence count,
the first place, and a category tag. Opening a row opens the book at the first
hit; the per-hit view is a snippet under the row, not a list of forty rows from
the same masechta.

**Evidence.** `/api/search2` returns `{books:[{b,occ,p,k}], total, ms}`; screenshot
`search_final`.

**Girsa.** Girsa's results are hit rows with facets and counts. A "by sefer" view
is a useful *second* projection for a broad query ("שבת" in 6,773 books), but the
hit view is the right default for a sugya. Do not replace it.

**Verdict — TAKE · S · P2.** A "group by sefer" toggle for large result sets,
with counts; default stays hits.

### F2 · Counts first, snippets lazily

**Site.** The first response carries counts and places only; snippets for the
first 12 rows are fetched by a second POST `/api/snips`. The request is abortable,
and a live timer shows how long the search is taking.

**Evidence.** `search.js` `runSearch`, `loadSnips`, `spdTime`.

**Girsa.** Girsa's rows carry their runs; no second round trip. The site's
two-phase shape is a server optimisation, not a UI idea. The *abort* and the
*visible time* are worth matching; Girsa already has `Latest` for stale answers.

**Verdict — ALREADY (mostly) / TAKE (elapsed time) · S · P3.** Show elapsed ms on
the result header; abort is done.

### F3 · The query grammar

**Site.** In the box: `/` alternatives, `-` exclusion, `*` suffix, `?` one
letter, `"..."` exact phrase. In the advanced dialog: phrase, ordered, near
within N, prefixes, substring. The dialog explains each with an example, and a
click inserts the operator.

**Evidence.** `search.js` `advancedSearch`; `runSearch` params.

**Girsa.** Girsa has the same controls as chips, plus regex, instruments,
citation, and an eight-rung ladder. The site's contribution is the *shorthand*:
`/`, `-`, `*`, `?` typed in the box, with a visible chip flip. Spec §9.5 already
says "typing a sigil flips the matching chip" — if that is not implemented, this
is a spec gap, not a new idea.

**Verdict — TAKE · S · P3 (if the sigils are not already wired).** Implement the
spec's sigil-to-chip rule and document the four operators on the empty state.

### F4 · Scope: everywhere, this book, a category, a list

**Site.** A scope select beside the search box: all books, the open book, a
category, favourites, a saved list. The chosen scope is part of the query and is
shown in the result footer.

**Evidence.** `search.js` `scopeParams`; screenshot `search_adv`.

**Girsa.** Girsa has a scope panel over shelves, and a find bar in the sefer. Does
it scope to a *saved query's* result set or a chaburah folder? If not, this is the
gap. Scope by folder is the natural fit for Girsa's model.

**Verdict — TAKE · S · P2.** Scope to a folder/list and to a saved result set;
say the scope in the header.

### F5 · Save a result set as a list

**Site.** "Save list" turns every book in the current results into a named list,
which then becomes a scope, a quick button and a shelf row. It is one click from
a search to a study set.

**Evidence.** `search.js` `rSaveList`; `core.js` lists.

**Girsa.** Girsa has saved queries (the query, not the books found). Saving the
*books* is a different object and a genuinely useful one: "the seforim that say
'ספק ספיקא'" as a folder.

**Verdict — TAKE · S · P1.** "Save these results as a folder" in the search
panel; dedupe; name defaults to the query.

### F6 · Category filter with counts

**Site.** A filter button on the results opens a menu of top categories with the
number of books in the current result for each.

**Evidence.** `search.js` `rFilterBtn`.

**Girsa.** Live facets already do this, with more dimensions and exclusion. No
action.

**Verdict — ALREADY.** Facets are stronger.

### F7 · Part-of-word search discloses what it matched

**Site.** "חלק ממילה" matches the letters anywhere in a word, then shows the
words it matched (capped, "most common"), offers "add letters" and "regular
search". The index stores words without final letters for this mode.

**Evidence.** `search.js` `sub`, `subFin`, `rsubw`.

**Girsa.** Girsa has `matchContains` ("a word containing these letters") but does
not, as far as the UI shows, tell the reader which words it caught. The site's
disclosure is the good half: a widening search must say what it widened to.

**Verdict — TAKE · S · P3.** Show the matched forms (capped) under the header,
with "add letters" and a one-click return to exact.

### F8 · The reference jump in the search box

**Site.** Typing a place — `חושן משפט קצ ג`, `ברכות ב:`, `שו״ע או״ח רנג ב` — opens
the book there instead of searching the words. It expands abbreviations (`שו״ע`,
`או״ח`, `חו״מ`), reads Hebrew numerals in order, handles daf/amud, picks the
right volume of a multi-volume set (Mishnah Berurah), and toasts what it opened.

**Evidence.** `search.js` `refJump`, `REF_ABBR`, `REF_VOLS`.

**Girsa.** Girsa has Citation mode and a landing offer above hits. The site's
abbreviation and multi-volume handling is a good checklist for Girsa's resolver,
not a new mode.

**Verdict — ALREADY (mechanism) / TAKE (abbreviation/volume cases) · S · P3.**
Use the site's cases as a resolver regression list.

### F9 · The question mode

**Site.** A separate "שאלה בהלכה" mode detects a yes/no question, extracts
keywords, and returns up to 40 sources with snippets. It explicitly does not
pasken; a note says the tool shows mekorot only and the ruling is a rabbi's. If
the words find nothing, it offers the word search. Voice input is supported.

**Evidence.** `search.js` `runAsk`, `ASK_NOTE`, `/api/ask`; screenshot
`search_adv`.

**Girsa.** Spec §14: **not a paskening machine**; the semantic lane is for a
half-remembered line, not a question, and it now says so when asked one. The
site's mode is a keyword extractor with a good disclaimer, not an answer engine.
Taking it would put a question-shaped box in front of a library whose own
measurement says questions are its weak case.

**Verdict — DECIDE.** If the owner wants it, it is a *query helper* that returns
sources and never a verdict, with the site's disclaimer copied verbatim in
spirit. It must not be called "ask" in a way that implies an answer. Default: no.

### F10 · The index that builds on first use

**Site.** The first search over Sefaria books starts an index build ("a few
minutes"); until it finishes, results are from PDFs only, and a line says so.
Scanned-page progress is also reported ("the scan is still running, X% of pages").

**Evidence.** `search.js` `state === 'building'`, `noIndexNote`.

**Girsa.** Girsa ships a prebuilt index and says when it is absent; the site's
cold start is the anti-pattern to avoid. The take is the *status vocabulary*:
never a silent gap, say which corpus is searchable and which is not.

**Verdict — NO (cold build) / ALREADY (status vocabulary).** Do not make the
first search pay for the index.

### F11 · Result ordering by representation

**Site.** Typeset books first, then text, then scans; a text book with a typeset
twin is merged into the typeset row. Within a kind, the order is the engine's
score (or occurrences/name/category, if the reader sorts).

**Evidence.** `search.js` `kindR`, `renderResults`.

**Girsa.** Girsa ranks by score and badges OCR. The site's rule is a *preference*,
not a score: the best reading surface first, the raw scan last. It is worth
offering as a sort, not imposing it.

**Verdict — TAKE · S · P3.** A "best form first" sort when a work has several
representations.

### F12 · The 600-book cap is said out loud

**Site.** The results list stops at 600 books and prints "the first 600 are
shown; narrow the search"; snippets stop at 12; linked words at 200. Each cap is
visible.

**Evidence.** `renderResults`, `loadSnips`, `showTexts`.

**Girsa.** Any cap must say so; this is a principle Girsa already follows. Keep
it when adding F1/F5.

**Verdict — ALREADY.** No action; a reminder for new surfaces.

---

## G. The personal layer and tools

### G1 · Works without an account; syncs with one

**Site.** A guest can read and search; preferences, bookmarks and settings stay
in the browser. Registering (username, email, password) moves the personal layer
to the server and syncs it across devices; the first account is the admin;
unused accounts are deleted after a year.

**Evidence.** `auth.js`; Privacy page.

**Girsa.** Girsa is local-first and has no account. That is a deliberate
advantage, not a gap. The site's *guest* behaviour is the part worth matching:
nothing about reading requires a login.

**Verdict — ALREADY (local-first) / NO (server accounts).** Do not add accounts to
get sync; the personal layer is a folder the reader owns.

### G2 · Nineteen tools behind the login wall

**Site.** A guest who clicks print, copy page text, highlight, bookmark, note,
key, named mark, link, favourites, hide, lists, quick buttons, the quick window,
region select, paper colour, or the personal panel is told to register —
nineteen reading verbs in all. The
report-a-problem action is deliberately free.

**Evidence.** `marks.js` `LOCKED`, `lockMenu`; `site.js` `sitePrintAllowed`,
`guardBm`.

**Girsa.** This is the clearest anti-pattern in the review. Reading tools are not
a perk of registering; a library that will not let you copy a page or print a daf
until you identify yourself has put a door where a desk should be. Girsa must
never gate a reading verb behind an account (it has no accounts, and should keep
it that way).

**Verdict — NO.** Record it as a rule: every reading verb works offline, on the
first run, with no identity.

### G3 · Mark types

**Site.** Highlight (six colours, region or text selection), note, bookmark band
(draggable up/down on the page), link to another page/book, topic key (an index
term), named mark (a citation/quote), each with edit/delete and a per-book list.
Marks on a scan are rectangles normalised to the page, so they survive a resize.

**Evidence.** `marks.js` `MARK_COLORS`, `drawMarks`, `markPopup`.

**Girsa.** Girsa's marks/notes are stronger in provenance and anchoring; the
site's *topic key* and *named mark* are useful distinctions Girsa may fold into
tags. The six-colour palette is cosmetic; do not add colours without a legend.

**Verdict — TAKE (named mark/key semantics) · S · P3.** Check whether Girsa's tag
already covers "key"; if not, add it as a tag kind, not a new store.

### G4 · A topic index built from your own marks

**Site.** "מפתח הנושאים שלי" collects every topic key into an index, so a reader
can find the pages where they wrote "ערובין" or "בורר" without a full-text search.

**Evidence.** `tools.js` `keysIndex`; `marks.js` `addKey`.

**Girsa.** `yoursview` has a tags panel; if it already indexes tags, this is
done. If tags are only labels on notes, a tag index is the missing view.

**Verdict — ALREADY (if tags are indexed) / TAKE · S · P3.** A tag index over the
personal layer, with counts and jump-to.

### G5 · Lists as objects

**Site.** A list has a name, members, a quick button, a search scope, and can be
created from a book or a result set. Rename/delete; deleting a list does not
delete the books.

**Evidence.** `core.js` `saveToList`; `site.js`; `search.js` `scopeParams`.

**Girsa.** Chaburah folders are the same object with more structure (a folder can
have subfolders, and it can be exported). The missing verbs are "create from
results" (F5) and "search this folder" (F4), not a new list store.

**Verdict — TAKE · S · P2.** Two verbs on the existing folder model.

### G6 · History and pinned searches

**Site.** Recent searches (30, pinned ones kept), clear-unpinned, a search history
entry in the general history, and reading history.

**Evidence.** `search.js` `rememberSearch`, `recentMenu`, `addHistory`.

**Girsa.** Saved queries exist; a *recent* list is a lightweight complement. Pin a
query is already possible by saving it; no need for a second mechanism.

**Verdict — TAKE (recent) · S · P3.** A recent-searches menu; saved queries stay
the durable object.

### G7 · The tools drawer

**Site.** A single "כלים" drawer: gematria (standard, final letters 500–900,
small), date converter (Hebrew↔civil, yahrzeit, aveilut, bar/bat mitzvah,
Sephardi/Ashkenazi seasons), Aramaic–Hebrew dictionary, acronym decoder, Talmud
sages (from *Toldot Tannaim veAmoraim*), shiurim calculator, Hebrew calendar,
zmanim.

**Evidence.** `tools.js` `toolsDrawer`; About page.

**Girsa.** Girsa has gematria, rashei/sofei tevot, atbash and dilug as *search
instruments*, plus the luach. It does not have the date converter, the Aramaic
dictionary, the sages lookup or the shiurim calculator. Of those, the date
converter and the sages lookup are the best fit; the dictionary is a data
project; the shiurim calculator is a small, self-contained win.

**Verdict — TAKE (date converter, sages, shiurim) · M · P2/P3.** Keep them as
tools, not search modes; each needs its own data source and licence.

### G8 · Look up a selected word

**Site.** Select a word, right-click: search it, look it up in the Aramaic
dictionary, decode it as an acronym, or find the sage. The same menu works on a
scan and on text.

**Evidence.** `marks.js` context menu; About page.

**Girsa.** Girsa's selection menu has search/cite/note. Adding "look up" for
acronyms and sages is the human face of G7 and fits the existing selection path.

**Verdict — TAKE · S · P2 (after G7's data).** A lookup submenu on selection,
with the source/licence on each result.

### G9 · The OCR queue is visible and steerable

**Site.** A scan drawer shows books/pages done, a progress bar, the current book,
pages per minute, pause/resume, thread count, and "a book you open or add goes to
the front of the queue". The queue survives a restart.

**Evidence.** `tools.js` `scanDrawer`; `ocrCtl`, `pollStatus`.

**Girsa.** Girsa runs OCR in the background but the reader has little control or
visibility. A queue panel with progress, pause, priority and rate is a direct
quality-of-life take; it also makes a partial index honest.

**Verdict — TAKE · M · P2.** A scan/OCR panel with the site's five facts and two
controls; do not add thread pinning unless the engine exposes it.

### G10 · Adding a book

**Site.** Add a PDF/text with title, author, category, contact; a duplicate check
by title+size; online, the upload becomes a request the manager approves. A book
can be hidden rather than removed; removal is manager-only.

**Evidence.** `site.js` `siteAddBook`, `siteRequests`; `tools.js`.

**Girsa.** Girsa has import and user-owned material; the takes are the duplicate
warning, the metadata form and "hide vs remove". The approval workflow is a
consequence of a central server and does not apply.

**Verdict — TAKE (dedupe/form/hide) · S · P2.** Reuse the import path; no admin
queue.

### G11 · Feedback with the page attached

**Site.** "Report a bug / suggest" carries the current book and page, accepts
pasted screenshots, and the manager can reply by email; "report a problem on this
page" is free and pre-fills the page.

**Evidence.** `site.js` `feedbackDialog`, `fbAdd`, `siteFeedbackList`; `marks.js`.

**Girsa.** Girsa's trouble codes are better for diagnosis; a page-context report
is better for the reader. Combine: a "report" action that attaches the ref,
the app version and the trouble code, and writes a local file the reader can
send. No server, no account.

**Verdict — TAKE · S · P2.** A local report bundle with context; no telemetry.

### G12 · Print, share and copy

**Site.** Print current page or a range, with header, marks and highlights;
PDF printing is capped per job, text/typeset unlimited. Share a book; copy page
text; copy citation.

**Evidence.** `viewer.js` `printDialog`, `PRINT_LIMIT`; `marks.js`.

**Girsa.** Girsa prints a section and exports; copy citation exists. The site's
*range* dialog and *marks/highlights included* options are worth matching in the
print sheet. Sharing a book by link is a web feature; Girsa's `girsa://` citation
is the better local equivalent.

**Verdict — TAKE (print range/options) · S · P3.** Add range + include marks to
the print dialog; skip share links.

---

## H. The study cycle

### H1 · Eight daily tracks, from a published calendar

**Site.** The home "daily learning" card shows eight tracks (Daf Yomi Bavli,
Mishnah Yomit, Yerushalmi, Nach Yomi, Rambam, Halacha Yomit, and others) from the
Bikurei Yosef calendar; a click opens the day's sefer at the right page. If the
calendar's edition is shorter than the library's, it opens the last daf and says
so.

**Evidence.** `home.js` `royLearning`; About page.

**Girsa.** The luach deliberately has two tracks (Daf Yomi, Mishnah Yomis) and a
written ruling on why Rambam/Amud/Yerushalmi are not guessed. The site is a
counter-example: it takes a *published* calendar for all eight. The take is not
"add eight"; it is "if a published calendar is available and licensed, add the
track with its source named". Do not derive a calendar.

**Verdict — DECIDE.** Add Nach Yomi / Halacha Yomit only with a published,
licensed calendar; keep the ruling for the three that are formulas in disguise.

### H2 · Zmanim by city

**Site.** Zmanim for a chosen city (Zemaneh Yosef): dawn, sunrise, latest Shema,
midday, afternoon, sunset, nightfall, with the weekly Shabbat times.

**Evidence.** `home.js` `zmanim`, `zmanimFullHtml`; screenshot `home`.

**Girsa.** The luach explicitly refuses to ask for a location and turns the day
over at a settable hour, with the reason written down. That is the more honest
choice for a local app that does not know where it is. Do not copy the city
assumption; if zmanim are added, make the city a setting the reader types, and
say that the times are the calendar's, not computed from a location.

**Verdict — NO (as a default) / DECIDE (as an explicit setting).** The current
ruling stands unless the owner reopens it.

### H3 · The Hebrew calendar and date converter

**Site.** A Hebrew calendar, a converter both ways, yahrzeit, aveilut, bar/bat
mitzvah, and Sephardi/Ashkenazi seasons. It is a tool, not a sefer.

**Evidence.** `tools.js` `calendarDialog`, `datesDialog`.

**Girsa.** The luach computes Hebrew dates; the *converter* and the lifecycle
calculations are absent. They are arithmetic over a calendar, no corpus, and
they fit the "luach is the shell's" boundary.

**Verdict — TAKE · M · P3.** Date converter + yahrzeit/aveilut/bar-mitzvah, with
the same no-location rule.

### H4 · Parashat hashavua, opened at the first verse

**Site.** A parasha card names the coming Shabbat's parasha; a click opens the
Chumash (typeset, with Rashi) at the parasha's first verse, falling back to the
text book.

**Evidence.** `home.js` `openWeekParasha`.

**Girsa.** A parasha card on the home pane (B1) is a small, high-value addition:
the weekly rhythm is why many readers open a library at all. It needs a parasha
table and a verse→segment resolver, both already in the luach/ref layer.

**Verdict — TAKE · S · P2.** The card + first-verse landing; no new calendar.

### H5 · Siddur, tikkun koreh, shenayim mikra

**Site.** A siddur, a tikkun koreh, and shenayim mikra (the weekly reading with
Onkelos), each as a book/tool.

**Evidence.** Home tiles; About page.

**Girsa.** These are corpus/edition projects, not UI features. A siddur is a
different reading model (tefillah, not a sefer with daf). Out of scope unless
the owner wants a liturgical lane; do not fake it with a text file.

**Verdict — NO (for now).** Note it as a corpus question, not a feature ticket.

### H6 · Saved study environments

**Site.** "סביבות לימוד" saves a set of open books/panes under a name.

**Evidence.** Home tile; About page.

**Girsa.** Desks already save tabs, panes, widths and follow relationships — the
stronger version. No action.

**Verdict — ALREADY.** Desks are ahead.

### H7 · Daf Yomi's nearest-page honesty

**Site.** Opening the day's daf looks up the page by OCR anchor; if the exact daf
is not recognised, it interpolates between neighbours or opens the nearest page
and *says* it did. If the tractate is absent, it falls back to Sefaria.

**Evidence.** `home.js` `findDafPage`, `openDaf`; toast text.

**Girsa.** The luach names the daf; the reader opens it. The take is the *honest
miss*: "the nearest page" rather than a silent wrong page, and a text fallback
when the scan is absent.

**Verdict — TAKE · S · P2.** Apply the same sentence to any page-mapping miss.

---

## I. Trust, correction and provenance

### I1 · Proof levels are first-class

**Site.** Three states per page: not proofed, level 1 (the marked/suspicious words
were checked), level 2 (the whole page was read against the image). A book's rows
show ✓ (all pages level 1), ✓✓ (all level 2), or the percentage; a progress bar
per book. The library list carries the badge.

**Evidence.** `proof.js` `PROOF_LV`, `proofBadge`, `PROOF_INTRO`;
`/api/proof/levels`.

**Girsa.** Girsa has OCR vs embedded badges and a correction layer, but not a
page-level *status a reader can see* or a book-level aggregate. This is the
single best idea in the review: it turns "OCR is dirty" into "this page has been
checked, this one has not".

**Verdict — TAKE · M · P1.** A page proof status (none / checked-suspects /
read-against-image), a book aggregate, and a badge in the shelf, tab and search
row. Store it in the personal layer for the reader's own work and in the corpus
layer for shipped proofing, if any.

### I2 · The proofing workplace

**Site.** Scan on the right, OCR text on the left. Suspicious words are marked
(`⟦...⟧`); clicking a mark clears it after checking; selecting text highlights
the corresponding area on the image; every change is drafted locally; "save and
continue later"; level 1 / level 2 buttons; pending corrections shown; staff
approve, trusted correctors' fixes go in immediately.

**Evidence.** `proof.js` `proofWindow`, `page`; screenshot `proof` (not captured
in the public run; the window is account-gated).

**Girsa.** `suspects.ts` and `girsa-fix` already detect and apply; what is missing
is the *workplace*: side-by-side, click-to-clear, draft, status. Girsa's overlay
model is stronger (base text never mutated), so this is a UI over the existing
engine, not a new correction model.

**Verdict — TAKE · L · P1.** A proof pane that uses the existing suspects/fix
commands, with the site's four verbs (check, clear, draft, status). No approval
workflow; the reader owns the layer.

### I3 · The OCR error rate is stated

**Site.** The proof intro says the computer is wrong "about one word in twenty"
(5%). The accessibility statement says scans are images with OCR errors. The
search badges mark OCR hits.

**Evidence.** `PROOF_INTRO`; accessibility page; `search.ts` badge.

**Girsa.** Girsa measures degradation (29.4% at worst) and badges OCR, but does
not tell the reader a rough error rate. A single sentence, with the source of the
number, is honest and cheap.

**Verdict — TAKE · S · P2.** State the measured rate (and its conditions) in the
proof panel; never present it as a guarantee.

### I4 · Corrections capture context

**Site.** A typo fix sends the old text, the new text, and 12 characters before
and after; a page report carries the page. The correction is applied inline on
success. "Report a problem" is free even for a guest.

**Evidence.** `site.js` `siteTypoCtx`, `siteFixTypo`; `marks.js` report.

**Girsa.** The correction overlay stores a span and provenance, which is better;
the take is the *context capture* for a report and the inline confirmation. Do
not add a server.

**Verdict — TAKE · S · P2.** A report bundle with ref + context; inline
confirmation after a fix.

### I5 · A book's sources and credits

**Site.** `sfBookInfo` shows the Sefaria book's author, source and licence; the
typeset page's credit line names every commentary's edition; the last page is the
colophon.

**Evidence.** `sefaria.js` `sfBookInfo`; `flow_gen.html`; About page.

**Girsa.** Metadata exists; the reader-facing panel does not. This overlaps A4/A5;
do it once, in one panel, generated from the record.

**Verdict — TAKE · S · P1.** See A4/A5.

### I6 · The terms are a product surface

**Site.** The terms say: free for personal study; software closed, all rights
reserved; no bulk download; books under their source licences with credit on the
last page; not pesak; OCR errors and AS IS; submissions declare public domain or
permission; rights holders can request removal; accounts, one-year deletion,
din Torah jurisdiction. The privacy page says guests keep settings in the browser,
registered users store only username/email/password plus their layer, no ads, one
cookie.

**Evidence.** Terms and privacy pages.

**Girsa.** Girsa is open source and local; its equivalent is a *provenance and
licence* page per sefer, not a site-wide terms page. But the site's clarity is a
model: state what the library is, what it is not, where the texts come from, and
how to report a rights problem.

**Verdict — TAKE (the clarity, not the text) · S · P3.** A short "where this
comes from" page and a per-sefer licence record; no legal boilerplate.

### I7 · "Not a posek" is said twice

**Site.** The question mode carries a note: the tool does not pasken, shows
mekorot only, and the ruling is a rabbi's. The terms repeat it. The search still
works as words.

**Evidence.** `ASK_NOTE`; terms.

**Girsa.** Spec §14 already forbids the paskening machine and the semantic lane
says when it is asked a question. The site's wording is a good model for the
*semantic lane* and for any future question helper: name the limit before the
results.

**Verdict — ALREADY (policy) / TAKE (wording) · S · P3.** Use the site's sentence
shape in the lane header and docs.

### I8 · Scanned vs recognised is badged

**Site.** A search hit from a scan carries an OCR badge; the proof status is
separate. A reader knows both *what kind of text* and *how checked* it is.

**Evidence.** `search.js` badge; `proofBadge`.

**Girsa.** Girsa badges OCR vs embedded; adding the proof status (I1) completes
the pair. Do not collapse them into one badge: "OCR" and "checked" are different
claims.

**Verdict — TAKE · S · P2.** Two independent badges, with hover explanations.

### I9 · The site has no link correction

**Site.** No UI to fix a wrong link, retype it, or undo it; the graph is shipped.

**Evidence.** Links pane has open only.

**Girsa.** Link drawing/retyping/undo/repair is a Girsa advantage. Do not import
the site's one-way model.

**Verdict — ALREADY.** Guard it.

---

## J. Performance and architecture

### J1 · The forced splash

**Site.** The opening screen is held for **at least 4 seconds** (6 s maximum),
even if the library is ready; it is a branding animation.

**Evidence.** `index.html` `hideSplash`, `Math.max(0, 4000 - …)`.

**Girsa.** A local app that makes the reader wait four seconds to look at a logo
is stealing the one resource the reader chose it for. Girsa should open to the
shelf as fast as the disk allows; no splash, no artificial delay. If a first-run
animation is wanted, it must be dismissible and never block.

**Verdict — NO.** Record it as an anti-pattern in the performance notes.

### J2 · The search budget

**Site.** Server 70–485 ms, end-to-end 0.55–0.78 s, 8–192 KB for five queries;
40 sources for a question in 627 ms; abortable; 600-book cap; 12 lazy snippets.

**Evidence.** Table above; `search.js`.

**Girsa.** A local index should beat this by an order of magnitude; the useful
target is not the latency but the *shape*: counts first, bodies lazy, a visible
cap, an abort. Girsa already has the last two. Add "by sefer" (F1) without
turning every search into a full payload.

**Verdict — TAKE (shape) · S · P2.** Keep payloads proportional; no server.

### J3 · The page-image budget

**Site.** 137 KB for a 1000 px JPEG, 0.78 s; width is DPR-aware and quantised;
aspect ratio cached per book; other tabs' current pages prefetched one at a time.

**Evidence.** Measured; `viewer.js`.

**Girsa.** Local scans avoid the network, but the *render* budget is the same:
do not decode a 4000 px scan for a 700 px column, cache aspect, prefetch the next
page. These are measurable and worth a benchmark.

**Verdict — TAKE · S · P2.** Quantised render width + aspect cache + prefetch;
measure on a 500-page scan.

### J4 · Catalogue payloads

**Site.** The PDF catalogue is 136 KB compressed; the typeset catalogue is 107 KB
compressed but 1.07 MB raw for 2,743 rows; the Sefaria catalogue is 3.0 MB raw
for 7,597. The site loads the big ones whole and keeps them in memory.

**Girsa.** Girsa's catalogue is local and can be binary/streamed; the take is the
*thin row*: id, title, category, kind, counts, status. Do not ship descriptions
in the catalogue; fetch them from the work record.

**Verdict — ALREADY (local) / TAKE (thin rows) · S · P3.** Keep the catalogue row
small; the detail panel (A4) can ask for the rest.

### J5 · Script and font weight, and no offline

**Site.** 26 scripts + 15 stylesheets, 382 KB compressed, 33 font faces, no
service worker; the app is online-only even though much of the content is static.

**Evidence.** `index.html`; asset sum; no `serviceWorker` registration.

**Girsa.** A bundled desktop app has none of this problem; the take is the font
policy: open-licence families with a local fallback (the site uses Alef,
Assistant, Bellefair, David Libre, Frank Ruhl Libre, Heebo, Noto Rashi/Serif,
Suez, Taviraj), not a vendored commercial face. Girsa's `glyphs.ts` already
argues this; keep it.

**Verdict — ALREADY (bundled) / TAKE (font stack) · S · P3.** Verify the shipped
font licences and the Rashi fallback chain.

### J6 · Server-rendered pages and landing pages

**Site.** Typeset pages are full HTML with inline data and credits; `/book?id=`
and `sf_land.html` generate SEO landing pages with schema.org `Book`, canonical,
Open Graph, breadcrumbs and an excerpt, then deep-link to `/#book=…`.

**Evidence.** `flow_gen.html`; `land.html`; `sf_land.html`.

**Girsa.** Not a web app (spec §14), so the SEO half is out. The *deep link* half
is already `girsa://`; the take is the landing page's discipline: a canonical
identity, a citation, a credit, and a way in.

**Verdict — NO (web) / ALREADY (deep links).** Do not build a web surface.

### J7 · The OCR queue is resumable

**Site.** OCR runs in the background, saves as it goes, resumes from the same
point, can be paused, and a book opened/added goes to the front. Rate and thread
count are visible.

**Evidence.** `tools.js` `scanDrawer`; `ocrCtl`.

**Girsa.** Spec §6.3 already requires background, resumable, non-blocking OCR.
The take is the *visibility and priority* (G9), not the engine.

**Verdict — ALREADY (engine) / TAKE (controls) · S · P2.** See G9.

### J8 · Stable ids and a redirect map

**Site.** Books moved between categories get new ids; `data/idmap.json` maps old
to new, and proofreading keeps the old id so a page's work is not lost. Search
results and links use the permanent id.

**Evidence.** `proof.js` `IDMAP`; `data/idmap.json`; `api.ts` redirects.

**Girsa.** Permanent segment ids and a redirect table are core; the site's lesson
is to apply the same rule to *works* when a sefer is renamed/moved, and to keep
the old id in every stored reference (marks, links, citations).

**Verdict — ALREADY.** Verify work-level renames use the redirect table; no new
work.

### J9 · No offline mode

**Site.** No service worker; a lost connection ends the library.

**Girsa.** Offline is the product. Do not copy anything here.

**Verdict — ALREADY / NO.** Offline is the moat.

---

## K. Accessibility, mobile and language

### K1 · The accessibility overlay

**Site.** A floating blue button opens a panel: larger text, high contrast,
grayscale, invert, highlight links, readable font, line spacing, stop animations,
big cursor, reset. The statement claims WCAG 2.0 AA / Israeli standard 5568, and
names the known limitation (scans are images).

**Evidence.** Accessibility page; screenshot `a11y`; `a11y.js`.

**Girsa.** An overlay is not a substitute for semantics; it can fight the OS and
the app's own theme. The take is the *set of settings* (contrast, spacing, stop
animations, readable font) implemented natively in the look settings, plus a
published statement. Do not ship a third-party overlay.

**Verdict — TAKE (settings/statement) · S · P3 / NO (overlay as the strategy).**

### K2 · Reading settings are per book

**Site.** Font, size, width and paper are per reader (and per book for the PDF
paper). The site remembers them across sessions.

**Girsa.** Look settings are global with per-book position; the site's per-book
font/width is a nice refinement for a commentary-heavy sefer vs a daf.

**Verdict — TAKE · S · P3.** Per-work look overrides, falling back to global.

### K3 · Mobile collapses the toolbar

**Site.** On a phone the book bar keeps only the name, place, back/forward and
letter size; everything else opens under "⚙ עוד". A floating bookmark button
appears; landscape gets a menu button; a page turns by a sideways swipe.

**Evidence.** `mobile.js`; `mobile.css`.

**Girsa.** Desktop-first; the *pattern* (progressive disclosure of the toolbar
under one "more" control) is useful for a narrow window, not a mobile port.

**Verdict — TAKE (narrow-window disclosure) · S · P3.** Hide advanced controls
under one button below a width breakpoint; do not port the app.

### K4 · The interface is fully bilingual

**Site.** A language switch changes every label (Hebrew/English), book titles get
English names, and the layout flips; the content stays Hebrew. English labels
exist for the search, tools, settings and errors.

**Evidence.** `i18n.js`; `en_home`, `en_lib` screenshots.

**Girsa.** `say.ts` already speaks both languages and `names.ts` handles English
titles. No action beyond keeping new strings in `say.ts` (the repo's rule).

**Verdict — ALREADY.** Do not add a second string table.

### K5 · Mixed direction is deliberate

**Site.** English book titles, emails and numbers are wrapped in `dir="ltr"` or
isolated; Hebrew labels stay RTL; the page itself is `dir="rtl"`.

**Evidence.** `site.js` (`dir="ltr"` on emails), `auth.js`, CSS logical
properties.

**Girsa.** `names.ts` and the RTL layout already handle this; the take is the
*test*: add a mixed Hebrew/English title case to the window tests if one is
missing.

**Verdict — ALREADY / TAKE (test) · S · P3.** A regression test for mixed titles.

### K6 · Reduced motion and focus

**Site.** The overlay can stop animations; modals trap focus (mostly), Esc
closes, and the search results container is focusable with an ARIA label. The
splash still animates for four seconds.

**Evidence.** `a11y.js`; `search.js` (`results.tabIndex`, `aria-label`).

**Girsa.** `controls.ts` already handles dialogs/focus and live regions; verify
`prefers-reduced-motion` is honored and that every new panel has a focus trap and
an Esc path.

**Verdict — ALREADY (mostly) / TAKE (audit) · S · P3.** An accessibility pass
over the new home/proof panels.

---

## What not to copy

| | The thing | Why not |
|---|---|---|
| L1 | A typeset page for text seforim | Spec §6.2: a typesetting project, not a layout problem; the scan is the daf (§6.3) |
| L2 | Translation / bilingual reading | Spec §14: not a translation project; ship what exists |
| L3 | A question-answering mode | Spec §14: not a paskening machine; the semantic lane's own measurement says questions are its weak case |
| L4 | Login-gated reading tools | Print, copy, highlight and bookmark are not a perk of registering |
| L5 | Server accounts and sync | Local-first is the product; the personal layer is a folder the reader owns |
| L6 | Bulk download of the library | The terms forbid it; the texts are upstream under open licences; the link graph is their work |
| L7 | The four-second splash | It steals the one thing a local app is for |
| L8 | SEO landing pages from raw OCR | The scan page's meta description is OCR garbage ("א . . ו א…") |
| L9 | A search index built on first query | The first reader pays minutes and gets a partial answer |
| L10 | A fixed city for zmanim | Girsa deliberately does not ask where the reader is; the luach's hour is the honest approximation |
| L11 | An accessibility overlay as the strategy | Settings, not a widget; semantics, not a filter |
| L12 | Centralised curation / manager-only removal | A local shelf belongs to its reader |

---

## Coverage against the five asks

| The ask | Where it is answered |
|---|---|
| **1. Seforim** | A1–A8 (kinds, breadth, representations, metadata, credit, catalogue, series, status) |
| **2. Links between one sefer and another** | E1–E9, D2–D6 (graph, counts, classes, addresses, previews, back stack, Girsa's advantages) |
| **3. UI — picking, searching, opening, mefarshim** | B1–B8 (picking), C1–C12 (reading), D1–D7 (mefarshim), F1–F12 (search UI) |
| **4. Search efficiency** | F1–F12, J2, J3 (measured latency/payload, result shape, caps, scopes) |
| **5. Anything else** | G (personal/tools), H (study cycle), I (trust/correction), J (performance), K (accessibility/mobile/i18n), L (anti-patterns) |

---

## Decisions this review cannot make

These are not tickets until the owner rules on them. Each one either reopens a
documented non-goal or needs a licence/permission we do not have.

1. **A page facsimile for text seforim.** Spec §6.2 says no; the site shows what
   it would look like. If the owner wants it, argue it in §6.2 first.
2. **A question-shaped search.** Spec §14 says not a paskening machine; the
   site's sources-only mode is the boundary. Either keep the current rule or
   define a "sources for a question" helper that never produces a verdict.
3. **A translation lane.** Spec §14 says not a translation project; the site's
   he/en/both is a reader feature, not a corpus feature.
4. **Expanding the luach.** Only with a published, licensed calendar; do not
   derive Rambam/Amud/Yerushalmi.
5. **Ingesting the site's public-domain scans.** Only from the original library
   (NLI/IA) with a licence record; not from the site. This is A2's follow-up.
6. **A floating quick window.** A shell question, not a library one.
7. **A web/landing surface.** Spec §14 says not a web app; the deep link already
   exists.
8. **A per-book look override.** Cosmetic, but it is a settings-model change;
   decide whether look is global or per work.

---

## Outreach: the email, and what to ask

This is the constructive alternative to bulk download. It asks for the things
that are legitimate to ask for — catalogue metadata, upstream pointers, licence
clarity, and collaboration — and offers something in return. It does **not** ask
for the library, the software, or the scans.

**To:** `info@otzarhabina.com` · **Cc:** `support@otzarhabina.com`

**Subject (Hebrew):** פרויקט גרסא — פנייה על קטלוג, קישורים ורישיונות (לא הורדה גורפת)

### Hebrew

> שלום רב,
>
> אני עובד על **גרסא** — ספריית קודש פתוחה (MIT/Apache) לקריאה, חיפוש, קישור
> והוצאה לכתיבה, שרצה על המחשב בלי אינטרנט. אני פונה אליכם מפני שאני מעדיף
> לשאול מלהעתיק, ומפני שאוצר הבינה הוא הדוגמה הטובה שראיתי לשלושה דברים:
> העימוד שסביב הטקסט, הקישורים לפי שורה, ודירוג ההגהה (רמה 1 / רמה 2).
>
> אני מבין שהתוכנה סגורה, ושאין להוריד את מאגר הספרים כולו או חלקים גדולים
> ממנו באופן אוטומטי. אני לא מתכוון לעשות זאת. לכן אני מבקש לדעת מה **כן**
> אפשרי:
>
> 1. **קטלוג:** האם תוכלו למסור קובץ (CSV/JSON) של הקטלוג — כותרות, מחברים,
>    קטגוריות, סוג (עימוד/טקסט/PDF), רישיון ומהדורה — לשימוש פנימי להשוואת
>    כיסוי, עם קרדיט ברור?
> 2. **מקורות:** האם יש ספרים שאצלכם ואין להם מקור פתוח בחוץ? אם כן, האם
>    תפנו אותי למקור או לרישיון שממנו אפשר לקחת, במקום להוריד מכם?
> 3. **קישורים:** גרף הקישורים שלכם מבוסס על ספריא ועוד עבודה שלכם? אם הוא
>    פתוח, באיזה רישיון? אם לא — אשמח לדעת מה מותר לצטט או לקשר.
> 4. **הגהה:** האם נתוני הטקסט המתוקן (רמה 1/2) זמינים, והאם יש עניין בתרומת
>    תיקונים דו-כיוונית?
> 5. **שיתוף פעולה:** האם יש עניין במזהים יציבים, החלפת קישורים או קרדיט
>    הדדי? מה הצורה הנוחה לכם?
>
> מה אני יכול להציע: קרדיט וקישור אליכם בכל מקום שבו הקטלוג שלכם עזר; תיקוני
> OCR/קישורים אם תרצו; ולא להפיץ סריקות שאינן בנחלת הכלל. אם התשובה היא "לא"
> לאחד הסעיפים — זה בסדר, ואומר תודה על מה שכן.
>
> תודה על העבודה,
> [שם] · [קישור לפרויקט]

### English

> Hello,
>
> I work on **Girsa**, an open-source (MIT/Apache) Torah library for reading,
> searching, linking and citing, which runs on a computer without a network. I am
> writing because I would rather ask than copy, and because Otzar HaBina is the
> best example I have seen of three things: typesetting around the text,
> line-level links between seforim, and proof levels (level 1 / level 2).
>
> I understand the software is closed and that automated download of the whole
> library, or large parts of it, is not allowed. I am not going to do that. So I
> would like to ask what **is** possible:
>
> 1. **Catalogue:** could you share a CSV/JSON of the catalogue — titles, authors,
>    categories, kind (typeset/text/PDF), licence and edition — for internal
>    coverage comparison, with clear attribution?
> 2. **Sources:** are there books you have that have no open source elsewhere? If
>    so, would you point me to the source or licence we could take them from,
>    instead of downloading them from you?
> 3. **Links:** is your link graph based on Sefaria plus your own work? If it is
>    open, under what licence? If not, what may we quote or link to?
> 4. **Proofing:** is the corrected text (levels 1/2) available, and would you be
>    interested in a two-way correction contribution?
> 5. **Collaboration:** would stable identifiers, link exchange or mutual credit
>    be useful? What form suits you?
>
> What I can offer: clear credit and a link wherever your catalogue helped;
> OCR/link fixes if you want them; and no redistribution of scans that are not
> public domain. If the answer to any item is no, that is fine — thank you for
> the ones that are yes.
>
> Thank you for the work,
> [name] · [project link]

### The asks, as a table the reply can fill in

| # | Ask | Yes / No / Partial | Terms |
|---|---|---|---|
| 1 | Catalogue export (titles, authors, categories, kind, licence, edition) | | |
| 2 | Upstream pointer for works with no open source | | |
| 3 | Link graph: provenance and licence; permission to link/quote | | |
| 4 | Corrected text / proof data; two-way corrections | | |
| 5 | Stable ids / link exchange / mutual credit | | |
| 6 | Permission for a one-off catalog fetch, at a polite rate, if export is not possible | | |
| 7 | Preferred attribution wording and contact for rights questions | | |

**Do not send this without the owner's approval.** The repository has no
authority to speak for the project; this is a draft for the owner to send from
their own address.

---

## Appendix: surfaces and endpoints observed

**Public pages:** `/` (home), `/#book=<id>` (deep link), `/book?id=<id>`,
`/pages/about.html`, `/pages/terms.html`, `/pages/privacy.html`,
`/pages/accessibility.html`, `/flow/<path>.html` (typeset pages), `land.html` /
`sf_land.html` (landing pages).

**APIs:** `/api/books`, `/api/search2`, `/api/snips`, `/api/ask`, `/api/page`,
`/api/region`, `/api/pagetext`, `/api/dafpage`, `/api/meta`, `/api/rename`,
`/api/site/shared`, `/api/site/request`, `/api/site/requests`,
`/api/site/decide`, `/api/site/feedback`, `/api/proof/levels`, `/api/proof/books`,
`/api/proof/page`, `/api/auth/*`.

**Search parameters:** `q`, `phrase`, `prefixes`, `ordered`, `dist`, `sub`,
`book`, `cats`, `ids`.

**Search response:** `{ books: [{ b, occ, p, k }], total, ms, state, sub }`.
**Question response:** `{ terms, sources: [{ b, p, k, snip }], total, ms }`.
**Page response:** JPEG, width-parameterised.
**Catalogue rows:** `{ id, t, c, f, o, tx, txc, lx }`, plus `a` (author) and `v`
(volumes) on some rows.

**Not tested:** registration/login and the account-gated proofing workplace (no
account was created); a real print dialog or printer; sending feedback; a real
mobile device (desktop emulation only); Safari/Firefox; a load test beyond five
queries; the downloadable Excel catalogue (linked, not fetched).

---

## One last thing

The site's strongest idea is not the page, the links or the search. It is that
**the reader can see how much to trust the text** — a ✓, a ✓✓, a percentage, an
OCR badge, a credit line, a licence on the last page. Girsa already has the
machinery to know all of that. What it does not have is a way to *say* it. That
is the take I would do first, and it is the one that makes every other take
honest.
