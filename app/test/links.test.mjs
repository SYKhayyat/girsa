// The one decision the links panel makes about its own rows.
//
// > *"all of it — i don't know what i'm looking at."* And: *"why are there
// > repeats. i just don't get it."*
//
// There were no repeats. On one se'if of Yoreh De'ah the Kaf HaChayim writes
// ס״ק א׳ through ס״ק ע״ח, so the panel drew seventy-eight rows carrying the same
// eight words and a different number — 280 rows from 61 seforim, and what
// repeated down the column was the **sefer's name**.
//
// What the panel decides is how to gather them, and it is the kind of thing
// that is wrong in a way nobody files: gather by title and two different
// seforim with one name merge; sort while gathering and the order the engine
// answered in is quietly replaced by an alphabet.

import { check } from "./harness.mjs";
import { grouped } from "../.tmp-test/linksview.mjs";

/** A row, with only the fields the grouper reads. */
function link(work, title, said) {
  return { work, title, said, at: `${work}#1` };
}

export function run() {
  // --- gathered by sefer, in the order they arrived --------------------------
  const rows = [
    link("kaf-hachayim", "כף החיים", "ס\"ק א'"),
    link("kaf-hachayim", "כף החיים", "ס\"ק ב'"),
    link("shach", "ש\"ך", "ס\"ק א'"),
    link("kaf-hachayim", "כף החיים", "ס\"ק ג'"),
  ];
  const groups = grouped(rows);
  check("one line per sefer, not one per link", groups.length, 2);
  check("the first sefer is the first one that appeared", groups[0].work, "kaf-hachayim");
  check("a sefer's rows are gathered even when they are not adjacent", groups[0].links.length, 3);
  check("the order the engine answered in survives", groups[1].work, "shach");

  // --- by slug and not by title ---------------------------------------------
  //
  // Two seforim can be called one thing. `כף החיים` is a Sefaria title on more
  // than one work, and merging them would make the panel say one sefer says
  // something it does not.
  const twins = grouped([
    link("kaf-hachayim/yoreh-deah", "כף החיים", "ס\"ק א'"),
    link("kaf-hachayim/orach-chayim", "כף החיים", "ס\"ק א'"),
  ]);
  check("two seforim with one name stay two", twins.length, 2);

  // --- nothing in, nothing out -----------------------------------------------
  check("no links is no groups, not one empty one", grouped([]).length, 0);

  // --- every link survives ----------------------------------------------------
  //
  // The count under the sefer's name is what tells a reader the seventy-eight
  // rows are a run rather than a repetition, so it has to be all of them.
  const many = Array.from({ length: 78 }, (_, n) => link("kaf-hachayim", "כף החיים", `ס"ק ${n}`));
  const one = grouped([...many, link("shach", "ש\"ך", "ס\"ק א'")]);
  check("every row is in a group", one.reduce((n, g) => n + g.links.length, 0), 79);
  check("and the big one is the big one", one[0].links.length, 78);
}
