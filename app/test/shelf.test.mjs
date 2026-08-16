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
import { countedOn, dropping, landing } from "../.tmp-test/shelf.mjs";

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

  // --- what a drop means ----------------------------------------------------
  //
  // The one path in `shelf.ts` that rearranges a reader's shelf, and until it
  // was lifted out of its `drop` listener nothing anywhere had run it: this
  // suite has no DOM, so a module's exported functions are reachable and its
  // event handlers are not — and the shelf panel was driven in the browser
  // build, where rows are not draggable at all because dragging is the shell's.
  //
  // What is still not exercised by any machine is the **gesture**: a real
  // pointer press, move and release over a native drag. That is said in
  // `docs/not-yet.md` rather than papered over here.
  const shelf = (id, from) => ({ what: "shelf", id, from });
  const work = (id, from) => ({ what: "work", id, from });

  check(
    "a sefer dropped on another shelf goes there",
    dropping(work("bavli/berakhot", "shas"), "mine/chaburah"),
    { what: "work", id: "bavli/berakhot", into: "mine/chaburah" },
  );
  check(
    "and so does a shelf",
    dropping(shelf("shas/moed", "shas"), "mine"),
    { what: "shelf", id: "shas/moed", into: "mine" },
  );

  // A `drop` can arrive from outside the window — a file, a selection, another
  // application — with nothing held. Moving *something* on the strength of a
  // drop nobody started is the worst of the three refusals.
  check("a drop with nothing held moves nothing", dropping(null, "mine"), null);
  check(
    "a shelf dropped on itself is not an edit",
    dropping(shelf("shas/moed", "shas"), "shas/moed"),
    null,
  );
  check(
    "and a sefer dropped back where it came from is not one either",
    dropping(work("bavli/berakhot", "shas"), "shas"),
    null,
  );

  // A shelf dropped inside its own child is **not** refused here, on purpose:
  // `girsa_app::Arrangement` refuses it with the one walk of the tree that
  // knows the whole shape, and a second check in the window would be a second
  // answer to that question. What this asserts is that the window does not
  // quietly grow one.
  check(
    "a shelf dropped into its own child is Rust's refusal to make, and is passed on",
    dropping(shelf("shas", ""), "shas/moed"),
    { what: "shelf", id: "shas", into: "shas/moed" },
  );
  // --- where a sefer picked off the shelf lands ------------------------------
  //
  // > *"add to that a way to open a new sefer in the same tab/workspace."*
  //
  // A tab in Girsa holds panes: a Gemara with its Rashi and its Tosafos is one
  // tab and three panes. Every route to that shape went through the mefarshim
  // door, which offers only what the link graph places on the sefer you are
  // reading — so two seforim a reader wants side by side, with nothing declared
  // between them, could not be put side by side at all. The bookcase and the
  // picker opened a tab and only a tab.
  check("a plain pick opens a tab of its own, which is what it always did", landing("tab", 4), "tab");
  check("and asking for it here splits the pane the reader is in", landing("here", 4), { beside: 4 });
  // The case the button would otherwise be broken in.
  check(
    "asking for here with nothing open opens a tab rather than nothing",
    landing("here", null),
    "tab",
  );
  // Pane ids start at zero, and `if (focused)` would have sent the first pane
  // of the first tab a reader ever opens down the no-tab path.
  check("pane zero is a pane", landing("here", 0), { beside: 0 });
}
