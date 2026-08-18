# Handoff — 18 August 2026

**`main` at `123ab41` plus the third sitting, and `sefer-crates` at `779225d`
(0.5.5). Working tree clean, in sync with both remotes, and
`node tools/verify.mjs` was green 9 of 9 before every commit.** Nothing is at
risk and nothing is half-finished on disk.

> **A reader sat with the shipped window on 18 August and wrote down
> twenty-four things.** All twenty-four are closed at the root and re-measured in
> that window against his own 7,189-sefer shelf —
> [`docs/the-third-sitting.md`](docs/the-third-sitting.md) is the record, with
> the two root causes that account for six of them. None of it touches the list
> below, which is still the list: nothing he found was in the corpus, the
> resolver, the addressing or the speed. `npm run eyes` went from 47 assertions
> to 60 over the two findings only a browser could have caught.

This page replaces the handoff of 16 August. Five of that page's seventeen items
are closed and are not repeated here; the rest are renumbered, because a list
whose numbers have holes in it is a list nobody trusts. It is still a **session
handoff** and not a permanent document: when the items below are done, delete it.

> **A note on where the last four items came from.** Items 1, 2, 9 and 11 were
> all closed or moved by *Shaul reading this page and arguing with it* — not by
> anybody working the list. The largest of them, item 2, this page had filed as
> needing a lexicon and therefore unreachable; it took one sentence — *a number
> will never have two tens digits* — to be wrong about that. That is item 12
> arriving from a fifth direction, and it is worth reading before starting on
> anything below.

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
| `534a565` `e962f7c` | Two CI failures that were about CI. The `nixos` job was rate-limited by `api.github.com` because an unpinned flake asks it who HEAD is on every run and was asking anonymously; it uses the run's own token now. And the macOS runner failed the correction-cost slope at 8.4 against an allowance of 6 by being **fast** rather than slow — the empty-layer baseline fell from 311 ms to 33 ms while the corrected end stayed put, so the ratio grew with nothing having got slower. The slope is asserted only above a baseline where a ratio means something, and says out loud when it is not. |
| `779225d` (sefer-crates) | **A numeral is the canonical spelling of its own value, and *descending* never was enough.** The rule said a numeral's letters must not ascend. `ייחוד` is 10, 10, 8, 6, 4 and does not ascend, so it read as 38 — and a test in `sefer-crates` pinned that, on the stated reasoning that separating a name from a numeral needs a lexicon. **That reasoning was wrong and the counter-example was in the sum:** thirty-eight is written `ל"ח`. Nobody reaches for a smaller letter when a bigger one covers the amount, so comparing the letters against `to_bare_letters` of their own total decides it without knowing one word. Measured over the shelf: **6,461 of 7,627 chalakim land, from 6,057** — 404 more, against 555 for the whole of 0.5.4. |
| `e9ef803` | **The shemos default to changed, and `אדני` is written.** `אדני` → `אמני` (ד for מ), guarded on the kamatz under the nun because `אֲדֹנִי הַמֶּלֶךְ` is *my lord the king* said to a person — the same guard `אל` and `שדי` carry. The setting now defaults **on**: the harm runs one way, since turning it off is one click and there is no click that un-prints a page. Four tests here asserted the old numeral readings and were rewritten rather than deleted — including the ambiguity guard, whose fixture the new rule had quietly disarmed. |
| `93f4979` | **`1 arrangements`, all twenty-five of them.** The window's `fill` substitutes and stops; Rust's `girsa_plain::said` can count and the window could not. Nine rows now read `שולחנות: {desks}` — a label with its count after it, correct at every number in both languages, with no agreement rule and no Hebrew guessed. Numbers are grouped to match `girsa_plain::thousands`. |
| `1ad8f26` `6a775d7` `3c5c5cd` | **A WebKitGTK window has been opened on NixOS and photographed — 830 colours, eight seconds**, and the picture is an artifact of the `nixos` job. Four red runs to get there, each a different real thing: an apostrophe that ended the container's shell script, `api.github.com` rate-limiting an unpinned flake, a guard grepping for a CSS class in a binary whose assets are brotli-compressed, and **no GL stack in the container at all** — the Debian list the flake was translated from never had to name mesa, and WebKitGTK has needed EGL since 2.42. Then the first person to look at the picture found a defect in it: the find bar drawn over the toolbar of a window with no sefer open, because `.find-here` sets `display: flex` and an author's rule beats the browser's `[hidden]`. Of everything toggled by `hidden`, it was the only panel missing its own `[hidden]` line. |

Against Otzaria the standing score is unchanged at **12 ahead / 7 level / 2
behind / 0 absent** over twenty-one axes. Both of the two behind — mobile, and
installing an update rather than checking for one — are inside item 11 below,
and both are decisions rather than backlog.

Working on both trees at once: `.cargo/config.toml` carries a commented-out
`paths` override with the reasoning on it. Use that, never `[patch]` — `[patch]`
rewrites `Cargo.lock` and drops the git pin out of it, so one distracted
`git add -A` breaks a fresh clone. Take the override out before committing.

