# The second sitting

*An hour with the running window, by somebody who wants to learn from it.*

The first reader opened Girsa, used it for five minutes, wrote down eighteen
things and said it looked like a toy. Those eighteen were answered, and
`the-five-minute-report.md` is the answer. This is what happened when the same
kind of reader came back and sat with it for an hour instead of five minutes.

**The headline: it is no longer a toy. It is a serious engine wearing a coat of
paint that comes off wherever you touch it.** Nothing I found this time is in
the corpus, the search, the resolver or the speed — every one of those is better
than what it is replacing. Everything I found is in the last inch: the place
where a correct answer has to become something a person can see, read and trust.

Three of the findings are severe enough that a reader would put the application
down inside ten minutes, and one of them means **the feature the whole
tick-a-mefaresh model rests on has never once been visible on a screen**, in any
build, since the day it was written.

**Part 6 was added after a second pass over the paths the first pass had not
touched** — a cold profile, a missing corpus, a long sitting, small screens — and
the first thing it found was larger than anything above it: **no build of this
application had ever been produced.** Every measurement in Parts 2 to 5, and
every fact anybody in this repository knows about how the window behaves, was
taken from a development server. The release binary was built, the worst finding
was re-measured on it and is identical to the pixel, and everything from finding
16 onwards is the real thing.

---

## The grade

| | |
|---|---|
| **Overall** | **C+** — up from an F, and a long way from shippable |
| **Getting it at all** | **F** — no build of this application has ever been produced. See finding 16 |

The overall grade is for the product and it does not move for Part 6: nothing
found in the second round is a new kind of defect, and finding 1 was re-measured
on a real release binary and is identical to the pixel. The second row is a
separate fact and deserves its own line — a reader cannot obtain this
application, and until Part 6 nobody had noticed, because the way you find out
is to run a command nobody in this repository had run.

| Area | Grade | Why |
|---|---|---|
| Reading a sefer | A− | The daf is beautiful. Fonts, nikud, spacing, dark theme, three columns |
| Speed | A | 11 ms to open an 18,000-segment sefer, 8–90 ms searches, 12 ms shelf tree |
| The search engine | A− | Fast, honest, badges its guesses, facets with counts, real scope control |
| The search *window* | C− | Opens on an English error message; chips are English; a zero is a blank screen |
| Mefarshim — the flagship | D | The data is right and the display is invisible. Also jumps to the wrong perek |
| Browsing the shelf | B− | Works are in true order at last; the folders are still sorted by size |
| Settings | B | Real, complete, rebindable. Two traps in it |
| Two languages | D | Every switch leaves the window half-translated until you restart |
| Keyboard | C− | Every shortcut dies the moment a search or the shelf is docked |
| Finish | C− | Raw slugs, raw ids, browser dialogs, an unlabelled black void |
| Does it lie to you? | C | Not on purpose. But a column can say *scrolling with Bereishis* while sitting fifteen chapters away |

A C+ is not an insult. Reading, speed, search and the Ksav loop are the hard
half and they are done. What is missing is the half that a reader actually
touches, and it is missing in a way that is cheap to fix — most of what follows
is one line of CSS, one moved statement, or one extra condition.

---

## Part 1 · The eighteen, re-checked

I checked every one of the original complaints against the running window rather
than against the report.

| # | The complaint | Now |
|---|---|---|
| 1 | Seforim sorted by name, not true order | **Half.** Bereishis→Devarim and Yehoshua→Malachi are right. The *folders* are still sorted by size — see finding 6 |
| 2 | Gap on one side, shelf over the text | **Fixed.** Docks on the reading's leading edge, minimises to a strip, the strip reopens it |
| 3 | No way to put the interface in English | **Half, and broken in a new way** — see finding 2 |
| 4 | הגדרות opens nothing; the size buttons do nothing | **Fixed.** Real panel; 90% → 140% moves the text from 17.1 px to 26.6 px |
| 5 | Nikud toggle backwards, no middle setting | **Fixed.** Three settings, labelled with the one you get. The button beside it still uses the opposite convention — finding 12 |
| 6 | Midrash Lekach Tov in a category of its own | **Half.** It no longer lands in a `מפרשים` folder, but it still floats alone above every heading in the mefarshim list, and untranslated Sefaria categories produce the same confusion — finding 7 |
| 7 | Bereishis counted as a peirush on Onkelos | **Fixed, well.** Opening Onkelos now heads the list *הספר שעליו נכתב* and calls Bereishis *הספר עצמו* |
| 8 | Several mefarshim at once; link/unlink scroll | **Fixed.** `＋` picks several, one button opens them all, each column carries its own scroll link |
| 9 | Ticking a mefaresh does not arm the line click | **Fixed in the wiring, dead on the screen** — finding 1 |
| 10 | Nobody knows what `עוקב` does | **Fixed.** `גלילה משותפת` / `גלילה נפרדת`, and it names the column it will tie you to |
| 11 | Links panel unclear and hard on the eyes | **Better.** Direction in words, lenses, provenance. Title still in the wrong language |
| 12 | Export and send cannot choose a folder | **Fixed.** A real Windows folder dialog, titled `בחר תיקייה`. Cancel writes nothing |
| 13 | כתוב does nothing | **Fixed** — the drawer opens. What it opens into is finding 10 |
| 14–15 | Opening a sefer; tabs and windows | **Fixed.** Tabs are arrangements, the picker leads with what is open, opening an open sefer goes to it |
| 16 | Search opens on nothing, then on the last thing | **Fixed.** It shows the previous results and says they are previous |
| 17 | No way to add/subtract from the search; the tree flashes | **Fixed.** A real scope panel: whole tree, `+`/`−` per row, counts |
| 18 | `within 5 words` not customisable | **Fixed.** 2, 3, 5, 10, 20 |

Twelve fixed, four half-fixed, two fixed in the data and broken in the display.
That is real work and it shows.

---

## Part 2 · What I found this time

