# Handoff — 17 August 2026

**`main` at `0517b0d`. Working tree clean, in sync with `origin/main`, and
`node tools/verify.mjs` was green 9 of 9 before every commit.** Nothing is at
risk and nothing is half-finished on disk.

This page replaces the handoff of 16 August. Five of that page's seventeen items
are closed and are not repeated here; the rest are renumbered, because a list
whose numbers have holes in it is a list nobody trusts. It is still a **session
handoff** and not a permanent document: when the items below are done, delete it.

> Point a new Claude Code session at this file and it has everything. The
> assessment it refers to is [`where Girsa stands`][stands], and the memory
> files under `.claude/projects/…/memory/`, which a session loads on its own.

[stands]: https://claude.ai/code/artifact/827950db-f162-45df-bbe3-4ad78fb6bf36

---

## What closed

| Commit | What |
|---|---|
| `31889f2` | **Three packages named a bundler that never runs them.** `appimagekit`, `dpkg` and `fakeroot` sat in the flake's devShell on the theory that `tauri-bundler` shells out to them. It does not: the .deb and the .rpm are written in Rust, and the AppImage is built by `linuxdeploy`, which the bundler **downloads during the build** — a prebuilt glibc ELF naming `/lib64/ld-linux-x86-64.so.2`, an interpreter NixOS does not have, and unlike `node_modules` it is not there to be patched first. `appimagekit` had also stopped being a name in nixpkgs, which is what made the job red. |
| `5f89d30` | **Four surfaces nobody had looked at, and the eye found two of them wrong on its first run.** `npm run eyes` went from 20 assertions to **47**, over four new specimens: the find bar, the grouped links row, the arrangements drawer, and the print sheet under emulated print media. Two real CSS defects fell out of it. Four window modules got tests at the same time — `chips`, `findhere`, `printview`, `desksview` — taking the suite to **24 files, 534 checks**. |
| `9499d38` | **A container has no display until somebody starts one.** The `nixos` job builds the shell with `--features tauri/custom-protocol`, opens it on `xvfb-run`, waits for the screen to stop being blank and counts the colours on it — because without `WEBKIT_DISABLE_COMPOSITING_MODE` the window draws nothing and exits cleanly, so a process that lived is evidence of nothing. |
| `5d22971` | A class on the disabled chip that no rule ever read. |
| `f1903be` | **A level word at the head of a name is part of the name** — `sefer-crates` 0.5.4, and Girsa moved onto it. Over the whole shelf, **6,057 of 7,627** chalakim now land, from 5,502. |
| `bf73ab8` | **The nixos job built for fifteen minutes, correctly, and then said `nix: command not found`.** Its container script was `sh -euc '…'` inside the workflow and one of the comments in it contained the word `job's`. The apostrophe closed the string, the last two commands left the container and ran on the host, and the host has no Nix. |
| `0517b0d` | **Two of the three surfaces nobody had ever pointed at, pointed at.** |

Against Otzaria the standing score is unchanged at **12 ahead / 7 level / 2
behind / 0 absent** over twenty-one axes; the two behind are items 10 and 11
below, and both are decisions rather than backlog.

---

## Everything still to do

The list is shorter than the last one, and it is also differently shaped, which
matters more. What is left divides three ways, and the divisions are not a way
of excusing the leftovers — they are the answer to *what would it take*, which
is the only question a handoff is for.

- **1–3 are work**, and a session with this file can start on any of them.
- **4–8 need a resource nobody at this desk has** — a Mac, a photograph, a
  printer, a password, a source you can actually check.
- **9–11 are yours to rule on.** Nothing is queued behind them and nothing
  should be. A gap analysis that files one of these as a defect has misread it.

And then item 12, which outranks all of them.

---

### 1–3 · work

#### 1 · The window says `1 arrangements` — *a work order, found by clicking*

Rust agrees with its numbers. `girsa_plain::said` has `plural(n, one, many)` and
`counted`, and its own header records that three composers wrote that ternary
**eleven** times before anybody gave it a name.

The window has `fill()`, which substitutes a hole into a string and stops.
**Twenty-five** entries in `app/src/say.ts` interpolate a count. So both columns
are wrong at one, and they are not wrong the same way: English wants a noun with
an `s` on it and Hebrew wants a different phrase, and Hebrew's agreement is not
English's rule spelled differently.

Deliberately **not** fixed at the one place it was noticed. One of twenty-five
is the exact shape of bug this repository keeps rediscovering a year later, and
guessing at Hebrew phrasing for twenty-four more nouns is the thing BUILDER rule
6 forbids. Somebody who knows how each of those reads should do all of them at
once.

