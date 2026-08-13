// Who gets the keypress while a panel is open.
//
// This was forty-eight lines in `main.ts` — nine panels, ten branches because
// `yoursview` needed two, and three different ways of asking whether a panel
// was open (`element.hidden`, an `is-open` class, a private boolean). Nothing
// asserted any of it, and the way you found out you had forgotten a line for a
// new panel was that Escape did nothing.
//
// It is a table and a function now, and this is the function.

import { route } from "../.tmp-test/panel.mjs";
import { check, ok, notOk } from "./harness.mjs";

/** A panel with no DOM: open or not, and it records being closed. */
function panel(open) {
  return {
    element: {},
    isOpen: open,
    closed: false,
    close() {
      this.closed = true;
    },
  };
}

/** A keypress with only the parts routing looks at. */
function press(key, ctrl = false) {
  return { key, ctrlKey: ctrl, metaKey: false, shiftKey: false, altKey: false };
}

/** The caret is typing in these panels, and away from every other. */
function caret(...held) {
  return (p) => (held.includes(p) ? "typing" : "away");
}

/** The caret is *on* these panels — focus inside them, but not in a box you
 * type into. Every result row in a docked search is one of these. */
function focusOn(...held) {
  return (p) => (held.includes(p) ? "on" : "away");
}

