// The window into a sefer stops growing.
//
// > *"Mishnah Berurah, 17,418 segments, scrolled to its end in rounds of sixty
// > jumps: 400 lines in the document on opening, 6,100 after 60, **17,418**
// > after 240 — 52,618 nodes. `pane.ts` bounds only the first render."*
//
// The header of `pane.ts` said, and had said since it was written, that a
// *window of lines around where the reader is* goes into the document. It was
// true of the first screen. `extend()` appended and prepended and nothing ever
// removed a line, so the sentence described the opening moment of a reading
// session and nothing after it.
//
// Which is why the arithmetic is a function now rather than four `Math.min`s
// spread through a method that also touches the DOM. **How big does this get**
// has an answer, the answer is checkable without a browser, and the way this
// went unnoticed for as long as it did is that nothing could ask.
//
// The assertions below name no ceiling. The finding is not *the number should
// be a thousand* — it is that there is a number at all, so what is tested is
// that the span after two hundred and forty edges is the span after twenty,
// which is false for any window that only ever grows and true for every
// bounded one whatever the bound.

import { check, ok } from "./harness.mjs";
import { grown } from "../.tmp-test/pane.mjs";

/** Mishnah Berurah, which is the sefer the audit sat with. */
const MISHNAH_BERURAH = 17_418;

const span = (w) => w.to - w.from;

export function run() {
  // ------------------------------------------------ it stops (the finding)
  //
  // Two hundred and forty edges, which is what the sitting did.
  let at = { from: 0, to: 400 };
  const spans = [];
  for (let i = 0; i < 240; i += 1) {
    at = grown(at, "down", MISHNAH_BERURAH);
    spans.push(span(at));
  }
  check("the page after 240 edges is the page after 20", spans[239], spans[19]);
  ok("and it is a fraction of the sefer", spans[239] < MISHNAH_BERURAH / 10);
  ok(
    "the span never exceeded that anywhere on the way",
    spans.every((n) => n <= spans[239]),
  );

  // The reader can still reach the end of it. A bound that stops the scroll is
  // not a bound, it is a wall.
  check("the end of the sefer is still reachable", at.to, MISHNAH_BERURAH);
  // And once there, asking again changes nothing — `extend` leans on this to
  // stop, so a reader parked on the last line is not re-rendering every frame.
  check("at the end it asks for the same page", grown(at, "down", MISHNAH_BERURAH), at);

  // ---------------------------------------------- it never eats what it drew
  //
  // The property that makes the ceiling safe rather than merely small. Growing
  // down draws `[have.to, want.to]` and trims to `want.from`; if the ceiling
  // were tight enough that `want.from` landed past `have.to`, an edge would
  // append three hundred lines and immediately take them off again.
  let walking = { from: 0, to: 400 };
  let ate = 0;
  for (let i = 0; i < 240; i += 1) {
    const next = grown(walking, "down", MISHNAH_BERURAH);
    if (next.from > walking.to) ate += 1;
    walking = next;
  }
  check("no edge trims into the lines it has just drawn", ate, 0);

  let back = { from: MISHNAH_BERURAH - 400, to: MISHNAH_BERURAH };
  ate = 0;
  for (let i = 0; i < 240; i += 1) {
    const next = grown(back, "up", MISHNAH_BERURAH);
    if (next.to < back.from) ate += 1;
    back = next;
  }
  check("nor going the other way", ate, 0);
  check("and the start of the sefer is reachable", back.from, 0);
  check("at the start it asks for the same page", grown(back, "up", MISHNAH_BERURAH), back);
  check("turning back holds the same span", span(back), spans[239]);

  // ------------------------------------------------- and a small sefer is left alone
  //
  // `Math.max(have.from, to - KEEP)` rather than a subtraction, so a sefer that
  // fits under the ceiling is never trimmed — Eichah is 154 segments and its
  // first line must still be on the page when the reader reaches its last.
  let small = { from: 0, to: 154 };
  for (let i = 0; i < 10; i += 1) small = grown(small, "down", 154);
  check("a sefer shorter than the ceiling keeps its first line", small.from, 0);
  check("and its last", small.to, 154);

  // A window that has not yet grown to the ceiling is not trimmed either: the
  // first edge a reader reaches must widen the page, not slide it.
  const first = grown({ from: 0, to: 400 }, "down", MISHNAH_BERURAH);
  check("the first edge widens the page rather than sliding it", first.from, 0);
  ok("which means it drew more than it had", span(first) > 400);
}
