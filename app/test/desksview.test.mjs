// The name in the box, which is what makes *keep it as it is* mean anything.
//
// An arrangement is a `Workspace`, kept, and the Rust side owns all of that.
// What the window decides is one thing: what the name box holds after a list of
// arrangements arrives. Get it wrong in one direction and *keep* is *invent a
// name again*, every time; get it wrong in the other and a redraw rewrites what
// a reader is half way through typing.
//
// The second one is the one worth having a test for, because a redraw arrives
// after **every** action in this panel — keep, open and forget all call `show`
// with a fresh list — and the reader typing at the time is the reader who just
// pressed one of them.

import { check } from "./harness.mjs";
import { nameForBox } from "../.tmp-test/desksview.mjs";

/** A row, with only the two fields this decision reads. */
const desk = (name, here = false) => ({ name, here });

export function run() {
  const four = [
    desk("הכל שוחטין"),
    desk("סוגיית שהחיינו", true),
    desk("הלכות שבת"),
    desk("ברכות פרק ב"),
  ];

  // --- the desk you are sitting at -------------------------------------------
  check("an empty box takes the name of the desk you are at", nameForBox(four, ""), "סוגיית שהחיינו");
  check("with no desk marked, there is nothing to put in it", nameForBox(four.map((d) => desk(d.name)), ""), undefined);
  check("and no desks at all is nothing either", nameForBox([], ""), undefined);

  // --- what the reader typed wins --------------------------------------------
  //
  // `undefined` and not the typed string: the caller leaves the box alone,
  // which is not the same as writing what is already there back into it. An
  // input whose `value` is reassigned to its own contents still moves the caret
  // to the end in every engine this window runs on, so *writing back the same
  // string* is a visible defect on a box somebody is typing in.
  check("a half-typed name is not overwritten", nameForBox(four, "סוגיית ה"), undefined);
  check("nor a name that matches a desk exactly", nameForBox(four, "הלכות שבת"), undefined);
  check("nor one that matches the desk you are at", nameForBox(four, "סוגיית שהחיינו"), undefined);

  // A box holding only spaces is a box the reader has typed in. `keep` trims
  // and refuses it, which is a different question from whether to write over
  // it — and writing over it would move the caret.
  check("a box of spaces is still the reader's", nameForBox(four, " "), undefined);

  // --- one desk, marked ------------------------------------------------------
  check("the only desk, and you are at it", nameForBox([desk("הכל שוחטין", true)], ""), "הכל שוחטין");
  // Two marked is a state Rust does not produce — `desk_open` writes one back —
  // and if it ever did, the first is the answer rather than the last, so the
  // box does not depend on the order a list happened to arrive in.
  check(
    "the first marked desk wins if ever there are two",
    nameForBox([desk("א", true), desk("ב", true)], ""),
    "א",
  );
}
