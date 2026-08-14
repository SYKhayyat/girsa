# Your first change

One contribution, from a fresh clone to a commit, with nothing skipped.

The change we will make is real and small: **add a keyboard shortcut.** It was
picked because it passes through almost every seam in the repository — a Rust
table, a generated documentation page, a test on each side of the wire, and the
gate. By the end you will have touched the parts of this project that surprise
people, on a change too small to break anything.

Assumed: you have read [`../CONTRIBUTING.md`](../CONTRIBUTING.md) §1 and §2, and
`cargo test` passes on a clean checkout.

**You do not need the corpus for any of this.**

---

## 0 · Get to green first

Before you change anything, run the gate. Not because you doubt the repository —
because you want to know that a red light later is *yours*.

```sh
node tools/verify.mjs
```

Nine steps across three directories. The first run compiles tantivy and candle
and will take a while; after that it is minutes. If it is red on an untouched
checkout, that is a finding — open an issue and say which step and what it
printed.

```sh
node tools/verify.mjs --list
```

is worth running once too, just to see what you are being held to.

---

## 1 · Find where the decision lives

The instinct is to search the window for a `keydown` handler. Do it, and see what
you find:

```sh
grep -rn "keydown" app/src/
```

The window turns an event into a **spelling** — `"Ctrl+Shift+C"` — and then asks
what that means. It does not decide. The table lives in Rust:

```
crates/girsa-app/src/keys.rs
```

This is the shape of nearly everything here, and §5 of
[`architecture.md`](architecture.md) is the general form of it: **if it is a
decision, it is in a crate, because a crate can be tested and a webview cannot.**

That file used to be eighteen `else if`s in the window, each comparing
`event.key.toLowerCase()` against a letter written in place. Three things
followed from that shape and all three were true: there was no list, so the
shortcut card had to be written by hand and kept in step by hope; a reader could
not rebind anything, because there was nothing to rebind *to*; and two shortcuts
could quietly claim the same key with nothing to say so. Building the table found
that last one immediately — the links button and the lane button both printed
*(Ctrl+L)* in their tooltips, and only one of them had ever been wired.

Open `keys.rs` and read the top. Then find `ACTIONS`.

---

## 2 · Write the failing test first

Rule 1: **write the failing test, run it, watch it fail.** A test you did not
watch fail is a test you hope works.

Say the change is *the shelf can be sent back to how it started, from the
keyboard.* At the bottom of `keys.rs`, in `mod tests`, next to
`out_of_the_box_the_shortcuts_are_the_ones_that_were_hardcoded`:

```rust
#[test]
fn the_shelf_can_be_reset_from_the_keyboard() {
    let bound = Bound::of(&BTreeMap::new());
    assert_eq!(
        bound.what(&press("r", true, true, false)),
        Some("reset-shelf")
    );
}
```

`Bound::of(&BTreeMap::new())` is *the table with no reader changes over it* —
the shortcuts out of the box — and `press(key, ctrl, shift, alt)` is the helper
the other tests in that module already use. Copy the two above yours rather than
this snippet; they are the reference.

Run just that one:

```sh
cargo test -p girsa-app the_shelf_can_be_reset_from_the_keyboard
```

**Watch it fail.** Read the failure. `None` where you wanted `Some("reset-shelf")`
— that is the test doing its job, and it is the only moment you will ever get
proof that the assertion is connected to anything.

---

## 3 · Make it pass

One row in `ACTIONS`:

```rust
Action { id: "reset-shelf", he: "החזר את המדף", en: "Reset the shelf", default: "Ctrl+Shift+R" },
```

Four fields, and each one is load-bearing:

- **`id`** is what a session file stores, so **it may never change.** Renaming an
  id silently unbinds every reader who had rebound that action.
- **`he` and `en`** are what the panel and the shortcut card print. Both are
  required — `every_action_is_named_in_both_languages_and_has_a_default` fails
  otherwise. This repository shipped a Hebrew window with an English-only wall of
  file paths in it once, and does not intend to again.
- **`default`** is the binding out of the box.

The table is `#[rustfmt::skip]`ed on purpose: one row per line, because a table
you can read down is the point of it being a table, and `rustfmt` would turn it
into a hundred and twenty lines that are neither a table nor a resolver.

Run your test again. Green.

---

## 4 · Now find out what else you just changed

This is the part that catches people, and it is the most useful habit this
repository can teach you. Run the whole crate, not your one test:

```sh
cargo test -p girsa-app
```

Things that may now be red, and each is telling you something true:

- **`no_two_actions_ship_bound_to_the_same_keys`** — you took a key something
  else already had. Pick another. This test exists because two actions *had*
  claimed one key, in prose, in two tooltips.
- **`an_id_is_never_used_twice`** — your id collides.
- **`every_action_is_named_in_both_languages_and_has_a_default`** — you left a
  field empty.
- **A test in `app/test/keys.test.mjs`** — the window's spelling of a press has
  drifted from Rust's. There are deliberately two implementations of *what a
  press is called*, because a `keydown` handler has to decide synchronously
  whether to call `preventDefault` and cannot await a round trip for it. That is
  the one place this project allows the duplication it bans everywhere else, and
  the price of the exception is a test that walks every shipped default and
  asserts the two sides agree.

Then wire the action to something in the window, which is the only part of this
that is ordinary work: the window asks *what did they mean* and does it.

