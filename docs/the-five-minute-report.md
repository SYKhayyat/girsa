# The five-minute report, answered

A reader opened Girsa, used it for under five minutes, and wrote down eighteen
things. Then two more while this was being fixed. Their closing line was *"all
in all, this looks like a toy written by ai"*, and the fair reading of that is
not that the code is bad — most of it is careful — but that **almost none of it
had been used**. Every one of the eighteen is real, and most of them are one
line each.

This is what each was, where it was, and what was done about it. The pattern
underneath is worth naming first, because it is the same pattern nine times:

> **Two things that had to agree, and nothing that made them.** A DOM list and a
> panel list. A logical CSS property and a physical one. A bool that carried
> three claims. A comparator that named the wrong field. Two coordinate systems
> for a highlight. In every case both halves were carefully written and nothing
> compared them.

So the fixes are mostly *deletions of the second copy*, and most of them come
with a test that reads the source rather than a unit test — because a unit test
of either half passes while they disagree.

---

## The eighteen, and the two that arrived after

### 1 · Seforim sorted by name, not by true order

The Chumash came back **במדבר · בראשית · דברים · ויקרא · שמות**. Five sites
sorted seforim and all five were `a.he_title.cmp(&b.he_title)`.

Sefaria states the order — `order` in the schema, `[1,1]` for Bereshis, `[2,1]`
for Shabbos — on 551 of its 6,595 schemas, and they are exactly the seforim
anybody browses to. It is now `Work::order`, read at import, and a commentary
that states none **inherits its declared base's**, so a rishon's five volumes on
the Torah come back in the order the Torah is in. 5,298 of the 7,189 works on
this shelf carry one.

`Work::by_order` is the one comparator; every list goes through it.

**This needs a re-import to take effect on an existing shelf**, and it is the
short one:

```sh
cargo run --release -p girsa-corpus --bin girsa-import -- --metadata-only corpus <otzaria>
```

A catalogue without the numbers falls back to the title, which is the old
behaviour — quietly, which is the one thing worth knowing about it.

### 2 · The gap on one side, the shelf on the other

`.shelf.is-docked { inset: 0 auto 0 0 }` — physical, always the left edge. The
reading's gap was `margin-inline-start` — logical, the right edge in an RTL
window. So the space was made on one side and the panel stood on the other, over
the text. Both are logical now, and the edge they agree on is the reading's
leading one.

And both the shelf and the search **minimise** to a strip that reopens them,
which is what was asked for.

### 3 · No way to put the interface into English

There was one language setting and it was the **seforim**: `names.ts` chooses
between a work's two titles. Every button, heading and sentence in the window
was a Hebrew string literal typed in place, in twenty modules.

Two settings now — `language` for the seforim, `interface` for the window — with
two commands and two rows in the settings panel. `app/src/say.ts` holds every
string in both languages, and `test/say.test.mjs` **reads every module** and
fails if one carries a Hebrew literal a reader would see.

### 4 · הגדרות opens nothing; the font-size buttons do nothing

Two separate bugs behind one complaint.

`main.ts` appended eleven panels at boot and then `draw()` rebuilt the document
with a hand-written list of **eight**. From the first redraw, `settingsview` and
`writing` were not in the document; their buttons still worked perfectly, setting
`hidden = false` on a node nobody was rendering. There was already a frozen list
of the panels in that file, with a test that sweeps the module for omissions —
the DOM list was a second list, three lines long, that nothing checked. It is
derived from the first one now, and `draw()` replaces the chrome only.

The font size was CSS: `calc(19px * var(--reading-size) / 100)` with
`--reading-size: 120%`. `calc` cannot multiply a length by a percentage, so the
declaration was invalid and thrown away — every size control worked, wrote the
session, redrew the window, and changed nothing. The variable is a number.

### 5 · The nikud toggle was backwards, and had no middle setting

The button printed the state you were **in** (`עם ניקוד` while nikud was on),
twenty lines from a language button whose own comment says *"a button labelled
with the state you are already in is a button nobody can predict."*

And a bool cannot hold the setting most people read a Chumash in. `Pointing` is
three — everything, nikud without te'amim, letters alone — the control names the
next one round, and `Pointing::draws` is the single predicate every surface asks
so a highlight cannot land on different letters in two places.

