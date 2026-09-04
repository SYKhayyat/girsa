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
import {
  buildRow,
  firstRow,
  inside,
  matching,
  nearestScroll,
  rowTop,
  windowed,
} from "../.tmp-test/tocview.mjs";

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

  // --- the window: which rows get drawn (Issue #13) --------------------------
  //
  // The filter redraw used to build every matching row as a fresh `<button>`,
  // which for a sefer whose table fills seventeen thousand rows is a freeze on
  // the UI thread per keystroke. The window is the fix: the slice drawn is
  // bounded by the *viewport*, so a redraw costs the same on a table of forty
  // and a table of 17,418. These assert the arithmetic, which is the whole of
  // the guarantee — if `windowed` ever answers in terms of `total` again, this
  // is the test that says so.
  const MB = 17418;
  check(
    "the window holds a viewport, not the list — Mishnah Berurah draws ~40 rows",
    windowed(MB, 600, 0).count,
    40,
  );
  check("and starts at the row asked for", windowed(MB, 600, 0).first, 0);
  check("mid-list, the window starts where the scroll says", windowed(MB, 600, 9000).first, 9000);
  check(
    "the count never depends on how far down the list you are",
    windowed(MB, 600, 9000).count,
    40,
  );
  check("an empty list draws nothing", windowed(0, 600, 0), { first: 0, count: 0 });
  check("a list shorter than the viewport draws all of it", windowed(10, 600, 0).count, 10);
  check(
    "the last rows are not cut off by the count",
    windowed(MB, 600, MB - 1).first + windowed(MB, 600, MB - 1).count,
    MB,
  );
  check("the window clamps a negative ask to the top", windowed(MB, 600, -50).first, 0);
  check("and a too-large ask to the last row", windowed(MB, 600, MB + 99).first, MB - 1);

  // --- scroll arithmetic -----------------------------------------------------
  //
  // `firstRow` answers *which row is at the top of the viewport* from the
  // scroll position, `rowTop` is its inverse, and `nearestScroll` is what the
  // old `scrollIntoView({ block: "nearest" })` answered — computed rather than
  // asked for, because the row to scroll to is not always in the document.
  check("row zero sits just inside the top pad", rowTop(0), 4);
  check("each row is ROW below the one before", rowTop(3) - rowTop(2), 27);
  check("a scroll to the first row's top asks for row zero", firstRow(4), 0);
  check("a scroll into row three asks for row three", firstRow(4 + 3 * 27 + 1), 3);
  check("and a scroll at the very top still asks for row zero", firstRow(0), 0);
  check("a scroll before the pad is row zero, not a negative", firstRow(1), 0);
  check("a row already in view needs no scroll", nearestScroll(5, 600, rowTop(5)), null);
  check("a row cut off above scrolls up to it", nearestScroll(5, 600, rowTop(5) + 10), rowTop(5));
  check(
    "a row past the bottom edge scrolls so it enters at the bottom",
    nearestScroll(30, 600, rowTop(0)),
    rowTop(30) + 27 - 600,
  );

  // --- the same fix, as a number --------------------------------------------
  //
  // `buildRow` is the unit of DOM work the windowing counts, so measure one
  // against a fake document that counts operations, and compare drawing *all*
  // of a Mishnah Berurah-sized table (the old redraw) with drawing the window
  // (the new one). The exact op counts are a property of the fake DOM and not
  // of a browser; the *ratio* is what the fix is about, and the assertion is
  // the ratio being enormous rather than any specific count.
  const ops = [];
  for (const entry of [
    { at: "x", address: "א", title: "ב", depth: 0 },
    { at: "y", address: "א", depth: 1 },
  ]) {
    const fake = countingDocument();
    const prev = globalThis.document;
    globalThis.document = fake.document;
    try {
      buildRow(entry, () => {});
    } finally {
      globalThis.document = prev;
    }
    ops.push(fake.count);
  }
  const perRowTitled = ops[0];
  const perRowBare = ops[1];
  const viewportRows = windowed(MB, 600, 0).count;
  // The old redraw built every match and walked every row again in `mark()`.
  const beforeOps = MB * perRowBare + MB * 2;
  // The new redraw builds the window and walks it.
  const afterOps = viewportRows * perRowBare + viewportRows * 2;
  check(
    "drawing the window touches fewer rows than drawing the list",
    viewportRows < MB,
    true,
  );
  check("the window is a viewport, not a fraction of the list", viewportRows, 40);
  check(
    `a redraw builds 40 rows (${afterOps} fake-DOM ops) where it built 17,418 (${beforeOps})`,
    afterOps * 100 < beforeOps,
    true,
  );

  // The filter itself was never the cost — the DOM construction was — and the
  // issue says as much. Measured, so the claim has a number: scanning 17,418
  // short titles and addresses is milliseconds, which is why the fix left the
  // filter in TypeScript at all.
  {
    const big = berakhotSized();
    const t0 = performance.now();
    const all = matching(big, "");
    const some = matching(big, "5").length;
    const dt = performance.now() - t0;
    check("the filter still answers correctly at 17,418 entries", [all.length, some > 0], [17418, true]);
    console.log(`      (toc: matching() over 17,418 entries took ${dt.toFixed(1)}ms — the scan was never the freeze)`);
  }
}

/** A document that counts what a browser would have to do for real. */
function countingDocument() {
  const state = { count: 0 };
  const element = () => {
    state.count += 1;
    return {
      type: "",
      className: "",
      textContent: "",
      dataset: {},
      style: {
        setProperty: () => {
          state.count += 1;
        },
      },
      append: () => {
        state.count += 1;
      },
      setAttribute: () => {
        state.count += 1;
      },
      addEventListener: () => {
        state.count += 1;
      },
    };
  };
  return {
    get count() {
      return state.count;
    },
    document: {
      createElement: () => element(),
    },
  };
}

/** Mishnah Berurah's 17,418 segments, as TOC rows with digit addresses. */
function berakhotSized() {
  const out = [];
  for (let i = 0; i < 17418; i += 1) {
    out.push({ at: `girsa:x/${i}`, address: `סימן ${(i % 1200) + 1}`, depth: 0, from: i });
  }
  return out;
}
