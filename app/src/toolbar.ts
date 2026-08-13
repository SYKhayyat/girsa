// The toolbar's rounds, and the one convention they all keep.
//
// > *"the nikud button is labelled with the state I am already in."*
//
// That was the reader's fifth complaint, and it was answered — on the nikud
// button. Eight pixels away, `showing` went on printing the state you were in,
// so the toolbar carried both conventions at once:
//
// ```
// nikud     shows "ניקוד בלי טעמים"  = the state you will get
// showing   shows "עם גרסאות"        = the state you are in
// ```
//
// And because *corrected* is one of `showing`'s three words, clicking it made
// the button say `מתוקן` while the toast also said `מתוקן` — the same word
// meaning *what will happen* in one place and *what happened* in the other, in
// the same second, in the same corner of the screen.
//
// # Why this is a module and not two functions in `main.ts`
//
// It was two functions in `main.ts`, and that is the whole story: `main.ts`
// builds views at import time, so nothing there can be imported by a test, so
// nothing held the convention. A control whose label is a **promise about what
// clicking does** is exactly the sort of thing that ought to be checkable, and
// `toolbar.test.mjs` checks the two properties that make the promise true:
//
//  1. clicking always moves — `next(x) !== x`, for every x;
//  2. the label always changes when it does — `said(next(x)) !== said(x)`.
//
// The second is the reader-visible one. A round of three states where two of
// them print the same word is a button that appears not to have worked.

import type { Pointing, Showing, Theme } from "./api.ts";
import { say } from "./say.ts";

/**
 * The pointing settings, in the order the control rounds them.
 *
 * Three and not a checkbox: the middle one is the one a reader asked for —
 * nikud with the te'amim off — and a boolean can hold *everything* and
 * *nothing* and not that.
 */
export const POINTING_ROUND: readonly Pointing[] = ["full", "nikud", "plain"];

/** The three correction settings (spec.md §7.1, §7.2), in rounding order. */
export const SHOWING_ROUND: readonly Showing[] = [
  "fixed",
  "as_printed",
  "fixed_with_variants",
];

/**
 * The next one round.
 *
 * A state the round does not carry rounds to the first, which is what an
 * unrecognised setting should do: land somewhere real rather than stick.
 */
export function nextIn<T>(round: readonly T[], now: T): T {
  const at = round.indexOf(now);
  return round[(at + 1) % round.length] as T;
}

/**
 * The three themes, in rounding order.
 *
 * > *"i dont want it stuck in dark mode — there should also be a light mode."*
 *
 * There was one, and it worked: `ערכת צבעים` in the reading settings, three
 * options, and choosing `בהיר` turns the page cream and writes `theme: light`
 * into the session file. Measured on the running release build before anything
 * here changed — the palette, the control, and the persistence were all correct.
 *
 * It was an `<option>` inside a `<select>` inside a panel behind a button, and
 * the default is *follow the system*. So on a machine whose Windows is dark,
 * Girsa is dark, and **nothing on the reading screen says it could be
 * otherwise**. A feature nobody can find is the same as a feature that is not
 * there — which is finding 1's lesson (a mefaresh's comment at `opacity: 0`)
 * and the old `לצד` button's (a complete feature behind a preposition), arriving
 * a third time in the one place a reader looks at for hours.
 *
 * So it joins the round it always belonged in. `system` first because it is the
 * default and a round has to start where the reader starts, then the two
 * explicit answers.
 */
export const THEME_ROUND: readonly Theme[] = ["system", "light", "dark"];

/** What each theme is called. The same three words the settings row uses —
 * `settingsview.ts` reads this list, so the panel and the button cannot come to
 * disagree about what `בהיר` means. */
export function themeSaid(theme: Theme): string {
  if (theme === "light") return say("themeLight");
  if (theme === "dark") return say("themeDark");
  return say("themeSystem");
}

/** What each pointing setting is called. */
export function pointingSaid(pointing: Pointing): string {
  if (pointing === "full") return say("pointingFull");
  if (pointing === "nikud") return say("pointingNikud");
  return say("pointingPlain");
}

/** What each correction setting is called. */
export function showingSaid(showing: Showing): string {
  if (showing === "as_printed") return say("showingAsPrinted");
  if (showing === "fixed_with_variants") return say("showingVariants");
  return say("showingFixed");
}