---

## 4½ · Look at it

Tests are not eyes. Two ways to see the change, and they are not equivalent:

```sh
npm --prefix app run tauri dev     # the real thing
```

The first one compiles the BERT and tantivy and takes about **fifteen minutes**;
every run after that is seconds. This is the only way to see anything that gets
**stored** — a ticked mefaresh, a note, a rebinding.

```sh
cargo run -p girsa-app --example dev-fixtures -- corpus app/public/dev
npm --prefix app run dev           # the browser build, at localhost:5174
```

Faster, and honest about layout, typography, Hebrew and nikud, RTL and the shape
of a panel. **Reads are real out here; writes are no-ops** — the fixture layer
answers *tick a mefaresh* with the same unchanged list it answers *read the list*
with, so the box reverts a moment later and nothing appears in the console.

For the change in this walkthrough it matters twice over: the resolved shortcut
table is served out of `state.json`, which `dev-fixtures` **generated** — so your
new row is not in the browser build until you re-run that command. Nothing tells
you; the key simply does nothing.

`../CONTRIBUTING.md` §2 has the rest of what the browser build will and will not
tell you the truth about.

---

## 5 · The generated page

Now the surprise. Run:

```sh
bash tools/check-card.sh
```

```text
FAILED: docs/shortcuts.md and girsa-card disagree.
```

[`shortcuts.md`](shortcuts.md) is the keyboard card a reader is handed, and it is
**generated** from the exact table you just edited — `girsa-card` reads
`ACTIONS`, so the card is wrong only if the application is.

That was true before this check existed too. The README said the card was
*"generated from the source, so it cannot drift"*, and nothing verified that
anybody had ever re-run the generator. A generated file with nothing checking
that it was regenerated is a hand-maintained file with a disclaimer on it.

Accept the new card:

```sh
bash tools/check-card.sh --write
```

and **commit the card with your change**, in the same commit. That is rule 3 —
when a thing is replaced, the old thing goes in the same change.

The same pattern appears twice more, and it is worth recognising:

| Generated | By | Checked by |
|---|---|---|
| `docs/shortcuts.md` | `girsa-card` | `tools/check-card.sh` |
| the fixture Ksav asserts on | `--example fixture-packet` | `tools/check-ksav-fixture.sh` |
| the marked numbers in `README.md` | `tools/readme-numbers.sh` | a test, on every push |

The tree is the source, the file is the copy, and a copy nothing regenerates is a
copy that rots.

---

## 6 · Run the gate

```sh
node tools/verify.mjs
```

All nine. Not `cargo test` and a good feeling — the first four steps run against
`default-members`, which excludes the Tauri shell, so they compile everything in
this repository **except** the lines that own all the interop. The runner has
separate steps for it, and it has caught real breakage twice.

If a step fails, fix it and pick up where it stopped:

```sh
node tools/verify.mjs --from 6
```

There is no `--skip`, deliberately. The reason is on the record: this gate lived
in prose for months and quietly shrank from nine commands to seven, because the
formatting check is the one that never fails when you are in a hurry, so it is
the one that stops being run. Eleven files had been unformatted for weeks by the
time anybody noticed.

---

## 7 · Write the commit

Not `feat: add reset-shelf shortcut`. The convention here is that the subject
line **states the finding** and the body **argues it**:

```text
The shelf could be reset from a terminal and not from the keyboard

`girsa-shelf reset` has existed since W10 and the window has never had a way to
reach it — a reader who arranged the shelf into a corner had to leave the
application to get out of it.

Ctrl+Shift+R, one row in `ACTIONS`. Ctrl+R was the first choice and
`no_two_actions_ship_bound_to_the_same_keys` refused it, which is that test
doing precisely what it was written for.

Siblings checked: the other three `girsa-shelf` verbs. `add` and `move` both have
window paths already; `list` is the shelf pane itself. None of them are missing a
door.

Card regenerated, since ACTIONS is what girsa-card prints. Gate green.
```

What that message does: says what was wrong before opening any code, gives the
evidence, names the siblings **including the ones it cleared**, and says what it
does not do. Somebody reading `git log` in a year learns something from it.

---

## 8 · Send it

```sh
git checkout -b shelf-reset-shortcut
git add -A
git commit
git push -u origin shelf-reset-shortcut
```

Then open a pull request. One reviewable idea; say what you checked and how.
[`../CONTRIBUTING.md`](../CONTRIBUTING.md) §6 has the rest.

---

## What that exercised

| Step | The general rule behind it |
|---|---|
| The table was in Rust, not the window | Decisions live where they can be tested |
| The failing test came first | A test you did not watch fail is not a test |
| An `id` may never change | Anything a reader's file stores is permanent — the same argument as segment ids, one layer up |
| Other tests went red | Fix the whole family; the siblings are the point |
| A page had to be regenerated | Generated files get a gate, or they rot |
| Nine steps, not four | A gate that lives in prose stops being a gate |

None of those is about keyboard shortcuts. Pick a bigger change next and they
all still hold.

## Where to next

| | |
|---|---|
| How the whole system fits together | [`architecture.md`](architecture.md) |
| Everything a change is expected to carry | [`../CONTRIBUTING.md`](../CONTRIBUTING.md) |
| Why any particular thing is the way it is | [`the-record.md`](the-record.md) |
| Every command in the repository | [`tools.md`](tools.md) |