*Where:* `app/src/say.ts`, against `crates/girsa-plain/src/said.rs`

#### 2 · 1,570 chalakim still miss — *cross-repo, and the easy half is done*

`sefer-crates` 0.5.4 taught the resolver that a level word at the **head** of a
name is part of the name. `אבודרהם הלכות ברכות הקדמה` used to arrive naming a
section no schema has, because `הלכות` was read as a level label and the rest was
handed back as the name. The whole shelf went from **5,502 of 7,627** landing to
**6,057** — 555 more chalakim reachable by typing them.

What still misses is 1,570, and they are not one cause. Measure before
theorising:

```sh
cargo run --release -p girsa-search --example measure-branch-citations -- corpus
```

One cause is known and is not fixable at that layer. A Hebrew word whose letters
do not ascend is a legal numeral, so `ייחוד` is 10+10+8+6+4 = 38 and
`ברכות שער ייחוד המעשה ב'` resolves to `38:המעשה:2`. There is a test in
`sefer-crates` pinning exactly that, rather than pretending it is not there.
Telling a numeral from a word needs to know which words are words, which is a
lexicon and not a parser.

#### 3 · Nobody has dragged a sefer with a mouse — *and only the drag is left*

What a drop *means* is a tested function, refusals and all. Two of the three
surfaces that had never been touched by a pointer now have been, with
`Input.dispatchMouseEvent` and `Input.dispatchKeyEvent` against the running
window — events the browser raises at the input layer, which go through
hit-testing, focus and every listener a real click would.

What remains is the **native HTML5 drag**, and it remains for a reason rather
than for want of trying: a press, a move and a release do not synthesize into a
`dragstart` through the debugging protocol, and a file drop is an operating
system event no browser can raise at all. That one needs hands.

---

### 4–8 · needs a resource

#### 4 · The luach knows one limud — *needs a source you can check*

`Limud` is a list and holds Daf Yomi Bavli. Mishnah Yomis, Rambam Yomi, Amud
Yomi and Daf Yomi Yerushalmi are **deliberately not written**, and this session
did not write them either.

Daf Yomi's epoch is checked three independent ways in the tests: the
1923-to-1975 span divides by 2,702 exactly seven times, and the 2012 and 2020
cycle starts fall *out* of the arithmetic rather than going into it. Nothing
available offline meets that standard for the others, and a wrong epoch is a
wrong limud every single day.

*Where:* `crates/girsa-app/src/luach.rs`

#### 5 · Nikud on WebKit is unknown — *needs a Mac*

CI has a macOS job; the Rust half passes and the shell compiles against macOS's
WebKit bindings. What that does not settle is rendering: the eyes tool drives
Chrome, and Chrome on macOS is the same Blink it is on Windows — a second
machine, not a second engine. W9 asks about Safari's WebKit, which is what the
shipped window there uses.

#### 6 · Nothing has been run against a photographed sefer — *needs a photograph*

Born-digital pages put through named degradations score 89.9% clean and **29.4%**
with all of them at once. No single degradation costs more than five points, so
reasoning about the parts would have been wrong by a factor of ten. It is still
a proxy — no uneven lighting, no gutter shadow, no show-through, no 1880 print —
so 29.4% is a floor and not a photograph.

#### 7 · Nothing has come out of a printer — *needs a printer*

The print stylesheet is measured rather than assumed now: `tools/eyes.mjs` asks
the browser to pretend the medium is paper and checks what `@media print` then
does — the sheet in the flow, the application gone from it, black ink on a white
page whatever the reader's theme is, and a se'if that does not break across two
sheets.

What is untested is everything after `window.print()`. No dialogue has been
accepted, no sheet has come out, and on a machine whose printer is a PDF writer,
where the file lands is unverified too.

#### 8 · Two release-shaped things — *needs a password and a key*

A local Linux build under WSL, which needs your password and which CI covers the
same ground as. And macOS signing and notarisation.

The Intel Mac row of the bundle matrix is **not** on this list: it is a
deliberate omission written into `ci.yml`, not a backlog item.

---

### 9–11 · yours to rule on

#### 9 · One case the numeral guard deliberately gets wrong

`יו"ד` is Yoreh De'ah and it is also, letter for letter, the number 20 — and the
resolver reads the numeral first. Girsa puts it back where the schema counts
nothing at the work's top level.

