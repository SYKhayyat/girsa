// The sibling's name.
//
// `כסב` is kaf-samekh-bet — a transliteration of the Latin "Ksav" back into
// Hebrew, and not a word. The application is `כְּתָב`, kaf-tav-bet, the Hebrew
// word for *writing*. The assertion that matters is the second one: the string
// that used to be on the toolbar of the first screen must not appear anywhere.

import { check, notOk, ok } from "./harness.mjs";
import { KSAV, GIRSA, withPrefix, titleIn, sameSeferTwice } from "../.tmp-test/names.mjs";

/** What was on the first screen, and must never be again. */
const WRONG = "כסב";

export async function run() {
  check("the sibling is כְּתָב", KSAV, "כְּתָב");
  notOk("and not the transliteration", KSAV.includes(WRONG));

  // The letters, not just the string: kaf-tav-bet, with the nikud stripped.
  const bare = [...KSAV].filter((c) => c >= "א" && c <= "ת").join("");
  check("its consonants are kaf-tav-bet", bare, "כתב");
  check("its consonants are not kaf-samekh-bet", bare === "כסב", false);

  check("this application is גִּרְסָא", GIRSA, "גִּרְסָא");
  const gbare = [...GIRSA].filter((c) => c >= "א" && c <= "ת").join("");
  check("whose consonants are gimel-resh-samekh-alef", gbare, "גרסא");

  // A prefixed name keeps a maqaf, so the preposition does not read as part of
  // the word.
  check("to it", withPrefix("ל", KSAV), "ל־כְּתָב");
  check("from it", withPrefix("מ", KSAV), "מ־כְּתָב");
  ok("and the name itself is unchanged by prefixing", withPrefix("ל", KSAV).endsWith(KSAV));

  // ------------------------------------------------- what a sefer is called (W41)
  //
  // > *"hebrew and english ui. all seforim names in hebrew ui should be heb all
  // > in english ui should be english."*
  //
  // Fifteen sites reached for `he_title` themselves, so there was no switch to
  // build — there were fifteen. The rule is one function of a work and a language,
  // which is why it can be tested here; the guard in `sources.test.mjs` is what
  // keeps a sixteenth from appearing.

  const BERAKHOT = { he_title: "ברכות", en_title: "Berakhot" };

  check("in Hebrew a sefer has its Hebrew name", titleIn(BERAKHOT, "hebrew"), BERAKHOT.he_title);
  check("in English it has its English one", titleIn(BERAKHOT, "english"), "Berakhot");

  // The corpus has works with one title and not the other. A row with no name on
  // it is worse than a row named in the wrong language.
  check(
    "a sefer with no English name keeps its Hebrew one in English",
    titleIn({ he_title: BERAKHOT.he_title, en_title: "" }, "english"),
    BERAKHOT.he_title,
  );
  check(
    "and the other way round",
    titleIn({ he_title: "   ", en_title: "Berakhot" }, "hebrew"),
    "Berakhot",
  );

  // Same rule as `Language::title_of` in Rust, deliberately: a sefer named one way
  // in a pane header and another in the tab above it is worse than either.
  check(
    "whitespace is not a name",
    titleIn({ he_title: "\n", en_title: "Berakhot" }, "hebrew"),
    "Berakhot",
  );

  // ------------------------------------------------ one sefer, drawn twice
  //
  // > *"Rabbeinu Chananel appears twice (`רבינו חננאל על בראשית`, `ר חננאל על
  // > בראשית`) — one from each corpus, undeduplicated, in the same list."*
  //
  // `sameSeferTwice` does not merge them and must not: two catalogue entries
  // are two seforim until something states otherwise, and deciding that two
  // titles name the same work is guessing at identity from a string. It
  // answers the smaller question — *would a reader read these as one sefer* —
  // so that both rows can say which corpus they came from.

  const row = (slug, title) => ({ slug, title });

  check(
    "the two spellings of Rabbeinu Chananel are found",
    [
      ...sameSeferTwice([
        row("sefaria/rabbeinu-chananel-on-genesis", "רבינו חננאל על בראשית"),
        row("otzaria/r-chananel-bereishis", "ר חננאל על בראשית"),
        row("sefaria/rashi-on-genesis", "רש\"י על בראשית"),
      ]),
    ].sort(),
    ["otzaria/r-chananel-bereishis", "sefaria/rabbeinu-chananel-on-genesis"].sort(),
  );

  check(
    "two seforim that are two seforim are left alone",
    [...sameSeferTwice([row("a", "רש\"י על בראשית"), row("b", "רמב\"ן על בראשית")])],
    [],
  );

  // Nikud and gershayim are how the two corpora differ from each other as often
  // as an honorific is, and neither is a different sefer.
  check(
    "pointing and gershayim are not a difference",
    [...sameSeferTwice([row("a", "רַשִׁ\"י עַל בְּרֵאשִׁית"), row("b", "רשי על בראשית")])].sort(),
    ["a", "b"],
  );

  // The comparison is on the title as **drawn** — the caller has already
  // decided which of a sefer's two names that is — so an English window finds
  // the same pair through the same function. Which is why the honorifics come
  // in both scripts.
  check(
    "an English window finds the same pair",
    [...sameSeferTwice([
      row("a", "Rabbeinu Chananel on Genesis"),
      row("b", "R Chananel on Genesis"),
    ])].sort(),
    ["a", "b"],
  );

  // A single row is never a duplicate of itself, and an empty list is the
  // ordinary case — most seforim have no twin anywhere.
  check("one row is not a pair", [...sameSeferTwice([row("a", "רש\"י")])], []);
  check("nothing is nothing", [...sameSeferTwice([])], []);
}
