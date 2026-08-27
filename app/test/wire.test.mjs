// The wire format Rust sends and the wire format TypeScript declares.
//
// # Why this is a test in `app/test` and not a Rust one
//
// It reads two files and compares them; either language could hold it. It is
// here because the failure it catches is a TypeScript one — `api.ts` declaring
// a field Rust does not send, or missing one Rust does — and the person who
// will see it fail is editing TypeScript when they see it.
//
// # What was there instead
//
// The wire format was described four times. `girsa-app`'s model types, checked
// by rustc. The shell's fifty-two structs, checked by rustc. `api.ts`'s
// fifty-nine interfaces, **hand-mirrored, and nothing verified they agreed**.
// And `dev-fixtures.rs`, which emits the same JSON for the browser build and
// could not import the third copy, so it rebuilt the shapes with
// `serde_json::json!` — and had already drifted three ways by 6 August 2026.
//
// The DTOs have moved into `crates/girsa-app/src/view.rs`, so the fixture now
// imports the real types and rustc holds that half. This holds the other half:
// **every `#[derive(Serialize)]` struct in `view.rs` has an interface here with
// the same field names.**
//
// # What it deliberately does not check
//
// Types. `Option<String>` against `string | null` is a mapping, not an
// equality, and a checker that understood it would be a second Rust parser in
// JavaScript — which is the shape of thing this whole file exists to argue
// against. Field *names* are where the drift was: `scan` missing from a card,
// `notes`/`fixes` missing from a gap, six keys missing from the opening state.

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { check, ok } from "./harness.mjs";
import { dirOf } from "../tools/paths.mjs";

const HERE = dirOf(import.meta.url);
const VIEW = path.join(HERE, "..", "..", "crates", "girsa-app", "src", "view.rs");
const API = path.join(HERE, "..", "src", "api.ts");

/** Rust structs that the window does not receive under their own name. */
const NOT_A_ROW = new Set([
  // Flattened into `Hit` and `Near` by `#[serde(flatten)]`; TypeScript spells
  // that as `extends At`, which this checker reads separately.
  "AtRow",
  // An argument the window *sends*, not a row it receives.
  "DrawnRow",
  // Emitted through a Tauri event rather than returned from a command.
  "LaneProgress",
]);

