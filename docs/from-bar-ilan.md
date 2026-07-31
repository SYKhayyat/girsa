# Coming from Bar Ilan (the Responsa Project)

You know the Responsa Project. You search with operators, you narrow by author and
era and category, you follow the citation links between a shu"t and the Gemara it
argues from, and you are used to a search engine that takes Hebrew morphology
seriously.

That last part is the one that matters, so this page starts there.

---

## The same task, both ways

**You want every place that cites Berakhot 2a and disagrees about what "מאימתי"
means.**

### In Bar Ilan

1. Search `ברכות ב ע"א` with the citation index.
2. Narrow to shu"t, then to a period.
3. Read the hit list; follow the citation.

### In Girsa

1. **Ctrl+F**, `ברכות ב.` — a mareh makom in the search box is resolved as one,
   and the panel says where it landed and what else it could have been.
2. Open it, then **Ctrl+L** on the line.
3. The panel lists what links to it. Commentary above citation, because *somebody
   wrote about this line* and *somebody mentioned this line* are different claims.
4. The **lens** row filters that list — a lens is a saved filter you made, not a
   category somebody else chose.

---

## Where Girsa is genuinely behind

**Morphology.** Bar Ilan understands Hebrew forms in a way Girsa does not. Girsa
tokenizes, strips nikud for the index (keeping maqaf as a word boundary, because
deleting it glues `אֶת־הַשָּׁמַיִם` into one token and the second pasuk of the Torah
stops being findable by either word in it), and offers **widenings** — it will tell
you *"nothing for what you typed; 41 for this related form — apply it?"* and it
will not apply anything without you clicking. That is honest, and it is not
morphological analysis.

**The corpus of shu"t.** Bar Ilan's responsa collection is the reason people buy
it. Girsa's shelf is thin there, and thin in exactly the places a poseik works.

**Query operators.** There is no boolean syntax. There are **chips** — a row of
controls above the box saying what the search will do, which you set rather than
type — and sigils that set a chip as you type. It is more discoverable and less
expressive, and which of those you want depends on whether you have already learned
Bar Ilan's syntax.

---

## Where Girsa does something different

### The engine never changes your query without telling you

Type something with no results, and you get **the ladder**: the widenings that
*would* have found something, each with its count, and **nothing is applied** until
you click one. Bar Ilan silently stems; Girsa asks. When a search has been widened,
the header says what actually ran, read off the search rather than off the box.

### A zero result is a list of what would have worked

Rather than an empty page. That is the same feature from the other side, and it is
the one thing to try first if you want to see whether this suits you.

### Facets over your shelf, including your own material

Shelf, era, author, sefer, link type — and **your tags**, which are yours and which
nothing in the shipped corpus has. The facets group by the same taxonomy the
bookcase browses by, deliberately: two mappings would put a sefer on one shelf in
the shelf view and another in a result list, and nothing would say which was lying.

### Links you can repair

A Bar Ilan citation link is a fact you accept. A Girsa edge is a claim with
provenance — how it was found, how much to believe it, what the corpus called it —
and you can **confirm it, reject it, retype it, move its end, or pin it to
particular words**. Your repairs are a layer over the shipped graph, never a rewrite
of it, and 40% of the shipped edges carry no label at all, which the panel says
rather than hides.

### The semantic lane, off by default

`Ctrl+Shift+L`. Finding by *subject* rather than by words, with a model that runs
locally. Off in a fresh install and off costs nothing — which is what makes
off-by-default a real default rather than a checkbox with a price. Results appear
*beside* the literal ones and never mixed into them.

### Writing

[`start-here.md`](start-here.md). Bar Ilan gets a citation onto your clipboard;
Girsa gets it into your document with the mekor under it and makes it clickable in
the PDF.

---

## Honest summary

If your work is **responsa research over a large shu"t corpus with morphological
search**, Bar Ilan is better at your job today and this page is not going to argue
otherwise.

If your work is **learning a sugya and writing about it**, the loop in
[`start-here.md`](start-here.md) has no equivalent in Bar Ilan, and the link layer
lets you interrogate a line rather than look a reference up.

Nobody has written a sefer in Ksav yet. That is the sentence that should temper all
of this.
