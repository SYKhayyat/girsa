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

/** `inside` for a test: the caret is in these panels and no others. */
function caret(...held) {
  return (p) => held.includes(p);
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
}
