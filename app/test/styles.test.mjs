// Every custom property the stylesheet reads is one it defines.
//
// # Why this is a test and not a linter rule
//
// `styles.css` is the healthiest file in this repository — 2,504 lines, maximum
// specificity 0-2-0, zero `!important`, zero ID selectors, theming through eight
// custom properties with explicit `data-theme` overrides in both directions.
//
// And it used two properties that were never defined anywhere: `--paper`, twice,
// and `--reading`, once. `var(--paper)` with no fallback resolves to *nothing*,
// so the semantic lane's results sheet had **no background at all** — a 560px
// panel of results with the reading showing through it — and the lane's own hits
// were the one place in the window drawing Hebrew in the fallback serif.
//
// Nothing said so. CSS has no undefined-variable error: a typo is a rule that
// quietly does not apply, which is the exact shape of silence this project
// refuses everywhere else. So it is checked here, where the rest of the window's
// checks are.

import { readFile } from "node:fs/promises";
import path from "node:path";
import { check, ok } from "./harness.mjs";
import { dirOf } from "../tools/paths.mjs";

const HERE = dirOf(import.meta.url);
const SHEET = path.join(HERE, "..", "src", "styles.css");

export async function run() {
  const css = await readFile(SHEET, "utf8");

  const defined = new Set();
  for (const [, name] of css.matchAll(/^\s*(--[\w-]+)\s*:/gm)) {
    defined.add(name);
  }

  // `var(--x)` with **no fallback**, which is the only shape that can silently
  // resolve to nothing. `var(--depth, 0)` and `var(--mark-wash, rgba(…))` are
  // properties the window sets on an element, with a value for when it has not,
  // and are exactly as intended.
  const read = new Map();
  let times = 0;
  for (const line of css.split("\n")) {
    for (const [, name] of line.matchAll(/var\(\s*(--[\w-]+)\s*\)/g)) {
      times += 1;
      if (!read.has(name)) read.set(name, line.trim().slice(0, 80));
    }
  }

  const missing = [...read]
    .filter(([name]) => !defined.has(name))
    .map(([name, where]) => `${name} — ${where}`);

  check(
    "every custom property styles.css reads is one it defines — `var(--x)` with no " +
      "fallback resolves to nothing, so the rule silently does not apply",
    missing,
    [],
  );

  // A check that passed because it found nothing to check would be the failure
  // `tools/check-ksav-fixture.sh:41` refuses by name.
  ok(
    `the sheet was actually walked (${defined.size} defined, ${times} reads)`,
    defined.size >= 8 && times >= 200,
  );

  containersAreNamed(css);
  everyDockablePanelLaysOutNarrow(css);
}

/**
 * Every `@container <name>` is a name some element declares.
 *
 * The same silence as `var(--paper)`, one level up. A container query naming a
 * container nobody established is not an error — it is a block of rules that
 * simply never apply, so the panel it was written for lays out as though the
 * query had never been added and the stylesheet reads as though it had.
 */
function containersAreNamed(css) {
  const established = new Set();
  for (const [, names] of css.matchAll(/^\s*container(?:-name)?\s*:\s*([^;{}]+);/gm)) {
    // `container: shelf / inline-size` and `container-name: shelf` both.
    for (const name of names.split("/")[0].trim().split(/\s+/)) {
      if (name && name !== "none") established.add(name);
    }
  }
  const asked = new Set();
  for (const [, name] of css.matchAll(/@container\s+([\w-]+)\s*\(/g)) asked.add(name);

  check(
    "every `@container <name>` names a container some element establishes — a query " +
      "on a container nobody declared never matches, and CSS says nothing",
    [...asked].filter((name) => !established.has(name)),
    [],
  );
  ok(`container queries were found at all (${asked.size})`, asked.size > 0);
}

/**
 * A panel that can stand in a 380px column does not lay out only for 1080.
 *
 * > *"The docked shelf squeezes the seforim to a 70px column, one word per
 * > line, with the era clipped to a single letter."*
 *
 * `.shelf-tree` is `width: 320px` and `--dock` is `380px`. Both numbers were in
 * this stylesheet, four hundred lines apart, and their sum was never in it —
 * which is the whole bug: what was left for the seforim, the thing the panel
 * exists to show, was about seventy pixels.
 *
 * The search had the identical two-column body and had been given a
 * `.find.is-docked .find-body { flex-direction: column }` — the right layout
 * keyed to the wrong question. It fixed the docked search and would have done
 * nothing for a floating one on a narrow screen, where `min(1180px, 95vw)`
 * produces the same squeeze by another route.
 *
 * So the rule here is about the **class**: a two-column body inside a panel that
 * can dock must have somewhere to go when the width is not there, and it must be
 * the width that is asked. A new dockable panel with a two-column body fails
 * this the day it is written rather than the day somebody docks it.
 */
function everyDockablePanelLaysOutNarrow(css) {
  const dockable = new Set();
  for (const [, panel] of css.matchAll(/\.([\w-]+)\.is-docked\b/g)) dockable.add(panel);

  const narrow = atRuleBodies(css);
  const missing = [];
  for (const panel of dockable) {
    const body = `.${panel}-body`;
    // Only panels that actually put two things side by side.
    if (!new RegExp(`\\${body}\\s*\\{[^}]*display:\\s*flex`).test(css)) continue;
    const stacks = new RegExp(`\\${body}\\s*\\{[^}]*flex-direction:\\s*column`).test(narrow);
    if (!stacks) missing.push(`${body} — a two-column body with no narrow layout`);
  }

  check(
    "every dockable panel's two-column body stacks when it is narrow, and asks the " +
      "width rather than the class that made it narrow",
    missing,
    [],
  );
  ok(`dockable panels were found at all (${dockable.size})`, dockable.size >= 2);
}

/** Everything inside a `@container` or `@media` block, concatenated. */
function atRuleBodies(css) {
  let out = "";
  for (const match of css.matchAll(/@(?:container|media)[^{]*\{/g)) {
    let depth = 1;
    let at = match.index + match[0].length;
    const from = at;
    while (at < css.length && depth > 0) {
      if (css[at] === "{") depth += 1;
      else if (css[at] === "}") depth -= 1;
      at += 1;
    }
    out += `${css.slice(from, at)}\n`;
  }
  return out;
}
