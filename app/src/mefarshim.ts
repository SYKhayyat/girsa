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

import type { Companion } from "./api.ts";

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
  return companions.filter((c) => c.declared);
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
    if (a.declared !== b.declared) return a.declared ? -1 : 1;
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
  return n === 0 ? "לצד" : `מפרשים · ${n}`;
}

/** Both things the button does, since the label only has room for one. */
export function doorTitle(companions: Companion[]): string {
  const n = mefarshim(companions).length;
  const found = n === 0 ? "אין מפרשים מוצהרים על הספר הזה" : `${n} מפרשים על הספר הזה`;
  return `${found} · פתח ספר בטור שלצדו (Ctrl+\\)`;
}
