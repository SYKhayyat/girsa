# Handoff — 21 August 2026

**Working tree clean, in sync with both remotes, `node tools/verify.mjs` green
9 of 9 before every commit, and `cargo test -- --ignored` green against a real
11 GB shelf.** `sefer-crates` is pinned at `a324852` (0.5.6). Nothing is at risk
and nothing is half-finished on disk.

> **Everything left on this page is something nobody at this desk can do.**
> That is the whole point of the rewrite: it used to be a work list with the
> blocked items filed among them, and every item that *could* be worked has been
> worked. What is here now needs a Mac, a photograph, a printer, a signing key, a
> pair of hands, or a zman of somebody learning — and the last of those outranks
> all the rest.

Point a new Claude Code session at this file and it has everything. The
assessment it refers to is [`where Girsa stands`][stands], and the memory files
under `.claude/projects/…/memory/`, which a session loads on its own. When the
items below stop needing what they need, delete this page.

[stands]: https://claude.ai/code/artifact/827950db-f162-45df-bbe3-4ad78fb6bf36

---

## Where to look first

| | |
|---|---|
| What it does, for a reader | [`docs/start-here.md`](docs/start-here.md) |
| When it will not do it | [`docs/troubleshooting.md`](docs/troubleshooting.md) |
| The rules that bind every change | [`BUILDER.md`](BUILDER.md) §0 |
| What is honestly not done | [`docs/not-yet.md`](docs/not-yet.md) |
| Why anything is the way it is | [`docs/the-record.md`](docs/the-record.md) |

```sh
node tools/verify.mjs          # the gate, from the repository root. Nine steps.
                               # A resume prints what it skipped and names the
                               # one case that matters: cargo fmt moves counts
                               # the README states and step 2 re-measures.
bash tools/check-card.sh       # docs/shortcuts.md against girsa-card
bash tools/check-ksav-fixture.sh
cargo test -- --ignored        # the nineteen that need the corpus
```

Working on Girsa and `sefer-crates` at once: `.cargo/config.toml` carries a
commented-out `paths` override with the reasoning on it. Use that, never
`[patch]`, and take it out before committing.

---

## 1 · Nikud on WebKit is unknown — *needs a Mac*

CI has a macOS job; the Rust half passes and the shell compiles against macOS's
WebKit bindings. What that does not settle is rendering: the eyes tool drives
Chrome, and Chrome on macOS is the same Blink it is on Windows — a second
machine, not a second engine. W9 asks about Safari's WebKit, which is what the
shipped window there uses.

## 2 · Nothing has been run against a photographed sefer — *needs a photograph*

Born-digital pages put through named degradations score 89.9% clean and **29.4%**
with all of them at once. No single degradation costs more than five points, so
reasoning about the parts would have been wrong by a factor of ten. It is still
a proxy — no uneven lighting, no gutter shadow, no show-through, no 1880 print —
so 29.4% is a floor and not a photograph.

## 3 · Nothing has come out of a printer — *needs a printer*

The print stylesheet is measured rather than assumed: `tools/eyes.mjs` asks the
browser to pretend the medium is paper and checks what `@media print` then does —
the sheet in the flow, the application gone from it, black ink on a white page
whatever the reader's theme is, and a se'if that does not break across two
sheets.

What is untested is everything after `window.print()`. No dialogue has been
accepted, no sheet has come out, and on a machine whose printer is a PDF writer,
where the file lands is unverified too.

## 4 · Two release-shaped things — *needs a password and a key*

A local Linux build under WSL, which needs your password and which CI covers the
same ground as. And macOS signing and notarisation.

The Intel Mac row of the bundle matrix is **not** on this list: it is a
deliberate omission written into `ci.yml`, not a backlog item.

## 5 · Nobody has dragged a sefer with a mouse — *needs hands*

What a drop *means* is a tested function, refusals and all, and two of the three
surfaces that had never been touched by a pointer now have been — with
`Input.dispatchMouseEvent` and `Input.dispatchKeyEvent` against the running
window, which go through hit-testing, focus and every listener a real click
would.

What remains is the **native HTML5 drag**, and it remains for a reason rather
than for want of trying: a press, a move and a release do not synthesize into a
`dragstart` through the debugging protocol, and a file drop is an operating
system event no browser can raise at all.

