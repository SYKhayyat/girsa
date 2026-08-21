# When it will not do what it should

Every failure in here is one somebody actually met — in the window, in the gate,
or on a runner. Nothing is hypothetical, and where a fix is *run this command*
the command is one that exists in this tree.

If you are setting up rather than repairing, [`start-here.md`](start-here.md) is
the five minutes and [`../CONTRIBUTING.md`](../CONTRIBUTING.md) is the setup.
This page is for when one of those did not go the way it reads.

---

## How to read a failure Girsa gives you

Three things are worth knowing before any particular symptom, because they turn
most of this page into a lookup.

**The sentence you see is written for you. The machine's words are on hover.**
Every user-visible failure names what failed, the thing you can act on, and one
place to look; the raw error — the `io::Error`, the compiler's line, whatever a
crate's `Display` produced — is on the `title` of the message. If a sentence is
too vague to act on, hover it before doing anything else. That is where the
detail went, deliberately: `app/src/trouble.ts` exists because a Hebrew window
once put `could not reach ksav: connection timed out` in a toolbar chip.

**A refusal carries a name.** Errors crossing from Rust to the window are
prefixed with a code — `no-index: there is no index here` — and the window looks
the sentence up by the code rather than by matching English prose. So the code is
stable even when the wording is not, and [the table below](#the-refusals-by-name)
is the honest index of *what the application can refuse and what to do about it*.

**Empty is not the same as unknown, and the window says which.** A sefer with no
links says *nothing links here* when the cache has been built and *I have not
been told* when it has not. Read which one you were given before concluding
anything: the second is a step nobody took, and more of this page is that than
is anything else.

---

## The window opens and there are no seforim

**What you see:** *there is no shelf here — the import may not have run*
(`no-shelf`), or the first-run screen with a folder picker.

The window looks for the corpus in three places, in this order: `GIRSA_CORPUS`,
then beside the executable, then two levels up from it. Nothing about a missing
corpus is an error — a fresh install *is* a window with no seforim — so the
question is only ever which of the six steps in the README's [Getting
it](../README.md) table has not been run.

Check what the shelf actually holds without opening the window at all:

```sh
cargo run -p girsa-app --bin girsa-shelf corpus personal
```

If that prints seforim and the window does not see them, the window is looking
somewhere else: set `GIRSA_CORPUS` (and `GIRSA_PERSONAL`) and reopen it.

**If instead you picked a folder** and were told *there are no seforim in that
folder — choose the one the import wrote to* (`not-a-corpus`), the window is
answering a different question and the distinction is deliberate: you chose a
folder that is not a corpus root. The root is the one `girsa-import` was given —
the directory that has `works/` in it — not the Otzaria download, and not the
folder you unpacked either of them into.

---

## A library you added is not on the shelf

**What you see:** `girsa-import` ran, said nothing was wrong, and the seforim you
put in a second library are not there.

`girsa-import corpus <library>...` reads a library through its **`אוצריא/`
directory**, the same way it reads Otzaria's. A tree whose seforim sit directly
in its root, or under a folder called anything else, gets
`CatalogueError::Missing` naming the directory it wanted — that one is loud.

The quiet one is **precedence**. A title an earlier library already supplied is
not read again from a later one, and Sefaria outranks all of them, so a sefer
that is already on the shelf under the same name will not appear twice and will
not say why. That is the intended behaviour and it is how a second library adds
what it alone has without arguing about the rest — but it does mean *the sefer I
added is missing* and *the sefer I added is already here under another
library's copy* look identical from the window. The line at the top of the run
is the one to read:

```text
7311 works: 5637 shared (Sefaria supplies them), 1100 Otzaria-only, 574 Sefaria-only
```

Grep `corpus/works/index.jsonl` for the title to settle it.

## A sefer says nothing about where it came from

**What you see:** a work on the shelf with no edition, no provenance and no
licence, where the ones beside it have all three.

Working as intended. A library states those in a `library.json` at its root:

```json
{ "edition": "OtzarLib", "provenance": "https://github.com/YairDaniel123/OtzarLib" }
```

A tree with no such file has nothing recorded for it, and Otzaria's own library
is the one exception — recognised by the `metadata.json` beside its `אוצריא/`.
`girsa-import` prints what it decided for each library before it does any work:

```text
library C:/Users/…/otzaria_latest: Otzaria (Unlicense)
library C:/Users/…/otzarlib:       OtzarLib (no licence stated)
```

**This used to be a constant**, and every work walked out of any `.txt` tree was
stamped `Otzaria`, Sivan22's repository, `Unlicense`. That was true of the only
tree Girsa had ever been pointed at and false for every other one. Leave
`license` out when you do not know it: a blank is a thing a reader can act on
and a confident wrong licence is not.

