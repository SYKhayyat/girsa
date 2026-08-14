# The semantic lane

*← [The chain](the-chain.md) · [The record](../the-record.md) · [A document of yours, with its shape](a-document-of-yours.md) →*

---

`spec.md` §9.9 asks for one thing the literal index cannot do: *I remember a
Rishon who says something like this but not the words.* It was the last thing in
the spec still unbuilt, and it was unbuilt because every route to it crossed a
line `BUILDER.md` §0.1 says a work order may not cross alone — a model fetched at
runtime, a licence that is not this repository's, and 5,000,545 segments to
embed before it answers anything. **Ruled** (§16 #20) and now built.

### The licence disagreed with itself, so it was checked

§9.4's candidate table called BEREL *unrestricted*. This README warned it carries
its own terms. Those are not the same claim, and W30's first instruction was to
settle it before writing a line. Checked three ways on 29 July 2026 — the model
card, its YAML frontmatter, and the Hub API's metadata for `dicta-il/BEREL_2.0`,
which redirects to **BEREL 3.0**: **`apache-2.0`**, with a request to cite the
paper. That is one of this repository's own two licences.

### What it does, and what it does not

This is the part worth reading, because the honest answer is narrower than the
feature sounds. BEREL is a **masked-language model, not a sentence encoder** —
nothing in its training gave it a similarity objective — and it shows. 240
se'ifim of Hilchos Tefillah, embedded, asked 22 questions with a known right
answer, scored by where the right se'if landed:

| asked as | rank 1 | top 5 | top 10 | worst |
|---|---|---|---|---|
| **a half-remembered statement** (10 pairs) | 8 | 9 | 10 | **16 of 240** |
| **a question about the se'if** (12 pairs, 5 sharing no word) | 1 | 1 | 1 | 97 of 240 |

*I think it says the drunk may not daven because he has no kavanah* — none of
`נשתכר`, `ביין`, `יעמוד` or `דעתו` is in the se'if — comes back **first**, out of
240. *How late may one daven shacharis?* comes back twenty-fourth.

So the lane's box asks for **a line as you remember it**, which is §9.9's own
sentence, and does not pretend to answer questions.

And now it says so **when you ask one.** Every answer already carried
`girsa_lane::MEASURED` — *works poorly on a question* — which is the right place
to start and the wrong place to stop: a reader who has just typed one is being
handed a general caveat over ten plausible-looking rows, and the specific figure
is not close. One in twelve against ten in ten is not a model having a bad day.
So a query that reads as a question gets `A_QUESTION` above the results, with
both numbers in it and with what to do instead.

Three things about that sentence, and each is a refusal:

- **It changes nothing about the ranking.** Same rows, same order. The lane does
  not decide it knows better than the reader what they meant to type, and a
  feature that silently rewrote a query would be worse than one that answers it
  badly and says so.
- **The rule is deliberately narrow.** A leading interrogative, or a question
  mark. `מה` is a prefix of ordinary words and turns up mid-sentence in
  perfectly good half-remembered lines, so only the first word is looked at —
  under-reporting leaves a reader exactly where they were, and over-reporting
  puts a wrong caveat over a good answer. A caveat a reader learns to ignore is
  worse than none.
- **It is worded once**, in `girsa-lane`, like `ADJACENT` and `MEASURED` and
  `SHORTLISTED` before it. The window, `girsa-lane` on a terminal and the MCP
  `adjacent` tool all say it, and none of them says it in their own words. On
  the terminal it prints *above* the rows rather than under them, because a
  reader reads downward and a caveat under ten results is read once the results
  have already been believed.

It is also said on a **refusal** — a lane that is off, or adrift — because a
reader who typed a question and got *the semantic lane is off* is about to turn
it on and type the same thing.

And the standard repair for
a raw BERT — subtract the mean of the space, since every sentence sits in a
narrow cone — was **tried and made it worse** (24→40, 97→123, 9→24). It is
measured in `examples/measure.rs` and it is not built. A plausible improvement
that does not survive measurement is exactly what §9 exists to refuse.

The measurement is why the side-loading matters more than it looks. Nothing in
`girsa-lane` is BEREL-specific: it reads a `config.json`, a `tokenizer.json` and
a `model.safetensors`, runs the forward pass in Rust on the CPU, and stamps the
store with a fingerprint of what made it. The day a contrastively trained
rabbinic-Hebrew encoder exists, that is **a setting and not a release** — point
the lane at it, re-embed, nothing to migrate. The same *make it reversible rather
than permanent* move W26 made for OCR.

### Off is off, and the numbers say what is missing

Four things hold, and each has a test that fails against a deliberately naive
version of it:

- **Off means off.** Not *a mode that returns nothing*: with the lane off no
  model is loaded, no vector is read, and the whole corpus tree is
  **byte-for-byte identical** before and after a run — asserted by comparing
  every file under `corpus/`. Everything the lane writes is in your own layer.
- **The absence has words.** The lane turned on with nothing to run says
  *"the semantic lane is on but cannot run: no semantic model is configured…"* in
  the search header. A reader who turned it on and got nothing is owed the reason
  rather than left to conclude the corpus has nothing like their query in it.
- **Every answer states its own coverage.** *"this lane covers משנה תורה, הלכות
  תפילה — 240 segments; 7,190 other seforim on this shelf aren't in it."* Found,
  empty, refused or off, the sentence is drawn — composed once in Rust so the
  window, the CLI, the MCP tool and the test cannot drift. A sefer half-embedded
  reports **both** numbers, for the reason a scan stopped at page 40 of 302 does.
- **And every answer states what the lane was measured to do** — added 6 August
  2026, and it is a correction rather than an addition. The sentence *"measured
  on a half-remembered statement, and it works poorly on a question. It does not
  pasken"* existed, in exactly one place: the MCP tool description. **A robot was
  told and the reader was not.** It is `girsa_lane::MEASURED` now, one string,
  drawn in the window under every answer and read by the MCP surface from the
  same constant.

  It gained a clause it never had: **over 240 se'ifim, not over the whole shelf.**
  Every number in the table above is at n=240. A 0.11 cosine margin — 0.74 for the
  right answer against 0.63 for unrelated se'ifim — is a different claim at 240
  candidates than at 5,000,545: at 240 the tail is empty, and at five million the
  tail *is* the answer set. Nothing measured says which way that goes. It may
  hold; nobody has looked. Re-running `examples/measure.rs` over ~50,000 segments
  would replace that clause with a number, and it is the one afternoon this
  feature still owes.
- **The offer of the whole shelf says what it costs.** `Chosen::everything()` is
  a first-class standing choice with a tested branch in the coverage sentence, so
  the thirteen days were being offered as an equal option to the 54 seconds the
  measurements came from — with the thirteen days written down in a module note
  the reader never opens. The sentence now spends the measured throughput:
  *"this lane covers the whole library — 1,200,000 of 5,000,545 segments so far,
  about 13 days of embedding left."*
- **It is never a rung on the ladder.** `girsa-search` does not depend on
  `girsa-lane`, so no relaxation rung can reach a vector even by accident — and
  adding a `Rung::Meaning` variant does not compile, which is the proof. Every
  rung is priced before the click; an embedding neighbourhood cannot be, and a
  chip with a made-up number on it is the one thing §9 forbids.

And two vectors from two models rank against each other perfectly happily, which
is the failure mode a reader could never spot from the results — so the store's
header records **which model made it** and a different one opens it empty, says
whose it was, and refuses to add to it until you ask for a restart.

### 4.5 segments a second, which is why you choose

Release build, one CPU, batches of sixteen: **54 seconds** for Hilchos Tefillah,
about **thirteen days** for all 5,000,545 segments. That is the whole argument for
§16 #20's *the corpus is yours to choose* — a lane that insisted on the library
before it answered anything would be a feature nobody ever switched on. So: a
sefer, a section, a shelf, your own notes, or all of it; added to whenever;
resumable, because the vectors on disk **are** the progress record; and on its own
thread sharing the one loaded model, so reading never waits on it.

### The button, and what it cost

The first form of §16 #20 said Girsa fetches no model at all — you point it at
one. Mid-order that was amended: the folder picker stays the default path, and a
**bring in BEREL** button sits behind a setting that is off in a fresh install.
With it off there is no code path from anywhere in the application to the
network, and `bring()` refuses even if something calls it. What §14 now promises
is *Girsa never **needs** the network*, which is the sentence that was worth
keeping; nothing is vendored either way, because the weights land in your own
layer beside your notes rather than in this repository (T7).

The download is resumable **inside** one file — `Range: bytes=N-`, appended to a
`.part`, length-checked at the end, renamed into place only when whole. The
corpus fetcher gets away with per-file atomicity because its files are a few
hundred kilobytes; one of these is 738 MB over a domestic line, and a fetcher
that started again from zero on every dropped connection would never finish.

---

*← [The chain](the-chain.md) · [The record](../the-record.md) · [A document of yours, with its shape](a-document-of-yours.md) →*
