# The chain

*← [Your own layer](your-own-layer.md) · [The record](../the-record.md) · [The semantic lane](the-semantic-lane.md) →*

---

### Direction is time, and the graph does not have any

`spec.md` §8.6 asks for four things, and every one of them has a direction in
it: *forward from a Gemara to how it became halacha; backward from a ruling to
where the posek got it; the path between two texts; and where two rishonim read
one Gemara into incompatible halachos.*

The graph has no direction. §8.2 stores an edge **once, in the shard of the work
it points from**, and which end that is was settled by whoever wrote the row.
Counted on the corpus here:

```
bavli/berakhot        → its commentaries    51,927 edges    earlier → later
mishnah-berurah       → shulchan-arukh      18,806 edges    later → earlier
shulchan-arukh o.c.   → turei-zahav          3,315 edges    earlier → later
shulchan-arukh o.c.   → tur                    719 edges    later → earlier
```

Two of those run one way and two the other, and the Shulchan Arukh does both.
Following arrows walks the first chain forwards, the second backwards, and calls
them the same thing. So a hop is forward when the sefer at the far end was
**written later** — [`girsa-corpus/src/era.rs`](../../crates/girsa-corpus/src/era.rs)
is the only thing that answers that, and it is the only place that is allowed to.

### The era code cannot make the hop the whole feature is for

Sefaria stamps an era on 4,812 of the 7,189 works — `T` `A` `GN` `RI` `AH` `CO`
— and it is too coarse by exactly one step. **The Shulchan Arukh and the Mishnah
Berurah are both `AH`.** On era codes alone, the most-asked hop in halacha is
two contemporaries and the chain stops before it starts.

`comp_date` is on 5,294 works and is a real year, so it carries the ordering the
era loses; it also reaches Tanach, which has dates and no era code at all. The
rule is **years first, era only where there are no years, and `Unknown` where
there is neither** — never an era stretched into a year range, because the
conventional span of *ראשונים* differs by a century between authorities and a hop
ordered on that is a claim nobody wrote down.

Six shapes of date in the corpus and all six are read, including the fifty
written `ה' תרלז - ה' תרלז (בקירוב)` — 5,637 anno mundi, 1877 CE. Those fifty are
Otzaria-side acharonim, which is the layer a halachic chain *ends* at, so
dropping them would shorten exactly the traces this is for.

**88.7% of the 4,182,337 edges point at a work that can be placed in time**
(78.2% on era codes alone). The other 11.3% are not walked, and are counted where
they were refused rather than quietly skipped — a chain that dropped what it
could not date would look shorter and surer than it is.

### Half of every link is stored at the far end

`what does this se'if answer to` is a question about edges that are **not** in
the Shulchan Arukh's own file: the Mishnah Berurah's shard holds all 18,806 of
them. Until now the sidebar answered it by reading the shards of every work the
companions cache listed — a few dozen files, and quietly capped, since
`girsa-companions` keeps the top 200 works per sefer and Berakhot is joined to
1,600.

So the graph is walked once more and each edge is written a second time, into the
file of the work its far end lands in:

```
$ cargo run --release -p girsa-link --bin girsa-link-types -- corpus personal
two caches written beside the edges:
  shards read        5790
  edges              4182337
  type rows          3637524   (both ends of each, deduplicated)
  inbound rows       4131100   (51237 skipped — both ends in one work, whose own shard holds them)
  took               139s
```

Identical rows to `edges.jsonl`, read back by the same reader, so the two halves
of a segment's links cannot come to mean different things. An edge whose two ends
are in one sefer is **not** written here — its own shard has it, and a caller
reading both files wants their union to be each edge once.

`personal` is there for the second cache. An edge's type is what the corpus
shipped **plus what you have said about it** — that is what `girsa_link::Repairs`
means in the link panel, which shows your type the moment you set it. The masks
were built from the shipped label alone, so a reader who retyped an edge saw the
new type in the sidebar and searched by the old one: one question, two answers,
and the facet was the one that could not be argued with. Leave `personal` off and
the masks are the shipped answer, which is still true — the run says so in a
sentence rather than leaving you to notice.

A hop is then two file reads, cached for the life of a walk. A three-deep trace
out of the first mishnah of Berakhot reads 8 works and takes 1.6 seconds; the
same walk over the companions scan would open several hundred files, some of
them 16 MB. The links panel now reads the same cache and has lost its 200-work
cap with it.

### A chain of *connected somehow* is not a chain

Every edge type present in the graph, counted:

```
2,123,215  comments-on   50.8%
2,048,326  references    49.0%   ← "these two are joined", and nothing further
    7,812  paraphrases    0.2%
    2,984  quotes         0.1%
```

