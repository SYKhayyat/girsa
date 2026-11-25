// What a chain panel asks the shell for.
//
// A chain is drawn from one walk and each hop names a place, but the words on
// the rows come from opening **sefоrim**, and that is the unit a request is
// priced in (#14): `link_words` takes one sefer and a list of places inside it,
// so a walk that visits eight places in five seforim is five reads.
//
// The grouping is three lines and it was wrong for exactly as long as it was
// invisible — an obvious `for` loop over hops would have been correct-looking
// and would have opened the same sefer eight times. So it is a function with a
// name and this file, rather than a shape inside a method that draws rows.

import { check } from "./harness.mjs";
import { byWork } from "../.tmp-test/chainview.mjs";

/** A hop, with only the fields the grouping reads. */
const hop = (work, at, depth = 0) => ({ work, at, depth });

export function run() {
  // --- one sefer, many places ------------------------------------------------
  const one = byWork([hop("bavli/berakhot", "1"), hop("bavli/berakhot", "2"), hop("bavli/berakhot", "3")]);
  check("three places in one sefer are one request", one.length, 1);
  check("and the request carries all three places", one[0][1], ["1", "2", "3"]);
  check("the sefer is the request's first half", one[0][0], "bavli/berakhot");

  // --- many seforim ----------------------------------------------------------
  const several = byWork([
    hop("bavli/berakhot", "1"),
    hop("rashi/rashi-on-berakhot", "7"),
    hop("bavli/berakhot", "9"),
    hop("mishneh-torah/madda", "12"),
  ]);
  check("four hops across three seforim are three requests", several.length, 3);
  check(
    "each sefer keeps its own places, in the order they were walked",
    several,
    [
      ["bavli/berakhot", ["1", "9"]],
      ["rashi/rashi-on-berakhot", ["7"]],
      ["mishneh-torah/madda", ["12"]],
    ],
  );

  // --- the empty cases -------------------------------------------------------
  //
  // A walk that found nothing asks nothing. `[]` and not a request per sefer
  // with no places, which would be a read of every sefer in the corpus.
  check("no hops is no requests", byWork([]), []);

  // One hop onto one place, which is the shape every chain starts as.
  check("a single hop is a single request", byWork([hop("bavli/berakhot", "5")]), [["bavli/berakhot", ["5"]]]);

  // The same place reached twice — the same sefer down two branches — is two
  // hops and one segment. Both are asked for: `link_words` looks up what it is
  // given, and deduplicating here would be this file deciding which of two rows
  // gets the words.
  const twice = byWork([hop("bavli/berakhot", "5"), hop("bavli/berakhot", "5", 2)]);
  check("a place reached twice is asked for twice, in one request", twice, [["bavli/berakhot", ["5", "5"]]]);
}
