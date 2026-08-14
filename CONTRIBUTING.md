# Contributing to Girsa

Welcome. This page gets you from a fresh clone to a change that lands.

If you have never opened Girsa as a user, spend five minutes on
[`docs/start-here.md`](docs/start-here.md) first. It is hard to have an opinion
about the shape of the code without having seen the thing it is shaped for.

| | |
|---|---|
| **Just want to make one change?** | [`docs/your-first-change.md`](docs/your-first-change.md) walks one through, end to end |
| **Want to know how it fits together?** | [`docs/architecture.md`](docs/architecture.md) |
| **Want to know why something is like that?** | [`docs/the-record.md`](docs/the-record.md) |

---

## 1 · The one thing to understand first

This repository does not trust prose, including its own.

The diagnosis it was given, and every convention below is downstream of it:

> **The design lives in prose, and prose is not checkable.** Every invariant was
> written down beautifully — and writing it down was mistaken for enforcing it.

So there are checks that read this repository's own source and its own
documentation, and they fail when a sentence stops being true. A doc comment
saying *"the importer calls this"* is a claim, and if the only callers are tests,
something here says so. A README that says *"a window and fifty commands"* while
there are a hundred is a bug with a test against it.

This has a practical consequence for you: **a change that edits prose can fail
the gate**, and that is working as intended. It is not the checker being fussy.
It is the one thing standing between this codebase and the state it was in.

---

## 2 · Setting up

### What you need

| | |
|---|---|
| **Rust** | stable, with `clippy` and `rustfmt` |
| **Node** | 22 or later — required, the gate runs on it |
| **Linux only** | `libwebkit2gtk-4.1-dev librsvg2-dev patchelf libxdo-dev libssl-dev libgtk-3-dev libayatana-appindicator3-dev` |

Not `libappindicator3-dev`. It and the ayatana package conflict, apt exits 100,
and you never reach a compiler.

### Clone and build

```sh
git clone https://github.com/SYKhayyat/girsa
cd girsa
cargo build --all-targets
cargo test
npm --prefix app install
```

**You do not need the corpus.** This is worth stating plainly because it was not
always true and the whole test suite was quietly broken by it: the tests build
their own shelf from source-shaped input — Sefaria `merged.json`, an Otzaria
`.txt`, a `links0.csv` with the misspelled `Conection Type` column intact —
through the real importer, in about two seconds. Twenty-eight works, a link
graph, both caches, a tantivy index. That is `girsa-fixture`, and it is a
dev-dependency only.

Ten checks genuinely need the 11 GB download — *Orach Chayim is 697 simanim of
4,171 se'ifim* is a fact about a Sefaria release and no fixture can stand in for
it. They are `#[ignore]`d, so they read as `10 ignored` instead of ten green
ticks:

```sh
cargo test -- --ignored      # on a machine that has run girsa-import
```

If you do want a corpus, [`docs/tools.md`](docs/tools.md) has the order to build
it in.

### Run it

```sh
npm --prefix app run tauri dev
```

Or without any of Tauri, in a browser, which is faster and needs no corpus:

```sh
cargo run -p girsa-app --example dev-fixtures -- corpus app/public/dev
npm --prefix app run dev
```

That second one is not in the gate, because it is not a check — it is a window.
Use it anyway. It found four defects in ten minutes that all nine gate steps had
passed over, three of them in code committed an hour earlier.

---

## 3 · Before you decide anything

Read, in this order:

1. **[`spec.md`](spec.md) §2 (Ground truth) and §3 (The landmine).** §2 is
   verified fact about the real data, not documentation. §3 is the one decision
   that cannot be retrofitted.
2. **[`spec.md`](spec.md) §16 (Decisions settled)** — 19 rows, closed. Do not
   reopen one silently.
3. **[`docs/architecture.md`](docs/architecture.md)**, for where the seams are
   and why.

### The traps, verified

These are not hypotheticals. Each was confirmed by reading the real files, and
[`BUILDER.md`](BUILDER.md) §0.2 has the full list with the evidence.

- **Line numbers are not addresses.** Otzaria links are `file + line_index`.
  Insert one line and every link below it points at the wrong text, silently. If
  you find yourself storing a line number as a durable reference, you have
  reintroduced the central defect of the corpus this project replaces.
- **`"Conection Type"` is misspelled** — in Otzaria's JSON *and* in Sefaria's
  CSVs. Match the typo exactly. Do not "correct" it on read.

### Stop and ask — do not decide these alone

