// The command surface, checked against the window that calls it.
//
//     node tools/commands.mjs               # the check
//     node tools/commands.mjs --where NAME  # every site a rename has to touch
//     node tools/commands.mjs --self-test   # prove the check fails on drift
//
// # Why this exists
//
// `app/src/api.ts` opens with a sentence — *"Every function here is one Tauri
// command"* — and nothing checked it. There are four lists of the same thing:
// the `#[tauri::command]` declarations under `app/src-tauri/src/` (one file
// `lib.rs` until #68 began splitting it, so this reads the whole directory),
// the `tauri::generate_handler![…]` list that registers them, the
// `call<T>("…")` sites in `app/src/api.ts` that reach them, and the browser
// shim's `case "…"` labels. They agree today (139 / 139 / 139 / 87), which is
// why nobody has had to notice, and they are kept in step by the fact that every
// rename so far was done by someone who remembered all of them.
//
// That is the same shape as the shortcut card and the Ksav fixture: an artifact
// that is correct because a person was careful, and that no program reads. Both
// of those got a generate-and-diff gate; this is the third, and it is the one
// with the widest blast radius, because a miss here is a button that throws at
// the moment it is clicked and nowhere earlier.
//
// # Argument names, which is the half that used to be a promise
//
// A command renamed in Rust and called under its new name with the *old*
// argument names compiles on both sides — TypeScript has no idea what Rust calls
// its parameters, and Rust has no idea what the window passes — and throws at the
// moment a reader clicks the thing. For a while the module doc here said that
// check needed "the wire contract" and left it. It does not: the argument names
// are both sides' *source*, `#[tauri::command] fn search(query: String)` and
// `call("search", { query })` are the same two words in two files, and comparing
// them is a parse and not a protocol.
//
// Two facts make the comparison exact rather than approximate. Tauri converts a
// parameter to camelCase for the wire unless the attribute says
// `rename_all = "snake_case"`, which accounts for `whole_word` → `wholeWord`.
// And a parameter of type `State`, `AppHandle`, `Window`, `WebviewWindow`,
// `Channel` or `CommandScope` is **given** by the framework rather than passed by
// the window, so it is not an argument at all and has no JS key.
//
// What is still unchecked, and is a different thing: argument *types*. `{ slug }`
// where the parameter is a `String` is checked here; `{ slug: 7 }` is not, because
// the value comes from TypeScript's own types and from `girsa_app`'s serde types,
// which are tested in `girsa_app`.
//
// # The self-test
//
// A check whose failure has never been observed is a check nobody knows works.
// `--self-test` breaks each of the three lists in memory and demands the check
// report it. `tools/mutation.mjs` does this by recompiling a crate; this needs
// no compiler, so it can live in the gate's own step.

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");

/** Where the lists live. */
const SHELL = "app/src-tauri/src/lib.rs";
const SHELL_DIR = "app/src-tauri/src";
const WINDOW = "app/src/api.ts";

/** A command name, as both sides spell it. */
const NAME = /^[a-z][a-z0-9_]*$/;

/**
 * The `#[tauri::command]` declarations.
 *
 * The attribute carries `(async)` on every command in this file — see the
 * module doc in `lib.rs` for why — so the match is on the attribute's start and
 * not on its closing bracket. `pub` and `async` are both optional, because the
 * two exceptions (`copy`, `sefer_sheet`) are plain `fn`; `pub` may be qualified
 * (`pub(crate)`, which is what a command in `commands/` uses), because a plain
 * `pub` at the crate root is not the only way to make one reachable.
 */
function declaredIn(shell) {
  const found = [];
  for (const m of shell.matchAll(
    /#\[tauri::command[^\]]*\]\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+([a-z0-9_]+)/g,
  )) {
    found.push(m[1]);
  }
  return found;
}

/**
 * The `tauri::generate_handler![…]` list.
 *
 * Read by balancing brackets rather than by a regular expression: the macro
 * takes bare identifiers today, and a regex that assumed that would silently
 * return the wrong list the first time an entry grew a path (`commands::search`)
 * — a check that quietly stops checking is the failure this file is against.
 */
function registeredIn(shell) {
  const at = shell.indexOf("tauri::generate_handler![");
  if (at < 0) throw new Error(`no tauri::generate_handler![ in ${SHELL}`);
  let depth = 0;
  for (let i = shell.indexOf("[", at); i < shell.length; i++) {
    if (shell[i] === "[") depth++;
    else if (shell[i] === "]" && --depth === 0) {
      return shell
        .slice(shell.indexOf("[", at) + 1, i)
        .split(/[,\s]+/)
        .map((s) => s.trim().split("::").pop())
        .filter((s) => NAME.test(s));
    }
  }
  throw new Error("unbalanced tauri::generate_handler![");
}

