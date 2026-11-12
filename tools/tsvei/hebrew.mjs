// The one thing every Hebrew text store gets wrong first: the vowel points.
//
// A sefer's text is written with nikkud and te'amim — `בְּרֵאשִׁית` — and a
// reader searching it types `בראשית`. Postgres indexes what it is given, so a
// tsvector built from the pointed text holds `בְּרֵאשִׁית` as one token and the
// unpointed query matches nothing. The same is true of every diacritic in every
// language, and the repair is the same: **index the unpointed text and keep the
// pointed text for display.** Two columns, one of them for the eyes.
//
// The maqaf (`־`, U+05BE) is *inside* the diacritic block and is not a
// diacritic: it joins two words into one written unit, `בית־המקדש`, and
// deleting it would produce a token no reader would ever type. A word break is
// what a reader types, so a word break is what it becomes.

/** `בְּרֵאשִׁית בָּרָא` → `בראשית ברא`. Spaces collapsed, ends trimmed. */
export function unpointed(text) {
  return String(text)
    // Before the diacritic sweep, because the maqaf lives in the same block.
    .replace(/[\u05BE\u2010\u2011]/g, " ")
    .replace(/[\u0591-\u05C7]/g, "")
    // A soft hyphen and a zero-width joiner arrive from copied text and are not
    // letters; they are not visible either, so nothing is lost by dropping them.
    .replace(/[\u00AD\u200B-\u200D\uFEFF]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * Hebrew numerals for a chapter and a verse: `בראשית א׳, א׳`.
 *
 * The letters are the ordinary gematria, with the two-letter forms kept apart
 * (טו and טז are written י״ה and י״ו, never יה and יו, which spell the Name).
 */
export function gematria(n) {
  const letters = [
    [400, "ת"], [300, "ש"], [200, "ר"], [100, "ק"],
    [90, "צ"], [80, "פ"], [70, "ע"], [60, "ס"], [50, "נ"],
    [40, "מ"], [30, "ל"], [20, "כ"], [10, "י"],
    [9, "ט"], [8, "ח"], [7, "ז"], [6, "ו"], [5, "ה"],
    [4, "ד"], [3, "ג"], [2, "ב"], [1, "א"],
  ];
  let left = Math.trunc(n);
  if (!Number.isFinite(left) || left <= 0 || left >= 1000) return String(n);
  let out = "";
  for (const [value, letter] of letters) {
    while (left >= value) {
      out += letter;
      left -= value;
    }
  }
  // 15 and 16 are never יה and יו.
  out = out.replace(/^יה/, "טו").replace(/^יו/, "טז");
  if (out.length === 1) return `${out}׳`;
  return `${out.slice(0, -1)}״${out.slice(-1)}`;
}
