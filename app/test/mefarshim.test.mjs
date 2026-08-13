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
  everywhereSaid,
  marking,
  nothingHere,
  ticked,
} from "../.tmp-test/mefarshim.mjs";

/**
 * As `Shelf::companions` returns them: related ones first, then by links.
 *
 * `stands` and not a `declared` bool. The bool was true for three different
 * claims — a mefaresh **on** this sefer, a sefer running **alongside** it, and
 * the sefer this one is a mefaresh **on** — and the last of those is why opening
 * Onkelos listed Bereshis under the word `פירוש`.
 */
function companion(slug, stands, links) {
  return { slug, he_title: slug, en_title: slug, stands, links };
}

export function run() {

  const ON_BERAKHOT = [
    companion("bavli/rashi-on-berakhot", "on", 3139),
    companion("bavli/tosafot-on-berakhot", "on", 812),
    companion("beit-yosef", null, 815),
    companion("mishnah-berurah", null, 41),
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
    doorLabel([companion("beit-yosef", null, 815)]),
    "לצד",
  );

  ok(
    "the tooltip says both things the door does, so the label can stay short",
    doorTitle(ON_BERAKHOT).includes("מפרשים") && doorTitle(ON_BERAKHOT).includes("ספר"),
  );

  ok("the tooltip carries the shortcut", doorTitle([]).includes("Ctrl+\\"));

  // ------------------------------------------------- and what it does not count
  //
  // > *"The mefarshim door promises 67 and lists 76."*
  //
  // Both numbers were true. The face counts the commentaries the corpus
  // **declares**; the list also carries the seforim running alongside, the sefer
  // this one comments on, and the ones joined by edges alone — each under a
  // heading saying which it is. A reader who counts rows finds nine more than
  // the button promised and nothing reconciles them.
  //
  // Making the face say 76 would be worse: it would claim seventy-six mefarshim
  // over a list whose last nine rows are, in the list's own words, merely
  // linked. So the tooltip carries the rest, and this is what holds it.

  ok(
    "the tooltip accounts for the rows the count does not promise",
    doorTitle(ON_BERAKHOT).includes("2 מקושרים"),
  );

  const MIXED = [
    companion("bavli/rashi-on-berakhot", "on", 3139),
    companion("shulchan-arukh", "alongside", 12),
    companion("mishnah/berakhot", "base", 40),
    companion("beit-yosef", null, 815),
  ];
  const title = doorTitle(MIXED);
  ok(
    "each kind behind the door is named and counted separately",
    title.includes("1 על סדר הספר") &&
      title.includes("1 הספר שעליו נכתב") &&
      title.includes("1 מקושרים"),
  );
  ok("and the face still counts only the declared", doorLabel(MIXED) === "מפרשים · 1");

  // Nothing extra behind the door, nothing extra in the tooltip: a sentence
  // that says *also 0 joined by links* is a sentence about nothing.
  notOk(
    "with nothing else behind it the tooltip says nothing else",
    doorTitle([companion("bavli/rashi-on-berakhot", "on", 3139)]).includes(
      "מקושרים",
    ),
  );

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
      companion("declared-but-unlinked", "on", 0),
      companion("bavli/rashi-on-berakhot", "on", 3139),
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

  // ── the weave moved to Rust (W44) ─────────────────────────────────────────
  //
  // Which rows carry a tick-box, the folders, and the four sections were
  // asserted here, against hand-written TypeScript constants — beside
  // `crates/girsa-app/src/mefarshim.rs`, which carries twenty-five Rust tests
  // about this same list and could not see any of it.
  //
  // They are Rust tests now, in that module, against the same types the window
  // is sent. What is left here is what the *window* decides: what the door
  // should say given what is behind it, and how to word the sentence under the
  // list. Those are label composition, and they belong on this side.

  // ------------------------------------------------------- what the margin marks
  //
  // > *"Ticking a targum marks every line. 1,533 of Bereishis' 1,533; Rashi
  // > marks 356 of 400 drawn lines of Shabbos. The `◆` was designed so that
  // > marking everything would say nothing, and for the most obvious mefarshim
  // > it marks everything."*
  //
  // `Marks::marked` takes the ticked set so that the marker means *one of
  // yours* rather than *somebody's*. The care was real, and the first mefaresh
  // anybody ticks defeats it: a targum comments on every posuk by construction,
  // so the answer is `true` 1,533 times.

  // The complaint itself. Every line marked, one mefaresh on each.
  const targum = Object.fromEntries(Array.from({ length: 1533 }, (_, i) => [`l${i}`, 1]));
  check(
    "a mefaresh who speaks on every line is not marked on every line",
    marking(targum, 1533).kind,
    "everywhere",
  );

  // And Rashi on Shabbos, which is the case the same fix must **not** swallow:
  // the lines without a diamond are the reader's answer to where Rashi stops.
  const rashi = Object.fromEntries(Array.from({ length: 356 }, (_, i) => [`l${i}`, 1]));
  check(
    "a mefaresh who speaks on most lines keeps the marker on the ones they speak on",
    marking(rashi, 400).kind,
    "some",
  );

  // Uniform in *coverage* and not in **count** is still worth drawing: this is
  // the half a bool threw away, and the whole reason the count is on the wire.
  const mixed = { a: 1, b: 3, c: 2 };
  check("every line marked, different numbers, still a marker", marking(mixed, 3).kind, "some");

  check("nothing ticked marks nothing", marking({}, 400).kind, "none");
  check("nothing ticked is not the same answer as everywhere", marking({}, 0).kind, "none");

  // The number the sentence carries is the one every line shares, so a reader
  // who ticked three targumim is told three and not one.
  const three = Object.fromEntries(Array.from({ length: 20 }, (_, i) => [`l${i}`, 3]));
  const how = marking(three, 20);
  check("and it says how many of them, not just that there are some", how.each, 3);
  ok("the sentence carries that number", everywhereSaid(3).includes("3"));

  // One mefaresh on every line does not read as `1 of the mefarshim you ticked`
  // — a count of one is a sentence about a person, and the words are different.
  notOk("one on every line is said without a number", everywhereSaid(1).includes("1"));
}
