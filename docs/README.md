# Girsa — documentation

Three audiences, and they want different pages.

| You are here to | Start at |
|---|---|
| **use** Girsa | [`start-here.md`](start-here.md) |
| **contribute** to it | [`../CONTRIBUTING.md`](../CONTRIBUTING.md), then [`your-first-change.md`](your-first-change.md) |
| **understand** it | [`architecture.md`](architecture.md), then [`the-record.md`](the-record.md) |

---

## For somebody using it

`spec.md` says what the application must do and `BUILDER.md` says what was built
and why, order by order. Both are builder-to-builder. B36 is the order that asked
for pages that are not:

> *"The documentation is outstanding builder-to-builder … It is almost useless
> switcher-to-switcher. There is no getting-started for a bochur, no 'coming from
> Otzar HaChochma / Bar Ilan / Word' page, no keyboard-shortcut card, no
> screenshots, no walkthrough, and nothing at all showing the Ksav↔Girsa loop that
> is the whole idea."*

| Page | For |
|---|---|
| [`start-here.md`](start-here.md) | **read this first.** The five minutes that are the whole idea, end to end |
| [`from-otzar.md`](from-otzar.md) | you use Otzar HaChochma — what is worse here, and what you can do that you could not |
| [`from-bar-ilan.md`](from-bar-ilan.md) | you use Bar Ilan — including where Girsa is genuinely behind |
| [`shortcuts.md`](shortcuts.md) | every keyboard shortcut, both languages. Generated from the source |
| [`tools.md`](tools.md) | every command this repository can be told to run, and what each is for |
| [`images/`](images/) | screenshots — four of them, and what they are and are not evidence for |

## For somebody working on it

| Page | For |
|---|---|
| [`../CONTRIBUTING.md`](../CONTRIBUTING.md) | **read this first.** Setup, the gate, the rules that bind every change, how to send one |
| [`your-first-change.md`](your-first-change.md) | one contribution end to end, with nothing skipped |
| [`architecture.md`](architecture.md) | how the pieces fit, and why the seams are where they are |
| [`the-record.md`](the-record.md) | why any particular thing is the way it is — the old README, kept whole, split into twelve pages under [`record/`](record) |

The split between those last two is worth knowing. `architecture.md` is the map:
what is where, and which rule holds it there. `the-record.md` is the argument:
every decision written down beside the defect that caused it, in the order the
defects were found. If you want to *change* something, read the map. If you want
to know why somebody would object to your changing it, read the record.

## What readers said, and what came of it

Two pages here are not instructions. They are what happened when somebody who
had not built this opened it and used it, written down without softening.

| Page | What it is |
|---|---|
| [`the-five-minute-report.md`](the-five-minute-report.md) | five minutes, eighteen complaints, and what each fix was |
| [`the-second-sitting.md`](the-second-sitting.md) | an hour with the running window afterwards: what got fixed, what came back, and a grade — then a second pass over the untested paths, which found that no build of the application had ever been produced |

Ksav's own pages are in the pen's repository, at `Ksav/docs/` — a
getting-started, a *coming from Word*, and its own generated shortcut card. Not
a link, because a link out of this repository is a link that is broken for
anybody who cloned only this one, which is everybody reading it on GitHub.

## The two generated pages

Neither shortcut card is written by hand, and that is the point. The reason B36
asks for a card is that nobody could find out what the shortcuts were; a
hand-maintained second list of them would be the same problem with one more copy to
forget about.

```
# Girsa
cargo run -p girsa-app --bin girsa-card > docs/shortcuts.md

# Ksav
cd Ksav/ksav/app && node tools/card.mjs > ../../docs/shortcuts.md
```

Girsa's reads `crates/girsa-app/src/keys.rs`, which is the table the window
resolves a key press against. Ksav's reads `ksav/app/src/bindings.ts` and its
`i18n.ts` labels. Either card is wrong only if the application is.

## What none of this hides

**Nobody has written a real sefer in either application.** Three separate audits
call that the most important line in any of them. Every page in here says so,
because a switcher who finds it out on their own has been misled by the ones that
did not.
