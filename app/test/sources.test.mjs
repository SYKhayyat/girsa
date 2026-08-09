// Guards over the source itself, for the two mistakes that were spread across it.
//
// These are not unit tests and they are the only kind of test that could have
// caught either finding. Both were *a string in the wrong place*, repeated: the
// sibling's name misspelled in six sites, and a raw caught error interpolated
// into user-facing text in thirteen. A test of any one module passes while the
// other twelve sites are still wrong, which is precisely how they survived.
//
// So these read the files. A fourteenth site cannot appear without this going red.

import { check } from "./harness.mjs";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { dirOf } from "../tools/paths.mjs";

const HERE = dirOf(import.meta.url);
const SRC = path.resolve(HERE, "..", "src");

/** kaf-samekh-bet: a transliteration of the Latin "Ksav", and not a word. */
const MISSPELLING = "כסב";

/**
 * A raw caught error assigned straight into something the reader reads.
 *
 * `String(e)` is fine as an argument to `trouble()` or `raw()`. It is not fine as
 * a `textContent`, nor as a hole in a Hebrew sentence.
 *
 * The right-hand side is matched loosely on purpose: the site that was on the
 * toolbar of the first screen was a ternary, not a bare assignment, and a pattern
 * tight enough to look precise would have walked straight past it.
 */
const RAW_INTO_UI = [
  /\.(?:textContent|innerText)\s*=[^;]*String\(\s*e\w*\s*\)/,
  /\.(?:textContent|innerText)\s*=[^;]*\$\{[^}]*\bwhy\s*\}/,
];

/** A comment is not something a reader sees; these files discuss the bug. */
function isComment(line) {
  const s = line.trim();
  return s.startsWith("//") || s.startsWith("*") || s.startsWith("/*");
}

async function sources() {
  const names = (await readdir(SRC)).filter((f) => f.endsWith(".ts"));
  const out = [];
  for (const f of names) out.push([f, await readFile(path.join(SRC, f), "utf8")]);
  return out;
}

export async function run() {
  const files = await sources();
  check("there is source to check", files.length > 10, true);

  // ------------------------------------------------------- the sibling's name
  const misspelled = [];
  for (const [f, body] of files) {
    body.split("\n").forEach((line, i) => {
      if (!isComment(line) && line.includes(MISSPELLING)) misspelled.push(`${f}:${i + 1}`);
    });
  }
  check("nowhere in src spells the sibling כסב", misspelled, []);

  // -------------------------------------------------- raw errors in the UI
  const leaks = [];
  for (const [f, body] of files) {
    if (f === "trouble.ts") continue; // the one module allowed to hold the raw string
    body.split("\n").forEach((line, i) => {
      if (RAW_INTO_UI.some((re) => re.test(line))) leaks.push(`${f}:${i + 1}`);
    });
  }
  check("no raw caught error is assigned into user-facing text", leaks, []);

  // ------------------------------------------------------ every control has a name
  //
  // An accessibility snapshot listed 29 controls and **neither text field was among
  // them** — not the search box, which is the one control the whole application is
  // about. A placeholder is not a name.
  //
  // `controls.ts` cannot make an unnamed control, because the name is a required
  // argument. This is the other half: a control built by hand, bypassing it, is a
  // failure here rather than an accessibility finding a year later.
  const bare = [];
  for (const [f, body] of files) {
    if (f === "controls.ts") continue; // where they are made, with a name
    body.split("\n").forEach((line, i) => {
      if (isComment(line)) return;
      if (/createElement\(\s*["'](?:input|textarea|select)["']\s*\)/.test(line)) {
        bare.push(`${f}:${i + 1}`);
      }
    });
  }
  check("no text field, textarea or select is built without a name", bare, []);

  // A `<button>` carries its own name in its text, so it is allowed — except when
  // its face is a glyph, which is not a name. `controls::glyph` is for those.
  const glyphs = [];
  for (const [f, body] of files) {
    if (f === "controls.ts") continue;
    body.split("\n").forEach((line, i) => {
      if (isComment(line)) return;
      // Only a *button*: a `…` in a paragraph is a spinner, and a spinner is not a
      // control anybody has to be able to name and press.
      if (
        /\.textContent\s*=\s*["'](?:×|−|\+|⚙|»|«)["']/.test(line) &&
        !/createElement\(\s*["']p["']\s*\)/.test(line)
      ) {
        glyphs.push(`${f}:${i + 1}`);
      }
    });
  }
  check("no glyph is used as a button's name", glyphs, []);

  // ------------------------------------------------ one place decides what a sefer is called
  //
  // > *"hebrew and english ui. all seforim names in hebrew ui should be heb all
  // > in english ui should be english."*
  //
  // There was nothing to switch, because fifteen places each reached for
  // `he_title` themselves. `names.ts` is the funnel now — `sefer()` and
  // `alsoCalled()` — and this is what keeps a sixteenth from appearing. `api.ts`
  // is exempt: it declares the shape of what Rust sends, and declaring a field is
  // not choosing between two of them.
  const reaching = [];
  for (const [f, body] of files) {
    if (f === "names.ts" || f === "api.ts") continue;
    body.split("\n").forEach((line, i) => {
      if (isComment(line)) return;
      const NAMED = new RegExp("(he_title|en_title)");
      if (NAMED.test(line)) reaching.push(`${f}:${i + 1}`);
    });
  }
  check("no module outside names.ts picks a sefer's language for itself", reaching, []);

  // One `button`, not four. There were four — `main.ts`, `scanview.ts`,
  // `linksview.ts` and `writing.ts` — which is the "two readers of one value" shape
  // this project bans, applied to the thing every screen is made of.
  const makers = files.filter(
    ([f, b]) => f !== "controls.ts" && /(?:^|\n)\s*(?:private )?(?:function )?button\(label:/.test(b),
  );
  check("there is one `button` helper", makers.map(([f]) => f), []);

  // ------------------------------------------- and the modules are used, not copied
  //
  // A constant nothing imports is not a single source of truth, it is a
  // fourteenth string.
  const importsNames = files.filter(([f, b]) => f !== "names.ts" && /from "\.\/names(\.ts)?"/.test(b));
  check("`names.ts` has readers", importsNames.length > 0, true);
  const importsTrouble = files.filter(([f, b]) => f !== "trouble.ts" && /from "\.\/trouble(\.ts)?"/.test(b));
  check("`trouble.ts` has readers", importsTrouble.length > 0, true);
  const importsPresence = files.filter(([f, b]) => f !== "presence.ts" && /from "\.\/presence(\.ts)?"/.test(b));
  check("`presence.ts` has readers", importsPresence.length > 0, true);
}
