// The chip row, and the one thing on it that must never be translated.
//
// `chips.ts` was extracted so a second search bar could exist without a second
// idea of what the options are, and it shipped with nothing under `app/test/`.
// What it holds is not markup — the markup is four `createElement` calls — it is
// **the table between the engine's keys and the reader's words**, and a table
// has exactly two ways of being wrong: a row that says the wrong thing, and a
// row that is not there.
//
// The second one is the one that gets you, because it fails soft. `chipSaid`
// ends `return choice.label`, and `label` is the engine's own English — so a
// mode added in Rust and forgotten here does not throw, does not blank, and does
// not look broken. It draws `Instruments ▾` in a fully Hebrew window, which is
// finding 7 all over again: *opening the search in a Hebrew window greeted them
// with `torat emet ▾ | whole shelf ▾`*. That fallback is right for the one chip
// whose labels are data, and it is a silent hole for the four whose labels are
// interface.
//
// So the last test in this file reads `crates/girsa-search` and asserts the
// table covers it. Same shape as `styles.test.mjs` reading the stylesheet and
// `check-card.sh` reading the Rust: the guard is only worth having if it reads
// the source of truth rather than a copy somebody kept in step by hand.

import { readFileSync } from "node:fs";
import path from "node:path";
import { check, ok } from "./harness.mjs";
import { chipName, chipSaid } from "../.tmp-test/chips.mjs";
import { ROOT } from "../tools/paths.mjs";

/** A choice off the wire, with only the fields these functions read. */
function choice(key, label, sigil = null) {
  return { key, label, sigil, chosen: false };
}

/**
 * Every variant of a `spelled!` table, read out of the Rust that declares it.
 *
 * `girsa_corpus::spelled!(Mode { ToratEmet => "ToratEmet", … })` is the macro
 * that decides what a key is on the wire, so it is what this asks. Reading the
 * enum's `pub enum` body instead would find the variants and not the spellings,
 * and the spellings are the protocol.
 */
function spelled(file, name) {
  const source = readFileSync(path.join(ROOT, file), "utf8");
  const table = new RegExp(`spelled!\\(${name}\\s*\\{([^}]*)\\}`, "u").exec(source);
  if (!table) throw new Error(`no spelled!(${name}) table in ${file}`);
  return [...table[1].matchAll(/=>\s*"([^"]+)"/gu)].map((m) => m[1]);
}

export function run() {
  // --- the chip whose labels are the corpus's own words ----------------------
  //
  // The scope chip's labels are **the names of shelves and seforim**, written by
  // whoever wrote the sefer, in whatever language they wrote it. Translating one
  // would be the window renaming a work.
  check("a shelf keeps the name the corpus gave it", chipSaid("where", choice("x", "ברכות")), "ברכות");
  check(
    "and an English one is not turned into Hebrew either",
    chipSaid("where", choice("x", "Mishneh Torah")),
    "Mishneh Torah",
  );
  // Empty is the one case that is not data: no scope set is *the whole shelf*,
  // which is the interface's sentence and not the corpus's.
  ok("no scope set says so in the reader's language", chipSaid("where", choice("x", "")) !== "");

  // --- the four chips whose labels are interface -----------------------------
  //
  // Every one of these arrives off the wire carrying an English `label`, and the
  // test is that the label is **not** what comes out.
  for (const [chip, key] of [
    ["mode", "ToratEmet"],
    ["match", "Word"],
    ["together", "Phrase"],
    ["instrument", "Gematria"],
  ]) {
    const said = chipSaid(chip, choice(key, "THE WIRE'S OWN ENGLISH"));
    ok(`${chip}/${key} is said in the reader's words, not the wire's`, said !== "THE WIRE'S OWN ENGLISH");
    ok(`${chip}/${key} is said as something`, said.length > 0);
  }

  // --- the number the reader set ---------------------------------------------
  //
  // `Near` carries its number in the key, because `Near5` and `Near12` are two
  // chip choices rather than one chip with a setting. So the label is a sentence
  // with a number in it and the number comes out of the key.
  const near = chipSaid("together", choice("Near12", "within 12 words"));
  ok("the distance the reader set is in what the chip says", near.includes("12"));
  ok("and it is not the wire's sentence", !near.includes("within"));
  check(
    "a different distance is a different sentence",
    chipSaid("together", choice("Near5", "x")) === chipSaid("together", choice("Near12", "x")),
    false,
  );

  // `Near` with no number and `Near` with something that is not one are both
  // refused in Rust — `chips.rs` asserts `Nearbanana` and bare `Near` are
  // errors. Nothing should be inventing a distance here either.
  for (const bad of ["Near", "Nearbanana"]) {
    check(
      `${bad} is not read as a distance`,
      chipSaid("together", choice(bad, "off the wire")),
      "off the wire",
    );
  }

  // --- a chip this window has not been taught --------------------------------
  //
  // Its own key is a worse label than a translation and a better one than
  // nothing, and it must never be silently blank — a chip with no face is a
  // control a reader cannot see.
  check("an unknown chip is named by its key", chipName("mizrach"), "mizrach");
  ok("every chip this window knows has a name", ["mode", "where", "match", "together", "instrument"].every((k) => chipName(k) !== k));

  // --- and the hole the fallback leaves --------------------------------------
  //
  // The whole point of this file. Every key `girsa-search` can put on the wire
  // has to have a word here, or it reaches a Hebrew window in English and
  // nothing anywhere says so.
  const tables = [
    ["mode", "crates/girsa-search/src/lib.rs", "Mode"],
    ["match", "crates/girsa-search/src/torat_emet.rs", "Match"],
    ["instrument", "crates/girsa-search/src/chips.rs", "Sounding"],
  ];
  for (const [chip, file, name] of tables) {
    const keys = spelled(file, name);
    ok(`${name} has variants to check`, keys.length > 0);
    for (const key of keys) {
      const label = `${name}::${key} — off the wire`;
      ok(
        `${chip}/${key} has a word in this window`,
        chipSaid(chip, choice(key, label)) !== label,
      );
    }
  }
  // `Together` is not a `spelled!` table — the spelling *is* the number for
  // `Near` — so its two fixed keys are named here and its third is the pair of
  // assertions above.
  for (const key of ["Anywhere", "Phrase"]) {
    const label = `Together::${key} — off the wire`;
    ok(`together/${key} has a word in this window`, chipSaid("together", choice(key, label)) !== label);
  }
}