### 6 · Midrash Lekach Tov in a category of its own

The shelf rule *a work that declares a base is filed one level down, with the
commentaries* fired on **any** declaration. Lekach Tov declares the five
chumashim and stands on a shelf named after itself — where Sefaria files its own
commentaries — so it was filed into a `מפרשים` folder among them.

The rule now compares against the base's **actual shelf**, which is what it
always meant. Over this corpus that is 25 works moved and 5 left alone, and the
5 are the Lekach Tov.

### 7 · Bereishis counted as a peirush on Onkelos

`onkelos-genesis` declares `commentary_on: genesis`, and `companions` offered
both directions of a declaration as one `declared: bool` — which is the field the
window prints `פירוש` from. `Related` has a direction now: `on`, `base`,
`alongside`, each with its own word, and the base text gets its own heading at
the top of the list, which is where you want it when you are reading a targum.

### 8 · Several mefarshim at once, and link/unlink scroll

The door closed on the first click. It now has a second tick-box per row —
*open these* — and opens each in its own column, every one following the sefer
you are reading rather than the mefaresh beside it.

### 9 · Ticking a mefaresh did not arm the line click

`choose_mefaresh` answered with the marked lines and the window patched the rest
of its own copy: it flipped `chosen` inside `works`, and the pane counts that
array to decide whether a click means anything. The list you tick in is `listed`,
which also carries the alongside seforim and every mefaresh the graph knows and
the catalogue does not — and on a masechta most of them are those. Tick one and
the count stayed at zero, so the pane went on ignoring clicks.

Rust answers a tick with the whole list now. One answer, from the one place that
builds it.

### 10 and 11 · `עוקב` and `קישורים`

`עוקב` is a bare participle with no object: it did not say what follows what,
whether clicking starts or stops it, or which column it is about — and it grabbed
`others[0]`, which on a three-way split is whichever pane the layout lists first.
It is named after the thing it does, says which column it will tie this one to,
and shows the state it will move to.

The links panel led each row with a bare arrow glyph. It leads with the sefer and
says the direction in words.

### 12 · Export and send could not choose a folder

They could not: an export went to `personal/exports/` and said the path
afterwards, on the argument that *"a reader who wants it somewhere else has a
file manager."* That is true of a debug artefact and false of a sefer you are
handing to a chavrusa. Both open a real folder dialog, and both remember where
the last one went so the question is asked once.

### 13 · כתוב did nothing

The writing drawer was the other panel `draw()` dropped from the document. See 4.

### 14 and 15 · Opening a sefer, tabs and windows

Tabs are arrangements, on the model settled next door in
`Ksav/decisions/2026-08-11-marking-up-the-ui-inventory.md`: a tab's label is its
**focused** pane's sefer (it was `panes[0]`, so a Gemara-and-Rashi tab said
`ברכות` whichever you were in), the strip hides itself at one tab, and the `×`
says *close* and never reads as delete.

**And the open set**, borrowed from Ksav, where the same absence produced seven
separate complaints (`ksav/app/src/opendocs.ts`, and the decision of 11 August).
*Which seforim are open* is not *which seforim exist*, and once a tab is an
arrangement the strip cannot answer it: a tab holding a Gemara, its Rashi and its
Tosafos is one entry in the strip and three seforim that are open. So
`Workspace::open` goes to a sefer that is already open instead of opening a
second tab on it, `Workspace::open_set` lists what is open most-recently-read
first, and the picker leads with that list — the switcher, on a surface that
already exists and already has a keyboard.

Girsa parts company with Ksav on one rule, deliberately. Ksav says a document is
never open twice, because two carets and two undo stacks over one text is how a
document gets eaten. A sefer is read-only, and two panes on two places in one
masechta is a thing people do all day — so the same sefer may be open more than
once here, and the **gesture** carries the rule: *open* goes to it, a split makes
another view.

### 16 and 17 · Search opened on nothing, then on the last thing

Opening the panel re-ran the previous query — silently, because the box keeps its
text between openings. It now shows the previous results **and says they are the
previous results**, and searches when you ask it to.

And the only surface for choosing where to look was the facet rail, which is
computed from a result set: it did not exist until a search returned hits and was
cleared at the start of the next one. That is the tree that *"flashes, then
flashes off"*. The scope is a panel now — the whole shelf tree with `+` and `−`
on every row, a box for finding one sefer, and one row per step with its own `×`,
because the only undo used to be *back to the whole shelf*, which threw away four
clicks to take back the fifth.