If a `library.json` will not parse, the run **stops** and names the file rather
than falling back — silently reverting to *no claim* would hide the reason at
exactly the moment somebody is asking where a sefer came from.

---

## The daf has no mefarshim, on any sefer

**What you see:** the מפרשים button reads `לצד` on every sefer, no `◆` markers in
any margin, the links panel says *I have not been told*.

Steps 4 and 5 have almost certainly not been run. They are the two the
getting-started list was missing until `658c11b`, and a shelf built from the
other four is complete, searchable, and has no link graph at all:

```sh
cargo run -p girsa-link --bin girsa-link-import corpus <otzaria>
cargo run -p girsa-link --bin girsa-link-types  corpus personal
```

The first reads Sefaria's and Otzaria's link data; the second builds the caches
that let a line be asked *what points at me*, which is what a mefaresh is.

**If you installed from a release and cannot find those two binaries**, you have
an archive built before this was noticed: `girsa-tools-<platform>.zip` carried
`girsa-fetch`, `girsa-import` and `girsa-index` and not the link tools. Take a
newer release, or build the two from source with the lines above.

Two more are worth running, and nothing refuses without them —
[`tools.md`](tools.md) says what each buys you:

```sh
cargo run --release -p girsa-app --bin girsa-companions corpus
cargo run -p girsa-link --bin girsa-link-orient corpus
```

Without `girsa-companions` the shelf offers the declared commentaries only and
says so rather than pretending the list is complete.

## One sefer has no links, and the rest of the shelf is fine

**What you see:** `girsa-link-import` printed a line like

```text
<slug>: …/<sefer>.txt: 7026 lines against 7027 segments on the shelf —
the file has changed since the import, and mapping them anyway would anchor
every link in this sefer one segment out
```

and that sefer alone has nothing on any line.

**Usually it means what it says**: the `.txt` was edited or replaced after
`girsa-import` ran, and the two no longer describe the same sefer. Re-run
`girsa-import`, then `girsa-link-import`.

**But check the size of the gap first.** A difference of one or two, in a sefer
with long sections, is the other cause: the importer **splits** a segment too
long to name a place — `#7` becomes `#7.1` and `#7.2` — so a source file's lines
and the shelf's segments are no longer one to one. That used to refuse the whole
mapping and cost six of the ten Encyclopedia Talmudit volumes every footnote
link they had. Fixed on 21 August 2026; if you are seeing it with a small gap on
a build from before that, the sefer is fine and only its links are missing.

`cargo run -p girsa-corpus --example measure-oversized -- corpus` lists what was
split and where.

---

## Search will not search

**What you see:** *there is no search index — build one: girsa-index build*
(`no-index`).

```sh
cargo run -p girsa-search --bin girsa-index build index corpus personal
```

It is the longest of the six steps and the one people leave for later and then
forget — 4.2 GB on the shelf these numbers were measured on. The window opens
without it on purpose, which is what makes it forgettable.

**If search runs but a sefer you know is in the corpus never appears in results**,
the index is older than the import. Rebuild it: the index is derived, and nothing
in the corpus is lost by throwing it away.

**If a hit is badged `OCR`**, that is not a failure. Text off a scan is marked
rather than mixed in quietly, and it is dirtier than the corpus text by design.

---

## Ksav is not connected

The chip in the toolbar says which of three things is true, and they are three
different problems:

| The chip says | What it means | What to do |
|---|---|---|
| *Ksav is not running* (`post-not-running`) | no endpoint file, or the pid in it is gone | open Ksav |
| *Ksav is not connected* (`no-desk`) | Girsa has no desk to send to | open a document in Ksav |
| *…was not answered in time* (`post-unreachable`) | something is holding the port and not answering — commonly a Ksav that was killed rather than closed | close what is left of it and reopen |

The third is the one that used to cost eight seconds of waiting per attempt.
Girsa asks the operating system whether the pid in the endpoint file is still
alive before it tries the socket, and a dead pid short-circuits — but only a dead
one. A live pid proves nothing about *which* process holds that port, so it falls
through to the health check, which stays the authority.

**`Ctrl+Shift+C` does nothing:** if the window says *nothing is chosen — highlight
what you mean first* (`nothing-chosen`), it is not a failure. Nothing was
highlighted.

---

## The window draws nothing, or opens white

**On Linux, and on NixOS especially.** Set it before the window starts:

```sh
WEBKIT_DISABLE_COMPOSITING_MODE=1
```

