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
import { intentOf, NavBox } from "../.tmp-test/navbox.mjs";

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

export async function run() {
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

  // --- the name was nearly right (#63) --------------------------------------
  //
  // The failure this guards is the sentence at the top of the file: an answer the
  // library understood perfectly well, read as *nothing*. `didYouMean` and `which`
  // are the shelf's opinion, and the window has to be able to tell them from the
  // lexicon's silence — one is a question a reader can accept, and the other is a
  // reader retyping from scratch.
  const near = intentOf({
    kind: "didYouMean",
    typed: "brachos 2a",
    slug: "bavli/berakhot",
    title: "\u05d1\u05e8\u05db\u05d5\u05ea",
    corrected: "\u05d1\u05e8\u05db\u05d5\u05ea 2a",
  });
  check("a nearly-right name is a question, not silence", near.what, "didYouMean");
  check("and the question offers the corrected line", near.corrected, [
    "\u05d1\u05e8\u05db\u05d5\u05ea 2a",
  ]);
  const unsure = intentOf({
    kind: "which",
    typed: "bava",
    candidates: [
      { slug: "bavli/bava-kamma", title: "kamma", corrected: "kamma 2a" },
      { slug: "bavli/bava-metzia", title: "metzia", corrected: "metzia 3b" },
    ],
  });
  check("two close seforim are a choice of two", unsure.what, "which");
  check("and both lines are offered", unsure.corrected, ["kamma 2a", "metzia 3b"]);

  await offeringTheQuestion();
}

/**
 * The offer, as a screen: the question under the box and the lines as choices.
 *
 * The failure this guards is a question with nowhere to click — and, on the other
 * side, a choice that resolved the line itself. Accepting one puts the corrected
 * **line** back in the box and asks the same question again, so there is still one
 * route from words to a place.
 */
async function offeringTheQuestion() {
  const realDocument = globalThis.document;
  globalThis.document = {
    createElement: (tag) => node(tag),
    documentElement: { classList: { toggle() {} }, lang: "", dir: "" },
  };
  try {
    const box = new NavBox();
    box.show(node("div"));
    const asked = [];
    box.onAsk(async (typed) => {
      asked.push(typed);
      return null;
    });

    box.offer("not found. did you mean this?", [
      { label: "one 2a", typed: "one 2a" },
      { label: "one 3a", typed: "one 3a" },
    ]);
    const offered = choices(box);
    check("the question is said under the box", said(box), "not found. did you mean this?");
    check("every line is a choice", offered.length, 2);
    check("a choice is the whole line and not the name", offered[0].textContent, "one 2a");

    fire(offered[0]);
    await Promise.resolve();
    await Promise.resolve();
    check("accepting one asks the corrected line", asked, ["one 2a"]);
    check("and the old choices are gone", choices(box).length, 0);

    box.offer("again?", [{ label: "x", typed: "x" }]);
    check("a second question brings its own choices", choices(box).length, 1);
    box.close();
    check("closing takes them away", choices(box).length, 0);
  } finally {
    globalThis.document = realDocument;
  }
}

/** The choices the box is currently offering. */
function choices(box) {
  const list = box.element.children.find((kid) => kid.className === "nav-choices");
  return list ? list.children : [];
}

/** The sentence under the box. */
function said(box) {
  const note = box.element.children.find((kid) => kid.className === "nav-note");
  return note ? note.textContent : null;
}

/** Fire a fake element's first listener for `click`. */
function fire(element) {
  const first = (element.listeners.click ?? [])[0];
  if (first) first();
}

/** Enough of an element for the box to be built against. */
function node(tag) {
  return {
    tagName: tag,
    className: "",
    textContent: "",
    value: "",
    placeholder: "",
    type: "",
    dir: "",
    hidden: false,
    offsetHeight: 34,
    children: [],
    listeners: {},
    classList: { add() {}, remove() {}, contains: () => false, toggle() {} },
    style: { setProperty() {} },
    append(...kids) {
      this.children.push(...kids);
    },
    replaceChildren(...kids) {
      this.children = [...kids];
    },
    querySelector: () => null,
    addEventListener(type, fn) {
      (this.listeners[type] ??= []).push(fn);
    },
    remove() {},
    focus() {},
    select() {},
    setAttribute() {},
  };
}