### 18 · `within 5 words` was not customisable

The chip offered exactly one distance and it was five unless a proximity was
already set, so every other distance was reachable only by knowing to type `~12`
— in the one panel whose governing rule is that nothing is reachable only by
typing. It offers 2, 3, 5, 10 and 20, and a distance you typed appears on it.

### 19 · The right results flashed and were replaced

> *"when i search, it gets the right search for a second, then that drops out and
> it has a list of totally different things."*

Every panel in the window is `await` then `replaceChildren`, which is correct
exactly once. Opening the search re-ran the previous query while the reader typed
a new one; two round trips in flight, and the older, broader, slower one landed
second. Nothing in the window had any idea which answer belonged to which
question.

`app/src/latest.ts` hands out tickets and drops an answer that is no longer the
newest. The search, the picker, the shelf, the links panel and the scope panel
all ask through it.

---

## What the browser found that the tests did not

`cargo run -p girsa-app --example dev-fixtures -- corpus app/public/dev` then
`npm run dev` serves the window at `localhost:5174`, and gstack's headless
Chromium drives it. Ten minutes of that found four defects that a typecheck, 221
window tests and 1,048 Rust tests had all passed over — three of them in code
committed an hour earlier:

- minimising a panel undid itself, because the click bubbled to the handler that
  restores a minimised one;
- every panel builds its labels **in its constructor**, which runs before
  `main()` has asked Rust anything, so the shelf said `המדף` over an English
  window and reloading made it worse;
- the settings panel stayed Hebrew while everything around it turned English;
- `dev-fixtures` grouped works by shelf key instead of calling `works_on`, so the
  *preview* drew the Chumash in slug order while the shell drew it correctly.

A second pass found the mefarshim door promising `מפרשים · 34` over an empty
list, again only in the preview, and two identical unlabelled checkboxes per row
where one ticks and one opens.

**The pattern is the same one this whole document is about**, and the preview is
now held to it: a build that disagrees with the thing it previews is worse than
no preview.

And one hole the gate itself had: `cargo build --all-targets` at the root builds
`default-members`, which excludes the Tauri shell — so the four verify commands
compiled everything **except the 4,054 lines that own all the interop**, and the
shell sat broken behind a green gate for two hours. BUILDER.md rule 4 has the
fifth command now.

## How fast it is, in numbers

`cargo run --release -p girsa-app --example measure-opening -- corpus personal`,
on the real 7,194-work shelf:

| | |
|---|---|
| open the shelf | 44 ms |
| build the whole bookcase tree | 7 ms |
| list every top shelf's seforim | 27 ms |
| open Berakhos (2,749 segments) | 9 ms read · 13 ms draw · 1.3 MB on the wire |
| open Mishnah Berurah (17,418 segments) | 68 ms read · 88 ms draw · **7.7 MB on the wire** |

`three_seconds` — the correction path on the largest sefer — is 80 ms with a
clean layer and 147 ms with a thousand corrections on it.

The shelf got faster on the way past: the shipped shelf for every work is worked
out once for the catalogue (`taxonomy::Shipped`) rather than from its categories
on every question, which is also what made the Lekach Tov fix possible.

**The 7.7 MB is fixed**, and it was fixed with the window running in front of
me, which is what made it safe to touch the reading path at all:

| Mishnah Berurah, 17,418 segments | draw | serialize | on the wire |
|---|---|---|---|
| the whole sefer, as it was | 60 ms | 9 ms | **7,708 KB** |
| the window, as it is | — | 3 ms | **315 KB** |

`open_sefer` sends a window around where the reader was, plus `from` and `total`;
`sefer_lines` serves a stretch at an edge; `sefer_index_of` answers *where is
this segment* for a line the pane has never loaded. Verified by scrolling to
2,800 of Rashi's 3,138 segments with no gaps and no duplicates, and by watching a
follower pane jump from 2a to 25a when the Gemara moved to 34b — which is the
`sefer_index_of` path, since those lines had never been loaded.

Two guards that only bite over a real IPC round trip and not against fixtures:
`fetching` makes a burst of scroll events one **request**, `extending` makes them
one **append**.
