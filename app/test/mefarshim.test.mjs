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
import {
  mefarshim,
  ordered,
  doorLabel,
  doorTitle,
  nothingHere,
  ticked,
  choices,
} from "../.tmp-test/mefarshim.mjs";

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

  // ------------------------------------------------- clicking a line that opens nothing

  // W43's fifth test: *clicking an unmarked line says so, rather than opening an
  // empty panel*. There are four reasons a click can come back with nothing and
  // they are four different sentences. Collapsing them into one — "no comments" —
  // is the failure mode this codebase has spent the week removing: a reader who
  // has ticked nobody would be told nobody wrote.
  const SOMETHING = { said: [{ work: "bavli/rashi-on-berakhot", lines: [] }], others: false };

  check("when there is something to read, nothing is said", nothingHere(SOMETHING, 3), "");

  ok(
    "having ticked nobody is reported as that, not as nobody having written",
    nothingHere({ said: [], others: true }, 0).includes("סמן"),
  );

  ok(
    "a line others wrote on says they did, so a reader can widen their list",
    nothingHere({ said: [], others: true }, 3).includes("לא סימנת"),
  );

  ok(
    "a line nobody wrote on says nobody wrote on it",
    nothingHere({ said: [], others: false }, 3).includes("אין"),
  );

  // The message for *you ticked nobody* must not depend on whether anybody wrote
  // here, because the reader's next move is the same either way: tick somebody.
  check(
    "with nobody ticked the advice is the same whether or not others wrote",
    nothingHere({ said: [], others: false }, 0),
    nothingHere({ said: [], others: true }, 0),
  );

  // ------------------------------------------------- the sentence under the tick-list

  ok(
    "the tick-list says how much of the sefer is commented on at all",
    ticked(2749, 0).includes("2749"),
  );

  ok("and how many you have ticked", ticked(2749, 3).includes("3"));

  // A sefer nothing comments on says that plainly. `0 שורות` is arithmetic; a
  // sentence is an answer.
  ok(
    "a sefer with no commentary at all says so rather than counting to zero",
    ticked(0, 0).includes("אין"),
  );

  // ------------------------------------------------- which rows carry a tick-box

  // One list, two jobs: click a row to open that sefer in the column beside you
  // (the split, which the reader asked to keep), or tick it to have its comments
  // marked on the daf. Every row does the first. Only a row whose comments the
  // link graph can actually place does the second — a checkbox that will never
  // mark a line teaches the reader the ticks do nothing.
  const CAN_MARK = [
    { slug: "bavli/rashi-on-berakhot", he_title: "רש״י", en_title: "Rashi", chosen: true },
    { slug: "bavli/ben-yehoyada", he_title: "בן יהוידע", en_title: "Ben Yehoyada", chosen: false },
  ];
  const DECLARED = [
    companion("bavli/rashi-on-berakhot", true, 3139),
    companion("bavli/tosafot-on-berakhot", true, 812),
  ];

  const rows = choices(DECLARED, CAN_MARK);

  check(
    "a mefaresh the graph can place is tickable, and says whether it is ticked",
    rows.filter((r) => r.tickable).map((r) => [r.slug, r.chosen]),
    [
      ["bavli/rashi-on-berakhot", true],
      ["bavli/ben-yehoyada", false],
    ],
  );

  // Tosafot on Berakhot declares itself a commentary and this graph has no edges
  // from it. It is still offered — the split opens it fine — but ticking it would
  // mark nothing, so it gets no tick-box rather than a dead one.
  notOk(
    "a declared commentary with no edges in the graph is offered but not tickable",
    rows.find((r) => r.slug === "bavli/tosafot-on-berakhot")?.tickable,
  );

  // The Ben Yehoyada is not in Sefaria's declared list for Berakhot and comments
  // on it all the same. Dropping it would hide a real mefaresh behind a metadata
  // gap.
  ok(
    "a mefaresh the graph knows and the metadata does not is still offered",
    rows.some((r) => r.slug === "bavli/ben-yehoyada"),
  );

  check("nothing is listed twice", rows.length, 3);

  check(
    "the declared ones keep their order and the rest follow",
    rows.map((r) => r.slug),
    ["bavli/rashi-on-berakhot", "bavli/tosafot-on-berakhot", "bavli/ben-yehoyada"],
  );

  check("with no graph read yet, every row is a row and none is tickable",
    choices(DECLARED, []).filter((r) => r.tickable).length, 0);

}
