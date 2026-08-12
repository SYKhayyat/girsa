// Which seforim are printed on this page, and what to call the button that
// opens them.
//
// > *"i have no clue how to even open mefarshim."*
//
// Nothing here was missing. `Shelf::companions` already returns the works the
// corpus declares are commentaries on what you are reading — read straight out
// of `Work::commentary_on`, which is Sefaria's `base_text_titles` and not a
// guess from a title. `picker.ts` already renders them and marks each one
// `פירוש`. The pane header already has a button that opens that list.
//
// The button was labelled **לצד** — *alongside*. So a reader looking for Rashi
// was looking at a preposition. The feature was complete and unnamed, which as
// far as a person using it is concerned is the same as absent.
//
// The two decisions live here rather than inline in `main.ts` so that a test can
// hold them — the same reason `preview.ts` holds B1's geometry. They are: what
// the door should say given what is behind it, and which sefer to offer first.

import type { Comments, Companion } from "./api.ts";
import { say } from "./say.ts";

/**
 * The declared commentaries, and only those.
 *
 * `declared` and `links` are different claims and this codebase keeps them
 * apart everywhere (see `companionRow` in `picker.ts`). The Beit Yosef cites
 * Berakhot 815 times and is not a commentary on it; the Rambam's peirush
 * declares itself one. A count is evidence that two seforim are connected. It
 * is not evidence of *how*, and promoting a tally to a claim is the mistake the
 * whole link layer is arranged to avoid.
 */
export function mefarshim(companions: Companion[]): Companion[] {
  // `stands === "on"`, and not `stands !== null`. The three relations are three
  // claims: a mefaresh **on** this sefer, the sefer this one is a mefaresh on,
  // and a sefer running alongside it. Counting all three would put `מפרשים · 30`
  // over a list whose thirtieth row is the Chumash the commentary in front of
  // you was written about.
  return companions.filter((c) => c.stands === "on");
}

/**
 * The order to offer them in: mefarshim first, and among those the one best
 * attached to the text.
 *
 * Sorted rather than taken as it arrived, because `companions()` builds its list
 * in two passes and a reader opening the same daf twice must not be handed two
 * different orders. Ties break on the slug for the same reason — a stable order
 * a person can learn beats a marginally better one they cannot.
 */
export function ordered(companions: Companion[]): Companion[] {
  return [...companions].sort((a, b) => {
    const related = (c: Companion) => (c.stands === null ? 0 : 1);
    if (related(a) !== related(b)) return related(b) - related(a);
    if (a.links !== b.links) return b.links - a.links;
    return a.slug < b.slug ? -1 : a.slug > b.slug ? 1 : 0;
  });
}

/**
 * What the button says.
 *
 * `מפרשים · 30` when there are thirty, because the count is the part that says
 * *there is something here* — the reason the old label failed is that it
 * promised nothing, so nobody pressed it.
 *
 * And `לצד` when there are none. The button still opens any sefer beside this
 * one, so it must not vanish; but a header that says `מפרשים` over an empty list
 * teaches a reader that the labels lie, which costs more than the word is worth.
 */
export function doorLabel(companions: Companion[]): string {
  const n = mefarshim(companions).length;
  return n === 0 ? say("beside") : `${say("mefarshimOf")} · ${n}`;
}

/** Both things the button does, since the label only has room for one. */
export function doorTitle(companions: Companion[]): string {
  const n = mefarshim(companions).length;
  const found = n === 0 ? say("doorNone") : `${n} ${say("doorSome")}`;
  return `${found} · ${say("doorWhy")}`;
}

// ── The weave moved to Rust (W44) ───────────────────────────────────────────
//
// `Choice`, `Listed`, `choices`, `following` and `listed` were here: 277 lines
// deciding four sections, three Hebrew headings, an ordering rule and a
// no-sefer-twice rule — beside `crates/girsa-app/src/mefarshim.rs`, which
// carries twenty-five Rust tests about this same list and could not see any of
// it. The giveaway was the shape this file needed to be given: `Mefarshim`
// arrived as four parallel arrays that only `listed()` knew how to weave.
//
// It is `girsa_app::mefarshim::listed` now, and it arrives woven. `Choice` and
// `Listed` are declared in `api.ts` with the rest of the wire format, where
// `wire.test.mjs` holds them against the Rust.
//
// What stayed here is what a *window* decides: what the door should say given
// what is behind it, and how to word the sentence under the list. Those are
// label composition, they are tested here, and they are not information
// architecture.

/**
 * What a click on a line says when it opens nothing (W43).
 *
 * There are four ways to end up with no comment in front of you, they are four
 * different facts, and the reader's next move is different in each. Collapsing
 * them into *no comments* would tell somebody who has ticked nobody that nobody
 * wrote — which is exactly the class of quiet lie the rest of this codebase
 * spent the week pulling out of its error messages.
 *
 * Nobody-ticked comes first, ahead of *others wrote here*, because when the list
 * is empty the advice is the same either way: tick somebody. Reporting on the
 * unticked mefarshim of one line to a reader who has ticked none of them
 * anywhere is answering a question they have not asked yet.
 */
export function nothingHere(comments: Comments, chosen: number): string {
  if (comments.said.length > 0) return "";
  if (chosen === 0) return say("tickSomebody");
  if (comments.others) return say("othersWroteHere");
  return say("nobodyWroteHere");
}

/**
 * The sentence under the tick-list: how much of this sefer is commented on, and
 * how much of that you asked to see.
 *
 * Both numbers, because they answer the two questions a reader has when the daf
 * shows no markers — *is there nothing here* and *have I asked for nothing*.
 */
export function ticked(touched: number, chosen: number): string {
  if (touched === 0) return say("noMefarshimAtAll");
  const of = `${say("mefarshimOn")} ${touched} ${say("lines")}`;
  return chosen === 0 ? `${of} · ${say("tickedNobody")}` : `${of} · ${say("tickedN")} ${chosen}`;
}
