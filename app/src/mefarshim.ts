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

import type { Comments, Companion, Mefaresh } from "./api.ts";

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

/** One row of the list behind the door: a sefer you can open, tick, or both. */
export interface Choice {
  slug: string;
  he_title: string;
  en_title: string;
  declared: boolean;
  links: number;
  /**
   * Whether ticking this one could mark a line — that is, whether the link graph
   * has it commenting somewhere in this sefer.
   *
   * Not the same question as `declared`. `Tosafot on Berakhot` declares itself a
   * commentary; whether *this* corpus holds edges placing its comments on
   * particular lines is a separate fact, and after W32 it is a fact worth
   * distrusting. A tick-box that can never mark anything is worse than no box.
   */
  tickable: boolean;
  chosen: boolean;
}

/**
 * The one list behind `מפרשים · N`, doing both jobs (W43).
 *
 * > *"there should be a way to have mefarshim also open like otzaria"* …
 * > *"but keep the split too — its also nice"*
 *
 * So: every row still opens that sefer into the column beside you, which is the
 * split, untouched. Rows the graph can place also carry a tick-box, which marks
 * their comments on the daf and opens them where you click. One door, because a
 * second door for the second reading of the same list is how a toolbar grows to
 * eleven buttons.
 *
 * The declared companions keep `ordered`'s order — a reader learns where Rashi
 * sits in the list and it must not move. Mefarshim the graph knows and the
 * metadata does not (the Ben Yehoyada on Berakhot, most of Otzaria's shelf)
 * follow, by slug, rather than being dropped.
 */
export function choices(companions: Companion[], can: Mefaresh[]): Choice[] {
  const graph = new Map(can.map((m) => [m.slug, m]));
  const rows: Choice[] = ordered(companions).map((c) => ({
    slug: c.slug,
    he_title: c.he_title,
    en_title: c.en_title,
    declared: c.declared,
    links: c.links,
    tickable: graph.has(c.slug),
    chosen: graph.get(c.slug)?.chosen ?? false,
  }));
  const listed = new Set(rows.map((r) => r.slug));
  const rest = can
    .filter((m) => !listed.has(m.slug))
    .sort((a, b) => (a.slug < b.slug ? -1 : a.slug > b.slug ? 1 : 0))
    .map((m) => ({
      slug: m.slug,
      he_title: m.he_title,
      en_title: m.en_title,
      declared: false,
      links: 0,
      tickable: true,
      chosen: m.chosen,
    }));
  return [...rows, ...rest];
}

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
  if (chosen === 0) return "סמן מפרשים ברשימה כדי לראות מה כתבו על השורה";
  if (comments.others) return "כתבו כאן מפרשים שלא סימנת";
  return "אין מפרש שכתב על השורה הזאת";
}

/**
 * The sentence under the tick-list: how much of this sefer is commented on, and
 * how much of that you asked to see.
 *
 * Both numbers, because they answer the two questions a reader has when the daf
 * shows no markers — *is there nothing here* and *have I asked for nothing*.
 */
export function ticked(touched: number, chosen: number): string {
  if (touched === 0) return "אין מפרשים על הספר הזה בגרסה שלך";
  const of = `מפרשים על ${touched} שורות`;
  return chosen === 0 ? `${of} · לא סימנת אף אחד` : `${of} · סימנת ${chosen}`;
}
