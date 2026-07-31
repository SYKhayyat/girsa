// The door to the mefarshim, and whether a reader can find it.
//
// > *"i have no clue how to even open mefarshim."*
//
// The door was already there. `Shelf::companions` returns the works the corpus
// declares are commentaries on what you are reading, `picker.ts` renders them
// and marks each `פירוש`, and the pane header has a button that opens it.
//
// The button said **לצד** — *alongside*. Everything worked and nothing said the
// word a person is looking for. A reader who wants Rashi does not scan a toolbar
// for a preposition.
//
// So this file is about a label and an order, which is why it can be a unit test
// at all: `mefarshim.ts` holds the two decisions — what to call the door given
// what is behind it, and which sefer to offer first — as functions over plain
// data, the same way `preview.ts` holds B1's geometry.

import { check, ok, notOk } from "./harness.mjs";
import { mefarshim, ordered, doorLabel, doorTitle } from "../.tmp-test/mefarshim.mjs";

/** As `Shelf::companions` returns them: declared ones first, then by links. */
function companion(slug, declared, links) {
  return { slug, he_title: slug, en_title: slug, declared, links };
}

export function run() {

  const ON_BERAKHOT = [
    companion("bavli/rashi-on-berakhot", true, 3139),
    companion("bavli/tosafot-on-berakhot", true, 812),
    companion("beit-yosef", false, 815),
    companion("mishnah-berurah", false, 41),
  ];

  // ---------------------------------------------------------------- which are mefarshim

  check(
    "a declared commentary is a mefaresh and a sefer that merely shares edges is not",
    mefarshim(ON_BERAKHOT).map((c) => c.slug),
    ["bavli/rashi-on-berakhot", "bavli/tosafot-on-berakhot"],
  );

  // The Beit Yosef cites Berakhot 815 times and is not a commentary on it. That
  // distinction is the whole reason `declared` exists, and a count is not a claim.
  notOk(
    "the most-linked sefer is not promoted to a mefaresh by its link count",
    mefarshim(ON_BERAKHOT).some((c) => c.slug === "beit-yosef"),
  );

  check("a sefer with no companions has no mefarshim", mefarshim([]), []);

  // ---------------------------------------------------------------- what the door says

  check(
    "the door names what is behind it, and how much",
    doorLabel(ON_BERAKHOT),
    "מפרשים · 2",
  );

  // The button must still exist when there are none, because it also opens any
  // sefer beside this one — but it must not promise mefarshim it does not have.
  check("with no declared commentary the door does not claim any", doorLabel([]), "לצד");

  check(
    "with companions but none declared, still no claim",
    doorLabel([companion("beit-yosef", false, 815)]),
    "לצד",
  );

  ok(
    "the tooltip says both things the door does, so the label can stay short",
    doorTitle(ON_BERAKHOT).includes("מפרשים") && doorTitle(ON_BERAKHOT).includes("ספר"),
  );

  ok("the tooltip carries the shortcut", doorTitle([]).includes("Ctrl+\\"));

  // ---------------------------------------------------------------- what is offered first

  check(
    "mefarshim come before seforim that merely share edges, however many edges",
    ordered(ON_BERAKHOT).map((c) => c.slug),
    [
      "bavli/rashi-on-berakhot",
      "bavli/tosafot-on-berakhot",
      "beit-yosef",
      "mishnah-berurah",
    ],
  );

  // Within the declared ones, the one actually linked to this line is the one a
  // reader means. Rashi on a daf has thousands; a commentary that declares itself
  // and links nowhere is real but is not what to offer first.
  check(
    "among mefarshim, the better attached comes first",
    ordered([
      companion("declared-but-unlinked", true, 0),
      companion("bavli/rashi-on-berakhot", true, 3139),
    ]).map((c) => c.slug),
    ["bavli/rashi-on-berakhot", "declared-but-unlinked"],
  );

  check(
    "ordering does not drop or duplicate anything",
    ordered(ON_BERAKHOT).length,
    ON_BERAKHOT.length,
  );

  // Order must not depend on the order it arrived in, or the list reshuffles
  // between two openings of the same daf.
  check(
    "the order is the same whichever way the list arrives",
    ordered([...ON_BERAKHOT].reverse()).map((c) => c.slug),
    ordered(ON_BERAKHOT).map((c) => c.slug),
  );

}