Ordered by how quickly a reader meets it and how badly it hurts.

### 1 · Tick a mefaresh, click a line, and nothing appears

**This is the most serious thing in the audit.** The whole Otzaria-model
interaction — tick Rashi, click a line, read what he says under it — puts the
right text in the document and renders it invisible.

`pane.ts:299` builds each commentary block as `el("div", "said")`. The class
`.said` was already taken: `styles.css:1768` is the toast at the foot of the
window — `position: fixed; bottom: 14px; opacity: 0`, waiting for `.is-on`.
Both rules apply, the later one wins, and so every comment is a fixed,
fully transparent box stacked at the bottom of the window.

Measured, live, on Rashi on Shabbos 31a with Rashi ticked:

```
box height        16 px          (the container, with nothing in flow)
blocks inside     3              position: fixed · opacity: 0 · bottom: 14px
text inside       "רש"י על שבת 31a:1:1 — שהמרו - נתערבו כמו שממרין את היונים…"
```

What the reader sees is an empty grey strip 16 px tall under the line.

The bitter part: the *failure* case works. When no ticked mefaresh speaks on a
line, the box holds a `<p class="said-none">` — a different class — and the
sentence is visible. **A line with nothing to say says so; a line with Rashi on
it shows a blank bar.**

`.line-said` and `el("div","said")` both arrived in `83c14c3` (W43, 31 July);
the toast rule was already at line 1330 of the stylesheet on that day. So this
has never worked, through two audits and the pass that answered the eighteen.
Nobody ticked a mefaresh and looked.

*Fix:* rename the block class. One line.

### 2 · Every language switch leaves the window half-translated — **fixed**

`say.ts:542` initialises the interface language from **localStorage**, written
by `speakInterface()` — which runs inside `main()`, after the module-level
`new ShelfView()`, `new SearchView()`, `new LinksView()`… have already built
their titles, buttons and placeholders. The cache is therefore always one switch
behind, in both directions.

Measured immediately after switching to English:

```
toolbar     nikud, no te'amim · with variants · A− · A+ · Settings
shelf       המדף · מדף חדש · החזר לסדר המקורי · צמצם · סגור
```

…and immediately after switching back to Hebrew:

```
toolbar     ניקוד בלי טעמים · עם גרסאות · א− · א+ · הגדרות
shelf       The shelf · New shelf · Back to how it shipped · Minimise · Close
search      Keep · Minimise · Close · placeholder "search the whole shelf…"
links       Links
settings    title "Settings" over Hebrew rows
```

It settles only when the application is restarted. A reader who tries English
once and goes back gets a permanently mixed window for the rest of that session —
which looks less like a setting and more like a broken build.

The reload that `settingsview.ts` performs for exactly this reason does not help,
because the cache is read before the truth arrives and written after.

*Fix:* have the panels ask for their strings when they draw rather than when
they are constructed, or construct them after `connect()`. Not one line, but not
large.

**Done, and neither of those was what it wanted.** The reload was never the
problem — it is the right mechanism, and `settingsview.ts` argues for it well: a
`retitle()` on eleven panels is a second list per panel and a twelfth panel
nobody adds one to, and the session lives in Rust so a reload costs the reader
nothing. What was missing is that **the write and the reload were two statements
in two files with nothing making them agree**, which is the pattern Part 4 names
under all eighteen of the first complaints.

They are one function now — `switchInterfaceTo` in `say.ts`, the module that
owns the cache — and the settings panel hands it the language rather than only
the news that one changed. Two guards, both of the class rather than the shape:
what the window is saying and what the next load will be built from may never
disagree, and no module but `say.ts` may call `location.reload()`.

### 3 · The keyboard dies after every search — including the send to Ksav — **fixed**

`main.ts:1238-1239` gives the search and the shelf `keyboard: "all"`, which
`panel.ts:120` reads as *swallow every key that is not Escape or my own toggle,
wherever the caret is*. That is right for a panel over the reading. But W48 made
clicking a result **dock** the panel instead of closing it, and the shelf docks
the same way when you open a sefer from it — so the ordinary path leaves a panel
open beside the reading, and with it open nothing on the keyboard works.

Measured:

```
search docked (after clicking a result) → Ctrl+C → nothing. No toast, no message.
search closed                            → Ctrl+C → "הועתק — שבת דף ל: שורה ז'"
shelf docked  (after opening a sefer)    → Ctrl+C → nothing.
```

The same applies to Ctrl+Shift+C, Ctrl+N, Ctrl+D, Ctrl+L, Ctrl+K, Alt+N,
Ctrl+= and Ctrl+−. **The five-minute story in `start-here.md` is: search, click
the hit, highlight, Ctrl+Shift+C.** In the shipped build that Ctrl+Shift+C does
nothing at all, silently, and the docked panel is why.

*Fix:* a docked panel is not an overlay. Give `dock()` a keyboard mode of
`inside`, or drop to `reading` while `is-docked` is on.

**Done, and `inside` turned out not to be enough.** A docked panel is full of
buttons — every result is one — and clicking a result leaves the focus on it, so
`inside` would have handed that button the keyboard for the whole time the
reader spent reading what they clicked. The caret has three positions now
(`away` · `on` · `typing`) and there is a fourth keyboard mode, `typing`: the
panel owns what is typed into its own boxes and nothing else. `Held.keyboard`
may be a function, because the search and the bookcase are overlays until you go
*through* them and columns afterwards, and one constant cannot say that.

`yoursview` was the sibling: it docks the moment it opens, and it was on
`inside`. Cleared with it. `linksview`, `lanepanel` and `suspects` also dock and
were already `reading`, so they were never affected — checked, not assumed.

The guard is the class rather than the shape (lesson 2): `panel.test.mjs` sweeps
`src/` for the modules that call `dock()`, and asserts that no panel built from
one of them is registered `"all"` or `"inside"` in `PANELS`. The pre-fix tree
fails it on three panels.

