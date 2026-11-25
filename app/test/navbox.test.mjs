// What the window does with each answer the one box gets.
//
// `nav_ask` returns six things on purpose, and the window has to do something
// different with each, because they are six different things for a reader to do
// next: read the page you asked for, read two pages one after another, read two
// passages as one sheet, choose between them, answer a question, or be told the
// words meant nothing here.
//
// The failure this file is written against is a shape that falls through to
// *nothing* — the window saying *those words meant nothing* when the library had
// understood perfectly well. `intentOf` is the one place that reading happens,
// so it is the one place that can be tested without a webview.

import { check } from "./harness.mjs";
import { intentOf } from "../.tmp-test/navbox.mjs";

/** An answer of one shape, with only the fields the reading touches. */
const go = { kind: "go", landing: { kind: "at", ids: ["1"] } };
const several = (refs, sheet = false) => ({ kind: "several", refs, unread: [], sheet });
const choose = (refs) => ({ kind: "choose", refs });
const ask = (about) => ({ kind: "ask", about });
const nothing = { kind: "nothing" };

/** Just enough of an answer to see what it was read as. */
const reading = (answer) => {
  const got = intentOf(answer);
  return [got.what, got.refs, got.sheet, got.refs.length > 0 || got.landing !== null];
};

export function run() {
  // --- one place -------------------------------------------------------------
  //
  // The landing travels with it, because the window goes straight there and a
  // second round trip to ask *what did that ref mean* would be asking the same
  // question twice.
  check("one place is a landing, not a ref to look up", reading(go), ["go", [], false, true]);
  const landed = intentOf(go);
  check("and the landing is the one the answer carried", landed.landing, go.landing);

  // --- two places ------------------------------------------------------------
  //
  // The parentheses are the whole difference: the same refs, one intention to
  // read them one after another and another to read them as a single sheet.
  check(
    "two places without parentheses are two visits",
    reading(several(["a/1", "b/2"])),
    ["several", ["a/1", "b/2"], false, true],
  );
  check(
    "the same two in parentheses are one sheet",
    reading(several(["a/1", "b/2"], true)),
    ["sheet", ["a/1", "b/2"], true, true],
  );
  // One ref in parentheses is still a sheet — the reader asked for an assembly of
  // what they named, and that they named one thing is not a reason to read their
  // parentheses as decoration.
  check(
    "even one place in parentheses is a sheet",
    reading(several(["a/1"], true)),
    ["sheet", ["a/1"], true, true],
  );
  // An empty list is not a visit to nowhere: it carries nothing to open, and the
  // window must not open a blank pane for it.
  check("an empty list opens nothing", reading(several([])), ["several", [], false, false]);

  // --- what could not be read stays attached --------------------------------
  //
  // `unread` travels beside the refs rather than instead of them (#74). An answer
  // that dropped it would land the reader on the first place and say nothing
  // about the one it could not read, which is the partial answer #74 exists to
  // make honest.
  const partial = intentOf({
    kind: "several",
    refs: ["a/1"],
    unread: [{ kind: "nothing", typed: "shabbos 3b" }],
    sheet: false,
  });
  check("the parts that could not be read are still there", partial.unread.length, 1);
  check("and the parts that could are still opened", partial.refs, ["a/1"]);

  // --- a question, and a choice ---------------------------------------------
  check("a bare location asks which sefer", reading(ask("a sefer")), ["ask", [], false, false]);
  check("and the question travels", intentOf(ask("a sefer")).about, "a sefer");
  check(
    "words that name two seforim are a choice of two",
    reading(choose(["bavli/berakhot", "mishnah-berakhot"])),
    ["choose", ["bavli/berakhot", "mishnah-berakhot"], false, true],
  );

  // --- nothing ---------------------------------------------------------------
  check("nothing matched is its own answer", reading(nothing), ["nothing", [], false, false]);
}