export function run() {
  // ── Escape ────────────────────────────────────────────────────────────────

  {
    const drawer = panel(true);
    const held = [{ panel: drawer, keyboard: "reading", escape: "anywhere" }];
    check("Escape closes a drawer wherever the caret is", route(held, press("Escape"), caret(), null), "closed");
    ok("and it was actually closed", drawer.closed);
  }

  {
    // The buffer. A reader pressing Escape over the daf is not closing what
    // they are writing.
    const buffer = panel(true);
    const held = [{ panel: buffer, keyboard: "inside", escape: "inside" }];
    check(
      "Escape from outside a text box does not close it",
      route(held, press("Escape"), caret(), null),
      null,
    );
    notOk("and it stayed open", buffer.closed);
    check(
      "Escape from inside it does",
      route(held, press("Escape"), caret(buffer), null),
      "closed",
    );
    ok("and it closed", buffer.closed);
  }

  {
    // The picker handles its own Escape, and a second one here would race it.
    const picker = panel(true);
    const held = [{ panel: picker, keyboard: "all", escape: false }];
    check("a panel that takes no Escape swallows it", route(held, press("Escape"), caret(), null), "swallowed");
    notOk("and nothing closed it from here", picker.closed);
  }

  {
    // The order is the table's. `writing` is above `linksview`, and with the
    // caret outside the buffer, Escape reaches past it.
    const buffer = panel(true);
    const drawer = panel(true);
    const held = [
      { panel: buffer, keyboard: "inside", escape: "inside" },
      { panel: drawer, keyboard: "reading", escape: "anywhere" },
    ];
    route(held, press("Escape"), caret(), null);
    notOk("Escape over the daf leaves the buffer alone", buffer.closed);
    ok("and closes the drawer behind it", drawer.closed);
  }

  // ── whose keyboard it is ──────────────────────────────────────────────────

  {
    // The shelf is a place, not an overlay: a typed letter goes into it.
    const shelf = panel(true);
    const held = [{ panel: shelf, keyboard: "all", escape: "anywhere" }];
    check("a place swallows a letter", route(held, press("n"), caret(), "note"), "swallowed");
  }

  {
    // A drawer over the reading is not a place: the reading shortcuts stay live
    // behind it.
    const drawer = panel(true);
    const held = [{ panel: drawer, keyboard: "reading", escape: "anywhere" }];
    check("a drawer lets the reading keep its shortcuts", route(held, press("n"), caret(), "note"), null);
  }

  {
    // A text box owns the keyboard only while the caret is in it.
    const box = panel(true);
    const held = [{ panel: box, keyboard: "inside", escape: false }];
    check("a text box swallows what is typed into it", route(held, press("c", true), caret(box), "copy"), "swallowed");
    check("and nothing typed elsewhere", route(held, press("c", true), caret(), "copy"), null);
  }

  {
    const shut = panel(false);
    const held = [{ panel: shut, keyboard: "all", escape: "anywhere" }];
    check("a panel that is not open takes nothing", route(held, press("n"), caret(), "note"), null);
  }

  // ── over the reading, or beside it (finding 3) ────────────────────────────
  //
  // The one that cost every shortcut in the application. W48 made clicking a
  // search result **dock** the panel rather than close it, so the ordinary path
  // leaves a column standing beside the daf — and it was registered
  // `keyboard: "all"`, which swallowed Ctrl+C, Ctrl+N, Ctrl+D, Ctrl+L, Ctrl+K,
  // Alt+N, Ctrl+= , Ctrl+− and, worst, the Ctrl+Shift+C that sends the line to
  // Ksav. Silently, with the reader looking at the daf.
  //
  // On the pre-fix tree the first two of these read `swallowed`.

  {
    const find = panel(true);
    let stood = "over";
    const held = [
      {
        panel: find,
        keyboard: () => (stood === "beside" ? "typing" : "all"),
        escape: "anywhere",
        toggle: "search",
      },
    ];

    check(
      "over the reading, the search owns what is typed",
      route(held, press("c", true), focusOn(find), "copy"),
      "swallowed",
    );

    stood = "beside";
    check(
      "docked, the send to Ksav reaches the reading",
      route(held, press("C", true), focusOn(find), "send"),
      null,
    );
    check(
      "docked, so does a copy with the focus on the result you clicked",
      route(held, press("c", true), focusOn(find), "copy"),
      null,
    );
    check(
      "docked, so does a shortcut pressed at the daf",
      route(held, press("n", true), caret(), "note"),
      null,
    );
    check(
      "but its own box still owns a letter typed into it",
      route(held, press("n"), caret(find), "note"),
      "swallowed",
    );
    check(
      "and Ctrl+C in that box is copy, not the line",
      route(held, press("c", true), caret(find), "copy"),
      "swallowed",
    );
    check(
      "and Escape still closes it from the daf",
      route(held, press("Escape"), caret(), null),
      "closed",
    );
  }

  {
    // `inside` is not good enough for a docked panel, and this is why: focus
    // lands on a *button* — every result you click is one — and a boolean
    // caret cannot tell that from a caret in the box.
    const docked = panel(true);
    const held = [{ panel: docked, keyboard: "inside", escape: "anywhere" }];
    check(
      "a panel on `inside` swallows a key pressed with focus on one of its buttons",
      route(held, press("c", true), focusOn(docked), "copy"),
      "swallowed",
    );
  }

  // ── a panel's own key reaches it ──────────────────────────────────────────

  {
    // The one B13 was not applying to itself. `Ctrl+F` was written out in
    // `main.ts` between the two `find` branches — so the shortcut whose whole
    // point is that it is rebindable was the one that was not, and the rule
    // *its own key still reaches it* was an ordering accident the next panel
    // would not have inherited.
    const find = panel(true);
    const held = [{ panel: find, keyboard: "all", escape: "anywhere", toggle: "search" }];
    check("the search panel's own key reaches it while it is open", route(held, press("f", true), caret(), "search"), null);
    check("and anything else does not", route(held, press("b", true), caret(), "shelf"), "swallowed");
  }

  // ── the extra key a panel answers ─────────────────────────────────────────

  {
    const buffer = panel(true);
    const held = [
      {
        panel: buffer,
        keyboard: "inside",
        escape: "inside",
        answers: (event) =>
          (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "e"
            ? (buffer.close(), true)
            : false,
      },
    ];
    check("Ctrl+E closes the buffer from inside it", route(held, press("E", true), caret(buffer), null), "answered");
    ok("and it closed", buffer.closed);
  }

  // ── nothing open ──────────────────────────────────────────────────────────

  check(
    "with nothing open the keypress is the reading's",
    route([{ panel: panel(false), keyboard: "all", escape: "anywhere" }], press("Escape"), caret(), null),
    null,
  );
  check("and an empty table takes nothing", route([], press("Escape"), caret(), null), null);

  sweep();
}

// ── the registry is every panel there is ────────────────────────────────────
//
// The table above fixed *"add a panel and the way you find out you forgot a
// line is that Escape does nothing"* — for the nine panels that were in it.
// Then two more were added and Escape did nothing, which the 9 August report
// found: *"a **function** in `main.ts:987` — silently omits `lanepanel` and
// `settingsview`, so Escape closes neither."*
//
// A table that has to be kept in step by hand needs the thing that keeps it in
// step. This reads `main.ts` for every module-level `const x = new Y()` whose
// class satisfies `Panel` — an `element`, an `isOpen` and a `close()` — and
// asserts each one is named in `PANELS`. A tenth panel that forgets the line is
// a red test, not a key that does nothing.

import { readFileSync } from "node:fs";
import { readdirSync } from "node:fs";
import path from "node:path";
import { SRC } from "../tools/paths.mjs";

/** Class names in `src/` that satisfy `Panel` structurally. */
function panelClasses() {
  const out = new Set();
  for (const file of readdirSync(SRC)) {
    if (!file.endsWith(".ts")) continue;
    const body = readFileSync(path.join(SRC, file), "utf8");
    for (const [, name] of body.matchAll(/^export class (\w+)/gmu)) {
      // The whole class body, up to the next top-level `export class`.
      const from = body.indexOf(`export class ${name}`);
      const next = body.indexOf("\nexport class ", from + 1);
      const inside = body.slice(from, next === -1 ? body.length : next);
      const hasElement = /^\s+(readonly )?element(!?): HTMLElement/mu.test(inside);
      const hasOpen = /^\s+get isOpen\(\)|^\s+(readonly )?isOpen(!?):/mu.test(inside);
      const hasClose = /^\s+(async )?close\(/mu.test(inside);
      if (hasElement && hasOpen && hasClose) out.add(name);
    }
  }
  return out;
}

export function sweep() {
  const classes = panelClasses();
  ok("the sweep found the panel classes", classes.size >= 5, [...classes].join(", "));

  const main = readFileSync(path.join(SRC, "main.ts"), "utf8");
  // The registry, as text: everything between `const PANELS` and the `]);` that
  // closes it.
  const at = main.indexOf("const PANELS");
  ok("main.ts has a PANELS table", at > 0);
  const table = main.slice(at, main.indexOf("\n]);", at));

  const missing = [];
  const built = new Map();
  for (const [, name, klass] of main.matchAll(/^const (\w+) = new (\w+)\(/gmu)) {
    if (!classes.has(klass)) continue;
    built.set(name, klass);
    // A plain string, not a template literal. `\s` inside backticks is the
    // letter `s` — which is how the first version of this line matched nothing
    // and reported all ten panels missing, including the eight that were there.
    if (!new RegExp("panel:\\s*" + name + "\\b", "u").test(table)) {
      missing.push(`${name} (${klass})`);
    }
  }
  ok(
    "and every panel the window constructs is in it",
    missing.length === 0,
    missing.length ? `not routed: ${missing.join(", ")}` : "all routed",
  );

  overTheReadingOrBesideIt(table, built);
}

// ── a panel that docks does not own the daf's keyboard ──────────────────────
//
// Lesson 2 of the second sitting: *the bespoke guard fits the bug that was, not
// the bug that is.* The sweep above exists because two panels were missing from
// this table; it could not see that two of the entries **in** the table
// swallowed every key in the application while docked.
//
// So this asserts the class rather than the shape. A panel that calls `dock()`
// is, at least some of the time, a column standing beside the reading — and a
// column beside the reading may not be registered `"all"` or `"inside"`, which
// both hand it keys the reader is pressing at the daf. It may be `"reading"`
// (the drawers), `"typing"` (its own boxes and nothing else), or a **function**
// that says which it is right now (the search and the bookcase, which are
// overlays until you go through them).
//
// The pre-fix tree fails this on `find`, `shelf` and `yoursview`.

/** The panel classes whose modules put themselves in the dock. */
function classesThatDock() {
  const out = new Set();
  for (const file of readdirSync(SRC)) {
    if (!file.endsWith(".ts") || file === "dock.ts") continue;
    const body = readFileSync(path.join(SRC, file), "utf8");
    // Imported from `dock.ts` and actually called with a panel name — not just
    // `undock`, which every panel calls on the way out.
    if (!/from "\.\/dock\.ts"/u.test(body)) continue;
    if (!/[^a-z]dock\("/u.test(body)) continue;
    for (const [, name] of body.matchAll(/^export class (\w+)/gmu)) out.add(name);
  }
  return out;
}

function overTheReadingOrBesideIt(table, built) {
  const docks = classesThatDock();
  ok("the sweep found the classes that dock", docks.size >= 3, [...docks].join(", "));

  // Each entry, as text. The table is a list of object literals and `panel:` is
  // the first key of every one, so splitting on it gives one chunk per panel.
  const chunks = table.split(/\bpanel:\s*/u).slice(1);
  const wrong = [];
  for (const chunk of chunks) {
    const name = /^(\w+)/u.exec(chunk)?.[1];
    if (!name || !docks.has(built.get(name))) continue;
    const mode = /keyboard:\s*("all"|"inside"|"reading"|"typing"|\(\))/u.exec(chunk)?.[1];
    if (mode === '"all"' || mode === '"inside"') wrong.push(`${name}: ${mode}`);
  }
  ok(
    "a panel that docks does not take the keyboard the reader is using at the daf",
    wrong.length === 0,
    wrong.length ? `beside the reading and swallowing it: ${wrong.join(", ")}` : "all beside",
  );
}