| Topic | Why it needs a ruling |
|---|---|
| **Any change to a `spec.md` §16 decision** | They were argued through and closed. Reopening one invalidates work downstream of it |
| **The segment-ID scheme** | Everything anchors to it forever |
| **Source Packet field changes** | A cross-repo contract. A field change is a semver break in `sefer-crates` and a coordinated release |
| **Anything that makes search widen a query silently** | The spec's §9 exists because another search does this |
| **Adding a network dependency at run time** | Offline is the product. Corpus updates are the only sanctioned network use |
| **Taking code from Zayit, HebMorph, or Sefaria-ElasticSearch** | This can poison the licence irreversibly |

Everything else is internal correctness. **Build it without asking.**

---

## 4 · The rules that bind every change

These are [`BUILDER.md`](BUILDER.md) §0, and they apply to a one-line fix as much
as to a work order.

**1 · Test-first.** Write the failing test, *run it, watch it fail*, then fix it.
A test you did not watch fail is not a test — it is a test you hope works.

**2 · Fix the whole family.** Patching only the reported site hides a bug rather
than fixing it. When you write it up, name the siblings you checked **including
the ones you cleared, and why**. This is the single highest-value habit in the
repository: nearly every finding in [`docs/the-record.md`](docs/the-record.md)
turned out to have two or three siblings.

**3 · No legacy.** When a thing is replaced, the old thing is deleted in the same
change — config keys, docs, tests and migrations included.

**4 · Verify.**

```sh
node tools/verify.mjs
```

Unverified is not done. The runner is the gate, and it is deliberately the only
place the steps are written down — `--list` prints them, `--from <n>` picks up
where a failure stopped it. There is no `--skip`, on purpose.

That is not tidiness. This rule used to *be* the list, in prose, in two
documents; it grew from four commands to nine across three directories, and on
13 August the formatting check — the fourth of the original four, named first in
the rule that listed them — was found failing on eleven files, some of them
unformatted for weeks. Nobody skipped it on purpose. It is the one that never
fails when you are in a hurry, so it is the one that stopped being run.

