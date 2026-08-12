// The window has a language of its own, and both columns of the table are full.
//
// > *"there is no way to change UI into english - only seforim names. there
// > should be 2 seperate commands."*
//
// Two things could go wrong with the fix and neither would be visible to
// whoever made the change:
//
//  1. a row with one language filled in — the reader switches to English and
//     finds a Hebrew button in the middle of a sentence;
//  2. a Hebrew string typed straight into a module, which is what every one of
//     these used to be, and which no amount of switching will translate.
//
// The first is checked over the table. The second is checked by **reading the
// source**, the same way `sources.test.mjs` checks for the sibling's misspelt
// name — a unit test of any one module passes while nineteen others still hold
// literals, which is exactly how this got to be the state it was in.

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { check, ok } from "./harness.mjs";
import { dirOf } from "../tools/paths.mjs";
import { everyWord, say, sayIn, speakInterface } from "../.tmp-test/say.mjs";

const HERE = dirOf(import.meta.url);
const SRC = path.resolve(HERE, "..", "src");

/**
 * Modules whose Hebrew is not interface text.
 *
 * `say.ts` **is** the table. `names.ts` holds the two applications' own names,
 * which are names and are not translated. `writing.ts` holds Ksav markup
 * keywords — `#כותרת1[…]` is a tag in a document format, and translating it
 * would produce a document Ksav cannot compile.
 */
const NOT_INTERFACE_TEXT = new Set(["say.ts", "names.ts"]);

/** Markup keywords and other Hebrew that is data rather than words. */
const NOT_A_SENTENCE = [/#[֐-׿_0-9]+\[/, /^"[֐-׿]"$/];

/** A comment is not something a reader sees; these files discuss the bug. */
function isComment(line) {
  const s = line.trim();
  return s.startsWith("//") || s.startsWith("*") || s.startsWith("/*");
}

export async function run() {
  // ------------------------------------------------------------ the table

  const words = everyWord();
  ok("there is a table to check", words.length > 50);

  const empty = [];
  for (const word of words) {
    if (!sayIn(word, "hebrew").trim()) empty.push(`${word}: no Hebrew`);
    if (!sayIn(word, "english").trim()) empty.push(`${word}: no English`);
  }
  check("every row is filled in on both sides", empty, []);

  // The two that must differ, because they are the point: if a key came back
  // the same in both languages for every row, the switch would be doing nothing
  // and this file would still be green.
  ok(
    "the two columns are not the same column",
    words.some((word) => sayIn(word, "hebrew") !== sayIn(word, "english")),
  );

  // ------------------------------------------------------------ the switch

  speakInterface("hebrew");
  check("the window speaks Hebrew when told to", say("settings"), "הגדרות");
  speakInterface("english");
  check("…and English when told that", say("settings"), "Settings");
  speakInterface("hebrew");

  // ------------------------------------------------- and nothing outside it

  const names = (await readdir(SRC)).filter((f) => f.endsWith(".ts"));
  ok("there is source to check", names.length > 10);

  const literals = [];
  for (const file of names) {
    if (NOT_INTERFACE_TEXT.has(file)) continue;
    const body = await readFile(path.join(SRC, file), "utf8");
    body.split("\n").forEach((line, i) => {
      if (isComment(line)) return;
      // A double-quoted string with a Hebrew letter in it, outside a comment.
      for (const found of line.matchAll(/"[^"]*[֐-׿][^"]*"/g)) {
        if (NOT_A_SENTENCE.some((allowed) => allowed.test(found[0]))) continue;
        literals.push(`${file}:${i + 1} ${found[0]}`);
      }
    });
  }
  check(
    "no module outside the table carries a Hebrew string a reader will see",
    literals,
    [],
  );
}