There is no `codifies` and **there is no `disputes` anywhere in it.** So half of
any long chain is built out of links that say only that two places are connected,
and a walk that drew those the same as a commentary would be manufacturing
scholarship. Each chain carries its weakest hop, and the answer says so out loud:

```
$ girsa-chain corpus personal back girsa:mishnah-berurah/58:1#1496 --depth=2

back from משנה ברורה 58:1  [1875–1905]
  (א) זמן ק"ש - וברכות ק"ש לפניה ג"כ אין לומר קודם הזמן …

  └ שולחן ערוך, אורח חיים 58:1  [1563]   (comments-on, the corpus said `commentary`)
    └ טור orach_chayim:58:1  [1300–1340]   (comments-on, the corpus said `commentary`)
      └ ספר מצוות גדול positive_commandments:19:1  [1243–1247]   (references, the corpus said `ein mishpat / ner mitsvah`)

13 chains, 3 of them a transmission all the way — the rest pass through a
link that only says the two are connected somehow, which is 49% of this graph.
not followed:
       54  the other way in time, which is the bulk of any graph
        7  written at the same time, so neither came from the other
       26  no date and no era in either corpus, so which way the hop goes is not known
      315  dropped by --width, best first
```

The last paragraph prints every time, including when it is empty. *Twenty-six of
the seforim that read this line could not be dated* changes what the thirteen
chains above it mean, and it is part of the answer rather than a diagnostic.

`path` keeps the same distinction one level up. A search that runs out of budget
reports **`not found within N hops`**, which is not the same sentence as *there
is no path*; only a search that exhausted everything reachable from both ends
says the second. Two-sided, because a daf of Gemara has tens of thousands of
links and a one-sided walk spends its whole budget on the first two hops.

### Where two readings were argued out later

Break analysis is the one thing in §8.6 the corpus cannot actually do. Nothing in
4.1 million edges says two seforim disagree — there is no `disputes` edge. What
the data *can* say is that two of them read the same line and that a later sefer
had to deal with both, which is the shape a machlokes leaves behind:

```
$ girsa-chain corpus personal fork girsa:bavli/berakhot/2a:1#1 --width=25

  1 pair read this line and is later cited together. Nothing here says they
  disagree — the corpus has no `disputes` edge anywhere in it. This is where to look.

  רש"י על ברכות 2a:1:2  [1065–1115]
  תוספות על ברכות 2a:1:1  [1150–1350]
      both cited by רשימות שיעורים על ברכות 2a:69  [1909–2011]
```

Rashi and Tosafos on the first mishnah of Berakhos, and the sefer that takes them
both up. It is offered as a place to look and never as a finding, and a pair with
an edge joining the two directly is marked as such — one of them may simply be
answering the other, which is a different thing.

### The panel, and the three judgements it is not allowed to make

For as long as this tier existed it was a command. `girsa-chain` printed all
four walks and nothing in the window drew any of them — so the whole of
`spec.md` §8 was a feature a reader could only see by leaving the application,
which `BUILDER.md` §0.3 says is not built.

The panel docks beside the reading like every other one, on `Ctrl+Shift+M` or
the button next to *links* — the neighbouring question, put next to it: the
links panel says what touches this line, this says where it went and where it
came from. Three tabs, which are the three walks a reader has a question for:
to halacha, back to a source, and two readings.

**The walk is `girsa-link`'s, unchanged.** `girsa_app::chaining` turns it into
rows and adds naming and the tree's shape, and nothing else — because a panel
and a terminal tool that could disagree about which hops are real would be two
answers to *how did this become halacha*, and the shape of the answer is the
whole claim. In particular the panel does not decide, and could not:

- **whether a chain is a transmission.** 49% of this graph is `references`,
  which says only that two places are connected somehow. A row that drew one of
  those like `quotes` would be presenting a shrug as a mesorah, so
  `Hop.transmission` is computed where the edge types are known and the
  stylesheet only reads it. The count under the list is *chains, and how many of
  them assert something at every hop* — the second number being the honest one.
- **what the weakest hop claims**, which is what the whole chain to that point
  is worth. Named on a row only when the chain is *not* a transmission: it
  matters when there is a weak link and is noise when there is not.
- **what the walk refused.** Carried on the answer rather than logged. *Nine of
  the eleven seforim that read this line could not be dated* changes what the
  chain above it means, and a reader who cannot see that number is reading a
  chain that looks complete. The panel ends with the same paragraph
  `girsa-chain` ends every command with, including the one that is not a count
  of edges but of blind spots: seforim whose incoming half was never built, each
  a place the walk may have missed a hop entirely.

