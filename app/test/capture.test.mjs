// The shortcut trap, as assertions.
//
// ```
// click "Ctrl+O" row → close panel with × → press "g"
// → open = G          (Ctrl+O no longer opens anything)
// ```
//
// The listener that waits for a key to bind was removed in exactly one place —
// inside itself, when a key arrived. Every other way of ending the wait left it
// on `window`, capturing, and the next bare letter typed anywhere in the
// application was bound to whichever row had been clicked. There was no
// confirmation and no message.
//
// The assertion that matters is not *close removes the listener*. It is the
// class: **at most one wait exists, and every way of ending it ends it.** A
// listener count that goes to zero on every path is what this file measures,
// because a listener that is still registered is the whole bug.

import { check, notOk, ok } from "./harness.mjs";
import { OneKey } from "../.tmp-test/capture.mjs";

/** A stand-in for `window`, which counts what is registered on it.
 *
 * `window` is not there in a unit test, and a rule about *when a listener is
 * removed* that cannot run without a browser is a rule held by nobody — which
 * is exactly how this one lasted. */
function ears() {
  const on = new Set();
  return {
    addEventListener: (_type, fn) => on.add(fn),
    removeEventListener: (_type, fn) => on.delete(fn),
    /** How many listeners are registered right now. Zero, unless waiting. */
    get count() {
      return on.size;
    },
    /** Press a key. Returns whether anything was listening for it. */
    press(key) {
      const event = {
        key,
        preventDefault: () => {},
        stopPropagation: () => {},
      };
      const listening = [...on];
      for (const fn of listening) fn(event);
      return listening.length > 0;
    },
  };
}

export function run() {
  // ---------------------------------------------------------- the ordinary way

  {
    const on = ears();
    const one = new OneKey();
    let got = "nothing";
    one.wait(on, (pressed) => {
      got = pressed ? pressed.key : "cancelled";
    });
    ok("waiting registers a listener", on.count === 1);
    ok("…and says so", one.waiting);
    on.press("g");
    check("the key arrives", got, "g");
    check("and the listener is gone", on.count, 0);
    notOk("nothing is waiting any more", one.waiting);
  }

  // ------------------------------------------------------------- the bug, twice

  {
    // Door one: the panel closed. This is the reproduction, exactly.
    const on = ears();
    const one = new OneKey();
    let got = "nothing";
    one.wait(on, (pressed) => {
      got = pressed ? pressed.key : "cancelled";
    });
    one.stop();
    check("closing the panel removes the listener", on.count, 0);
    check("…and the row is told it was cancelled", got, "cancelled");
    notOk("pressing a key afterwards binds nothing", on.press("g"));
  }

  {
    // Door two: a second row's key button. Two listeners used to sit on
    // `window` at once, so one press bound two actions and the first row went
    // on showing `…` for ever.
    const on = ears();
    const one = new OneKey();
    const ended = [];
    one.wait(on, (pressed) => ended.push(pressed ? pressed.key : "cancelled"));
    one.wait(on, (pressed) => ended.push(pressed ? pressed.key : "cancelled"));
    check("a second wait leaves exactly one listener", on.count, 1);
    check("and the first is told it was cancelled", ended, ["cancelled"]);
    on.press("g");
    check("the key goes to the second row only", ended, ["cancelled", "g"]);
  }

  // ------------------------------------------------------- what is not a key

  {
    const on = ears();
    const one = new OneKey();
    const ended = [];
    one.wait(on, (pressed) => ended.push(pressed ? pressed.key : "cancelled"));
    // Holding Ctrl on the way to Ctrl+Shift+K fires three `keydown`s, and
    // binding the first would make every shortcut in the table `Ctrl`.
    for (const modifier of ["Control", "Shift", "Alt", "Meta", "AltGraph"]) {
      on.press(modifier);
      check(`${modifier} alone keeps waiting`, on.count, 1);
    }
    check("…and reports nothing yet", ended, []);
    on.press("K");
    check("the combination's own key ends it", ended, ["K"]);
  }

  {
    const on = ears();
    const one = new OneKey();
    let got = "nothing";
    one.wait(on, (pressed) => {
      got = pressed ? pressed.key : "cancelled";
    });
    // Escape backs out and is the one key that can never be bound: a reader
    // who bound it would have no way to cancel the next binding, including
    // that one.
    on.press("Escape");
    check("Escape cancels rather than binding", got, "cancelled");
    check("and takes the listener with it", on.count, 0);
  }

  // ------------------------------------------------------------ calling it twice

  {
    const on = ears();
    const one = new OneKey();
    let ends = 0;
    one.wait(on, () => {
      ends += 1;
    });
    one.stop();
    one.stop();
    one.stop();
    check("stopping what is already stopped reports once", ends, 1);
    check("and leaves nothing registered", on.count, 0);
  }

  {
    // The panel calls `stop()` on close and on every redraw, most of the time
    // with nothing waiting at all. That must be free.
    const one = new OneKey();
    one.stop();
    notOk("stopping when nothing waits is the ordinary case", one.waiting);
  }
}
