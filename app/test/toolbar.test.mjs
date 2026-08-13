// One convention on the toolbar, and the two properties that make it true.
//
// > *"the nikud button is labelled with the state I am already in."*
//
// The reader's fifth complaint. It was answered on the nikud button and not on
// the one beside it, so the toolbar carried both conventions at once — and
// because *corrected* is one of the correction round's three words, clicking
// `showing` made the button say `מתוקן` while the toast said `מתוקן`, in the
// same second, eight pixels apart.
//
// A label that is a **promise about what the next click does** rests on two
// things, and neither of them was checked by anything:
//
//  1. **clicking always moves.** `next(x) !== x`, for every x in the round. A
//     round with a fixed point in it is a button that does nothing, once.
//  2. **the label always changes when it does.** `said(next(x)) !== said(x)`.
//     This is the reader-visible one: three states where two print the same
//     word is a button that appears not to have worked, and no amount of
//     correctness underneath rescues it.
//
// Neither could be tested while the rounds lived in `main.ts`, which builds
// views at import time and cannot be imported at all. That is why `toolbar.ts`
// exists — the convention is a thing with a home now, and this is the home's
// front door.

import { check, ok } from "./harness.mjs";
import {
  nextIn,
  POINTING_ROUND,
  pointingSaid,
  SHOWING_ROUND,
  showingSaid,
} from "../.tmp-test/toolbar.mjs";
import { speakInterface } from "../.tmp-test/say.mjs";

/** The rounds the toolbar walks, each with the function that labels it. */
const ROUNDS = [
  { what: "pointing", round: POINTING_ROUND, said: pointingSaid },
  { what: "showing", round: SHOWING_ROUND, said: showingSaid },
];

export function run() {
  for (const { what, round, said } of ROUNDS) {
    ok(`${what} has a round to walk`, round.length >= 2);
    check(
      `${what} lists each state once`,
      [...new Set(round)].length,
      round.length,
    );

    // 1 · clicking always moves.
    const stuck = round.filter((state) => nextIn(round, state) === state);
    check(`${what}: clicking always moves`, stuck, []);

    // …and the round is a cycle rather than a walk that ends: from anywhere,
    // `round.length` clicks come back. A control a reader can get lost in is
    // not a control they can predict.
    for (const state of round) {
      let at = state;
      for (let i = 0; i < round.length; i += 1) at = nextIn(round, at);
      check(`${what}: ${round.length} clicks from ${state} come back`, at, state);
    }

    // A state the round does not carry lands somewhere real rather than
    // sticking — which is what a setting written by a newer version, or a
    // corrupt session file, will hand this.
    check(`${what}: an unknown state lands on the first`, nextIn(round, "—"), round[0]);

    // 2 · the label always changes, in **both** languages. A window switched to
    // English is still a window whose buttons have to be predictable.
    for (const language of ["hebrew", "english"]) {
      speakInterface(language);
      const same = round.filter((state) => said(nextIn(round, state)) === said(state));
      check(`${what}: the label changes on every click, in ${language}`, same, []);
      // …and no two states in the round print the same word at all, which is
      // the stronger statement and the one a reader actually depends on.
      const words = round.map(said);
      check(
        `${what}: no two states print the same word, in ${language}`,
        [...new Set(words)].length,
        words.length,
      );
    }
    speakInterface("hebrew");
  }
}