A hop that is yours — a link you drew or confirmed — says so. A panel that hid
that would be handing your own guess back to you as evidence.

The fork tab carries the caveat above the list rather than under it, because it
is true of every row: **the graph has no `disputes` edge anywhere in it.**
Nothing in the data says two seforim disagree. What it says is that two of them
read one line and a later one had to deal with both, which is the shape a
machlokes leaves behind — offered as a place to look and not as a finding.

### A reading is one hop and a witness is not

A fork used to be found only where one sefer linked to **both** readings
directly, and that was the same number twice: the walk that finds the readings
and the walk that finds who dealt with them were both bounded at one hop. Only
one of those bounds was a definition.

*A reading of this line* means a place that links to this line. Widening that
would make it mean *anything downstream*, and every sefer that ever quoted a
sefer that quoted this one would become a reading of it. So the readings stay at
one hop, and `a_reading_is_one_hop_even_when_a_witness_is_not` is that.

*A witness* is a later place that had to deal with both readings, and there the
one-hop bound was a limit wearing a definition's clothes. The shape it could not
see is the ordinary one: the Beis Yosef quotes the Rosh and quotes the Rif, and
the Mishnah Berurah reaches one of them **through** the Shulchan Arukh. Under
the old rule that pair was not a fork at all — which is not a claim about the
sugya, it is an artefact of how far the walk was allowed to go.

So the witness walk goes as deep as the caller asked, and **how far** each
witness is comes back on the answer rather than being flattened into a count.
That number is the point of the type: *these two were argued out on one page*
and *these two are both somewhere above a sefer six hops down* are different
claims, and a panel that drew them alike would be inventing the first out of the
second. The window says *the nearest N hops down* where the near case would have
said nothing; the MCP answer carries `steps` on every witness and
`nearest_witness_steps` on the pair; `girsa-chain` prints `(N hops down)` where
it is not one.

Ranking changed with it. A fork whose nearest witness quotes both sides itself
now outranks one whose witnesses are all further down, however many there are —
the count was the only signal available when every witness was one hop away, and
it is the weaker of the two now.

And a fork does not testify about itself. A deeper walk reaches the other
reading wherever one side links to it, so anything in either sefer is excluded:
counting `b` as evidence that somebody had to deal with `a` and `b` would be the
pair vouching for itself.

### What the chain does not do yet

- **Two readings are found at one hop from the line.** A sefer that reads this
  sugya only by way of another sefer is a witness, not a reading, and there is
  no way to ask for it as one. Whether that is a limit or the right definition
  is a question about how a sugya travels, not about the walk.

### Your own layer's dates, and the comment that was not an instruction

This section used to carry a third line — *nothing walks into your own layer's
dates; a note has no `comp_date`, so it is `Unknown` against everything and is
never a hop, which is the truthful answer and not a useful one.* It was neither
truthful nor one problem. It was two, and the second was the real one.

**A note's date was never unknown.** `when` has been on every note since the
format existed, and it is the only date in this library that is known to the
second rather than estimated to the century — Sefaria's schemas say
`c.1065  – c.1115 CE`, and a note says exactly when you saved it. It simply was
not copied onto the catalogue entry, which went out with `era: None,
comp_date: None`. It now carries `CO` and the year, written by
`girsa_corpus::era::written_at` — which lives beside `parse_comp_date`, the
function that reads it back, so the round trip is a test rather than a hope.

Dated from `when` and not `edited`: a chain asks when a thing was *written*, and
rewording a paragraph in 2030 should not move a note behind the sefer it was
answering.

**And it would still have been `Unknown` here.** `Timeline::load` has carried
the note *call it again per root to merge in your own layer* since the type was
written. Four callers build a timeline — this command, the MCP server, the
lane's `ask`, and the window — and **all four** read the corpus root alone. So
even a dated note was invisible to every one of them.

That is the more useful finding, and it is not about notes at all: an
instruction that lives only in a doc comment is not an instruction. It is a hope
with syntax highlighting. The fix is `Timeline::across(root, personal)`, which
is what the four callers use now, because the way to stop the fifth caller
getting it wrong is a function that cannot be called the wrong way — not a
louder comment above the one that can.

A dropped-in PDF is still undated, and deliberately: the day you obtained a
sefer is not the year it was written, and `Unknown` beats a confident wrong
answer. The distinction that matters is between what you *wrote* and what you
*acquired*.

---

*← [Your own layer](your-own-layer.md) · [The record](../the-record.md) · [The semantic lane](the-semantic-lane.md) →*
