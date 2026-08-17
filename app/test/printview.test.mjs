// The address on a sheet of paper.
//
// `printview.ts` has 6 Rust tests under it, for `girsa_app::printing` — which
// section a line is in, and where that section starts and stops. What is here is
// the one decision the window makes about the result, and it is the smallest
// possible decision, which is exactly why it had no test: the sheet's address
// line is one address or two, and the case that goes wrong is a section of one
// line, where the first address and the last one are the same string.

import { check } from "./harness.mjs";
import { sheetWhere } from "../.tmp-test/printview.mjs";

export function run() {
  check(
    "a run of se'ifim is printed from where it starts to where it stops",
    sheetWhere("סימן א׳ סעיף א׳", "סימן א׳ סעיף י״ד"),
    "סימן א׳ סעיף א׳ — סימן א׳ סעיף י״ד",
  );

  // The case. A siman of one se'if, a perek of one mishnah, a daf printed as a
  // single line: `printSection` reads the first line's address and the last
  // line's address off the same line, and the unguarded form puts the same
  // words on the page twice with a dash between them.
  check(
    "a section of one line is one address",
    sheetWhere("סימן א׳ סעיף א׳", "סימן א׳ סעיף א׳"),
    "סימן א׳ סעיף א׳",
  );

  // A sheet with no lines. `printSection` builds `address` and `to_address` out
  // of `lines.first()` and `lines.last()` with `unwrap_or_default()`, so both
  // are empty rather than absent — and the sheet must not carry a bare dash.
  check("an empty sheet says nothing rather than a dash", sheetWhere("", ""), "");

  // Two addresses where one end is missing is still two ends, and hiding that
  // would be the sheet claiming to be a line it is not.
  check("one end missing is still a range", sheetWhere("דף ב׳.", ""), "דף ב׳. — ");

  // The dash is an em dash with spaces around it, and it is the same dash the
  // reader sees everywhere else in this application. A hyphen here would be a
  // second convention on the one page that leaves the window.
  check("the dash is the one the rest of the window uses", sheetWhere("א", "ב").includes(" — "), true);
}