### 4 · A commentary column can jump fifteen chapters and still say it is following — **fixed**

`beside.rs:306` places a follower by address, which is exact and right. When the
address has nothing — Rashi wrote nothing on Bereishis 12:12 — it falls through
to `by_edge`, which takes **any** edge joining the two works at that segment and
scrolls to `ids[0]`. Sefaria's graph includes *this Rashi elsewhere quotes that
pasuk*, so:

```
base at Bereishis 12:12  → Rashi column told to go to rashi-on-genesis/35:18:2
base at Bereishis 12:5   → Ramban column told to go to ramban-on-genesis/2:3:1
                           (the answer had three ids; the first one was wrong)
```

Live, after ordinary scrolling, I had Bereishis at 12:1, Rashi at 12:1:1 and the
Ramban at **27:41:1**, with the header saying *גלילה עם בראשית* under it.

The code comment defends the fallback: *"An edge is still a fact somebody
recorded, so it is used before giving up."* It is a fact; it is not this
reader's question. Landing a chavrusa's Ramban in Vayishlach while he is in Lech
Lecha is worse than landing nowhere — and *nowhere* is already implemented
(`Place::NoPlace`, which prints `אין כאן`).

*Fix:* only use an edge whose own address is near the leader's, or do not use it
at all for a declared commentary.

**Done, and it is a shape rather than a distance.** A distance would need the
two addresses to be in one vocabulary to mean anything, and half of these pairs
are not: the 978 Otzaria-only mefarshim are numbered `1..N` against a Gemara
addressed `2a:3`, and comparing those is comparing a comment number with a daf.

What is true of every declared commentary is that **its address extends its
base's** — `Rashi on Berakhot 2a:1:3` is the third comment on `Berakhot 2a:1`.
So the shorter address has to be a prefix of the longer one and the longer one
has to be the commentary's. `[12,12]` against `[35,18,2]` compares `[12,12]`
with `[35,18]` and they are not the same perek. And when the lengths run the
other way the two are not in one scheme at all, there is nothing to disagree
about, and the edge stands — which is what keeps the Otzaria works placeable.

The ids are filtered rather than the answer taken or dropped whole, because the
Ramban answer measured above had three of them and the wrong one was merely
first.

### 5 · A column in a three-way split has no name — **fixed**

At 1360 px with a Gemara and two mefarshim, measured:

```
pane 1 (בראשית)            title 42 px wide
pane 2 (רש"י על בראשית)    title  0 px wide     ← invisible
pane 3 (רמב"ן על בראשית)   title  6 px wide     ← one letter
                            note "אין כאן" 14 px wide
```

`.pane-title` has `min-width: 0; overflow: hidden; text-overflow: ellipsis`
(`styles.css:453`) and the five header buttons never shrink, so the sefer's name
is the first thing squeezed and it goes all the way to zero. An ellipsis on a
zero-width box shows nothing. In English the header overflows the other way and
clips the leftmost **button** to `se`.

So the app's signature arrangement — the daf with three commentaries — gives you
three columns you cannot tell apart, and the honest *nothing here* note is
clipped to a smudge. The `.pane-follows` rule three lines above has a comment
about this exact problem being fixed once already, for a different label.

*Fix:* the title is the last thing to shrink, not the first. Give the buttons an
overflow menu, or drop them to icons below a width.

**Done, and the owner's answer to *which gives way first* is neither.** The five
buttons are one box now — `toolStrip()`, used by both the reading pane and the
scan pane — and the box wraps to a second row when the header runs out of room.
The title keeps its width and a `7ch` floor, the address and the `אין כאן` note
stop shrinking at all, and no button is ever clipped in either language. It
costs a row of header in a narrow column, which is the cheapest thing on the
screen; an overflow menu would have cost a popup, a keyboard route and an
Escape.

`npm run eyes` measures the title, the note and the last button at 240px, 430px
and 1000px, and a second specimen in English — twelve assertions where there was
one assertion and a printed note.

### 6 · Shas is filed by folder size, so Berakhos is at the bottom under "Guides" — **fixed**

Complaint 1 was answered for *works* — `Work::order`, read from Sefaria, applied
through one comparator. **Shelves never got it.** `taxonomy.rs:211` sorts a
shelf's children by a hand-written rank table of **eight** names
(`girsa-corpus/src/taxonomy.rs:164`), then by **count descending**. The six
sedarim are not in the eight. What a reader sees under תלמוד → בבלי:

```
ראשונים 641 · אחרונים 717 · מחברי זמננו 125 · Commentary on Minor Tractates 48 ·
גמרא נוחה 36 · מסכתות קטנות 15 · סדר מועד 11 · סדר קדשים 9 · סדר נזיקין 8 ·
סדר נשים 7 · Guides 5 · סדר זרעים 1 · סדר טהרות 1
```

Zeraim — which is where ברכות lives, alone — is second from the bottom, below an
English folder called *Guides*. The sedarim are in size order, not
זרעים-מועד-נשים-נזיקין-קדשים-טהרות. Inside Seder Moed the masechtos are right,
which makes the folder order look even more like a mistake.

Related, same table: `TERM` translates **30** Sefaria categories and leaves the
rest in English, so a Hebrew bookcase carries *Commentary on Minor Tractates*,
*Guides*, *Chida* and *Mechokekei Yehudah* among its shelves. And `אחר` holds
two Otzaria README files — *הודעה חשובה* and *עריכת ספר באוצריא* — presented as
seforim.

*Fix:* shelves need an order the way works got one — from the corpus, not from a
list of eight.

**Done, and the same for the names.** Measured on the running tree, the same
shelf now reads:

```
סדר זרעים · סדר מועד · סדר נשים · סדר נזיקין · סדר קדשים · סדר טהרות ·
גמרא נוחה · מסכתות קטנות · מדריכים · ראשונים · אחרונים · מחברי זמננו ·
מפרשים על מסכתות קטנות
```

