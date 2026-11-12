// The two facts every part of this tool depends on: which letters come off, and
// which ones a chapter number is written with.

import { test } from "node:test";
import assert from "node:assert/strict";

import { gematria, unpointed } from "../hebrew.mjs";

test("the points come off and the letters stay", () => {
  assert.equal(unpointed("בְּרֵאשִׁית בָּרָא אֱלֹהִים"), "בראשית ברא אלהים");
  // Te'amim (the cantillation) are in the same block and go the same way.
  assert.equal(unpointed("וַיֹּ֥אמֶר אֱלֹהִ֖ים"), "ויאמר אלהים");
  // A plain string is returned as it stands.
  assert.equal(unpointed("שלום עולם"), "שלום עולם");
});

test("a maqaf is a word break and not a letter to delete", () => {
  // Deleting it would make `ביתהמקדש`, which is a word nobody types. A reader
  // searching the pair types a space, so a space is what it becomes.
  assert.equal(unpointed("בית־המקדש"), "בית המקדש");
});

test("the invisible characters copied text carries are dropped", () => {
  assert.equal(unpointed("אב\u200bגד"), "אבגד");
  assert.equal(unpointed("  רווח   כפול  "), "רווח כפול");
});

test("a chapter is written the way a sefer writes it", () => {
  const cases = [
    [1, "א׳"],
    [3, "ג׳"],
    [10, "י׳"],
    [14, "י״ד"],
    // 15 and 16 are never יה and יו, which spell the Name.
    [15, "ט״ו"],
    [16, "ט״ז"],
    [21, "כ״א"],
    [99, "צ״ט"],
    [100, "ק׳"],
    [123, "קכ״ג"],
    [400, "ת׳"],
  ];
  for (const [n, written] of cases) assert.equal(gematria(n), written, `${n}`);
});

test("a number that is not a number is printed as it is", () => {
  // Better a `0` in the reference than a crash halfway through an import.
  assert.equal(gematria(0), "0");
  assert.equal(gematria(-2), "-2");
  assert.equal(gematria(1200), "1200");
});
