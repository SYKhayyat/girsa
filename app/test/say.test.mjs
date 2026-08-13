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
import {
  everyWord,
  interfaceLanguage,
  nextLoadSpeaks,
  say,
  sayIn,
  speakInterface,
} from "../.tmp-test/say.mjs";

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

/**
 * Hebrew that is data rather than words, taken **off the line** before it is
 * looked at.
 *
 * Subtraction and not a list of allowed lines, because an allowed line is a
 * line where a real sentence can hide next to a legitimate one. What is left
 * after these three is either interface text or nothing.
 */
function withoutData(line) {
  return (
    line
      // A Ksav markup keyword. `#כותרת1[…]` is a tag in a document format;
      // translating it produces a document Ksav cannot compile.
      .replace(/#[֐-׿_0-9]+\[/g, "")
      // A one-letter prefix — `ksavAs("ל")`. Hebrew glues *to* and *in* onto
      // the front of a word; the letter is grammar, not a sentence.
      .replace(/"[֐-׿]"/g, "")
      // A Unicode range in a regular expression's character class. `api.ts`
      // strips nikud with `/[֑-ׇ]/` written out as the letters
      // themselves, and a range of code points is not words in any language.
      .replace(/\/\[[^\]]*\]\/[a-z]*/g, "")
  );
}

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

  // -------------------------------------------------- the cache, and finding 2
  //
  // Every panel builds its titles, buttons and placeholders **in its
  // constructor**, at module load — before `main()` has asked Rust anything —
  // so what they are built from is the cache, and the only thing that relabels
  // them is a reload. The switch used to write the cache *after* those
  // constructors had already read it, so the reload following a language change
  // rebuilt the window in the language before the one the reader had chosen:
  //
  // ```
  // after switching to English:  toolbar English, shelf Hebrew
  // after switching back:        toolbar Hebrew,  shelf English
  // ```
  //
  // …in both directions, until the application was restarted. The invariant
  // that was missing is the one below: **what the window is saying now and what
  // the next load will be built from may never disagree.**

  const store = new Map();
  globalThis.localStorage = {
    getItem: (k) => store.get(k) ?? null,
    setItem: (k, v) => store.set(k, String(v)),
  };
  try {
    for (const language of ["english", "hebrew", "english"]) {
      speakInterface(language);
      check(
        `switching to ${language} leaves the cache saying the same thing`,
        nextLoadSpeaks(),
        interfaceLanguage(),
      );
    }
  } finally {
    delete globalThis.localStorage;
    speakInterface("hebrew");
  }

  // …and the reload that acts on it lives in the same module, so the two cannot
  // be put in the wrong order at a call site again. That ordering, spread over
  // two files, was the entire defect.
  const reloaders = [];
  for (const file of (await readdir(SRC)).filter((f) => f.endsWith(".ts"))) {
    if (file === "say.ts") continue;
    const body = await readFile(path.join(SRC, file), "utf8");
    body.split("\n").forEach((line, i) => {
      if (isComment(line)) return;
      if (/location\.reload\s*\(/.test(line)) reloaders.push(`${file}:${i + 1}`);
    });
  }
  check("only the module that owns the cache reloads the window", reloaders, []);

  // ------------------------------------------------------------- the holes
  //
  // A row with a `{name}` in it is a sentence with a number, a filename or the
  // sibling's name in the middle, filled by `fill`. The two columns are allowed
  // to put the hole in different places — that is the point, because *3 of 8
  // pages read* and *3 מתוך 8 עמודים נקראו* do not order them the same way —
  // but they must be the **same holes**. A translation that dropped `{count}`
  // would print a sentence with a number missing out of the middle of it, and
  // nothing else in this codebase would notice.

  const holes = (said) => [...said.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
  const mismatched = [];
  let filled = 0;
  for (const word of words) {
    const he = holes(sayIn(word, "hebrew"));
    const en = holes(sayIn(word, "english"));
    if (he.length > 0 || en.length > 0) filled += 1;
    if (he.join(",") !== en.join(",")) {
      mismatched.push(`${word}: hebrew has {${he.join("} {")}}, english has {${en.join("} {")}}`);
    }
  }
  ok("there are sentences with holes in them", filled > 20);
  check("both columns of a row have the same holes in them", mismatched, []);

  // ------------------------------------------------- and nothing outside it

  const names = (await readdir(SRC)).filter((f) => f.endsWith(".ts"));
  ok("there is source to check", names.length > 10);

  // **Any Hebrew letter on a line a reader could see, not a Hebrew letter
  // between double quotes.**
  //
  // This read `/"[^"]*[֐-׿][^"]*"/` for as long as it existed, and passed over
  // a window with `${notes.length} הערות` in it five times over, because a
  // template literal is not a double-quoted string. The guard fitted the bug
  // that was — twenty modules of `"הגדרות"` — and the bug that is arrived in
  // backticks, which is the whole of the second sitting's second lesson.
  //
  // So the assertion is the **class**: outside the table and outside a
  // comment, this source is not written in Hebrew. Nothing has to be said
  // about how a string is quoted, spliced, or continued onto a second line,
  // and the next way to smuggle one in is caught before it is invented.
  const literals = [];
  for (const file of names) {
    if (NOT_INTERFACE_TEXT.has(file)) continue;
    const body = await readFile(path.join(SRC, file), "utf8");
    body.split("\n").forEach((line, i) => {
      if (isComment(line)) return;
      if (!/[֐-׿]/.test(withoutData(line))) return;
      literals.push(`${file}:${i + 1} ${line.trim()}`);
    });
  }
  check(
    "no module outside the table carries a Hebrew string a reader will see",
    literals,
    [],
  );
}