Four rules where there were two. The **sefer comes before the commentaries on
it** — the same rule `branch()` already applied to the loose seforim it gathers,
one level up, and without it the 641 rishonim inherit their base's order and
sort above the masechta they are written on. Then the **corpus's own order**:
the earliest `Work::order` beneath a shelf. Sefaria orders the masechtos in the
sequence they are learned — Berakhos `[1]`, Shabbos `[2]`, Yevamos `[14]` — so
זרעים-מועד-נשים-נזיקין-קדשים-טהרות falls out of the corpus without the six
names being typed anywhere. Then era, then size.

The English shelves are the same finding and needed a census, not a longer
table. The shipped catalogue has **533 distinct categories, 376 of them without
a Hebrew letter in them** — and almost none of the 376 are words. They are the
names of seforim and of the men who wrote them, and the corpus already knows
every one in Hebrew:

| | |
|---|---|
| named by the `X on Y` split — `מפרשים על מסכתות קטנות` | both halves translated |
| named by their own seforim's titles — `ברטנורא` off `ברטנורא על ברכות` | **234** |
| named by the one author they all carry — `חיים דוד אזולאי` for `Chida` | **38** |
| left for the term table, which grew from 30 to 80 | **50** |

Latin shelves left in the whole 7,189-work bookcase: **none**, asserted by
`every_shelf_in_the_bookcase_is_named_in_hebrew` against the real corpus. A
reader's own rename still wins over all of it.

And the two Otzaria README files stand on `אחר / על אוצריא` rather than among
the seforim nobody has filed yet.

### 7 · The search panel greets a Hebrew reader in English

Open Ctrl+F in a fully Hebrew window and the whole control surface is English,
because the chip labels are English string literals in Rust
(`girsa-search/src/chips.rs:186-252`) and the chip's *name* is also its API key,
so it cannot be translated without changing the protocol:

```
torat emet ▾   whole shelf ▾   the word ▾   anywhere in a segment ▾
nothing to search for
```

That last line is the header, in red, before you have typed anything — the panel
opens by telling you off. Then:

* the result header reads `the words מאימתי קורינ את שמע, anywhere in a segment`
  — English frame, and the query echoed back **with its final letters folded**
  (`קורינ`), which reads as a typo;
* the era facet's largest row is `no era recorded` (`facets.rs:523`);
* a search with no hits shows the header, a bare `0`, and an entirely blank
  panel — no sentence, no suggestion;
* `instruments` is offered as a search mode, which is opaque in either language.

### 8 · Typing a mareh makom searches for it instead of going there

The resolver is excellent and nearly unreachable. In the default mode:

```
"שבת לא."                    → 92,384 word hits
"ברכות ב."                   → 8,131 word hits
"משנה ברורה סימן ש"          → 12 word hits
```

With the `@` sigil — which nothing on screen teaches — every one of them lands
exactly:

```
@שבת לא.              → girsa:bavli/shabbat/31a
@משנה ברורה סימן ש    → girsa:mishnah-berurah/300
@רש"י על בראשית א:א   → girsa:rashi-on-genesis/1:1
```

There is no other *go to a place* control in the application, and no other way
to reach siman 300 of a 17,418-segment sefer than scrolling.

And when it lands, **what the reader is shown is the internal id**. The panel
prints, three times over:

```
girsa:bavli/shabbat/31a
girsa:bavli/shabbat/31a
```

The window already knows how to say this properly — Ctrl+C on the same line
produces `הועתק — שבת דף לא. שורה א'`. Two formatters; the reading surface got
the wrong one.

### 9 · The reading pane says `31a:1`, the citation says `שבת דף לא.`

Every line of Gemara is addressed in the margin in English daf notation —
`30b:11`, `31a:4` — inside a Hebrew window, next to Hebrew text, while
`girsa_cite` sitting one call away renders the same place as `שבת דף לא. שורה
א'`. `start-here.md` promises the reader a choice of citation style; the setting
exists in the session (`Session::cite`), a command exists to change it
(`set_cite_style`), `api.setCiteStyle` exists in the window — and **nothing
calls it**. A documented preference with no control anywhere.

### 10 · The writing drawer opens as a black void

Ctrl+E takes the bottom 342 px of the window. Measured, the textarea is
1360 × 306, `background: rgba(0,0,0,0)`, `border: none`, **no placeholder**. On
a dark theme that is a black rectangle with nothing in it — no frame, no caret
until you click, no hint that this is where you type. Above it sits a date field
and a grey absolute path.

Typing works. Nothing tells you that.

### 11 · Notes are taken in a browser dialog

Ctrl+N — *write a note on this line*, one of the eleven things on the shortcut
card — is `window.prompt` (`main.ts:1511`). In the shell that renders as the
webview's own modal, captioned:

> **localhost:5174 says**
> מה אתה אומר על השורה?
> \[OK\] \[Cancel\]

Naming a saved query does the same (`main.ts:1555`), as do *new shelf* and
*reset the shelf* (`shelf.ts:356, 367`). A packaged build says
`tauri.localhost says` instead, which is not better.

### 12 · Two buttons side by side, two opposite conventions

Complaint 5 was *the nikud button is labelled with the state I am already in*.
The nikud button was fixed and its neighbour was not:

```
nikud     shows "ניקוד בלי טעמים"  = the state you will get
showing   shows "עם גרסאות"        = the state you are in
```

Click *showing* and it becomes `מתוקן`, and the toast also says `מתוקן` — so the
same word means *what happened* in one place and *what will happen* in the
other, eight pixels apart. This is the exact sentence the report wrote about the
first one: *two buttons, two conventions, one toolbar.*

### 13 · Closing the settings leaves a shortcut trap armed

Click a shortcut's key button (it shows `…`), then close the panel with `×`
instead of pressing a key. The `keydown` listener on `window` stays. The next
key you press anywhere — a bare letter — is bound. Reproduced:

```
click "Ctrl+O" row → close panel with × → press "g"
→ open = G          (Ctrl+O no longer opens anything)
```

