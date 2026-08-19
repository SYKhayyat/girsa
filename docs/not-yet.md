# What Girsa does not do yet

Every tier in [`../spec.md`](../spec.md) is built and each one is asserted on
something — [`../BUILDER.md`](../BUILDER.md), *What holds, per work order*, has
the twenty rows. That is a true sentence and on its own it is a misleading one,
because seven of the twelve pages in [`the-record.md`](the-record.md) carry a
section saying what the thing just described still cannot do, and those sections
are the only honest account of where this stands.

They were written where the work was argued, which is the right place to argue
them and the wrong place to find them. This page is the same eight lists in one
place, each linking back to the section that makes the case. Nothing here is
new; if this page and the record ever disagree, **the record is right and this
page has rotted.**

---

## Built, and the window cannot reach it

**Empty, and worth leaving here for that reason.** The pattern this section
watched for is the one that costs the most and shows the least: the crate works,
the tests are green, and the only way to it is a terminal. A reader does not have
a terminal, so a tier in that state is a tier that does not exist for them.

The last entry was the transmission chain, which had a library, a terminal tool
and no door for as long as `spec.md` §8 had been "built". It has a panel now.
The heading stays because the next thing built crate-first will belong under it,
and a category with nothing in it is easier to notice filling up than one that
has to be invented again.

## Built on both sides of a seam, and not joined across it

The larger group, and the more interesting one. In each case two pieces of
machinery exist, they are the same shape, and nobody has connected them —
which is a different and generally harder problem than either piece was.

