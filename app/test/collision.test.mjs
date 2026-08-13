// A class that hides the thing it is put on may belong to one module only.
//
// # The bug this file exists for
//
// `main.ts` builds the toast at the foot of the window:
//
// ```ts
// toast.className = "said";
// ```
//
// and `pane.ts` built each mefaresh's comment the same way:
//
// ```ts
// const block = el("div", "said");
// ```
//
// Two modules, one class, two meanings. The stylesheet carried a rule for each
// of them — `.said + .said` for stacked comment blocks at line 600, and `.said`
// itself at line 1768:
//
// ```css
// .said {
//   position: fixed;
//   inset-inline: 0;
//   bottom: 14px;
//   opacity: 0;
//   pointer-events: none;
// }
// ```
//
// The later rule wins. So **every comment this window has ever drawn was
// invisible**: taken out of the flow, stacked at the foot of the window at zero
// opacity, in a container measured at 16px tall. Ticking a mefaresh and clicking
// a line — the feature W43 exists for — had never once put words on the screen,
// and the toast half worked perfectly, because `announce` adds `is-on` and a
// comment block never could.
//
// Nothing caught it. Not the 231 window tests, not `styles.test.mjs` (the sheet
// is valid CSS and defines every property it reads), not the type checker, not a
// reader — because a feature that draws nothing looks exactly like a feature
// with nothing to say, and the *empty* case uses a different class and works.
//
// # Why this is the rule
//
// Sharing a class across modules is ordinary and fine: `tool`, `is-on`, and
// `is-docked` are shared on purpose. What is not fine is sharing one that **puts
// its element somewhere the reader cannot see it** — `position: fixed` off in a
// corner, `opacity: 0`, `visibility: hidden`, `display: none`. Those classes are
// chrome. A module that builds content into one has said something it did not
// mean, and the failure is silent by construction.
//
// So: a class whose own bare rule hides it is constructed by at most one module.
// Two is the bug above, and the assertion is cheap enough to run on every file.

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { check, ok } from "./harness.mjs";
import { dirOf } from "../tools/paths.mjs";

const HERE = dirOf(import.meta.url);
const SRC = path.join(HERE, "..", "src");

/** Declarations that take an element out of the reader's sight. */
const HIDES = [
  /(^|;)\s*position\s*:\s*fixed\b/,
  /(^|;)\s*opacity\s*:\s*0\s*(;|$)/,
  /(^|;)\s*visibility\s*:\s*hidden\b/,
  /(^|;)\s*display\s*:\s*none\b/,
];

/**
 * The classes whose **own** rule hides them.
 *
 * Bare single-class selectors only — `.said {`, never `.said.is-on {` or
 * `.tabs .tool {`. A variant that hides is how a panel is closed and is not a
 * fact about the class; the base rule is what an element gets for wearing the
 * name, which is the thing a second module inherits by accident.
 */
function hidden(css) {
  // Comments go first. The rule this guard was written for — `.said` at line
  // 1768 — sits directly under a comment, and a pattern anchored on the closing
  // brace of the rule before it does not see the rule at all. Which the first
  // version of this file did, and it passed while the bug was still in the tree:
  // only the two named assertions at the foot caught it.
  const plain = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const out = new Map();
  for (const [, selector, body] of plain.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    // One bare class and nothing else. `.said.is-on` is a variant, `.tabs .tool`
    // is a descendant, and neither is what an element gets for wearing the name.
    if (!/^\.[\w-]+$/.test(selector.trim())) continue;
    const name = selector.trim().slice(1);
    const declarations = body.replace(/\s+/g, " ").trim();
    const why = HIDES.find((rule) => rule.test(declarations));
    if (why) out.set(name, declarations.slice(0, 70));
  }
  return out;
}

/**
 * Every class name a module constructs.
 *
 * The three shapes this window uses, and no others: `el("div", "a b")`,
 * `x.className = "a b"`, and `classList.add("a", "b")`. A template literal is
 * skipped rather than half-parsed — `"said" + (on ? " is-on" : "")` contributes
 * `said`, which is what matters, and a computed name nobody can read statically
 * is a separate thing to complain about.
 */
function built(source) {
  const names = new Set();
  const add = (list) => {
    for (const one of list.split(/\s+/)) if (/^[\w-]+$/.test(one)) names.add(one);
  };
  for (const [, list] of source.matchAll(/\bel\(\s*"[\w-]+"\s*,\s*"([^"]*)"/g)) add(list);
  for (const [, list] of source.matchAll(/\.className\s*=\s*"([^"]*)"/g)) add(list);
  for (const [, list] of source.matchAll(/\.classList\.add\(([^)]*)\)/g)) {
    for (const [, one] of list.matchAll(/"([^"]*)"/g)) add(one);
  }
  return names;
}

export async function run() {
  const css = await readFile(path.join(SRC, "styles.css"), "utf8");
  const chrome = hidden(css);

  // A guard that found nothing to guard is the failure it was written to catch.
  ok("styles.css has classes whose own rule hides them", chrome.size > 0);

  const modules = (await readdir(SRC)).filter((f) => f.endsWith(".ts")).sort();
  ok("there are modules to read", modules.length > 0);

  /** class → the modules that construct it. */
  const builders = new Map();
  for (const file of modules) {
    const source = await readFile(path.join(SRC, file), "utf8");
    for (const name of built(source)) {
      if (!builders.has(name)) builders.set(name, []);
      builders.get(name).push(file);
    }
  }

  ok("classes are being constructed somewhere", builders.size > 10);

  const shared = [...chrome]
    .filter(([name]) => (builders.get(name) ?? []).length > 1)
    .map(([name, why]) => `.${name} — ${builders.get(name).join(", ")} — ${why}`);

  check(
    "a class whose own rule hides it is built by one module only — two modules " +
      "means one of them is drawing content the reader cannot see",
    shared,
    [],
  );

  // The bug itself, named, so that reverting the rename fails here and not only
  // in the rule above. `said` is the toast; a comment block is `said-one`.
  const saidBuilders = builders.get("said") ?? [];
  check("the toast class `said` belongs to main.ts alone", saidBuilders, ["main.ts"]);
  ok("a mefaresh's comment has a class of its own", builders.has("said-one"));
  ok("and the stylesheet styles it", /\.said-one\b/.test(css));
}