No confirmation, no message, and the only way back is `↺` in a panel you now
cannot open with its shortcut if you happened to rebind that one.

### 14 · The tab strip shows internal slugs after a restart

On startup the strip read:

```
genesis +2 | mishnah-berurah | שבת ×
```

`titleOf()` falls back to the slug and `named` is filled only when a pane is
drawn, so every tab except the active one is labelled with its English internal
id until you visit it. First thing on screen, every launch.

### 15 · Smaller things worth a line each

* **The mefarshim door promises 67 and lists 76.** The button counts declared
  commentaries; the list also carries nine works joined only by links, labelled
  `מפרש` where the declared ones say `פירוש` — two words a reader reads as
  synonyms carrying a distinction the reader cannot see.
* **A heading that looks empty.** `תנ״ך · 66` is a parent whose children are
  indented 14 px; it reads as a category with 66 seforim and nothing under it.
* **Rabbeinu Chananel appears twice** (`רבינו חננאל על בראשית`, `ר חננאל על
  בראשית`) — one from each corpus, undeduplicated, in the same list.
* **Ticking a targum marks every line.** 1,533 of Bereishis' 1,533; Rashi marks
  356 of 400 drawn lines of Shabbos. The `◆` was designed so that marking
  everything would say nothing, and for the most obvious mefarshim it marks
  everything.
* **The docked shelf squeezes the seforim to a 70 px column**, one word per
  line, with the era clipped to a single letter.
* **`cargo build --release -p girsa-shell` silently ships a stale frontend.**
  My first run showed a UI two commits old with buttons the source no longer
  has; the binary had not been relinked after `npm run build`. Only touching a
  Rust file forced the embed. Anyone testing a release build this way is testing
  something else.
* **Five calls wired into `api.ts` that no view ever makes**: `setCiteStyle`,
  `fixes` (the list of corrections you have made), `linkDraw`, `scanFix`,
  `yours`. The first is a preference `start-here.md` tells the reader they can
  choose; the rest are backend features with no door into them. (`linkify` and
  `who_cites` are absent from `api.ts` on purpose — they belong to Ksav's
  loopback, not to this window.)

---

## Part 3 · What is genuinely good

This deserves as much space as the faults, because the faults are all in the
last inch and this is the other ninety-nine.

**The daf.** Hebrew with nikud and te'amim at a comfortable measure, real
leading, a dark theme that does not glare, the address quiet in the margin. It
looks like something made by somebody who reads. Three columns side by side with
a Gemara and two mefarshim is exactly the picture people want and nobody else
ships offline.

**Speed, everywhere, measured against the real 7,189-work shelf:**

| | |
|---|---|
| open Mishnah Berurah, 17,418 segments | **11 ms**, 205 KB on the wire |
| a 300-line page at an edge | 6 ms |
| open Bereishis | 6 ms |
| the whole bookcase tree | 12 ms |
| mefarshim / companions for Berakhos | 3 ms / 2 ms |
| search, four real queries | 8, 63, 73, 90 ms |
| window on screen from a cold start | 0.2–1.0 s |

The 7.7 MB reading path really is 315 KB now. Scrolling a 17,000-segment sefer
holds ~17 ms frames with two spikes in twenty screens and no gaps or duplicates.

**The search results.** Hits highlighted inside the line with `<mark>`, the
sefer and address above each, facets with counts and a `−` on every row, a real
scope panel with the whole tree and `+`/`−`. This is better than the products it
is replacing.

**The citation resolver.** `@שבת לא.` → the daf, `@משנה ברורה סימן ש` → siman
300. It is the best thing in the repository and it is hidden behind a sigil.

**The Ksav loop works.** With Ksav running, selecting a passage and pressing
Ctrl+Shift+C produced `נשלח ל־כְּתָב — שבת דף לא. שורה א'` — the right words with
the right mareh makom, live, first try. That is the thing the two applications
exist for and it does what the documentation says.

**The settings panel** is complete and honest: theme, two font families, size,
leading, measure, three-state nikud, both languages, and every shortcut
rebindable by pressing the keys. **Escape closes every panel** — I tested all
eight. **Minimise and restore work.** **The folder dialog is real.** **The
correction box** is well made: it appears by the words, shows what is there,
names the two kinds, and tells you the keys.

**At 740 × 620** — the minimum window — nothing overflows and nothing breaks.

**231 window tests pass.** So does the typecheck. Which is the whole problem.

---

## Part 4 · The lessons

The first report named the pattern under its eighteen: *two things that had to
agree, and nothing that made them.* That pattern is still here — finding 2 is a
session in Rust and a cache in localStorage; finding 6 is Sefaria's order and a
list of eight names; finding 12 is two buttons and two conventions. But the
first report is not wrong and not the whole story any more, because the second
sitting turned up a different and larger one.

### Lesson 1 · Nothing in this project has eyes

Every guard in the repository reads **source**. `say.test.mjs` sweeps modules
for Hebrew literals. `panel.test.mjs` sweeps `main.ts` for panels missing from
the registry. `sources.test.mjs` fails the build over an unlabelled control.
`styles.test.mjs` — written after an invisible panel was shipped — checks that
every custom property the stylesheet reads is defined.

Not one of them asks *is it on the screen, and can it be read*. So:

* a comment block renders at `opacity: 0` — 231 tests pass;
* a pane title computes to 0 px wide — 231 tests pass;
* a panel's buttons come out in the other language — 231 tests pass;
* every reading shortcut stops working — 231 tests pass.

The four bugs found by ten minutes in a browser last time were not a lucky
accident, and the answer to them was not "open a browser once more". Four of
this audit's top five findings are single assertions in a headless window:
`getBoundingClientRect().width > 0` on a pane title, `getComputedStyle(comment)
.opacity === "1"`, a keypress after docking, a screenshot diff of the toolbar
after a language switch. **The gate needs a browser in it, not another source
sweep.**