---

## Everything still to do

The list is shorter than the last one, and it is also differently shaped, which
matters more. What is left divides three ways, and the divisions are not a way
of excusing the leftovers — they are the answer to *what would it take*, which
is the only question a handoff is for.

**Six of the twelve are closed.** 1, 4, 9, 10 and half of 11 went in one
session, and item 2 went from *needs a lexicon* to 404 more chalakim. What is
left divides as before:

- **1–3 are work**, and a session with this file can start on any of them —
  though 1 and 4 are now done, so 2 and 3 are what is actually there.
- **4–8 need a resource nobody at this desk has** — a Mac, a photograph, a
  printer, a password, a source you can actually check.
- **9–11 are yours to rule on.** Nothing is queued behind them and nothing
  should be. A gap analysis that files one of these as a defect has misread it.

And then item 12, which outranks all of them.

---

### 1–3 · work

#### 1 · ~~The window says `1 arrangements`~~ — *done, `93f4979`*

Closed. Nine rows carry the count after a colon — `שולחנות: {desks}` —
which agrees at every number in both languages without an agreement rule and
without a word of Hebrew guessed. `fill` groups thousands now, matching
`girsa_plain::thousands`, which it had never done.

**Two Hebrew wordings in that change are the assistant's judgment and want a
second pair of eyes**, because they moved a word rather than a number:
`chainForkFarWitnesses` reads `הקרוב שבהם במרחק {steps}` (it was
`{steps} צעדים מכאן`, which says `1 צעדים` at one), and `chainTally` is now two
labelled counts rather than a sentence.

#### 2 · 1,166 chalakim still miss — *and the remainder is a different family*

Two rules have landed, and the second came from arguing with this page rather
than from working it.

`sefer-crates` 0.5.4 taught the resolver that a level word at the **head** of a
name is part of the name: **5,502 → 6,057**.

0.5.5 then closed what this page had called unfixable. A numeral is not a run of
letters that fails to ascend, it is **the canonical spelling of its own value** —
thirty-eight is `ל"ח`, so `ייחוד` is a word; twenty is `כ'`, so `יו"ד` is Yoreh
De'ah; three is `ג'`, so `בא` is the parsha. **6,057 → 6,461**, and no lexicon
was needed.

Measure before theorising about the rest:

```sh
cargo run --release -p girsa-search --example measure-branch-citations -- corpus
```

The 1,166 that remain are visibly *not* the old family. Skimming them, they are
parsha and haftarah names and a `שער` / `מערכת` group — `אדרת אליהו כי תצא`,
`אהבת יהונתן הפטרת יום א' של פסח`, `עין זוכר מערכת א`, `עטרת זקנים שער`. `כי תצא`
never was a numeral: כ, י, ת ascends. Something else is eating these, and
nobody has measured what.

**The one case that genuinely needs a lexicon is now narrow and is written
down:** `נח` is 50, 8, which is exactly how 58 is written. A word spelled the way
its own number is spelled cannot be told from the number by any rule about
spelling. That — and not the whole long tail — is what a lexicon would buy.

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

#### 4 · ~~The luach knows one limud~~ — *done, and it knows two*

**The ruling was Daf Yomi and Mishnah Yomis only, and Mishnah Yomis is
written.** Two mishnayos a day through all 4,192 of Shas, with the perek table
**generated from the corpus** by `examples/mishnah-table.rs` rather than typed —
525 numbers cannot be recalled, and one wrong one is a wrong limud for a day
with nothing to catch it.

Three tests hold it up: the table is 63 masechtos summing to 4,192; all three
published cycle starts open <span dir="rtl">ברכות א':א'-ב'</span> and both spans
are exactly 2,096 days; and every day of a cycle names a mishnah that exists,
with the total seen coming to 4,192 so none is learned twice.

**One thing found by the tests rather than by reasoning.** A day *does* straddle
two masechtos — Berachos holds 57, which is odd, so day 29 is
<span dir="rtl">ברכות ט':ה' — פאה א':א'</span>. The assert written to confirm
the opposite is what disproved it, and the proof it is correct is in the dates:
padding an odd maseches would make a cycle longer than 2,096 days.

The three that stay unwritten, and why. Rambam Yomi, Amud Yomi and
Daf Yomi Yerushalmi are not to be written — Rambam has three tracks and 1,000
perakim that do not divide by three, so it is a published calendar rather than a
formula, and *Amud Yomi is not one program*: there is a 1973 one and Dirshu's,
and some run five to seven amudim a week rather than one a day. Those are not
lookups, and picking one would be a guess about which luach a reader keeps.

The three anchors it rests on, each of them published and none of them fed into
the arithmetic that produced the other two:

| | |
|---|---|
| 12th cycle | 22 Tammuz 5770 · 4 July 2010 |
| 13th cycle | 20 Adar-B 5776 · 30 March 2016 |
| 14th cycle | 21 Teves 5782 · 25 December 2021 |

