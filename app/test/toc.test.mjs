// The two decisions the table of contents makes (A3).
//
// > *"there should be a table of contents on the side for each sefer, so you
// > can jump around."* And: *"their toc … is easy to steal and much better than
// > ours."*
//
// What the contents *are* is Rust's — `girsa_app::contents`, built from the
// segments' own addresses. What is here is the two things the panel decides
// about them, and both are the kind of thing that is wrong in a way nobody
// files: an off-by-one marks the siman above the one you are reading, and a
// filter that matches the wrong field shows an empty column on a sefer that
// names nothing.

import { check } from "./harness.mjs";
import { inside, matching } from "../.tmp-test/tocview.mjs";

/** A row of the contents, with only the fields these decisions read. */
function entry(from, address, title) {
  return { at: `girsa:x/${from}`, address, title, depth: 0, from };
}

export function run() {
  // --- which entry the reader is inside --------------------------------------
  //
  // *Inside* means the last one that began at or before you. Standing on se'if
  // 4 of siman 12, the place you are in is siman 12 — not siman 13, which has
  // not started, and not "none", which is what a strict match against the
  // siman's own first segment would say for every se'if but the first.
  const simanim = [entry(0, "סימן א'"), entry(14, "סימן ב'"), entry(31, "סימן ג'")];
  check("the first line of a siman is inside it", inside(simanim, 14), 1);
  check("and so is every line after it, until the next one", inside(simanim, 30), 1);
  check("the next siman's first line is inside the next siman", inside(simanim, 31), 2);
  check("and the last siman runs to the end of the sefer", inside(simanim, 4000), 2);
  // Before the first entry is a real place: the front matter of a sefer sits
  // there, and answering `0` would mark siman א while the reader is looking at
  // the הקדמה.
  check(
    "a line before the first entry is inside none of them",
    inside([entry(6, "סימן א'")], 2),
    -1,
  );
  check("and a sefer with no contents has nothing to be inside", inside([], 9), -1);

  // --- what a typed filter keeps ---------------------------------------------
  //
  // Otzaria filters on the heading text alone, which works because its corpus
  // is headings. Half of this one names nothing — Berakhos has 125 dapim and
  // not one title — so filtering by title only would answer *no such place* to
  // a reader typing a daf, on the seforim where a table of contents is most of
  // what there is to navigate by.
  const mixed = [
    entry(0, "סימן א'", "מי הם הכשרים לשחוט"),
    entry(9, "סימן ב'", "אם שחיטת עובד כוכבים כשרה"),
    entry(20, "דף ל."),
  ];
  check("nothing typed keeps everything", matching(mixed, "").length, 3);
  check("and so does whitespace", matching(mixed, "   ").length, 3);
  check(
    "a word matches the title",
    matching(mixed, "שחיטת").map((e) => e.address),
    ["סימן ב'"],
  );
  check(
    "and an address matches the address, on the seforim that name nothing",
    matching(mixed, "דף ל").map((e) => e.address),
    ["דף ל."],
  );
  check("and something in neither keeps nothing", matching(mixed, "זזזז"), []);
}
