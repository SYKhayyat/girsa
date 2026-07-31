// Whether the sibling application is there, said in three sentences.
//
// `Presence` has exactly three states and each one means something different to a
// reader, which is the point of it existing at all (spec.md §10.6, and the
// `girsa-post` module note argues it better than this one can). What the chip
// used to say was:
//
//     ksav.state === "stale" ? `כסב — ${ksav.why}` : "כסב אינו פועל"
//
// Two sentences for three states, one of them misspelling the application's name
// and interpolating `PostError`'s English `Display` string into a Hebrew toolbar.
//
// The distinction the reader needs is not "running or not". It is:
//
//   - **live** — send it something.
//   - **not_running** — start it, and there is nothing wrong.
//   - **stale** — it *said* it was there and it is not answering. Something went
//     wrong, on the sibling's side, and the reader can do something about it.
//
// A single "not running" for the last two throws away the only one of the three
// that is actionable.

import { KSAV } from "./names.ts";

export type Presence =
  | { state: "live"; version: string }
  | { state: "not_running" }
  | { state: "stale"; why: string };

export interface Said {
  /** The sentence for the chip. */
  said: string;
  /** The developer's string, for `title`. Empty when there is nothing to add. */
  detail: string;
  /** Whether this state is something gone wrong, as opposed to something off. */
  trouble: boolean;
  /** Whether the send affordance is offered. Never offered when it would fail. */
  canSend: boolean;
}

export function presenceSaid(p: Presence): Said {
  switch (p.state) {
    case "live":
      return {
        said: `${KSAV} ${p.version}`,
        detail: "",
        trouble: false,
        canSend: true,
      };
    case "not_running":
      // Not a fault. Nothing to hover, nothing to fix.
      return {
        said: `${KSAV} אינו פועל`,
        detail: "",
        trouble: false,
        canSend: false,
      };
    case "stale":
      // The one that carries information: the endpoint file outlived the
      // listener, which is what `Presence::Stale` was built to notice. The
      // sentence says what happened and what it means; `why` — the transport's
      // own English — goes behind the hover.
      return {
        said: `${KSAV} רשום אך אינו עונה — ייתכן שנסגר שלא כשורה`,
        detail: p.why,
        trouble: true,
        canSend: false,
      };
  }
}