### Lesson 2 · The bespoke guard fits the bug that was, not the bug that is

`styles.test.mjs` exists because an undefined custom property made a panel
invisible. It checks undefined custom properties. It does not check the class
that appears twice with contradictory meanings in the same file — which made a
different panel invisible, in the same way, in the same stylesheet, and was
already there when the test was written.

`panel.test.mjs` exists because two panels were missing from a keyboard table.
It checks that panels are in the table. It cannot see that two of the entries in
that table swallow the keyboard while docked.

Each fix hardens the shape of the failure instead of the *class*: **a thing the
reader was supposed to see and did not**. One assertion of the class is worth
five of the shapes.

### Lesson 3 · A comment defending a behaviour is not the same as having watched it

`beside.rs` explains why an edge is used when the address has nothing: *"An edge
is still a fact somebody recorded, so it is used before giving up."* The
sentence is reasonable. The behaviour it produces is a Ramban column in
Vayishlach while the reader is in Lech Lecha. Nobody scrolled a Chumash with a
Ramban beside it and watched.

This repository's comments are its best feature and its most dangerous one: they
are so well argued that they read as evidence. They are not evidence. The
running window is.

### Lesson 4 · Two of the eighteen came back wearing new clothes

Complaint 1 was answered for works and not for shelves, so the reader who
complained that seforim were in the wrong order will open Shas and find the
sedarim in the wrong order. Complaint 3 was answered with a second setting whose
first use leaves the window in two languages. Complaint 5 was answered on one
button while the button next to it kept the convention that was being fixed.

**A fix is not finished at the site of the complaint.** When the answer is *the
data already knows the order*, every list gets it. When the answer is *the
window has a language*, every string in it does. When the answer is *label the
state you will get*, every toolbar button does.

### Lesson 5 · The last inch is a feature, not a finish

Slugs in the tab strip, `girsa:bavli/shabbat/31a` in the results, `31a:1` in a
Hebrew margin, `localhost:5174 says` over a note box, a black rectangle for the
writing drawer, `torat emet` and `no era recorded` in a Hebrew panel — none of
these is a bug in the ordinary sense. Every one of them is the machine's own
name for something, shown to a person, because at that one point nobody asked
what the person should see.

Collect them and they are the entire distance between *this is a serious library*
and *this looks like a toy written by AI*. The first reader was not reviewing the
architecture. They were reading the screen.

---

## Part 5 · What I would fix first

In this order. The first four are a day's work between them and they change the
grade more than anything else on the list.

0. **Produce a build** (finding 16). `npx tauri build` — 58 seconds, works first
   try, and nobody had run it. Everything below is invisible until somebody can
   hold the application in their hands.
1. ~~**Rename the commentary block class** (finding 1).~~ **Done** — `pane.ts`
   builds `said-one` and `styles.css` styles it, guarded by
   `collision.test.mjs`. Two lines. It turned the flagship interaction from dead
   to working.
2. ~~**Let a docked panel give the keyboard back** (finding 3).~~ **Done** — a
   fourth keyboard mode, `typing`, and a caret with three positions instead of
   two. It restores every shortcut in the application, including the send to
   Ksav that the whole project is about.
3. ~~**Make the panels ask for their strings at draw time** (finding 2), and drop
   the localStorage cache or write it before the panels are built.~~ **Done** —
   the second of the two: the cache is written before the reload that rebuilds
   the panels, and the write and the reload are one function.
4. ~~**Stop the follower jumping on an unrelated edge** (finding 4) — an edge is
   only a place if its address is near the leader's.~~ **Done** — near turned
   out to be the wrong test; the right one is whether the commentary's address
   extends its base's.
5. ~~**Let the pane title win the header** (finding 5).~~ **Done** — the buttons
   are one box and the box wraps; nothing is squeezed and nothing is hidden.
6. ~~**Give shelves an order the way works got one** (finding 6), and translate
   the categories from the corpus rather than from a list of thirty.~~ **Done** —
   both. 272 of the 376 English categories are named by the seforim standing on
   them; none is left in the bookcase.
7. **Make a mareh makom the default reading of a query** (finding 8), and print
   places as places, never as ids (findings 8, 9, 14).
8. ~~**Put a browser in the gate.**~~ **Started** — `npm run eyes` is a headless
   Edge over the real stylesheet, and `collision.test.mjs` is in the default
   suite. Between them they hold findings 1 and 5. Findings 2, 3, 10 and 14 need
   the running window, which is the next step and is now possible, because there
   is a build.

---

## Part 6 · The paths I had not tested, and one I should have

Everything above was measured against a window that was already open, on this
machine, with this reader's profile: an existing session, an existing set of
preferences, a warm cache and the corpus where it has always been. Seven paths
were left untested and are named as such at the foot of this section's list. This
part is what happened when they were tested.

The first one moved the grade.

### 16 · Nobody had ever built the application

`cargo build --release -p girsa-shell` — the command this document's own appendix
recommended, and the command used for every measurement in Part 2 — does not
produce a window that contains Girsa. It produces a window that **navigates to
`http://localhost:5174`**, the Vite dev server, because `tauri-build` leaves
`cfg(dev)` set for any build that does not go through the Tauri CLI. With the dev
server up you cannot tell. With it down, the application is this:

```
url: "chrome-error://chromewebdata/"
title: "localhost"
```

A Chromium *this site can't be reached* page, in a window called
`גִּרְסָא · Girsa`.

The evidence that nobody had ever gone further:

* `target/release/bundle/` does not exist. No installer, no MSI, no NSIS, no
  portable exe has ever been produced by this repository.
* `README.md:367` gives one way to run the window — ``npm --prefix app run tauri
  dev`` — and no page anywhere says how to build one.
* The entire first sitting, and the first three hours of this one, were conducted
  against the dev server. The *"stale embedded frontend"* that cost twenty
  minutes and became finding 15 was not a stale embed. There was no embed.