4,192 mishnayos, two a day, 2,096 days a cycle. **Both spans are exactly 2,096
days.** The 2016 and 2021 dates were computed forward from the 2010 anchor
*before* they were found published, so they fall out of the arithmetic rather
than into it. And the corpus's own count agrees: 62 `mishnah-*` works plus
`pirkei-avot` — which Sefaria does not name `mishnah-*`, so a glob finds 62 and
not 63 — come to **4,192 exactly**. (The same glob also sweeps in
`mishnah-berurah` and its 17,418 segments, which is the shape of mistake that
would produce a silently wrong luach.)

The fork this posed was whether `limudim(rd)` should gain corpus access or keep
being pure arithmetic. It kept the arithmetic: the table is generated **once**,
pasted in, and asserted — so the luach stays a function of a day and nothing
else, exactly as Daf Yomi is.

*Where:* `crates/girsa-app/src/luach.rs`, `examples/mishnah-table.rs`,
`docs/tools.md`

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

#### 9 · ~~One case the numeral guard deliberately gets wrong~~ — *the cost is gone*

Closed by item 2, and the shape of the closing is the interesting part.

The guard put a number back where the schema counts nothing at the work's top
level, and it was paid for with one named wrong answer: `ערוך השולחן כ' א'`
opening Yoreh De'ah siman `א'`. That price depended on something mapping 20 back
to `יורה דעה`, and the only thing that ever did was `יו"ד` parsing as a numeral.
**Under the canonical rule it does not**, so that citation fails visibly now
instead of opening the wrong chelek.

**The guard stays, measured rather than argued:** with `read_as_a_number` 6,461
chalakim land, without it 6,235. It is worth 226 — cases like `נח`, which really
is spelled the way its own number is spelled.

*Where:* `crates/girsa-corpus/src/sections.rs`

#### 10 · ~~The daf turns over at midnight~~ — *done, it turns over at seven*

**A stock hour the reader can change**, rather than a location prompt. Where
nightfall falls needs to know where the reader is standing, and Girsa will not
ask — that would be the first thing this application ever asked about the person
using it. So the hour is an approximation and the setting says so, which is
honest in a way both alternatives are not: midnight is silently wrong for four
hours a day, and a fixed hour presented as a computed tzeis is a lie that looks
precise.

All five pieces landed: `luach::at(date, hour, turns_at)`, `Session
::day_turns_at`, the `set_day_turns_at` command (which **refuses** an hour above
23 rather than clamping it, because a clamped 23 hides a wiring bug), `main.ts`
sending `now.getHours()` beside the date it already sent, and a 24-row control
in the settings panel labelled `19:00`, which needs no word in either language.

**The part that mattered more than the daf.** `of_fixed` derives the civil date
from the shifted day, so after the turnover the Hebrew date, the weekday and the
daf all say tomorrow *together*. Left alone, the panel would have shown the 4th
of January over the 5th's daf — internally inconsistent, externally plausible,
and worse than the midnight behaviour it replaced because it would have looked
right. It is asserted directly.

*Where:* `crates/girsa-app/src/luach.rs`

#### 11 · Two shemos, mobile, and installing an update

Three rulings in one item, because all three are settled and are here only so
that nothing files them as defects.

**One shem is not touched, and it is `אהיה`.** `אדני` is written now — ד for מ,
giving `אמני`, which preserves the length the way every other substitution in
that module does. It is guarded on the kamatz under the nun, because
`אֲדֹנִי הַמֶּלֶךְ` is *my lord the king*, said to a person; on an unpointed page
it does nothing, which is the same cost `אל` and `שדי` already pay.

`אהיה` is left, **and not for want of a swap** — ה for ק gives `אקיק` and the
length holds. It is left because `אֶהְיֶה` the shem and `אֶהְיֶה` the plain verb
*I will be* are pointed identically, mark for mark, so unlike every other
conditional shem there is nothing to hang a guard on. Changing all of them
rewrites `וְאֶהְיֶה עִמָּךְ`, which is a promise and not a Name; catching only
`אהיה אשר אהיה` needs the word *after* it, and that module looks backwards only.
Reopen it if you know a mark that separates them.

**And the setting is on by default now**, which it was not: `#[default]` sat on
`AsWritten`, so the substitution only happened for a reader who went looking for
it. The harm runs one way — turning it off is one click, and there is no click
that un-prints a page.

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

The pattern held again today, five times. The two CSS defects were found by
**looking** at four surfaces nobody had looked at. The nixos failure was found
by **reading a log** that had already printed `Finished dev profile in 15m 39s`.
`1 arrangements` was found by **clicking Keep**. A WebKitGTK window drew a find
bar over its own toolbar, and that was found by **looking at a photograph**.

And the largest of the five was found by **reading this page and disagreeing
with it**. Item 2 said the `ייחוד` case needed a lexicon and was therefore out
of reach; one sentence — *a number will never have two tens digits* — was enough
to show it was not, and 404 chalakim followed. The page had the wrong idea
written down confidently, with a test pinning it, and four green suites agreeing.

Not one of the five came from reasoning about the code.

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
