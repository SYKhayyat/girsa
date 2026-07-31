// What a key press is called (B13).
//
// > *"No keyboard rebinding."*
//
// The table and the resolution are in Rust, where they are tested and where the
// shortcut card comes from. This side spells a press, and only because a `keydown`
// handler has to decide synchronously whether to swallow the key and cannot await
// a round trip to find out.
//
// That makes two implementations of *what a press is called*, which is the shape
// this project bans everywhere else. So: the second half of this file walks the
// shipped defaults and asserts this side spells each of them exactly the way
// `Press::said` does. If the two ever drift, this says so — rather than a reader
// discovering that Ctrl+Shift+C stopped sending.

import { check, ok, notOk } from "./harness.mjs";
import { said, whatKey } from "../.tmp-test/keys.mjs";

function press(key, mods = {}) {
  return { key, ...mods };
}

export function run() {

  // ---------------------------------------------------------------- one spelling
  check("a plain letter with control", said(press("f", { ctrlKey: true })), "Ctrl+F");
  check("a named key keeps its name", said(press("Escape")), "Escape");
  check("and is not shouted", said(press("Escape")), "Escape");
  check("a function key", said(press("F3")), "F3");

  // Modifier order is fixed, so a binding cannot be spelled two ways and become
  // two bindings for one combination.
  check(
    "the order is always Ctrl, Alt, Shift",
    said(press("c", { shiftKey: true, ctrlKey: true })),
    "Ctrl+Shift+C",
  );
  check(
    "all three",
    said(press("k", { altKey: true, shiftKey: true, ctrlKey: true })),
    "Ctrl+Alt+Shift+K",
  );

  // A Mac reader pressing ⌘F means search. The alternative is a second table for
  // one platform.
  check("cmd counts as control", said(press("f", { metaKey: true })), "Ctrl+F");

  check("a backslash is a key like any other", said(press("\\", { ctrlKey: true })), "Ctrl+\\");
  check("and so is a comma", said(press(",", { ctrlKey: true })), "Ctrl+,");

  // ---------------------------------------------------------------- what it means
  const TABLE = {
    "Ctrl+F": "search",
    "Ctrl+B": "shelf",
    "Ctrl+Shift+C": "send",
    "Alt+N": "nikud",
  };

  check("a bound press says what it is", whatKey(TABLE, press("f", { ctrlKey: true })), "search");
  check(
    "a bound combination too",
    whatKey(TABLE, press("c", { ctrlKey: true, shiftKey: true })),
    "send",
  );
  check("an unbound press means nothing", whatKey(TABLE, press("q", { ctrlKey: true })), null);
  // A bare letter is somebody typing.
  check("and a letter on its own means nothing", whatKey(TABLE, press("f")), null);
  check("an empty table binds nothing", whatKey({}, press("f", { ctrlKey: true })), null);

  // Ctrl+C is bound in the real table and deliberately does **not** prevent the
  // default (spec.md §10.2 — *the user does nothing different*), which is a fact
  // about the handler and not about this function. Here it is only a spelling.
  check("control C spells", said(press("c", { ctrlKey: true })), "Ctrl+C");

  // ------------------------------------------- the same spelling as Rust's
  //
  // Every default in `girsa_app::keys::ACTIONS`, as `Press::said` writes it, with
  // the event a browser would report for it. Both columns are written out by hand
  // on purpose: a fixture generated from either side would agree with that side by
  // construction and prove nothing.
  const SHIPPED = [
    ["Ctrl+O", press("o", { ctrlKey: true })],
    ["Ctrl+B", press("b", { ctrlKey: true })],
    ["Ctrl+F", press("f", { ctrlKey: true })],
    ["Ctrl+E", press("e", { ctrlKey: true })],
    ["Ctrl+\\", press("\\", { ctrlKey: true })],
    ["Ctrl+L", press("l", { ctrlKey: true })],
    ["Ctrl+Shift+L", press("l", { ctrlKey: true, shiftKey: true })],
    ["Ctrl+W", press("w", { ctrlKey: true })],
    ["Ctrl+Shift+C", press("c", { ctrlKey: true, shiftKey: true })],
    ["Ctrl+C", press("c", { ctrlKey: true })],
    ["Ctrl+N", press("n", { ctrlKey: true })],
    ["Ctrl+D", press("d", { ctrlKey: true })],
    ["Ctrl+Shift+H", press("h", { ctrlKey: true, shiftKey: true })],
    ["Ctrl+M", press("m", { ctrlKey: true })],
    ["Ctrl+K", press("k", { ctrlKey: true })],
    ["Ctrl+Shift+K", press("k", { ctrlKey: true, shiftKey: true })],
    ["Ctrl+J", press("j", { ctrlKey: true })],
    ["Alt+N", press("n", { altKey: true })],
    ["Ctrl+=", press("=", { ctrlKey: true })],
    ["Ctrl+-", press("-", { ctrlKey: true })],
    ["Ctrl+,", press(",", { ctrlKey: true })],
  ];
  const wrong = SHIPPED.filter(([spelled, event]) => said(event) !== spelled).map(
    ([spelled, event]) => `${spelled} != ${said(event)}`,
  );
  check("every shipped shortcut is spelled the way Rust spells it", wrong, []);

  // The whole set has to be distinct, or two of them are one binding. Rust asserts
  // this over its own table; this asserts it over the spellings the window will
  // actually compute, which is the pair that has to agree.
  const spellings = SHIPPED.map(([spelled]) => spelled);
  check("no two shortcuts spell the same", new Set(spellings).size, spellings.length);

  // Ctrl+Shift+L is the lane, and it is a fix: the lane button's tooltip said
  // Ctrl+L, which the links panel already had — so the tooltip named a key that
  // did nothing. Building the table is what found it.
  ok("the lane has a key of its own", spellings.includes("Ctrl+Shift+L"));
  notOk(
    "and it is not the links panel's",
    spellings.filter((s) => s === "Ctrl+L").length !== 1,
  );
}