Every Tauri application on Linux needs it under a compositor that WebKitGTK does
not get on with, and without it the window opens, draws nothing, and exits
cleanly — which means *a process that lived is evidence of nothing*. `nix
develop` sets it for you; `flake.nix` is where.

**In a container or over Xvfb**, there is a second half: an Xvfb screen has no
graphics card behind it, and WebKitGTK has needed EGL since 2.42. A mesa stack
has to be in the environment or the window is blank for that reason instead.
`tools/nixos-window.sh` is the worked example — it opens the window, waits for
the screen to stop being blank, and counts the colours on it, because a
screenshot that is one colour is a failure no exit code will report. An Xvfb root
is one colour; a drawn page is hundreds.

---

## Building

| What you see | What it is |
|---|---|
| `apt` exits 100 before a compiler runs | `libappindicator3-dev` and `libayatana-appindicator3-dev` conflict. Install the ayatana one only — Tauri v2 wants it |
| `build.rs` panics naming another command | you ran `cargo build --release -p girsa-shell`. That embeds no frontend and produces a window that navigates to the dev server. Use `npx tauri build` from `app/`, which is what the panic says |
| the shell will not compile, missing `app/dist` | the frontend has not been built. `npm --prefix app run build`, or use `tauri build`, which does it for you |
| `cargo metadata` fails before any compiler | historical, and worth recognising: the manifest used to name a sibling checkout. The shared crates are pinned by git rev in `Cargo.toml` now and cargo fetches them itself |
| the first `tauri dev` takes fifteen minutes | it is compiling a BERT and tantivy. Every run after that is seconds |

**Working on Girsa and `sefer-crates` at once:** `.cargo/config.toml` carries a
commented-out `paths` override with the reasoning on it. Use that, and take it
out before committing. Never `[patch]` — it rewrites `Cargo.lock` and drops the
git pin out of it, so one distracted `git add -A` breaks a fresh clone.

---

## The gate is red

```sh
node tools/verify.mjs
```

Nine steps in two lanes, and **neither lane short-circuits the other** — so one
run tells you everything that is wrong, and two reds are two problems rather than
one problem twice.

| Step | When it is red |
|---|---|
| 1 `build` | ordinary. Everything after it is noise until it is green |
| 2 `test` | includes the checks that read this repository's own documentation. A prose edit can fail here, and that is working as intended |
| 3 `clippy` | `-D warnings`, so a lint is a failure |
| 4 `fmt` | run `cargo fmt --all`. **Then read the next paragraph** |
| 5–6 shell clippy and fmt | `app/src-tauri`, which the first four do not build: `default-members` excludes it. This is where interop breakage shows up |
| 7 `types` | `tsc --noEmit` in `app/` |
| 8 `window tests` | `node test/run.mjs` — 24 files |
| 9 `eyes` | a real browser against real CSS. With no browser installed it says so and passes; CI sets `EYES_REQUIRED=1` so that a missing browser is a failure there |

**The one trap in `--from`.** Resuming is honest when step 4 failed and you do not
want to rebuild the world to get back there. But the usual fix for step 4 is
`cargo fmt`, which **rewrites source files** — and two of those files' line counts
are numbers `README.md` states and **step 2** re-measures. So `fmt` red → `cargo
fmt` → `--from 4` skips the check the fix just invalidated. The runner now prints
what a resume skipped and names this case; when in doubt, run the whole gate.

**If step 2 fails on a number**, that is the README's marked-number check. The
fix is one command:

```sh
tools/readme-numbers.sh
```

Run it *after* `cargo fmt`, never before — formatting moves the counts that
script writes.

**Two checks are CI-only and will not be run by the gate**, because they need
something a developer's machine should not need on every change:

```sh
bash tools/check-card.sh          # docs/shortcuts.md against what girsa-card prints
bash tools/check-ksav-fixture.sh  # the packet Girsa sends against the fixture Ksav asserts on
```

Both take `--write` to accept the new output. The second wants Ksav as a sibling
checkout, or `KSAV=/path/to/Ksav`.

---

## The browser build is lying to you (about writes)

```sh
cargo run -p girsa-app --example dev-fixtures -- corpus app/public/dev
npm --prefix app run dev
```

Same `app/src`, same stylesheet, real text off the shelf — served by Vite over
static JSON instead of by the shell over IPC. **Reads are real. Writes are
no-ops, and they fail silently rather than refusing.**

The one everybody meets first: tick a mefaresh and the box reverts about a
millisecond later with nothing in the console. `api.ts` answers *tick a mefaresh*
with the same static file it answers *read the list* with, so the picker
faithfully redraws from an unchanged list. Neither that nor *open the ticked ones*
finding nothing is a defect.