`npm run tauri build --no-bundle` works, first try, in **58 seconds** — the
frontend transforms 41 modules in 1.4 s and only the shell crate recompiles. So
this is not a broken build. It is a build nobody had run, which is a different
and more interesting problem: every fact anybody knows about how this application
behaves was learned from a development server, including every fact in Part 2.

**Everything below was re-measured against the real thing** —
`http://tauri.localhost/`, frontend embedded, strict CSP, no dev server anywhere.

### 17 · What the real build changed, and what it did not

Finding 1 survives exactly, to the pixel. On the release binary, Rashi on Berakhos
2a:8 — real text, `אקרא קאי – ושם למד חובת הקריאה:` — renders as:

| | |
|---|---|
| the container | **16px** tall |
| the comment inside it | 66 × 242, `position: fixed`, `opacity: 0`, `pointer-events: none` |

The same numbers the dev server gave. The commentary has never been visible in
any build that has ever existed.

The stylesheet holds the whole story in two rules 1,168 lines apart:

```css
/* line 600 — written for stacked comment blocks */
.said + .said { margin-top: 0.7em; padding-top: 0.7em; border-top: 1px solid var(--rule); }

/* line 1768 — the toast at the foot of the window */
.said { position: fixed; bottom: 14px; opacity: 0; pointer-events: none; }
```

Both meanings are in the sheet. The later one wins.

Also unchanged on the release build: the Latin line addresses (`2a:1` … `2b:1`
down the side of a Hebrew daf, and `רש״י על ברכות 2a:8:1` inside the commentary
header), and the untranslated group heading `Rif · 4` in the mefarshim chooser,
between `ראשונים · 13` and `מפרשים · 3`.

### 18 · The cold start is the best screen in the application

With no session file and a fresh profile — a genuinely new origin, so no stored
preferences at all — the window opens in Hebrew, right-to-left, on a centred
`גִּרְסָא`, the four shortcuts spelled out in words, three buttons, and
`7189 ספרים`. It is calm and it is correct. Typing `ברכות` into the picker offers
eight sensible works in under a second, and Enter lands on 2a with 400 lines
drawn.

The first screen a new reader sees is the one part of this application that needs
nothing.

### 19 · With no corpus, the window is a wall of English file paths

`Looked::said()` is careful engineering: it lists every candidate directory it
tried, in order, so that the usual cause — looking one directory away from where
the reader is standing — can be seen rather than guessed at. It is also the only
thing on the screen:

```
no shelf found. Looked in: C:\Users\…\no-corpus-here,
C:\Users\Administrator\Videos\Girsa\target\release\corpus,
C:\Users\…\no-corpus-here\corpus, C:\Users\…\no-corpus-here\../../corpus.
Run girsa-fetch and girsa-import, or set GIRSA_CORPUS.
```

Four lines of Latin paths across the top of a right-to-left window, with the
trailing `../../corpus.` reversed into `.corpus./../..` by the bidi algorithm.
No Hebrew. No *there are no seforim here yet*. No button — although
`tauri-plugin-dialog` is already in the build for the model picker, and *"export
and send cannot choose a folder"* was complaint 12 of the original eighteen. And
the handsome empty screen from finding 18 is not shown; the body below the error
is black and empty.

The toolbar still works, which is the right decision: `חפש` opens with the caret
in the box, `מדף` and `הגדרות` open. Searching then produces the other half:

```
torat emet ▾ | whole shelf ▾ | the word ▾ | anywhere in a segment ▾
no-index: there is no shelf to search
```

Every chip in English, and an error carrying its own raw code as a prefix.

### 20 · The shell writes user-facing sentences in English, in Rust

Finding 7 said the search chips were English. The cause is wider than the chips.
`app/src-tauri/`, whose README says it decides nothing, composes five sentences
that reach the screen without passing through `say.ts`:

| Where | What the reader sees |
|---|---|
| `clipboard.rs:53` | `the source packet would not serialize: …` |
| `clipboard.rs:63` | `no clipboard here: …` |
| `clipboard.rs:82` | `the clipboard refused it: …` |
| `lib.rs:853`, `lib.rs:860` | `there is no shelf to search` |

Against exactly one that is written in Hebrew, at `lib.rs:677`. Pressing `Ctrl+C`
in the release window produced, in a Hebrew right-to-left toast:

```
the clipboard refused it: Empty clipboard error, code = OSError(1418):
Thread does not have a clipboard open.
```

A raw Windows error number, in English, as the reader's whole explanation. The
underlying `OSError(1418)` is an artefact of the automated session and not a
defect. The sentence around it is the defect. Credit where it is due: the window
reported a failure rather than claiming a success it had not had.

### 21 · A tab is named in Hebrew until you restart, then it is a slug

Finding 14 said the tab strip shows internal slugs after a restart. The release
build shows exactly why. In one sitting the tabs read:

```
תוספות על ברכות +1    משנה ברורה    שבת
```

After a restart, the same three tabs read:

```
bavli/tosafot-on-berakhot +1    mishnah-berurah    bavli/shabbat
```

A tab knows its Hebrew name only while the pane that made it is in memory. A
restored tab is drawn from the session file, which stores the slug, and nothing
asks the catalogue what that slug is called.

There is a second wart on the same strip: the tab is named after the **focused
column**, so a reader learning Berakhos with Tosafos beside it has a tab called
*תוספות על ברכות +1* and the masechta is the `+1`.

### 22 · A long sitting: it holds, and the pane never gives anything back

Mishnah Berurah, 17,418 segments, scrolled to its end in rounds of sixty jumps:

| | lines in the document | nodes | JS heap |
|---|---|---|---|
| on opening | 400 | 2,903 | 2.4 MB |
| after 60 jumps | 6,100 | — | — |
| after 240 jumps | **17,418** | **52,618** | **19.6 MB** |

Two readings, and both are fair. The engine holds up: once nothing more is being
fetched, sixty scroll events cost 1,237 ms, which is the 20 ms delay between them
and essentially no work of its own. Nineteen megabytes for the largest sefer in
the corpus is nothing.