The trade is written down on `read_as_a_number`: **`ערוך השולחן כ' א'` now opens
Yoreh De'ah siman `א'` instead of failing.** That citation names no chelek and so
names no place, and the wrong reading announces itself in the margin — but it is
a guess in the one place BUILDER rule 6 forbids one. Removing it is deleting
`read_as_a_number` and its two tests.

*Where:* `crates/girsa-corpus/src/sections.rs`

#### 10 · The daf turns over at midnight, not at nightfall

Where nightfall falls is a function of where the reader is standing, and Girsa
does not know and will not ask for a location. Otzaria makes the same choice and
says nothing about it; here *tomorrow's* daf is named beside today's, so the few
evening hours where the two disagree are visible rather than silently wrong.
Closing it properly means asking for a location, which is a product decision and
not a small one.

*Where:* `crates/girsa-app/src/luach.rs`

#### 11 · Two shemos, mobile, and installing an update

Three rulings in one item, because all three are settled and are here only so
that nothing files them as defects.

**Two shemos are not touched.** `אדני` and `אהיה` are left as written. Every
substitution in that module is one Hebrew letter for one Hebrew letter — same
character count, same byte count — because a span here is a pair of offsets, and
`יהוה` becoming `ה'` would leave every mark, link anchor, search hit and quote
range pointing two characters left of where it was drawn. Neither of those two
has a one-letter swap anybody prints. If you know a convention that preserves
the length, it is four lines.

**Mobile.** You said it: *"android might be hard — i dont need it, so you can
forego it."* It is a platform choice made when Tauri was chosen.

**Installing an update.** Girsa checks for a newer release on a button and
refuses to install one. The button half is deliberate — spec.md §14, *offline is
the product*, and a window that has not been asked makes no request, keeps no
timer and needs no setting to turn it off, which is a stronger promise than a
setting that defaults to off. The install half needs **a release-signing key
that only you can make**:

```sh
npm run tauri signer generate -- -w ~/.tauri/girsa.key
```

Public half into `app/src-tauri/tauri.conf.json` under
`plugins.updater.pubkey`, private half and its password into the repository
secrets, add `tauri-plugin-updater`, and the plugin does the rest. An updater
that ran an unsigned binary off the internet would be the worst thing in the
application by a distance, which is why there is not one.

---

### 12 · the one that outranks all of it

**Nobody has learned a sugya in it.**

Everything on this page is what a person who built the thing can tell you about
the thing they built. None of it is the finding a zman of real use would
produce, and the two documents that come closest — the
[five-minute report](docs/the-five-minute-report.md) and
[the second sitting](docs/the-second-sitting.md) — are between them eighteen
complaints and an hour, from somebody who opened it once. Both found things no
test had.

The pattern held again today, three times. The two CSS defects were found by
**looking** at four surfaces nobody had looked at. The nixos failure was found
by **reading a log** that had already printed `Finished dev profile in 15m 39s`.
And `1 arrangements` was found by **clicking Keep**. Not one of the three came
from reasoning about the code.

That is the shape of the evidence still missing, and no amount of work on 1–11
substitutes for it.

---

## How to pick any of it up

```sh
node tools/verify.mjs          # the whole gate, from the repository root. Nine
                               # steps. Never --from: a resume skips the README
                               # measurement that cargo fmt then invalidates.
bash tools/check-card.sh       # docs/shortcuts.md against girsa-card
bash tools/check-ksav-fixture.sh
```

Driving the real window, which is where three of today's findings came from:

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
cd app; npm run tauri dev
```

Then talk to it over CDP on 9222. Two different things live there and the
difference matters:

- `window.__TAURI_INTERNALS__.invoke(name, args)` in the page context is the
  real Rust command against the real corpus and the real session. It drives the
  application and touches nothing the reader touches.
- `Input.dispatchMouseEvent` and `Input.dispatchKeyEvent` are raised by the
  browser at the input layer, so they go through hit-testing, focus and every
  listener. That is how a control is shown to be *reachable* rather than merely
  wired, and it is what `1 arrangements` fell out of.

Documents worth reading before changing anything:

| | |
|---|---|
| The rules that bind every change | [`BUILDER.md`](BUILDER.md) §0 |
| What is honestly not done | [`docs/not-yet.md`](docs/not-yet.md) |
| What it does, for a reader | [`docs/start-here.md`](docs/start-here.md) |
| The keys | [`docs/shortcuts.md`](docs/shortcuts.md) — `Ctrl+F` is the sefer, `Ctrl+Shift+F` is the shelf |

Every number on this page was read from the running application, from the corpus
on disk, or from a CI log on 16 and 17 August 2026 — not from documentation.
