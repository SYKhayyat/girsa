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
}