/**
 * The `call<T>("name")` sites in the window.
 *
 * The generic is optional and may itself contain angle brackets, so the type
 * argument is skipped by counting rather than by one regex character class.
 * Every call in `api.ts` today is `call<Something>("name"` or `call("name"`.
 */
function calledIn(window) {
  const found = [];
  for (const m of window.matchAll(/\bcall\s*</g)) {
    let depth = 0;
    let i = m.index + 4;
    for (; i < window.length; i++) {
      if (window[i] === "<") depth++;
      else if (window[i] === ">" && --depth === 0) break;
    }
    const after = window.slice(i + 1).match(/^\s*\(\s*"([a-z0-9_]+)"/);
    if (after) found.push(after[1]);
  }
  for (const m of window.matchAll(/\bcall\s*\(\s*"([a-z0-9_]+)"/g)) found.push(m[1]);
  return found;
}

/**
 * The browser shim's `case "…"` labels — the fourth list.
 *
 * `api.ts` answers a subset of commands out of `public/dev/` when there is no
 * Tauri around, so `npm run dev` in Chrome can be looked at. It is a list a
 * rename has to touch or that one browser mode breaks while the app is fine,
 * and it is a *subset* on purpose: 87 of the 138 commands have a fixture. So
 * the check runs one way only — every case must name a command that exists.
 */
function devCasedIn(window) {
  return [...window.matchAll(/case\s+"([a-z0-9_]+)"/g)].map((m) => m[1]);
}

/**
 * The parameters of every command, as Rust declares them.
 *
 * Read from the same attribute-and-`fn` match the name list uses, then from the
 * parenthesised parameter list by balancing, so a parameter whose type contains
 * commas (`Result<String, String>`, `HashMap<String, u32>`) is one parameter and
 * not three. That is the whole reason this is a parser and not a regex: the
 * obvious regex splits `HashMap<String, u32>` in half and then reports a command
 * with two arguments where the tree has one.
 */
function paramsIn(shell) {
  const found = new Map();
  for (const m of shell.matchAll(/#\[tauri::command([^\]]*)\]\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+([a-z0-9_]+)/g)) {
    const [, attribute, name] = m;
    const open = shell.indexOf("(", m.index + m[0].length - 1);
    if (open < 0) continue;
    let depth = 0;
    let close = open;
    for (let i = open; i < shell.length; i++) {
      if (shell[i] === "(") depth++;
      else if (shell[i] === ")" && --depth === 0) {
        close = i;
        break;
      }
    }
    const inside = shell.slice(open + 1, close);
    const parts = [];
    let at = 0;
    let level = 0;
    for (let i = 0; i < inside.length; i++) {
      const ch = inside[i];
      if ("<([{".includes(ch)) level++;
      else if (">)]}".includes(ch)) level--;
      else if (ch === "," && level === 0) {
        parts.push(inside.slice(at, i));
        at = i + 1;
      }
    }
    parts.push(inside.slice(at));
    const renameAll = /rename_all\s*=\s*"([a-z_]+)"/.exec(attribute)?.[1] ?? "camelCase";
    const params = [];
    for (const part of parts) {
      const one = part.replace(/\/\/[^\n]*/g, "").trim();
      if (one === "") continue;
      const colon = one.indexOf(":");
      if (colon < 0) continue;
      params.push({
        name: one.slice(0, colon).trim(),
        type: one.slice(colon + 1).trim(),
      });
    }
    found.set(name, { renameAll, params });
  }
  return found;
}

/** Types the framework hands a command, which the window therefore cannot pass. */
const GIVEN = ["State", "AppHandle", "WebviewWindow", "Window", "Channel", "CommandScope"];

/** Does the window pass this one? */
function fromWindow(param) {
  return !GIVEN.some((one) => new RegExp(`\\b${one}\\b`).test(param.type));
}

/** The JS key Tauri will read for a parameter, given the command's renaming. */
function keyOf(param, renameAll) {
  if (renameAll !== "camelCase") return param.name;
  return param.name.replace(/_([a-z0-9])/g, (_, ch) => ch.toUpperCase());
}

/** A parameter the window may leave out: `Option<T>` is `undefined` on the wire. */
function optional(param) {
  return /^Option\s*</.test(param.type);
}

/**
 * The keys each `call("name", { … })` actually passes.
 *
 * A site with no object at all is `[]`, and that is only correct for a command
 * with nothing to pass — which the comparison decides, not this reader. A site
 * whose argument is anything other than a literal object is reported as
 * `unknown` rather than guessed at, because a check that quietly skipped it would
 * be the drift it exists to catch wearing a green tick.
 */
function argsIn(window) {
  const sites = [];
  const starts = [];
  for (const m of window.matchAll(/\bcall\s*</g)) {
    let depth = 0;
    let i = m.index + 4;
    for (; i < window.length; i++) {
      if (window[i] === "<") depth++;
      else if (window[i] === ">" && --depth === 0) break;
    }
    starts.push(i + 1);
  }
  for (const m of window.matchAll(/\bcall\s*\(/g)) starts.push(m.index + 4);
  starts.sort((a, b) => a - b);
  for (const from of starts) {
    const call = /^\s*\(\s*"([a-z0-9_]+)"/.exec(window.slice(from));
    if (!call) continue;
    const name = call[1];
    const after = from + call[0].length;
    const rest = window.slice(after);
    const trimmed = rest.replace(/^\s*/, "");
    if (!trimmed.startsWith(",")) {
      sites.push({ name, keys: [] });
      continue;
    }
    const arg = trimmed.slice(1).replace(/^\s*/, "");
    if (!arg.startsWith("{")) {
      sites.push({ name, keys: null });
      continue;
    }
    let level = 0;
    let end = 0;
    for (let i = 0; i < arg.length; i++) {
      if ("{([".includes(arg[i])) level++;
      else if ("})]".includes(arg[i]) && --level === 0) {
        end = i;
        break;
      }
    }
    sites.push({ name, keys: keysOf(arg.slice(1, end)) });
  }
  return sites;
}

/** The top-level keys of an object literal's body. */
function keysOf(body) {
  const keys = [];
  let at = 0;
  let level = 0;
  const pieces = [];
  for (let i = 0; i < body.length; i++) {
    const ch = body[i];
    if ("{([".includes(ch)) level++;
    else if ("})]".includes(ch) && --level === 0) level = 0;
    else if (ch === "," && level === 0) {
      pieces.push(body.slice(at, i));
      at = i + 1;
    }
  }
  pieces.push(body.slice(at));
  for (const piece of pieces) {
    const one = piece.trim();
    if (one === "") continue;
    if (one.startsWith("...")) {
      keys.push("...");
      continue;
    }
    const colon = one.indexOf(":");
    keys.push((colon < 0 ? one : one.slice(0, colon)).trim());
  }
  return keys;
}

/**
 * Every disagreement between the parameters Rust declares and the keys the window
 * passes, in both directions.
 *
 * Both directions because they are two different mistakes: a key the shell does
 * not take is a typo that throws *invalid args*, and a parameter the window never
 * passes is a `Missing required key` — and the second is the one that reads as
 * *the button is broken* rather than as *the wire is wrong*.
 */
function shapeDrift(params, sites) {
  const problems = [];
  for (const { name, keys } of sites) {
    const declared = params.get(name);
    if (!declared) continue; // the name check owns that one
    if (keys === null) {
      problems.push(`${name}: the window's arguments are not a literal, so they cannot be read`);
      continue;
    }
    const wanted = declared.params.filter(fromWindow).map((p) => ({
      key: keyOf(p, declared.renameAll),
      optional: optional(p),
    }));
    const wantedKeys = new Set(wanted.map((w) => w.key));
    const passed = new Set(keys);
    if (passed.has("...")) {
      problems.push(`${name}: the window spreads its arguments, so they cannot be read`);
      continue;
    }
    for (const key of passed) {
      if (!wantedKeys.has(key)) {
        problems.push(
          `${name}: the window passes \`${key}\`, the shell's parameters are ${
            [...wantedKeys].join(", ") || "none"
          }`,
        );
      }
    }
    for (const want of wanted) {
      if (!want.optional && !passed.has(want.key)) {
        problems.push(`${name}: the shell needs \`${want.key}\`, the window never passes it`);
      }
    }
  }
  return problems;
}

/** How much of the surface the argument check can see, for the row it prints. */
function shapesSeen(params, sites) {
  const seen = new Set();
  const unread = new Set();
  for (const { name, keys } of sites) {
    if (!params.has(name)) continue;
    if (keys === null || keys.includes("...")) unread.add(name);
    else seen.add(name);
  }
  return { seen: seen.size, unread: unread.size };
}

/** Every Rust file the shell is made of.
 *
 * `lib.rs` alone until #68 started moving domains into `commands/`, and the
 * whole directory now — because a declaration that moves out of `lib.rs` while
 * this tool kept reading one file would look exactly like a command that had
 * been deleted, and the report would be right about the file and wrong about
 * the tree.
 */
async function shellSources() {
  const found = [];
  const walk = async (dir) => {
    for (const entry of await readdir(path.join(ROOT, dir), { withFileTypes: true })) {
      const rel = `${dir}/${entry.name}`;
      if (entry.isDirectory()) await walk(rel);
      else if (entry.name.endsWith(".rs")) found.push(rel);
    }
  };
  await walk(SHELL_DIR);
  found.sort();
  const texts = await Promise.all(found.map((rel) => readFile(path.join(ROOT, rel), "utf8")));
  return { files: found, text: texts.join("\n") };
}

/** The four lists, once, from source text. */
function surface(shell, window, declaredText = shell) {
  return {
    declared: declaredIn(declaredText),
    registered: registeredIn(shell),
    called: calledIn(window),
    devCased: devCasedIn(window),
  };
}

/** Everything the three lists disagree about, in both directions. */
function drift({ declared, registered, called, devCased }) {
  const problems = [];
  const pairs = [
    ["declared", declared, "registered", registered],
    ["declared", declared, "called", called],
    ["registered", registered, "called", called],
  ];
  for (const [aName, a, bName, b] of pairs) {
    const aSet = new Set(a);
    const bSet = new Set(b);
    for (const name of aSet) {
      if (!bSet.has(name)) problems.push(`${name}: ${aName}, never ${bName}`);
    }
    for (const name of bSet) {
      if (!aSet.has(name)) problems.push(`${name}: ${bName}, never ${aName}`);
    }
  }
  // The browser shim is a subset, so it is checked one way: a case with no
  // command behind it is a rename that stopped halfway.
  const declaredSet = new Set(declared);
  for (const name of new Set(devCased)) {
    if (!declaredSet.has(name)) problems.push(`${name}: dev fixture case, never declared`);
  }
  // A duplicate is the same drift in one file: two `#[tauri::command]` on one
  // name does not compile, but two `call<>` sites for one command is a second
  // path to the same place, which the comment at the top of `api.ts` denies.
  for (const [label, list] of Object.entries({ declared, registered, called, devCased })) {
    const seen = new Set();
    for (const name of list) {
      if (seen.has(name)) problems.push(`${name}: appears twice under ${label}`);
      seen.add(name);
    }
  }
  return problems;
}

/** The names, then the argument names — one report, one exit code. */
function everything(shell, window, declaredText = shell) {
  return [
    ...drift(surface(shell, window, declaredText)),
    ...shapeDrift(paramsIn(declaredText), argsIn(window)),
  ];
}

function say(s, shapes) {
  const source = {
    declared: `${SHELL_DIR}/**/*.rs`,
    registered: "tauri::generate_handler![]",
    called: WINDOW,
    "dev cases": `${WINDOW}, public/dev/ fixtures`,
  };
  const row = (label, list) =>
    `  ${label.padEnd(11)} ${String(list.length).padStart(4)}  ${source[label]}\n`;
  process.stdout.write(
    "the command surface\n" +
      row("declared", s.declared) +
      row("registered", s.registered) +
      row("called", s.called) +
      row("dev cases", s.devCased) +
      `  ${"arg shapes".padEnd(11)} ${String(shapes.seen).padStart(4)}  ${WINDOW}, ${SHELL_DIR}/**/*.rs` +
      `${shapes.unread ? ` (${shapes.unread} unreadable)` : ""}\n`,
  );
}

/** Every line that names a command, so a rename is mechanical rather than remembered. */
async function where(name) {
  if (!NAME.test(name)) throw new Error(`--where takes a command name, not ${name}`);
  const sources = await shellSources();
  const files = [sources.files, WINDOW, "app/src/main.ts"].flat();
  let found = 0;
  for (const rel of files) {
    let text;
    try {
      text = await readFile(path.join(ROOT, rel), "utf8");
    } catch {
      continue;
    }
    text.split("\n").forEach((line, i) => {
      if (new RegExp(`\\b${name}\\b`).test(line)) {
        found++;
        process.stdout.write(`${rel}:${i + 1}: ${line.trim()}\n`);
      }
    });
  }
  process.stdout.write(`\n${found} site${found === 1 ? "" : "s"} to touch\n`);
  return found > 0;
}

/** Break each list in turn and demand the check report it. */
async function selfTest(shell, window) {
  const clean = everything(shell, window);
  // Each case names a real command and breaks one list in one direction, so a
  // failure here is the check being wrong about the tree it is standing in.
  const cases = [
    [
      "a caller the shell does not declare",
      shell,
      `${window}\nexport async function ghost(): Promise<number> {\n  return call<number>("ghost_verdict");\n}\n`,
      "ghost_verdict: called, never declared",
    ],
    [
      "a declaration nothing registers",
      shell.replace(/\bsuspects,/, ""),
      window,
      "suspects: declared, never registered",
    ],
    [
      "a dev case left behind by a rename",
      shell,
      `${window}\n// case "ghost_fixture": nothing behind it\nconst ghostCase = \`case "ghost_fixture":\`;\n`,
      "ghost_fixture: dev fixture case, never declared",
    ],
    [
      "a command the window never calls",
      shell,
      window.replace(/call\s*<[^>]*>\("recent"\),?/, ""),
      "recent: declared, never called",
    ],
    // The argument names, which is what this file could not see until now. Both
    // directions, because they are two different mistakes: a key the shell does
    // not take is a typo that throws *invalid args*, and a parameter the window
    // never passes is a *missing required key*.
    [
      "a key the shell does not take",
      shell,
      window.replace(
        /call<SourceSheet>\("sheet_open", \{ refs \}\)/,
        'call<SourceSheet>("sheet_open", { refs, shelf: refs })',
      ),
      "sheet_open: the window passes `shelf`",
    ],
    [
      "a parameter the window never passes",
      shell,
      window.replace(/call<Text>\("open_sefer", \{ slug \}\)/, 'call<Text>("open_sefer", {})'),
      "open_sefer: the shell needs `slug`",
    ],
    [
      "a camelCase key passed under its Rust name",
      shell,
      window.replace(/^\s*fromWord,$/m, "      from_word,"),
      "scan_mark: the shell needs `fromWord`",
    ],
    // …and the other direction, which must not be drift: a parameter that is
    // `Option` arrives as nothing when the window leaves it out, and a check that
    // demanded it would turn every optional argument into a false alarm.
    [
      "an `Option` parameter left out is not drift",
      `${shell}\n#[tauri::command]\nasync fn ghost_optional(seen: Option<String>) {}\n`,
      `${window}\nexport async function ghostOptional(): Promise<void> {\n  return call<void>("ghost_optional");\n}\n`,
      "!ghost_optional: the shell needs",
    ],
  ];
  let escaped = 0;
  for (const [what, rs, ts, expected] of cases) {
    const got = everything(rs, ts);
    const caught = expected.startsWith("!")
      ? !got.some((line) => line.startsWith(expected.slice(1)))
      : got.some((line) => line.startsWith(expected));
    if (!caught) escaped++;
    process.stdout.write(
      `${caught ? "caught " : "ESCAPED"}  ${what}\n   wanted: ${expected}\n   got:    ${
        got.length ? got.join(" | ") : "(nothing)"
      }\n`,
    );
  }
  process.stdout.write(
    `\nself-test: ${cases.length - escaped}/${cases.length} caught; the unbroken tree reports ${
      clean.length ? `${clean.length} problem(s)` : "no drift"
    }\n`,
  );
  return escaped === 0 && clean.length === 0;
}

const [sources, window] = await Promise.all([
  shellSources(),
  readFile(path.join(ROOT, WINDOW), "utf8"),
]);
const shell = sources.text;

if (process.argv.includes("--self-test")) {
  process.exit((await selfTest(shell, window)) ? 0 : 1);
}

const at = process.argv.indexOf("--where");
if (at >= 0) {
  if (!process.argv[at + 1]) throw new Error("--where needs a command name");
  process.exit((await where(process.argv[at + 1])) ? 0 : 1);
}

const s = surface(shell, window);
const problems = everything(shell, window);
say(s, shapesSeen(paramsIn(shell), argsIn(window)));
if (problems.length === 0) process.exit(0);

process.stdout.write(`\n${problems.length} disagreement(s):\n`);
for (const line of problems) process.stdout.write(`  ${line}\n`);
process.stdout.write(
  "\nThe three lists are the same surface spelled three ways. Fix the one that is\n" +
    "wrong, or use `node tools/commands.mjs --where <name>` to see every site.\n",
);
process.exit(1);