**5 · Commit per unit of work**, with a message saying what changed and what it
does *not* yet do. Format is [below](#6--commits-and-pull-requests).

**6 · Never guess at a citation, a link, or a ref.** Ambiguity is surfaced to the
user as a choice. This is a product rule, not a style preference: a wrong ref is
worse than no ref, everywhere in this system.

**7 · A test may not pass because it could not find what it checks.** No
`if !present { return }`. If the input is missing, the test either builds it —
`girsa-fixture` exists for exactly this — or it is `#[ignore]`d so the run prints
`ignored` and says so.

Rule 7 is not theoretical either. It was written down, and then broken by
forty-three test functions across ten files which spent their entire existence
printing `ok … 0 passed … finished in 0.00s` in CI. Among them was the one that
would have caught permanent segment ids being renumbered by every re-import —
the defect the whole project exists to avoid. **Rule 1 says watch it fail; this
is what happens when nobody can.**

---

## 5 · Where code is allowed to live

Two structural rules that a reviewer will hold you to, both of them enforced by
tests. [`docs/architecture.md`](docs/architecture.md) is the long version.

### The window decides nothing

`app/src-tauri` is a bridge. Where a pane lands, what may sit beside what, how
many seforim stay in memory, which fonts a Hebrew reader is offered, what an
unknown search chip means — all of that is answered in a crate, because a crate
can be tested and a webview cannot.

The shell used to decide those, and three of them were not misplacements but
bugs: the sefer cache was a queue rather than an LRU, so the sefer you had open
all morning was evicted on its twelfth neighbour; *who is writing* was two
implementations that disagreed, so one environment variable changed the name on
your notes and not the name on your corrections; and each chip family ended
`_ => the default`, so a mistyped chip key came back as a search that ran,
answered, and answered a different question than the one asked.

If your change puts a decision in TypeScript or in `src-tauri`, expect a test to
fail and expect that to be correct.

### A refusal carries a name

Errors crossing the wire are prefixed with a code — `no-index: there is no index
here` — and the window looks up the sentence by code. Not by matching English
prose, which is what it used to do: twenty-one regular expressions against the
`Display` output of Rust errors, which made every error string in the repository
load-bearing API. Reword one and both halves stay green while the reader stops
being told what to run.

If you add a refusal, give it a code and give it a line in the window's table. A
test fails if a code Rust can send has no sentence.

---

## 6 · Commits and pull requests

### Commit messages

The convention here is unusual and worth matching, because the log is one of
this project's genuinely good artifacts.

- **The subject line states the finding**, in plain words, as a sentence.
  Not `fix: eyes timeout` — *"The eye gave the browser thirty seconds, and the
  browser took twenty to say anything."* Somebody scanning `git log` should learn
  what was wrong without opening anything.
- **The body argues it, with evidence.** Timestamps, counts, run numbers, the
  exact failing output. If you changed a number, say what it was and what
  measurement moved it.
- **Say what it does not do.** An honest limit in the message is worth more than
  a clean-looking diff.
- **Name the siblings you checked**, including the ones you cleared.

Present the change as a finding rather than as a task completed, and the message
writes itself.

### Before you push

```sh
node tools/verify.mjs
```

If you touched anything under `app/src-tauri`, note that the first four steps run
against `default-members`, which excludes the shell — the runner has its own
steps for it, which is why running the gate matters rather than running `cargo
test` and calling it done. The shell is the crate that owns all of the interop
and it is the one the workspace build does not see.

### Pull requests

- One reviewable idea per PR. If the diff has two arguments in it, it is two PRs.
- Say what you checked and how. "Gate green" is the floor, not the report.
- If it changes what a reader sees, say what changed for them, in the words they
  would use.
- If it is not finished, say which part.

---

## 7 · Continuous integration

[`.github/workflows/ci.yml`](.github/workflows/ci.yml), three jobs:

| Job | What it holds |
|---|---|
| `rust` | build, test, clippy, fmt — plus two generate-and-diff checks: the shortcut card, and the packet Ksav's own test asserts against |
| `shell` | the window: `npm test`, `npm run eyes` with a browser **required**, then the frontend build and the Tauri crate's own clippy and fmt |
| `bundle` | on a `v*` tag or manual dispatch: the real Windows installer, attached to the release |

Two of these exist because of the same finding twice. `npm test` had been defined
since the window's test runner was written and no gate had ever called it. And
`eyes.mjs` — the only check here that has ever looked at a pixel — returned
success on a machine with no browser, which is a check that passes because it
could not find its input, the exact thing rule 7 forbids. `EYES_REQUIRED=1` makes
a missing browser a failure.

CI checks out Ksav beside this repository to catch drift between what Girsa
writes and what the pen asserts on. That is deliberately **not** something your
machine does on every change, which is why the local gate and CI are not the same
list.

---

## 8 · Editing documentation

Docs are gated here, so a few things are worth knowing before you edit a page.

**Every command a document names must exist**, and every command that exists must
be named by a document. Both directions are checked. The getting-started page
once opened by telling a stranger to run a binary that had never existed in this
tree; separately, fifteen tools had never been named in any runnable line,
including one the window told the reader — in Hebrew — to go and run.

If you add a binary or an example, give it a line in
[`docs/tools.md`](docs/tools.md). If it is genuinely not for a reader, say so in
that test's `NOT_FOR_A_READER` list with a reason that would survive somebody
arguing with it.

**Every relative link must resolve inside this repository.** A `../../Ksav/…`
link works on the one machine with both checkouts and is broken for everybody
reading on GitHub.

**Numbers in `README.md` are marked and re-counted.** A count is followed by an
invisible HTML comment naming the measurement, and a test re-measures it on every
push. If you change something a marked number counts:

```sh
tools/readme-numbers.sh
```

A number spelled as a word cannot be marked, by design — the claim that started
all of this was *"a window and **fifty** commands"*: a word, unsearchable, and
wrong by 50 for weeks.

**Some files are generated. Do not hand-edit them.**

| File | Written by | Checked by |
|---|---|---|
| `docs/shortcuts.md` | `girsa-card`, from the key table the window resolves against | `tools/check-card.sh` |
| the Ksav test fixture | `--example fixture-packet` | `tools/check-ksav-fixture.sh` |
| `README.md`'s marked numbers | `tools/readme-numbers.sh` | a test, on every push |

The pattern is the same in all three: the tree is the source, the file is the
copy, and a copy nothing regenerates is a copy that rots.

**Prose is not checked**, and that is the honest limit of all of this. Nothing
here would have caught *fifty*. If you are writing a sentence that makes a claim
about this repository, consider whether it can be a marked number or a test
instead.

---

## 9 · Where to ask

Open an issue. If it is a bug, the shape that helps most is the one this project
uses on itself: what you did, what happened, what you expected, and the evidence
— the exact output, not a paraphrase of it.

If you are unsure whether something is in the *stop and ask* table, ask. Asking
is free.