/** Rust name → the TypeScript interface it is declared as. */
const CALLED = new Map([
  ["AnchorRow", "Anchor"],
  ["CarriedRow", "Carried"],
  ["CoveredRow", "LaneCovered"],
  ["GapRow", "Gap"],
  ["HitRow", "Hit"],
  ["LandingRow", "Landing"],
  ["LaneRow", "LaneState"],
  // It was `LinkKind` — the lens row and the kind row are both `{key, title}`,
  // so one TypeScript interface answered for both until the lens grew the four
  // fields that say what it filters.
  ["LensRow", "LensRow"],
  ["ModelOffer", "ModelOffer"],
  ["OfferRow", "Offer"],
  ["NearRow", "Near"],
  ["Opening", "AppState"],
  ["PageWordsRow", "PageWords"],

  ["ReadingRow", "Reading"],
  ["ScanMarkRow", "ScanMark"],
  ["ScanView", "ScanOpen"],
  ["ScannedRow", "Scanned"],
  ["SettingsView", "Settings"],
  ["WordRow", "WordBox"],
]);
/** The Rust structs, as {name: [field, …]}. */
function rustRows(source) {
  const rows = new Map();
  const lines = source.split("\n");
  for (let i = 0; i < lines.length; i += 1) {
    const declared = /^pub struct (\w+) \{$/.exec(lines[i]);
    if (!declared) continue;
    // Only the ones that go on the wire.
    let derives = "";
    for (let back = i - 1; back >= 0 && lines[back].startsWith("#["); back -= 1) {
      derives += lines[back];
    }
    if (!derives.includes("Serialize")) continue;

    const fields = [];
    for (let j = i + 1; j < lines.length && lines[j] !== "}"; j += 1) {
      const field = /^    pub (\w+):/.exec(lines[j]);
      if (!field) continue;
      // `#[serde(rename = "x")]` and `#[serde(flatten)]` on the line above. A
      // flattened field is recorded as `…TheStructItFlattens`, resolved below.
      const above = lines[j - 1].trim();
      const renamed = /#\[serde\(rename = "([^"]+)"/.exec(above);
      if (above.includes("flatten")) {
        const held = /^ {4}pub \w+: (\w+),$/.exec(lines[j]);
        fields.push(`…${held ? held[1] : field[1]}`);
      } else {
        fields.push(renamed ? renamed[1] : field[1]);
      }
    }
    rows.set(declared[1], fields);
  }
  return rows;
}

/** The TypeScript interfaces, as {name: {fields, extends}}. */
function tsRows(source) {
  const rows = new Map();
  const twice = [];
  const lines = source.split("\n");
  for (let i = 0; i < lines.length; i += 1) {
    const declared = /^export interface (\w+)(?: extends (\w+))? \{$/.exec(lines[i]);
    if (!declared) continue;
    const fields = [];
    for (let j = i + 1; j < lines.length && lines[j] !== "}"; j += 1) {
      const field = /^ {2}(\w+)\??:/.exec(lines[j]);
      if (field) fields.push(field[1]);
    }
    // TypeScript merges a name declared twice, so the last one silently won
    // the `Map` above — the exact way `Found` and `Choice` were each declared
    // twice and the wire test could not see it (audit minor 5). A second
    // declaration is a failure, not a merge.
    if (rows.has(declared[1])) twice.push(declared[1]);
    rows.set(declared[1], { fields, extends: declared[2] });
  }
  return { rows, twice };
}

export async function run() {
  const [view, api] = await Promise.all([readFile(VIEW, "utf8"), readFile(API, "utf8")]);
  const rust = rustRows(view);
  const ts = tsRows(api);

  ok(
    `view.rs was walked (${rust.size} rows) and api.ts was walked (${ts.rows.size} interfaces)`,
    rust.size >= 40 && ts.rows.size >= 40,
  );
  ok(
    `no interface is declared twice (TS would silently merge it and the map above would keep the last one)`,
    ts.twice.length === 0,
  );

  const missing = [];
  const differ = [];
  for (const [name, fields] of rust) {
    if (NOT_A_ROW.has(name)) continue;
    const called = CALLED.get(name) ?? name;
    const declared = ts.rows.get(called);
    if (!declared) {
      missing.push(`${name} → no \`export interface ${called}\` in api.ts`);
      continue;
    }
    // A flattened field is `extends` on the TypeScript side: both sides gain
    // the fields of the struct being flattened in.
    const theirs = new Set(declared.fields);
    const ours = [];
    for (const field of fields) {
      if (!field.startsWith("…")) {
        ours.push(field);
        continue;
      }
      if (!declared.extends) {
        differ.push(`${name} flattens a struct and ${called} extends nothing`);
      }
      for (const inherited of ts.rows.get(declared.extends)?.fields ?? []) {
        theirs.add(inherited);
      }
      for (const inherited of rust.get(field.slice(1)) ?? []) {
        ours.push(inherited);
      }
    }
    const absent = ours.filter((f) => !theirs.has(f));
    const invented = [...theirs].filter((f) => !ours.includes(f));
    if (absent.length > 0 || invented.length > 0) {
      const said = [];
      if (absent.length > 0) said.push(`api.ts is missing ${absent.join(", ")}`);
      if (invented.length > 0) said.push(`api.ts declares ${invented.join(", ")} which Rust does not send`);
      differ.push(`${name} / ${called}: ${said.join("; ")}`);
    }
  }

  check(
    "every row `girsa_app::view` sends has an interface in api.ts that names the same fields",
    missing,
    [],
  );
  check("and none of them disagree about which fields those are", differ, []);

  await everyCallHasADoor();
}

/**
 * Every call the window declares is a call the window makes.
 *
 * The second sitting found five: `setCiteStyle`, `fixes`, `linkDraw`,
 * `scanFix` and `yours` were wired to live Tauri commands and **no view called
 * any of them**. One of the five was a documented promise —
 * `start-here.md` said changing the citation style "reformats every citation",
 * and the promise could not be kept by any sequence of clicks, because there
 * was no control to click.
 *
 * A dead wire is not a harmless one. It is a feature the repository believes it
 * has: it is counted in the command tally, it is described in the docs, it is
 * kept working by the type checker and the wire test above, and it is worked
 * around by nobody because nobody knows it is missing. The window's own source
 * is the only place that says whether a door exists.
 *
 * Deliberately not the converse — a command Rust exposes that `api.ts` does not
 * declare is `girsa-mcp`'s business or nobody's, and that is a different
 * question with a different answer.
 */
async function everyCallHasADoor() {
  const source = await readFile(API, "utf8");
  // The object, and not the rest of the file: `whenLaneWorks(handler: …)`
  // below it is a function parameter two spaces in, which reads as a member to
  // anything that only looks at indentation.
  const opens = source.indexOf("export const api = {");
  const rest = source.slice(opens);
  const object = rest.slice(0, rest.indexOf("\n};"));
  const declared = [];
  for (const line of object.split("\n")) {
    // One level in, a name, a colon and an arrow: how every member of the
    // object is written. Nested option objects are two levels in and their
    // keys are not calls.
    const member = /^ {2}(\w+): (\(|async )/.exec(line);
    if (member) declared.push(member[1]);
  }
  ok("there are calls to check", declared.length > 80);

  const src = path.join(HERE, "..", "src");
  let callers = "";
  for (const file of (await readdir(src)).filter((f) => f.endsWith(".ts"))) {
    if (file === "api.ts") continue;
    callers += await readFile(path.join(src, file), "utf8");
  }
  const called = new Set(
    // `api\n  .scanGap()` is a call. Whitespace across the dot is how a
    // `.then` chain gets formatted, and a pattern that did not allow it
    // reported two live calls as dead ones — which is how a guard like this
    // gets switched off by the second person to see it fail.
    [...callers.matchAll(/\bapi\s*\.\s*(\w+)/g)].map((found) => found[1]),
  );
  check(
    "every call api.ts declares is made by some view",
    declared.filter((name) => !called.has(name)),
    [],
  );
}
