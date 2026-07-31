// Whether the reading has to make room, and who gets to decide.
//
// > *"there should be a way to open while keeping madaf open. same for search -
// > be able to go there while keeping search open."*
//
// The bug this file exists for was in the first version of that fix. Both panels
// set `is-docked` on the document root themselves, so: dock the shelf, then close
// the search, and the search's close undocked the shelf's column while the shelf
// was still standing in it — the panel back over the text, which is the whole
// thing being fixed.
//
// Two writers, one value. So the set of docked panels has one owner and the class
// is derived from it, and `docked()` is a function of a set, which is what makes
// it testable here rather than by opening two panels and looking.

import { check, ok, notOk } from "./harness.mjs";
import { docked } from "../.tmp-test/dock.mjs";

export function run() {

  notOk("with nothing docked the reading has the window", docked(new Set()));

  ok("one panel docked and the reading makes room", docked(new Set(["search"])));

  // The bug, as an assertion. Closing one of two docked panels must not give the
  // reading back a column that is still occupied.
  ok(
    "two docked and one closed still leaves the reading narrower",
    docked(new Set(["shelf"])),
  );

  ok("both docked is still one dock", docked(new Set(["search", "shelf"])));

  // Not a counter: `undock` of a panel that was never docked is the ordinary
  // case — closing a panel you never docked — and must not go negative and leave
  // the reading permanently narrow.
  const after = new Set(["search"]);
  after.delete("shelf");
  check("undocking a panel that was never docked changes nothing", docked(after), true);
  after.delete("search");
  check("and undocking the one that was gives the room back", docked(after), false);

}
