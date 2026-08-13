// Whether the reading has to make room, how much, and who gets to decide.
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
// is derived from it, and `docked()` is a function of that map, which is what
// makes it testable here rather than by opening two panels and looking.

import { check, ok, notOk } from "./harness.mjs";
import { A_COLUMN, docked, room, width } from "../.tmp-test/dock.mjs";

/** The docked panels, as the module holds them: name → how wide. */
function standing(...panels) {
  return new Map(panels.map((p) => (Array.isArray(p) ? p : [p, A_COLUMN])));
}

export function run() {
  notOk("with nothing docked the reading has the window", docked(standing()));

  ok("one panel docked and the reading makes room", docked(standing("search")));

  // The bug, as an assertion. Closing one of two docked panels must not give the
  // reading back a column that is still occupied.
  ok(
    "two docked and one closed still leaves the reading narrower",
    docked(standing("shelf")),
  );

  ok("both docked is still one dock", docked(standing("search", "shelf")));

  // Not a counter: `undock` of a panel that was never docked is the ordinary
  // case — closing a panel you never docked — and must not go negative and leave
  // the reading permanently narrow.
  const after = standing("search");
  after.delete("shelf");
  check("undocking a panel that was never docked changes nothing", docked(after), true);
  after.delete("search");
  check("and undocking the one that was gives the room back", docked(after), false);

  // ---------------------------------------------------------------- how much

  // The panels are not one width. The bookcase and the search are a column; the
  // links panel wants 620 and your own layer 680. The reading makes room for the
  // **widest one standing** — anything less and the wide one is back over the
  // text, which is the complaint this whole mechanism answers.
  check("nothing docked, nothing to make room for", room(standing(), new Set()), 0);
  check("one panel, its own width", room(standing(["links", 620]), new Set()), 620);
  check(
    "two panels, the widest of them",
    room(standing(["shelf", 380], ["yours", 680]), new Set()),
    680,
  );
  check(
    "and the order they were docked in does not decide it",
    room(standing(["yours", 680], ["shelf", 380]), new Set()),
    680,
  );

  // A panel shrunk to a strip is not standing in the column any more, so it must
  // not go on reserving one.
  check(
    "a minimised panel reserves nothing",
    room(standing(["shelf", 380]), new Set(["shelf"])),
    0,
  );
  check(
    "and the ones still open keep their room",
    room(standing(["shelf", 380], ["links", 620]), new Set(["shelf"])),
    620,
  );

  // ---------------------------------------------------------------- strip or column

  check("nothing docked is neither", width(standing(), new Set()), "none");
  check("one open panel is a column", width(standing("shelf"), new Set()), "full");
  check(
    "every docked panel minimised is a strip",
    width(standing("shelf"), new Set(["shelf"])),
    "small",
  );
  check(
    "one of two minimised is still a column, because the other is standing in it",
    width(standing("shelf", "search"), new Set(["shelf"])),
    "full",
  );
}