But `pane.ts` bounds only the *first* render. `WINDOW = 400` is applied by
`render()`; `extend()` appends and prepends and **nothing ever removes a line**.
The header says the window *"grows when they reach an edge"*, which is true; it
does not claim it shrinks, and it does not. A reader who works through Mishnah
Berurah front to back is carrying all 17,418 lines by the end of it.

### 23 · There is no citation-style control anywhere in the window

`start-here.md` promises that changing the citation style *"reformats every
citation"*. The settings panel has three sections — `הקריאה`, `שפה`, `מקשים` —
holding a theme picker, two font boxes, three numbers, a nikud selector, two
language selectors and twenty-one rebindable keys. There is no citation style in
it, and no other panel offers one. `api.ts` exports `setCiteStyle`; no view calls
it. The promise cannot be kept by any sequence of clicks.

### 24 · What is still untested, and why

Three things on my list were not settled, and saying so is more useful than
implying they were:

* **Ksav → Girsa.** The `girsa` URL scheme *is* registered
  (`HKCU\SOFTWARE\Classes\girsa` → `URL:org.girsa.app protocol`, pointing at
  `target\release\girsa-shell.exe`), and both listeners are wired at
  `main.ts:161–162`. The live end-to-end fire — a loopback `POST /open` and a
  protocol launch — was refused by the guardrails on the automated session, so
  the reverse direction is **wired and unproven**. Note also that the registered
  command points into a build tree, which is what happens when there is no
  installer (finding 16).
* **Copy fidelity.** The clipboard is not reachable from the automated session at
  all, in either direction. What reaches the clipboard on a real machine is
  unverified; what happens when it fails is finding 20.
* **Real typing.** Text was inserted through the automation channel rather than
  through a Hebrew keyboard layout. Hebrew went in correctly and searched
  correctly; the IME path is untested.

Three others were tested and are findings 18, 19 and 22 above. The seventh —
display scaling — held: at 1366 × 768, 1280 × 720 and 1024 × 768 the single-pane
window keeps its title, its toolbar and its reading column with no horizontal
scroll and nothing off the edge. The machine these numbers were taken on runs at
125%, so the scaling path was exercised throughout.

### 25 · The eye, built and pointed at the two worst findings

Lesson 1 said nothing in this project has eyes. It has two now, and they are in
the tree:

**`app/test/collision.test.mjs`** — static, no dependencies, in the default `npm
test`. A class whose own bare rule hides it — `position: fixed`, `opacity: 0`,
`visibility: hidden`, `display: none` — may be constructed by **one module
only**. `.said` was built by `main.ts` (the toast) and `pane.ts` (the comment
block); that is the entire bug, and it is now an assertion.

The first version of this guard passed while the bug was still in the tree,
because it anchored its selector match on the closing brace of the previous rule
and `.said` at line 1768 sits directly under a comment. Worth recording: a guard
written to catch a silent failure failed silently on its first outing.

**`app/tools/eyes.mjs`** — `npm run eyes`. Headless Edge, the same engine the
window runs on, over the real `src/styles.css`, measuring the markup the modules
build. Seven assertions. Pointed at the original bug it reproduces the live
numbers exactly:

```
FAIL a mefaresh's comment is in the flow, not fixed to the window
  position was fixed — the toast rule reached it
FAIL a mefaresh's comment can be seen
  opacity was 0
FAIL the box around the comments is as tall as the comments
  box 16px around a 66px comment
```

`box 16px around a 66px comment` is the number taken off the running release
window, arrived at independently by a specimen in a headless browser. That is the
check working.

It also reports, without asserting, that the pane header measures its title at
**0px** in a 240px column. That is finding 5, and it is deliberately not fixed
here: what should give way first in that header — the name of the sefer or the
fifth button — is the owner's decision, not an auditor's. The number is there so
the decision can be made against a measurement.

### One thing was fixed, and it is named here

The class rename from finding 1 — `pane.ts` now builds `said-one`, and
`styles.css` styles `.said-one + .said-one`. Two lines, plus the comments that
say why. It is the only change to shipped behaviour in this pass, and it was made
because a guard that lands red is a guard nobody keeps. **The flagship
interaction of this application now puts words on the screen for the first
time.** It has not been read by a person yet; it has been measured.

---

## Appendix · How this was tested

The real shell, the real corpus, the real link graph — not the browser fixtures.

```sh
cd app && npx tauri build --no-bundle         # the only build that embeds the frontend
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 \
  target/release/girsa-shell.exe
```

With the port set, WebView2 speaks CDP, so the window can be clicked, typed
into, screenshotted and measured from outside — which is how every number in
this document was taken. Corpus: 7,189 works, 11 GB, with the 3.6 GB search
index. Window: 1360 × 900 at 125% scaling; 1366 × 768, 1280 × 720 and
1024 × 768 for the small-screen tests, and 740 × 620 for the narrow one.

**A correction to this appendix, which was wrong for a day.** It used to say that
`cargo build --release -p girsa-shell` *"relinks nothing and the binary keeps the
frontend it was built with"*, and prescribed touching `lib.rs` to force the
embed. That is not what happens. `cargo build` on this crate produces a binary
that embeds no frontend at all and navigates to the Vite dev server — finding 16.
Nothing was stale; nothing was embedded. Both sittings measured a development
server until somebody ran the real build. The wrong note is quoted rather than
deleted, because *a diagnosis that fits every symptom and is still wrong* is the
more useful thing to have written down.

Findings 1–15 were taken on the dev server, 16–25 on the release binary, and
finding 1 was re-measured on the release binary and is identical to the pixel.

Three side effects worth knowing about, all mine: a source was sent into the open
Ksav document during the presence test; Rashi and Tosafos are ticked on Berakhos
in this session; and `target/release/girsa-shell.exe` is now a real release build
rather than a dev-URL one, which is an improvement and is still a change.
