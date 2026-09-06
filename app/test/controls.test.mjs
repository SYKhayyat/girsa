// The seam every control passes through does not drop a handler's rejection.
//
// `controls.ts` is where the four `button()` helpers went, so it is the one
// place to hold the rule that gives them their name: **a click handler that
// fails must not fail silently.** `addEventListener` does not read a handler's
// return value, so a handler typed `async` used to have its rejection dropped
// at the listener — an unhandled rejection, in nobody's face. The audit fixed
// that once, caller by caller, in `copySource`; this guards the seam instead.
//
// These run in node, so the two globals the helpers touch are stubbed to just
// the parts they touch: `document.createElement` and `window.reportError`.

import { check, ok } from "./harness.mjs";
import { button, glyph, shut } from "../.tmp-test/controls.mjs";

function element() {
  return {
    type: "",
    className: "",
    textContent: "",
    title: "",
    aria: {},
    click: null,
    setAttribute(name, value) {
      this.aria[name] = value;
    },
    addEventListener(_type, fn) {
      this.click = fn;
    },
  };
}

/** Fire the button's click listener, with an optional event. */
function fire(control, event) {
  control.click(event);
}

const reported = [];

export async function run() {
  const realDocument = globalThis.document;
  const realWindow = globalThis.window;
  globalThis.document = {
    createElement: () => element(),
    documentElement: { classList: { toggle() {} }, lang: "", dir: "" },
  };
  globalThis.window = { reportError: (e) => reported.push(e) };
  try {
    // ------------------------------------------------------- a sync handler
    let runs = 0;
    const plain = button("label", "why", () => {
      runs += 1;
    });
    fire(plain);
    check("a sync click handler runs", runs, 1);
    check("a sync handler reports nothing", reported, []);

    // --------------------------------------- an async handler that resolves
    let done = 0;
    const resolving = button("label", "why", async () => {
      await Promise.resolve();
      done += 1;
    });
    fire(resolving);
    await Promise.resolve();
    await Promise.resolve();
    check("an async click handler runs to completion", done, 1);
    check("a resolving handler reports nothing", reported, []);

    // ----------------------------------------- an async handler that rejects
    // The defect: the rejection used to be dropped at the listener. It is
    // reported now, once, as itself.
    const boom = new Error("the pane refused");
    const failing = button("label", "why", async () => {
      throw boom;
    });
    fire(failing);
    await Promise.resolve();
    await Promise.resolve();
    check("an async rejection is reported", reported.length, 1);
    ok("the reported error is the rejection itself", reported[0] === boom);

    // A **sync** throw keeps its old road: it propagates out of the listener,
    // which is the browser's own report path, and it is not double-reported.
    const syncBoom = new Error("sync");
    const syncThrow = button("label", "why", () => {
      throw syncBoom;
    });
    let caught = null;
    try {
      fire(syncThrow);
    } catch (e) {
      caught = e;
    }
    ok("a sync throw still propagates", caught === syncBoom);
    check("a sync throw is not double-reported", reported.length, 1);

    // ------------------------------------------------- the glyph and its ×
    let got = null;
    const event = { target: "row" };
    const g = glyph("×", "close", (e) => {
      got = e;
    });
    fire(g, event);
    check("glyph hands the event to the handler", got, event);

    const glyphBoom = new Error("the panel refused");
    const gf = glyph("×", "close", async () => {
      throw glyphBoom;
    });
    fire(gf, event);
    await Promise.resolve();
    await Promise.resolve();
    check("glyph reports an async rejection", reported.length, 2);
    ok("…and it is the glyph's rejection", reported[1] === glyphBoom);

    const shutBoom = new Error("the shelf refused");
    const s = shut(async () => {
      throw shutBoom;
    });
    fire(s);
    await Promise.resolve();
    await Promise.resolve();
    check("shut reports an async rejection", reported.length, 3);
    ok("…and it is the shut's rejection", reported[2] === shutBoom);
  } finally {
    globalThis.document = realDocument;
    globalThis.window = realWindow;
  }
}