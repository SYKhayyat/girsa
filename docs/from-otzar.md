# Coming from Otzar HaChochma

You know Otzar. You have 130,000 seforim, you find them by name, and you read
photographs of the printed page. This page is about what is the same, what is
worse, and what you can do here that you cannot do there.

It is written to be honest about the second one, because a page that only listed
the third would waste your time.

---

## The same task, both ways

**You are learning Bava Metzia and you want to see what the Ketzos HaChoshen says
on the sugya.**

### In Otzar

1. Open the search, type `קצות החושן`.
2. Pick the volume from the results.
3. Find the siman in the index or by turning pages.
4. Read the photograph.
5. If you want the Gemara beside it, open a second window and find your place
   again.

You did that in maybe forty seconds and you are very fast at it, because you have
done it ten thousand times.

### In Girsa

1. **Ctrl+F**, type the words of the sugya — not the sefer's name.
2. Click the hit. The daf opens; the search stays docked beside it.
3. **Ctrl+L**. The panel lists what the corpus says is connected to *this line*,
   commentary first, each row showing the first words at the other end.
4. Click the row.

The difference is step 1 and step 3. You did not have to know that the Ketzos
discusses this sugya — you asked the line what discusses it.

---

## What is worse here, plainly

| | Otzar HaChochma | Girsa |
|---|---|---|
| Seforim | ~130,000 | ~7,200 |
| The page you see | the printed page, photographed | typeset text |
| Finding by name | excellent, and you know it | works, and is not the point |
| Offline | yes | yes |
| Nobody has written a sefer in it | irrelevant | **true, and it matters** |

**7,200 against 130,000 is the number that should decide this for you if what you
need is breadth.** Girsa's shelf is Sefaria's corpus plus Otzaria's, and there are
whole categories — most of the acharonim, most of the shu"t, nearly all of the
twentieth century — where Otzar has the sefer and this does not. If your learning
depends on a Maharsham you cannot find here, none of what follows helps.

**A photograph is not worse than typeset text, it is different.** Otzar shows you
the page the mechaber's printer set, with its layout and its typos and its
marginalia. Girsa shows you a transcription. For a girsa question that is exactly
backwards: the photograph is the evidence and the transcription is somebody's
reading of it. Girsa handles this by keeping scans as a first-class thing — a scan
opens as the daf itself, and a hit inside one is badged `סריקה` or `OCR` so you
know whether the words came from a file that said what they were or from a machine
guessing at a photograph.

---

## What you can do here that you cannot do there

### Search the words, not the titles

Otzar searches text too. What is different is what comes back: Girsa's hit shows
the matched words highlighted **inside the line**, and every result carries a
permanent id — so a hit is a place you can cite, note, link, and come back to,
rather than a page number in a volume.

### Ask a line what is connected to it

**Ctrl+L** on any line. Four million edges, from Sefaria and Otzaria, saying *this
comments on that*, *this quotes that*, *this is the parallel sugya*. Each row says
how it was found and how much to believe it, behind a disclosure so it is not in
your way.

This is the thing with no equivalent in Otzar. You are not looking a sefer up; you
are asking a sentence who has spoken about it.

### Tick your mefarshim once and see them on every daf

Tick Rashi, Tosafot, the Ritva and the Meiri on Berakhot. Now every line one of
them wrote about carries a mark, and clicking it opens theirs — on that line.
Per masechta, remembered, in the order you ticked them.

### Write about a line and have the note stay attached

**Ctrl+N** on a line writes a note that is anchored to a segment id, not to a page
number. The corpus can be re-imported, the text can be re-typeset, and the note is
still on the line it was about. **Ctrl+M** lists everything you have written.

### Correct the text, and see what you corrected

The transcriptions have OCR errors. **Ctrl+K** fixes a word — and the fix is a
*patch over* the corpus, never a rewrite of it, so you can turn corrections off and
see exactly what the file says. **Ctrl+J** opens a queue of words the machine
thinks are scanning errors, ranked, with the place to go and look.

That is a category Otzar does not have at all: the text is a photograph, so there
is nothing to correct and no way to be wrong.

### Send it into what you are writing

The whole of [`start-here.md`](start-here.md). This is the actual argument.

---

## Things that will annoy you in the first ten minutes

- **The shelf is Hebrew and so is everything else, by default.** `Ctrl+,` →
  שמות הספרים → English, if you want `Berakhot` instead of `ברכות`. The interface
  itself is still Hebrew; only the sefer names follow that setting so far.
- **Nikud is on.** `Alt+N` turns it off. Berakhot is fully menukad and it is a lot
  if you are not used to reading it that way on a screen.
- **The dark theme is the default.** `Ctrl+,` → ערכת צבעים → בהיר.
- **There is no volume-and-page.** A sefer is addressed the way the sefer addresses
  itself — `ברכות ב.`, `אורח חיים א:א` — not by which printing you have.
- **The first search after opening is slower** than the ones after it. The index is
  memory-mapped and the first query pages it in.

## What to do if a sefer is missing

Drop the file on the window. Girsa reads `.txt`, `.docx`, `.pdf` and its own
`.ksav`, files you own go on the shelf beside everything else, and a PDF opens as
a scan you can map to dapim. It is your shelf; the shipped taxonomy is a default
you can rearrange by dragging.