| | Where it is argued |
|---|---|
| **The shell writes Hebrew sentences the reader sees, and works out Hebrew number agreement itself.** The `1 arrangements` half is closed — `93f4979` established that the window says a count as *label: number*, because agreement is `girsa_plain::said`'s job and nobody else's — and the same defect reopened in Rust, twice: `unfix` returned a Hebrew string, and `export_sefer` composed `בלי תיקונים` / `תיקון אחד` / `{n} תיקונים` by hand, in the shell, three weeks later. Both are on **success** paths, which is why `the_shell_does_not_write_sentences_for_the_reader` — the fence written for exactly this — did not see them. The guard looks at four shapes of refusal and there are fifty-three more (`ok_or`, `ok_or_else`, `map_err`), each of them a sentence the shell composed, each reaching a Hebrew reader as *something went wrong*. | [corrections](record/corrections.md#what-has-not-been-checked) |
| **The results header counts a correction the index already has** — the note half is closed. A note absorbed as you write it is no longer reported as unsearchable: what each index has taken in is recorded per work, beside it, and the notes clause reads off that. The corrections clause still answers to the build stamp, so a correction an absorb has already applied is still counted. Left over-reporting rather than widened, because telling which correction belongs to which work means one crate parsing another's records by hand — the string surgery that counter was rewritten to stop doing. | [your own layer](record/your-own-layer.md#what-this-does-not-do) |

## Built narrower than the name suggests

Not gaps so much as the honest width of a thing, written down where the feature
is described so that nobody has to discover it by being disappointed.

| | Where it is argued |
|---|---|
| **Two readings of a line are found at one hop from it.** A sefer that reads this sugya only by way of another sefer counts as a witness to a fork and never as a side of one. Whether that is a limit or the right definition is a question about how a sugya travels. | [the chain](record/the-chain.md#what-the-chain-does-not-do-yet) |
| **The semantic lane answers a half-remembered line, not a question — and now says so when you ask one.** BEREL is a masked-language model, not a sentence encoder: over 240 se'ifim, a half-recalled statement puts the right se'if in the top ten **ten of ten** times, and a question about that se'if manages it **one of twelve**. A query that reads as a question is now marked as one, with both numbers and what to do instead. What remains is the model: this is a limit named, not lifted, and lifting it means a contrastively trained encoder — which is a setting here, not a release. | [the semantic lane](record/the-semantic-lane.md#what-it-does-and-what-it-does-not) |
| **A scan is highlightable finer than the page; a *link* is not.** A highlight on a page is anchored to the ink and survives a re-read. Pinning a link onto words (spec.md §8.4) still takes a character span, which a page has nothing to count into — and a personal scan has no edges to pin, so nothing has needed it yet. | [scans](record/scans.md#a-highlight-on-a-photograph-is-on-the-ink-and-one-rectangle-per-line) |
| **The MCP end is one end.** Its writes can now be undone: `forget_note`, `undraw_link` and `uncorrect`, each taking an argument that cannot be filled in without having read what is about to go — the note's words, the link's current type, the words the correction reads. A wrong answer is refused, the thing is left standing, and the refusal does not print the right answer. `undraw_link` takes back a link you drew and refuses an edge the corpus shipped. A search is still capped at 50 rows whatever `limit` says, and says so. Ksav's server is Ksav's repository. | [answering a program](record/answering-a-program.md#what-the-mcp-end-does-not-do) |
| **The shemos are changed for the seven that can be, and `אהיה` is the one left.** יהוה, the אלהים family, אלוה, אל, שדי, צבאות and now אדני are written with a letter swapped — one letter for one letter, so every mark, link and search hit on the page stays where it was drawn. Four of them are changed **only where the text is pointed**, because the mark is the only thing separating the shem from an ordinary word: `אל` from *to*, `שדי` from *my field*, and `אדני` from `אֲדֹנִי` — *my lord*, said to a person. On an unpointed page those four do nothing, which is the right answer; the alternative rewrites the sefer. **`אהיה` is untouched, and not for want of a swap** — ה → ק gives `אקיק` and preserves the length. It is untouched because `אֶהְיֶה` the shem and `אֶהְיֶה` the plain verb *I will be* are pointed identically, so unlike the other four there is no mark to hang the guard on. Catching only `אהיה אשר אהיה` needs the word *after* it, and the module looks backwards only. | [`girsa_app::shemos`](../crates/girsa-app/src/shemos.rs) |
| **The find bar declines one of the modes the chip row offers.** *A mareh makom* is a jump out of the sefer the bar is inside — `sefer_find` matches `Answer::Cited(_)` and hands back an empty list — so it was a control a reader could set and get silence from. It is disabled now with the reason on it, which is a narrowing of the promise *"the same as regular girsa search, with all the options"* and is said out loud rather than discovered. Everything else on the row works there, the instruments included: `Bar::over_the_text` refuses a scope naming more than a few seforim, so one sefer is the case a dilug wants. | [`findhere.ts`](../app/src/findhere.ts) |
| **The daf turns over on an hour, not at nightfall.** Seven in the evening by default, and a setting. Where nightfall actually falls is a function of where the reader is standing, and Girsa does not know and will not ask for a location — that would be the first thing this application ever asked about the person using it. So the hour is an approximation and the setting says so, which is honest in a way both alternatives are not: midnight is silently wrong for four hours a day, and a fixed hour presented as a computed tzeis is a lie that looks precise. Tomorrow's daf is still named beside today's either way. | [`girsa_app::luach::at`](../crates/girsa-app/src/luach.rs) |
| **Nothing installs itself, and that is not going to change here.** *Is there a newer Girsa* is a button and never a background check — spec.md §14, and a window that has not been asked makes no request at all, which is a stronger promise than a setting that defaults to off. What it will not do is download and run one: that needs a signature verified, which needs a private key that signs releases, and that key belongs to a release process rather than to this repository. | [`girsa_app::newer`](../crates/girsa-app/src/newer.rs) |
| **The luach knows two limudim, and the three it does not are a ruling.** Daf Yomi Bavli and Mishnah Yomis. **Rambam Yomi** has three tracks and a thousand perakim that do not divide by three, so it is a published calendar rather than a formula; **Amud Yomi is not one programme** — there is a 1973 one and Dirshu's, and some run five to seven amudim a week rather than one a day; **Daf Yomi Yerushalmi** skips Yom Kippur and Tisha B'Av, so it is writable but nobody has. Picking an Amud Yomi would be a guess about which luach a reader keeps. | [`girsa_app::luach`](../crates/girsa-app/src/luach.rs) |
| **A `.ksav` on the shelf is read, not written.** Its nesting is in the address now, so a sub-point is inside its point. What the shelf still does not do is *edit* one: the file is the truth and Ksav is what writes it, and Girsa's own writing pane is the door that exists. | [a document of yours](record/a-document-of-yours.md#what-a-document-does-not-carry-yet) |

## Built, and nobody has exercised it

The most uncomfortable group, because everything in it may well work. *Not
known to be broken* is not the same claim as *known to work*, and this section
exists so the two do not get written down as one.

| | Where it is argued |
|---|---|
| **Nobody has dragged a sefer with a mouse — the gesture, not the logic.** What a drop *means* is a tested function now, refusals and all. What no machine here can raise is the gesture itself: a native HTML5 drag is not synthesizable through the debugging protocol the eyes tool drives, and a file drop is an OS event no browser can fire. So what is untested is whether the events arrive, not what happens when they do. The find bar and the arrangements drawer used to be in this row and are not: both were worked with real `Input.dispatchMouseEvent` and `Input.dispatchKeyEvent` against the running window on 17 August — chip menus, choices, walk arrows, the ✕, and Keep/open/forget on a named arrangement. A synthesized press is still a program's press, and the drag is a different event family either way. | [corrections](record/corrections.md#what-has-not-been-checked) |
| **Nikud on WebKit is unknown — the code builds on macOS, the rendering is unseen.** CI has a macOS job now: the Rust half passes there and the shell compiles against macOS's WebKit bindings. What it does not settle is rendering, because the eyes tool drives Chrome and Chrome on macOS is the same Blink it is on Windows — a second machine, not a second engine. W9 asks about Safari's WebKit, which is what the shipped window there uses. | [corrections](record/corrections.md#what-has-not-been-checked) |
| **A WebKitGTK window has now been opened on a machine with no FHS, and one picture of it is all the evidence there is.** `flake.nix` was written by translating this repository's own Debian package list, which is a good way to be right and no evidence at all. The `nixos` job in `ci.yml` enters the shell it declares — inside the `nixos/nix` image, so there is no `/usr/lib` to accidentally link against — runs the window's tests and `cargo build --workspace` in it, and then `tools/nixos-window.sh` starts the binary on `xvfb-run`, waits for the screen to stop being blank and counts the colours on it. On 17 August 2026 it counted **830**, in eight seconds, and the picture is a job artifact. Getting there took four red runs and every one of them was a different real thing: an apostrophe in a comment that ended the container's shell script, `api.github.com` rate-limiting an unpinned flake's input fetches, a guard grepping for a CSS class in a binary whose assets are brotli-compressed, and **no GL stack in the container at all** — the Debian list never had to name mesa, because an Ubuntu runner has it under `/usr/lib` whether anybody asked or not, and WebKitGTK has needed EGL since 2.42. What it does **not** settle is that the window is *right*: nobody is reading a sefer off a colour count, and the one thing a person did read off that picture was a defect — the find bar drawn over the toolbar of a window with no sefer open. **Nor the AppImage**: `tauri-bundler` downloads `linuxdeploy` at build time — a prebuilt glibc ELF naming `/lib64/ld-linux-x86-64.so.2`, the interpreter this machine does not have — and unlike `node_modules` it arrives during the build and is not there to be patched first, so `--bundles deb` is what the shell's banner says. The .deb and the .rpm are written in Rust and shell out to nothing. | [`flake.nix`](../flake.nix) |
| **Nothing has ever come out of a printer — the print stylesheet is measured, the dialogue is not.** `tools/eyes.mjs` asks the browser to pretend the medium is paper (`Emulation.setEmulatedMedia`) and measures what `@media print` then does: the sheet is off-screen and laid out beforehand, in the flow on paper, the application gone from it, black ink on a white page whatever the reader's theme is, and a se'if that does not break across two sheets. That is the CSS. What is untested is everything after `window.print()` — no dialogue has been accepted, no sheet has come out, and on a machine whose printer is a PDF writer, where the file lands is unverified too. | [`printview.ts`](../app/src/printview.ts) |
| **Nothing has been run against a photographed sefer — the damage is bounded, not measured.** `tools/degraded-ocr.mjs` puts a born-digital page through named degradations and scores against the PDF's own text: clean 89.9%, and all of them at once **29.4%**, a third of what clean found. No single degradation costs more than five points, so reasoning about the parts would have been wrong by a factor of ten. It is a proxy — no uneven lighting, no gutter shadow, no show-through, no 1880 print — so 29.4% is a floor and not a photograph. | [scans](record/scans.md#what-a-photograph-costs-bounded--and-it-is-not-what-the-parts-suggested) |

---

## And the one that outranks all of it

**Nobody has learned a sugya in it.**

Everything above is a list of things a person who built this can tell you about
the thing they built. None of it is the finding that a zman of real use would
produce, and the two documents in this repository that come closest — the
[five-minute report](the-five-minute-report.md) and
[the second sitting](the-second-sitting.md) — are between them eighteen
complaints and an hour, from somebody who opened it once. Both found things no
test had. That is the shape of the evidence still missing, and no amount of
work on the lists above substitutes for it.

---

## Keeping this page honest

The rule is the one this repository applies to every other copy: **the record
is the source and this is the copy, and a copy nothing regenerates is a copy
that rots.** Nothing regenerates this one. So when you close one of these,
close it in the record's own section first — that is where the argument lives
and where the next person will read it — and then strike the line here.

When you open a new one, write it in the record where the work is argued, and
add it here. A gap that is only on this page is a gap nobody had to think hard
enough about to write down next to the code.

---

| | |
|---|---|
| Why any of this is the way it is | [`the-record.md`](the-record.md) |
| What each work order is asserted on | [`../BUILDER.md`](../BUILDER.md) |
| What it does today, for a reader | [`start-here.md`](start-here.md) |
