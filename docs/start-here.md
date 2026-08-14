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
it is about 4 GB of text plus the link graph.

```
cargo run -p girsa-corpus --bin girsa-fetch          # the seforim
cargo run -p girsa-link  --bin girsa-link-import     # the links between them
cargo run -p girsa-link  --bin girsa-link-types      # the caches that read them backwards
cargo run -p girsa-search --bin girsa-index          # the search index
```

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
so you can see at a glance which of the eleven hits is the one you meant. Click
it.

The search does **not** close. It docks to a column on the left and the daf opens
beside it, narrower — so you can read the second result without searching again.
That is deliberate; it is the one thing every library search gets wrong.

## 2 · Put Rashi on it (20 seconds)

**Ctrl+\\** — or the button that says **מפרשים · 30**.

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

## What to read next

| You are coming from | Read |
|---|---|
| Otzar HaChochma | [`from-otzar.md`](from-otzar.md) |
| Bar Ilan / the Responsa Project | [`from-bar-ilan.md`](from-bar-ilan.md) |
| Word, for writing | `Ksav/docs/from-word.md`, in the pen's own repository |
| Nothing in particular | [`shortcuts.md`](shortcuts.md), and press things |

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

Those are the four a reader meets first. Seventeen more — which features are a
command with no panel, where two working pieces were never joined, and which
measurements have simply never been taken — are in [`not-yet.md`](not-yet.md),
each with the argument behind it.
