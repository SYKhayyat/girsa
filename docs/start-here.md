# Start here — the five minutes that are the whole idea

There is one thing Girsa and Ksav do together that nothing else does. It takes
about five minutes to see and it is the reason both applications exist. Everything
else in these two repositories is in service of it.

**You find a mekor while learning. You put it in what you are writing. The
citation in the finished PDF opens the page it names.** No retyping, no
transcription errors, no "which daf was that again" three weeks later.

Here it is, in order. Do it once and the rest of the documentation will make
sense.

---

## Before you start

You need the corpus on disk. It is not in the repository — it is a download, and
by the end of the six commands below it is **about 15 GB**, of which roughly a
third is the search index. **One step in the middle is yours to do by hand**:
Sefaria's half is fetched for you and the `.txt` libraries are not, so
`<library>` below is a path to a tree you downloaded yourself.

```sh
cargo run -p girsa-corpus --bin girsa-fetch       corpus/sefaria         # the seforim, 3.4 GB
#                            download a .txt library yourself — nothing here fetches it
cargo run -p girsa-corpus --bin girsa-import      corpus <library>...    # onto permanent ids
cargo run -p girsa-link   --bin girsa-link-import corpus <library>...    # the links between them
cargo run -p girsa-link   --bin girsa-link-types  corpus personal        # the caches that read them backwards
cargo run -p girsa-search --bin girsa-index build index corpus personal  # the search index, 4.0 GB
```

Every one of those takes its roots as words on the line, and every one answers
`--help` if you would rather read it there than here.

`<library>` is a `.txt` tree with an `אוצריא/` in it. You can name more than
one — Otzaria's first, then any others — and the same list, in the same order,
goes to both tools. [`docs/tools.md`](tools.md) has the rest of it, including
the `library.json` a tree uses to say where its seforim came from.

Then open Girsa. If it says **there is no shelf here**, it did not find the
corpus: it looks at `GIRSA_CORPUS`, then beside the executable, then two levels
up. Set `GIRSA_CORPUS` to wherever you put it.

> If you skip the last two, everything still opens. Search says why it cannot
> search, and the mefarshim list is empty and says which kind of empty it is. That
> is deliberate — an application that refuses to start because a cache is cold is
> an application that will one day refuse to start.

---

## 1 · Find the sugya (30 seconds)

**Ctrl+F**, and type. Not a title — the words.

```
מאימתי קורין את שמע
```

The result rows show the words you searched for **highlighted inside the line**,
so you can see at a glance which row is the one you meant.

There will be a lot of rows. On the full download those five words are in **317**
places — the Gemara says them twice, and then Rashi, the Meiri, the Yerushalmi,
Ein Yaakov and two hundred later seforim quote the line to talk about it. That is
the library working, and it is also more than anybody wants to read. Two ways
down to one:

- **the facet column** — `narrow by: sefer`, and pick ברכות;
- **or type the mareh makom instead.** `ברכות ב.` is read as a place, not as
  words, and lands on the line.

Click the row. The search does **not** close: it docks to a column on the left
and the daf opens beside it, narrower — so you can read the second result without
searching again. That is deliberate; it is the one thing every library search
gets wrong.

## 2 · Put Rashi on it (20 seconds)

**Ctrl+\\** — or the button that says **מפרשים · 34**.

The list is the mefarshim on this masechta, in their folders: rishonim together,
acharonim together, modern commentary after them. Two things you can do with any
row:

- **Click it.** It opens in a column beside the Gemara and stays in step as you
  scroll. This is the daf you already know.
- **Tick it.** Tick Rashi, Tosafot and the Rosh, close the list, and the daf now
  has a small **◆** in the margin of every line one of those three wrote about.
  **Click a marked line** and their comments open under it — that line only.

![The mefarshim panel, 34 on this masechta, in folders: rishonim together, then the Rif, then later commentary](images/mefarshim.png)

Clicking a row opens it beside the Gemara, and the two stay in step as you
scroll:

![Berakhot 2a menukad with Rashi in the column beside it](images/reading.png)

Tick nothing and nothing is marked. That is on purpose: 2,749 of Berakhot's lines
carry commentary from somebody, so marking every line with any commentary marks
the daf and tells you nothing.

## 3 · Send it to Ksav (5 seconds)

Highlight the words you want. **Ctrl+Shift+C**.

If Ksav is open, the chip in Girsa's toolbar says so. The words arrive in your
document with the mareh makom already under them, formatted the way you have your
citations set — `ברכות ב.` or `Berakhot 2a`, your choice, and changing that
choice reformats every citation you have ever made because what was stored was
the *reference*, not the printed string.

