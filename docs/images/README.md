# Screenshots — how to take them, and why there are none here yet

B36 asks for screenshots, and is right to:

> *"**Screenshots.** There are none, in three repositories, for two graphical
> applications."*

There still are none, and this file is the honest account of why rather than a
silence.

## What was tried

Girsa's window is a Tauri shell around WebView2, which renders in a **separate
process with GPU compositing**. That defeats the two ordinary ways of capturing a
window from a script:

| Method | Result |
|---|---|
| `Graphics.CopyFromScreen` | an entirely black image — the composited surface is not on the desktop DC |
| `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` | **worked once**, then returned empty frames on every subsequent attempt in the same session |

The one successful capture proves the approach is sound, not that it is reliable.
Interleaving it with `SendKeys` to open a panel first made it fail every time, and
`ShowWindow(SW_MAXIMIZE)` on this session returned a degenerate window rect —
which is the tell that there is no real interactive desktop behind it.

## What it needs

A logged-in interactive session with a real display, and somebody pressing the
keys. That is a person with the application open, not a script — which is also the
right way to get screenshots that show something worth showing, since the useful
ones are of a particular daf with a particular set of mefarshim ticked.

## What to capture, in this order

The four that carry the argument:

1. **`reading.png`** — a daf with three or four mefarshim ticked, so the `◆`
   markers are visible in the margin, and one line clicked open with their comments
   under it. This is W43 and it is the picture that explains the application.
2. **`search.png`** — a search with results, showing the matched words highlighted
   inside each line (W39) and the panel **docked** beside the open daf rather than
   over it (W48).
3. **`settings.png`** — the settings panel (`Ctrl+,`), scrolled to the shortcut
   rows, so B13's answer to *"there is no settings panel"* is visible.
4. **`loop.png`** — Girsa and Ksav side by side, the same mekor in both, taken
   right after `Ctrl+Shift+C`. This is the one picture nothing else in the world
   can take.

## How

PowerShell, in an interactive session:

```powershell
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Cap {
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RC r);
  public struct RC { public int L, T, Ri, B; }
}
"@
$H = (Get-Process girsa-shell | Where-Object { $_.MainWindowHandle -ne 0 }).MainWindowHandle
$r = New-Object Cap+RC; [Cap]::GetWindowRect($H, [ref]$r) | Out-Null
$bmp = New-Object System.Drawing.Bitmap ($r.Ri - $r.L), ($r.B - $r.T)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc(); [Cap]::PrintWindow($H, $hdc, 2) | Out-Null; $g.ReleaseHdc($hdc)
$bmp.Save("docs\images\reading.png", [System.Drawing.Imaging.ImageFormat]::Png)
```

Two things that will waste your time otherwise:

- **Flatten the alpha.** The captured bitmap comes back with `A = 0` on every
  pixel, so the PNG is correct data that every viewer renders as blank white. Draw
  it onto an opaque `Format24bppRgb` bitmap before saving.
- **Do not maximise first.** `ShowWindow(SW_MAXIMIZE)` and then `GetWindowRect` can
  give a rect the `Bitmap` constructor refuses. Size the window with `MoveWindow`
  to something that fits the display, and capture that.

Or press <kbd>Alt</kbd>+<kbd>PrtSc</kbd>, which is what a person would have done.