## 6 · Installing an update — *needs a release-signing key only you can make*

Girsa checks for a newer release on a button and refuses to install one. The
button half is deliberate — spec.md §14, *offline is the product*, and a window
that has not been asked makes no request, keeps no timer and needs no setting to
turn it off, which is a stronger promise than a setting that defaults to off.

The install half needs the key:

```sh
npm run tauri signer generate -- -w ~/.tauri/girsa.key
```

Public half into `app/src-tauri/tauri.conf.json` under `plugins.updater.pubkey`,
private half and its password into the repository secrets, add
`tauri-plugin-updater`, and the plugin does the rest. An updater that ran an
unsigned binary off the internet would be the worst thing in the application by
a distance, which is why there is not one.

---

## Ruled on, and here only so nothing files them as defects

**Mobile.** You said it: *"android might be hard — i dont need it, so you can
forego it."* It is a platform choice made when Tauri was chosen.

**`אהיה` is the one shem not touched**, and not for want of a swap — ה for ק
gives `אקיק` and the length holds. It is left because `אֶהְיֶה` the shem and
`אֶהְיֶה` the plain verb *I will be* are pointed identically, mark for mark, so
unlike every other conditional shem there is nothing to hang a guard on.
Changing all of them rewrites `וְאֶהְיֶה עִמָּךְ`, which is a promise and not a
Name; catching only `אהיה אשר אהיה` needs the word *after* it, and that module
looks backwards only. Reopen it if you know a mark that separates them.

**Rambam Yomi, Amud Yomi and Daf Yomi Yerushalmi are not written.** Rambam has
three tracks and 1,000 perakim that do not divide by three, so it is a published
calendar rather than a formula, and *Amud Yomi is not one program*: there is a
1973 one and Dirshu's, and some run five to seven amudim a week rather than one
a day. Picking one would be a guess about which luach a reader keeps.

**224 of 7,627 chalakim still cannot be reached by typing their name**, and that
number is where the work stopped rather than where it failed — see below.

---

## The one that outranks all of it

**Nobody has learned a sugya in it.**

Everything above is what people who built the thing can tell you about the thing
they built. None of it is the finding a zman of real use would produce, and the
three documents that come closest — the
[five-minute report](docs/the-five-minute-report.md), [the second
sitting](docs/the-second-sitting.md) and [the third
sitting](docs/the-third-sitting.md) — are between them an hour, eighteen
complaints and twenty-four findings from people who opened it once. Every one of
them found things no test had.

The pattern has held every single time. The two CSS defects came from **looking**
at four surfaces nobody had looked at. `1 arrangements` came from **clicking
Keep**. A WebKitGTK window drawing its find bar over its own toolbar came from
**looking at a photograph**. The largest resolver gain of the whole project came
from **reading this page and disagreeing with it**. And the last session's seven
findings came from **running the commands the documentation tells a reader to
run** — one of which had never produced a green result and two of which would
have made the tree worse.

Not one of them came from reasoning about the code.

That is the shape of the evidence still missing, and no amount of work on
anything above substitutes for it.

---

## The measurement that stopped rather than finished

`measure-branch-citations` is the one open number, and it is here rather than in
the list above because what is left of it is either working as designed or worth
less than it costs.

```sh
cargo run --release -p girsa-search --example measure-branch-citations -- corpus
```

**7,403 of 7,627** chalakim are reachable by typing their name, from 5,502 in
early August and 6,461 at the start of 21 August. The 224 that are not are
counted apart, because they have four different causes and only one of them is a
defect:

| | |
|---|---|
| **117** | a word of the name was read as a level label and its number taken, in works where **more than the first level** was cut. The guard added on 21 August looks at the front of an address; extending it further in is more of the same grinding for a smaller return each time |
| **94** | the name is not one the schema carries in a form this can match — mostly works whose *titles* are sentences with semicolons in them (`בלשון עתיד; חזון ליהודים…`), which is a lexicon question and not a sections one |
| **13** | **refused on purpose.** The schema calls two sections by one name — the Chafetz Chaim really does have two `הקדמה` — and BUILDER rule 6 says an ambiguity is shown and never picked from. These are the guard working |

The residue is the same shape in both directions: a name a schema uses twice,
and a title nobody would type as written. Neither is a rule waiting to be
discovered, which is the difference between this and every earlier round of the
same measurement.
