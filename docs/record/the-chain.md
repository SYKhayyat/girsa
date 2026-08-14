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

### What the chain does not do yet

- **It is a command, not a panel.** `girsa-chain` prints all four; nothing in the
  window draws them.
- **A fork is one hop wide on each side.** Two readings joined through an
  intermediate sefer are not found, and the ones that are found are bounded by
  `--width` with the drop counted.
- **Nothing walks into your own layer's dates.** A note has no `comp_date`, so it
  is `Unknown` against everything and is never a hop — which is the truthful
  answer, and not a useful one.

---

*← [Your own layer](your-own-layer.md) · [The record](../the-record.md) · [The semantic lane](the-semantic-lane.md) →*
