# The audits, and what each one is for

Five documents of this kind were committed between August and November, ~200 KB
of narrative that nothing runs and nothing cross-checks. Four of them are now in
git history and only their conclusions are here (#32); the fifth,
[`../AUDIT-lamdan-2026-08-30.md`](../AUDIT-lamdan-2026-08-30.md), stays because
its nineteen items are still open issues and code comments cite it by item
number.

**Nothing in this page is a decision.** The durable rules they argued for are
[`../CONTRIBUTING.md`](../CONTRIBUTING.md) §4 and [`../BUILDER.md`](../BUILDER.md);
this is the index of what was found, so that a later reader can tell which
criticism the tree has already answered and which it has not. Where a conclusion
still stands, it says so.

To read a removed volume in full:

```sh
git log --oneline --diff-filter=D -- AUDIT.md        # the commit that removed it
git show <commit>^:AUDIT.md                          # the text, whole
```

---

## `AUDIT.md` — performance, 18 August 2026

**Asked:** where does the tree do more work than it needs to, and where does it
block when it should not. **Method:** a source read; every timing in it is the
tree's own published measurement, cited. Baseline `c60e1ef`.

**What it concluded.**

1. **Two findings were one story.** One global `Mutex` serialised all 138
   commands, and three commands held it across unbounded I/O — so the fix was
   the lock's shape, not the three commands. Siblings got named for the first
   time in the same breath: the audit's own successor found nine more commands
   still holding it.
2. **The gate was nine serial steps, three of which shared nothing with
   `cargo`.** Its finding 9 asked for the two lanes `tools/verify.mjs` now runs,
   which is why that file's header argues about ordering rather than counting.
3. **A source read cannot price a change.** Its method note tells an agent to
   measure before and after rather than trusting the estimate, and the same
   sentence is why `docs/coverage.md` prefers a guard that fails to a number
   that is remembered.

**Status.** The lock findings were implemented in `15cebb3`; the lanes in the
gate. The rest of the list — picker debounce, panes drawn one at a time,
`topLine()` materialising every line — has been overtaken by events rather than
answered: the views were rewritten for the `Latest` ticket pattern, and the
numbers in the document are about a window that no longer exists.

---

## `AUDIT-CORRECTNESS.md` — correctness, interop and concurrency, 18 August 2026

**Asked:** what is wrong, what costs more than it should, what races, what is
built halfway — with the Girsa↔Ksav seam as a first-class subject. **Method:** a
source read of every crate and the whole shell, plus three findings
*executed*: one reproduced crash against the real `girsa-ksav`, and the window
driven over CDP against the reader's shelf.

**What it concluded.**

1. **The defects cluster in exactly two places: the seams between two
   processes, and the paths a guard was written for but does not reach.** That
   second clause is the one worth keeping. It is not a bug in a guard; it is a
   guard applied where the author was looking. Four of its thirteen headings are
   that shape.
2. **A design that writes beside-and-renames must do it everywhere it writes.**
   Three findings were the reader's own work — the document being typed, the
   shelf arrangement, the saved copy — written with a truncating `fs::write`, in
   a layer whose append-only log already renamed into place and whose module
   note says why.
3. **A silent wrong answer is worse than a refusal, and the seam is where it
   happens.** `/refresh` dropped citations it could not parse from a list the
   pen zips by position, so one bad ref re-quoted everything after it from the
   wrong place. That is the project's own first rule (T1) reached from the other
   side: not *a wrong ref*, but *a right ref in the wrong slot*.

**Status.** The crash, the three atomic writes and the refresh zip are fixed;
the lock-discipline list was extended to the nine commands the audit named. Its
§13 small things are fixed where they were found, and §10's shape — *a guard
that does not reach what it was written for* — is now a rule rather than a
finding: `docs/coverage.md`'s manifest is the machine for it.

---

## `AUDIT-2026-08-23.md` — the graded audit

**Asked:** how far is each area of the code from honoring what it promises, with
a grade per subsystem and every major finding re-verified individually against
the source. **Method:** seven parallel deep-readers, then a re-check of all
twenty-one majors and a large sample of minors.

**What it concluded.**

1. **The failure modes the tree obsesses over were closed; the ones it stopped
   obsessing one layer early were open.** Its own progress note, a day later,
   records all twenty-one majors and all thirty-seven minors fixed — which is
   the useful half of a graded audit: it is a *diff*, not a verdict.
2. **Grades measured distance from the tree's own stated standard, not from
   industry.** `girsa-app` scored A− and the window B−; the window is where the
   `Latest` ticket pattern was invented and where four views then did not get
   it. The lesson survives the grades: a pattern that lives in one view's doc
   comment has not been adopted.
3. **A torn append poisoned a store permanently**, and the fix was to truncate
   before appending. Written down here because it is the cleanest example in the
   tree of a recovery path that recovers for the reader and not for the file.

**Status.** Closed, per its own progress note and the per-phase commits from
`7592831`. The document is history; the one thing not to lose is that the audit
was written to be *closed*, item by item, rather than read.

---

## `lamdan/girsa-2026-08-06.md` — the design review

**Asked, in three lenses:** was this the right software, is this the right
shape, is the right code inside the shape. **Method:** one reader, twenty-one
days of the tree, and an explicit list of what it tried to break and could not.

**What it concluded.**

1. **§3 — permanent segment IDs — was not implemented at the one moment it
   exists for.** The importer assigned root ids by enumeration position and
   wrote no redirect table, so re-running `girsa-import` after any upstream
   re-segmentation renumbered every segment and silently moved every anchor one
   line. Verdict: `rewrite`. **Fixed:** `redirects.jsonl` is written beside
   `segments.jsonl` and read back, and `an_anchor_survives_a_split_at_import.rs`
   holds it.
2. **The design lived in prose, and the prose was not checkable.** Four doc
   comments asserted properties nobody tested — one `slug_of`, one `find_index`,
   `Prepared` built once per query, corrections not rewritten per keystroke —
   and rule 7 of [`../CONTRIBUTING.md`](../CONTRIBUTING.md) is the answer that
   stuck: a test may not pass because it could not find what it checks.
3. **Its four changes, ranked: the format, a fixture, the doc comments into
   tests, and eight lines of CI.** The first three landed (`redirects.jsonl`,
   `girsa-fixture`, and the rules-tests that now read the tree instead of
   trusting it). The fourth is `ci.yml`, which checks out Ksav beside this
   repository to catch drift.

**Status.** Findings 1–4 are done. Its closing question — *is anyone but you
going to run `girsa-import` twice?* — got its answer the hard way: #65 was the
documented shelf build dying at step 3 because the lexicon came from an example
nobody ran.

---

## Coverage, and what these audits did not reach

Every one of them has a section of that name, and it is the part worth copying.
`AUDIT.md` said which findings were a reading of the code and not a measurement.
`AUDIT-CORRECTNESS.md` said which crates it read for shape only, and that the
running-window checks were one shelf and one machine. `AUDIT-2026-08-23.md` said
which minors were sampled rather than re-verified. `lamdan/girsa-2026-08-06.md`
listed what it tried to break and could not, which is the only one of the four
that made its *failures* legible.

That discipline is why `docs/coverage.md` prints its uncovered and
platform-gated rows on every green run: a claim about what is not tested is only
worth the paper it is on if something says it without being asked.

## What is not in here

`AUDIT-lamdan-2026-08-30.md` is still a work list, not history: its items are
issues #29, #30, #32 and the rest of that open set, and four test headers cite
it by number. It moves here when its last item closes.
