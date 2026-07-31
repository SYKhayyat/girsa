# Girsa — documentation

The repository's own documentation is builder-to-builder: `spec.md` says what the
application must do and `BUILDER.md` says what was built and why, order by order.
Both are for somebody working on it.

This directory is for somebody **using** it. B36 is the order that asked for it:

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
| [`images/`](images/) | screenshots, and an honest note about why there are none yet |

Ksav's own pages are at [`../../Ksav/docs/`](../../Ksav/docs/) — a getting-started,
a *coming from Word*, and its own generated shortcut card.

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