Two more that look like bridge failures and are not: the resolved shortcut table
ships in `state.json`, so shortcuts work out here — but a shortcut you just added
is not in that file until you re-run `dev-fixtures`; and Escape closes the
mefarshim picker only while focus is still in its filter box.

**The rule: if the thing you changed is state, check it in `tauri dev`.** Layout,
typography, Hebrew and nikud, RTL and the shape of a panel are all honest in the
browser.

---

## Tests

**`cargo test` needs no corpus.** The suite builds its own shelf from
source-shaped input through the real importer, in about two seconds.

**`cargo test -- --ignored` needs one.** Nineteen checks are facts about a real
Sefaria release — *Orach Chayim is 697 simanim of 4,171 se'ifim* — and no fixture
can stand in for them. On a machine that has run `girsa-import`, all nineteen
should pass; if they fail at the first one, check `GIRSA_CORPUS` before reading
any further.

**A test that passes because it could not find its input is a bug here**, by
rule. If you are about to write `if !present { return }`, either build the input
with `girsa-fixture` or mark the test `#[ignore]` so a run says `ignored` out
loud.

---

## `cargo --version` is not the pinned version

The toolchain is pinned in [`../rust-toolchain.toml`](../rust-toolchain.toml) and
`rustup` installs it, with `clippy` and `rustfmt`, the first time you run `cargo`
in this directory. If your version disagrees, something is overriding the file:
`RUSTUP_TOOLCHAIN` in the environment beats it, and so does a `+stable` on the
command line. Node is pinned the same way in [`../.nvmrc`](../.nvmrc) — `nvm use`
reads it.

Both were, for a while, pinned in the tree and ignored by CI, which is the same
class of problem one layer out: a version named in two places drifts at whichever
place nobody edits.

---

## The refusals, by name

Every one of these is a deliberate refusal with a code on it. The window looks up
its sentence by the code; a test fails if a code Rust can send has no sentence in
the window's table.

| Code | What the window says | What to do |
|---|---|---|
| `no-shelf` | there is no shelf here — the import may not have run | run the import, or point `GIRSA_CORPUS` at the corpus |
| `not-a-corpus` | there are no seforim in that folder — choose the one the import wrote to | pick the root the import wrote, the one with `works/` in it |
| `no-index` | there is no search index — build one: `girsa-index build` | build the index |
| `no-sefer` | no sefer on the shelf is called that | the citation names something that is not imported |
| `will-not-open` | the sefer is on the shelf and will not open — details on hover | hover it. This is a real fault, not a missing step |
| `no-page` | there is no such page in the scan | the scan is shorter than the page you asked for |
| `no-lane` | the adjacent lane is off — you can turn it on in the settings | the semantic lane ships off until a model is side-loaded |
| `no-desk` | Ksav is not connected | open a document in Ksav |
| `offline` | there is no network here | only the update check needs a network. Nothing else does |
| `poisoned` | the internal state is broken — reopen the window | reopen it, and report it: a thread died holding a lock |
| `read-only` | …failed — the personal layer will not take a write | check permissions on `personal/`, and that a second Girsa is not holding it |
| `cycle` | a shelf cannot go inside itself | you dropped a shelf onto its own descendant |
| `nothing-chosen` | nothing is chosen — highlight what you mean first | not a failure |
| `already-there` | something is already there — nothing was replaced | a stop, not an error: the write would have destroyed a document you do not have open |
| `already-working` | the lane is already working — stop it first, or wait for it | one long job at a time |
| `not-kept` | the arrangement on screen has no name, so there is nowhere to put it back | name the desk first, then sit down at the other one |
| `no-clipboard` / `clipboard-refused` | no clipboard is available / the system refused the copy | another application is holding the clipboard. Try again |
| `will-not-serialize` | the source would not pack — copy again | if it repeats, report the line: this one is ours |

---

## None of the above

Two things are worth doing before opening an issue, and they are the two this
project uses on itself.

**Say what you did, what happened, what you expected, and paste the evidence** —
the exact output rather than a paraphrase of it. Hover the message first: the
developer's string is there, and it is usually the half that identifies the
fault.

**Check whether it is already written down as not done.**
[`not-yet.md`](not-yet.md) collects everything this application does not do yet,
each with the argument behind it, and the three sittings —
[the five-minute report](the-five-minute-report.md),
[the second sitting](the-second-sitting.md) and
[the third sitting](the-third-sitting.md) — are what happened when people who had
not built it sat down with it. A surprising amount of *this is broken* turns out
to be in one of those four pages, with a reason.