Nothing was retyped. The citation is not a piece of text somebody typed that
happens to look like a reference; it is the reference.

## 4 · Write, and compile (2 minutes)

In Ksav, write around it. `#כותרת[...]` for a heading, `#הערה[...]` for a
footnote, or use the toolbar and never learn the markup — the prose view is the
default because a Word replacement that opens in raw markup is asking you to
learn a syntax before you can type a sentence.

It compiles as you pause. **Ctrl+S** saves; **Ctrl+P** gives you the PDF.

## 5 · Click the citation in the PDF

Open the PDF. Click the mareh makom.

**Girsa opens at that line.** Not at the sefer, not at the chapter — the line.
Three weeks later, on a different machine, from a PDF you sent to somebody else:
the citation still resolves, because `girsa://` is registered and a segment id is
permanent.

That is the loop. Everything else is detail.

---

## The four a bachur reaches for first

Not part of the five minutes above, because none of them needs a Ksav or a
compile — they are what an ordinary morning is made of.

| | |
|---|---|
| **Today's daf, and today's mishnayos** | The button at the left of the toolbar says the daf — `דף היומי · ברכות ב'` — and opens it. Mishnah Yomis is there too, two mishnayos a day, straight through Shas. Hover for the Hebrew date, where the cycle stands, and tomorrow's. The daf turns over at **7pm** by default rather than at midnight; nightfall depends on where you are standing and Girsa does not ask, so that hour is an approximation that says so, and you can move it in Settings. |
| **Find in this sefer** | `Ctrl+F`. The whole sefer, not the lines on screen, and **the same engine and the same options as the shelf search** — the modes, the match, the phrase and the near-N are all on the bar. What differs is the shape of the answer: the next one, then the one after that, and a count. `Ctrl+Shift+F` is the search across the whole shelf, which is a different question. |
| **Print the siman** | `Ctrl+P` prints the section you are standing in — the siman, the amud, the perek — with the sefer, the printed edition and the terms at the head of it. `.docx` is still there for the whole sefer. |
| **A page you can throw away** | Already on. The shemos are written with a letter changed — `יקוק`, `אלקים`, `קל` — on the page, in the search results, in what you copy and in what you print, so a printout may be discarded. It is the default because the harm runs one way: turning it off is one click in Settings → `שמות הקודש`, and there is no click that un-prints a page. Four of them — `אל`, `שדי`, `אדני` and `צבאות` — change only where the pointing or the word beside them says it is the shem and not an ordinary word. |

One more, on a daf: the mefarshim door (`Ctrl+\`) opens with **רש"י · תוספות**
at the top of it — the two printed on the page with the Gemara, in one press.
The list under it stays in aleph-beis order, which is where you go when you want
the Taz.

There is no separate box for a mareh makom. Type it into the shelf search —
`Ctrl+Shift+F`, the same box you searched for words in at step 1, which says so
in its own placeholder — and it opens rather than searching: `שבת לא.`,
`שו"ע יו"ד סימן א`, `רמב"ם הלכות שחיטה א`, `טור או"ח סימן א`, `ערוך השולחן
יורה דעה א`. The branch works could not do this before their chalakim had names
anything could read.

## What to read next

| You are coming from | Read |
|---|---|
| Otzar HaChochma | [`from-otzar.md`](from-otzar.md) |
| Bar Ilan / the Responsa Project | [`from-bar-ilan.md`](from-bar-ilan.md) |
| Word, for writing | `Ksav/docs/from-word.md`, in the pen's own repository |
| Nothing in particular | [`shortcuts.md`](shortcuts.md), and press things |
| Something on this page not working | [`troubleshooting.md`](troubleshooting.md) |

## What this does not do

Stated here rather than discovered later:

- **Nobody has written a real sefer in it.** Three separate audits call that the
  most important line in any of them, and it is still true. Everything above works;
  none of it has survived a zman of somebody actually using it.
- **There is no sync and no account.** Your notes, corrections and dictionary are
  files on your machine. That is a deliberate choice and it means a second machine
  is a copy, not a login.
- **The OCR'd scans are dirtier than the text.** Results from them are badged, not
  demoted — you will see `OCR` beside a hit and you should trust it less.
- **Not every commentary in the world is linked.** The link graph is Sefaria's and
  Otzaria's, and it is incomplete in ways that are visible: a sefer with no links
  says *nothing links here* only when the cache exists, and says *I have not been
  told* when it does not.

Those are the four a reader meets first. The rest — which features are a command
with no panel, where two working pieces were never joined, and which measurements
have simply never been taken — are in [`not-yet.md`](not-yet.md), each with the
argument behind it.
