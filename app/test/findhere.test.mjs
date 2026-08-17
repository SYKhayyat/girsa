// The find bar's two arithmetics and the one option it declines.
//
// `findhere.ts` shipped with 15 Rust tests under it and nothing at all over it,
// and the split matters: the Rust half is asked *where in this sefer are the
// words*, and everything in this file is asked *what does the bar then say and
// do*. That is where the count read backwards, and it is where a step off the
// end of the list would land.

import { check, ok, notOk } from "./harness.mjs";
import { countSaid, stepTo } from "../.tmp-test/findhere.mjs";
import { whyNot } from "../.tmp-test/chips.mjs";
import { say } from "../.tmp-test/say.mjs";

export function run() {
  // --- what the count says ---------------------------------------------------
  //
  // Four states and three of them are not a number.
  check("nothing typed is not a count of nothing", countSaid("", 0, 0, 0), "");
  check("and neither is a box of spaces", countSaid("   ", 0, 0, 0), "");
  check("nothing found says so in words", countSaid("כארי", 0, 0, 0), say("findHereNone"));
  check("one of thirty-three", countSaid("כארי", 0, 33, 33), "1 / 33");
  check("the reader's place is one-based", countSaid("כארי", 2, 33, 33), "3 / 33");

  // The list is cut and the count is not. `3 / 900` alone would promise nine
  // hundred stops where the bar can walk to five hundred — a count that lies
  // about what the ↓ button will do.
  const cut = countSaid("כארי", 2, 900, 500);
  ok("a cut list says the count is not the whole of it", cut.includes(say("findHereCut")));
  ok("and still says where the reader is", cut.startsWith("3 / 900"));
  notOk(
    "an uncut list says nothing about being cut",
    countSaid("כארי", 2, 500, 500).includes(say("findHereCut")),
  );
  // 500 places out of 500 is not a cut list, and `>=` here would have said it
  // was — the off-by-one that turns the note into permanent furniture.
  check("exactly as many places as matches is not cut", countSaid("א", 0, 500, 500), "1 / 500");

  // --- walking, and wrapping -------------------------------------------------
  //
  // `%` in JavaScript keeps the sign of its left operand, so `(0 - 1) % 33` is
  // `-1`. Every wrap backwards in this function rests on the `+ places` that
  // is there to stop that, and `-1` as an index into `places` is `undefined`,
  // which `show()` returns on silently: Shift+Enter on the first match would
  // do nothing at all, once, and then work again.
  check("forward one", stepTo(0, 1, 33), 1);
  check("back one", stepTo(5, -1, 33), 4);
  check("forward off the end wraps to the start", stepTo(32, 1, 33), 0);
  check("back off the start wraps to the end", stepTo(0, -1, 33), 32);
  check("one place is a step to itself, forwards", stepTo(0, 1, 1), 0);
  check("and backwards", stepTo(0, -1, 1), 0);
  // The guard the caller also has. Belt and braces on purpose: `% 0` is `NaN`,
  // and `NaN` as an index is the same silent nothing as `-1`.
  check("no places is no step", stepTo(0, 1, 0), 0);
  check("and no step backwards either", stepTo(0, -1, 0), 0);
  for (let at = 0; at < 33; at += 1) {
    const next = stepTo(at, 1, 33);
    ok(`a step from ${at} lands somewhere real`, Number.isInteger(next) && next >= 0 && next < 33);
  }

  // --- the option this bar cannot honour -------------------------------------
  //
  // A mareh makom is a jump out of the sefer the bar is inside: `sefer_find`
  // matches `Answer::Cited(_)` and hands back an empty list. It was on the row
  // and it quietly found nothing.
  const cannot = { "mode/Citation": say("findHereNoCitation") };
  ok("a mareh makom is declined here", whyNot(cannot, "mode", "Citation") !== undefined);
  ok("and the reader is told why", (whyNot(cannot, "mode", "Citation") ?? "").length > 20);

  // **And nothing else is.** The handoff that filed this said the instruments
  // were the other half, on the grounds that they are a whole-shelf thing. They
  // are not: `Bar::by_instrument` hands `chips.scope` to `prepare_instrument`,
  // and `Bar::over_the_text` — the dilug and notarikon path — refuses a scope
  // wider than a few seforim. One sefer is the case those two want. This is the
  // assertion that keeps somebody from closing that item as filed.
  for (const key of ["ToratEmet", "Smart", "Regex", "Instruments"]) {
    check(`mode/${key} still works inside one sefer`, whyNot(cannot, "mode", key), undefined);
  }
  for (const [chip, key] of [
    ["match", "Letters"],
    ["together", "Phrase"],
    ["instrument", "Dilug"],
    ["instrument", "Gematria"],
  ]) {
    check(`${chip}/${key} is not declined`, whyNot(cannot, chip, key), undefined);
  }

  // A row with nothing declined declines nothing, which is the search panel.
  check("the panel's row declines nothing", whyNot(undefined, "mode", "Citation"), undefined);
  check("nor does an empty table", whyNot({}, "mode", "Citation"), undefined);
}
