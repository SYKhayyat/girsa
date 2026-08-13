// What the number beside a shelf's name counts.
//
// > *"`תנ״ך · 66` is a parent whose children are indented 14 px; it reads as a
// > category with 66 seforim and nothing under it."*
//
// Two faults in one row. The indentation is a stylesheet's business and is held
// in `styles.test.mjs`; this file is about the number, which is the half nobody
// would have filed as a bug.
//
// `Branch` carries two counts — `here`, what stands on the shelf, and `count`,
// that and everything beneath — and the row drew `count` with nothing saying
// which of the two it was. On תנ״ך `here` is 0 and `count` is 66, so the row
// promised sixty-six seforim, and clicking it produced an empty column. The
// number was never wrong. It just never said what it was a number of, which is
// the same fault as the mefarshim door promising 67 over a list of 76, and
// takes the same answer: keep the number a reader wants and say what it counts.
//
// So the property under test is not *what the arithmetic is* — it is one field
// read off a record — but **that the three cases are three claims**. A `why`
// shared between two of them, or a `below` that is false for a shelf you cannot
// click through to, is the bug back, whatever the number says.

import { check, ok, notOk } from "./harness.mjs";
import { countedOn } from "../.tmp-test/shelf.mjs";

/** A shelf, with only the fields this decision reads. */
function branch(key, here, count, children = []) {
  return { key, title: key, here, count, mine: false, edited: false, children };
}

/** The gathered-seforim child (W42): not a shelf, and carries its parent's key. */
function loose(parent, count) {
  return { ...branch(parent, count, count), loose: true };
}

export function run() {
  // --- a leaf: one number, one meaning -------------------------------------
  const leaf = countedOn(branch("torah/genesis", 5, 5));
  check("a shelf with nothing under it counts what stands on it", leaf.said, "5");
  notOk("and a reader who clicks it gets a list of five", leaf.below);

  // --- the shelf the finding was written about ------------------------------
  const tanakh = countedOn(
    branch("tanakh", 0, 66, [branch("tanakh/torah", 5, 5), branch("tanakh/neviim", 21, 21)]),
  );
  check("a shelf holding nothing itself still shows the total below it", tanakh.said, "66");
  ok("but says the total is below it, not on it", tanakh.below);
  notOk(
    "so the two kinds of number are never one word",
    tanakh.why === leaf.why,
  );

  // --- some here, more below ------------------------------------------------
  const both = countedOn(branch("halacha", 4, 30, [branch("halacha/sa", 26, 26)]));
  ok("a shelf with some of its own and more below is still not a list", both.below);
  check("three cases, three sentences", new Set([leaf.why, tanakh.why, both.why]).size, 3);

  // --- the one that looks like a parent and is not --------------------------
  //
  // Every shelf that holds loose seforim *and* child shelves gets a gathered
  // child (W42) carrying the parent's own key. Counting it as a shelf under this
  // one would mark a plain leaf `below` and send a reader looking for shelves
  // that are not there — and it is the exact record `nothingStandsHere` filters
  // for the same reason.
  const gathered = countedOn(branch("mussar", 12, 12, [loose("mussar", 12)]));
  notOk("the gathered-seforim child is not a shelf under this one", gathered.below);
  check("so a shelf that only gathers is still a leaf", gathered.why, leaf.why.replace("5", "12"));

  // --- the number itself is untouched ---------------------------------------
  //
  // The fix is what the row *says*, not what it counts. A `said` that started
  // disagreeing with `count` would be a second number to keep honest.
  for (const [name, got, want] of [
    ["leaf", leaf.said, "5"],
    ["parent", tanakh.said, "66"],
    ["mixed", both.said, "30"],
  ]) {
    check(`the ${name}'s number is still the branch's own count`, got, want);
  }

  // Every `why` opens with the number it is explaining, so the hover reads as a
  // sentence rather than as a label needing the row beside it.
  ok(
    "each why says the number it is about",
    [leaf, tanakh, both].every((c) => c.why.startsWith(c.said)),
  );
}
