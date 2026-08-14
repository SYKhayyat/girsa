# Screenshots

Four, taken 14 August 2026. What each shows, and how it was taken, because how
it was taken is a limit on what it can be evidence for.

| File | What it is |
|---|---|
| `reading.png` | Berakhot 2a menukad, with Rashi in a column beside it and the two in step. The margin refs — `ב. א׳`, `ב. ב׳` — are the permanent segment ids doing their job |
| `mefarshim.png` | The mefarshim panel: 34 on this masechta, in folders (ראשונים · 13, ריי״ף · 4, מפרשים · 3), each row tickable or openable-beside. This is W43 and B36's *"the mefarshim have no door"* |
| `settings.png` | The settings panel: colour scheme, both font families, reading size, leading, column width, nikud, citation style, and the two languages |
| `dark.png` | The same daf, dark. Three themes and *follow the system* is one of them, not the only one |

## How they were taken, and what that means

**From the browser build, not from the installed shell.** The frontend is the same
files — `app/src/`, the same `styles.css`, the same real text off the shelf — but
it is served by Vite and fed static JSON instead of by the Tauri window over IPC:

```sh
cargo run --release -p girsa-app --example dev-fixtures -- corpus app/public/dev
npm --prefix app run dev
```

Then a headless Chromium at 1440×900 against `http://localhost:5174`.

So these are honest about **layout, typography, Hebrew and nikud rendering, RTL,
and the shape of every panel** — which is most of what a screenshot is for. They
are not evidence about the shell, the IPC, or WebView2's idea of where a nikud
point sits. `dev-fixtures.rs` says the same thing in its own header: a screenshot
from one engine is not evidence about another.

## What still cannot be captured this way

Three of the pictures worth having are still missing, and each is missing for a
reason rather than for want of trying.

**A daf with mefarshim ticked and the `◆` markers in the margin.** The panel
opens and the rows tick, and the marks do not persist: writing a ticked set goes
through a command the fixture build does not serve, so the footer still reads
*לא סימנת אף אחד*. This is the picture that explains the application, and it needs
the shell.

**The shortcut rows in settings.** The מקשים section renders its header and its
hint and no rows. The table comes from `girsa_app::keys::ACTIONS` over the same
bridge. The card in [`../shortcuts.md`](../shortcuts.md) is generated from that
exact table, so the information is not lost — only the picture of it.

**Girsa and Ksav side by side, the same mekor in both, right after
`Ctrl+Shift+C`.** This is the one picture nothing else in the world can take, and
it needs two applications running and a person pressing the keys.

## Capturing the shell itself

Still the honest account, and still unsolved by script. WebView2 renders in a
separate process with GPU compositing, which defeats the two ordinary ways of
capturing a window:

| Method | Result |
|---|---|
| `Graphics.CopyFromScreen` | an entirely black image — the composited surface is not on the desktop DC |
| `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` | **worked once**, then returned empty frames on every subsequent attempt in the same session |

The one successful capture proves the approach is sound, not that it is reliable.
Interleaving it with `SendKeys` to open a panel first made it fail every time, and
`ShowWindow(SW_MAXIMIZE)` returned a degenerate window rect — the tell that there
is no real interactive desktop behind it.

If you are on a machine with a real display and the application open, the two
traps that will otherwise waste your afternoon:

- **Flatten the alpha.** The captured bitmap comes back with `A = 0` on every
  pixel, so the PNG is correct data that every viewer renders as blank white.
  Draw it onto an opaque `Format24bppRgb` bitmap before saving.
- **Do not maximise first.** `ShowWindow(SW_MAXIMIZE)` then `GetWindowRect` can
  give a rect the `Bitmap` constructor refuses. Size the window with `MoveWindow`
  to something that fits the display, and capture that.

Or press <kbd>Alt</kbd>+<kbd>PrtSc</kbd>, which is what a person would have done.

## Keeping them true

Nothing regenerates these, and this repository is unusually clear about what that
means: a copy nothing regenerates is a copy that rots. Redo them when the window
changes shape — the two commands at the top are the whole procedure, and they
need the corpus.
