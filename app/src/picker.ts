// Choosing a sefer.
//
// Two jobs, one overlay: open a sefer in a new tab, or put one — or several —
// in the columns beside what you are reading. The second opens showing **what
// the corpus says belongs there** — the commentaries on this sefer, and the
// seforim the link graph joins it to — rather than 7,189 titles in alphabetical
// order, which is a list and not a choice.

import { api, type Card, type Mefarshim, type OpenSefer, type Related } from "./api.ts";
import { field, glyph } from "./controls.ts";
import { Latest } from "./latest.ts";
import { sameSeferTwice, sefer } from "./names.ts";
import { say } from "./say.ts";
import { ticked } from "./mefarshim.ts";
import type { Choice, Listed, Source } from "./api.ts";

type Chosen = (slugs: string[]) => void;

/**
 * What the mefarshim door is opened with.
 *
 * Both jobs at once, because it is one list: `chosen` opens seforim into the
 * columns beside this one (the splits), and `tick` marks a mefaresh's comments
 * on the daf. The reader asked for the second and asked to keep the first.
 */
export interface Beside {
  slug: string;
  title: string;
  /** Open these, in the order they were picked. */
  chosen: Chosen;
  /** Who the link graph can place on this sefer's lines, and who is ticked. */
  mefarshim: Mefarshim;
  /** Tick or untick one. The caller redraws the markers and hands the refreshed
   * list back through [`Picker.refreshMefarshim`]. */
  tick: (work: string, on: boolean) => void;
}

export class Picker {
  readonly element: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly list: HTMLElement;
  private readonly heading: HTMLElement;
  private readonly open: HTMLButtonElement;
  private rows: { slug: string; node: HTMLElement }[] = [];
  private cursor = 0;
  private chosen: Chosen = () => {};
  private beside: string | null = null;
  private mefarshim: Mefarshim = {
    works: [], alongside: [], folders: [], listed: [], marked: {}, touched: 0, unbuilt: null,
  };
  private tick: (work: string, on: boolean) => void = () => {};
  /**
   * The seforim picked to open, in the order they were picked.
   *
   * > *"there should be a way to open at one time multiple windows with
   * > multiple meforshim."*
   *
   * A click still opens one and closes the door, because that is what a click
   * means. This is the other gesture: pick several, then open them together.
   */
  private picked: string[] = [];
  /** The sentence under the list: how much of the sefer has commentary at all. */
  private readonly said: HTMLElement;
  /** One answer at a time — see `latest.ts`. The filter asks per keystroke and a
   * slow early one used to land on top of a fast later one. */
  private readonly draws = new Latest();

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "picker";
    this.element.hidden = true;

    const sheet = document.createElement("div");
    sheet.className = "picker-sheet";
    this.heading = document.createElement("p");
    this.heading.className = "picker-heading";
    // The second field the snapshot could not see.
    this.input = field(say("filterList"), {
      className: "picker-input",
      type: "search",
      dir: "auto",
    });
    this.list = document.createElement("ul");
    this.list.className = "picker-list";
    this.said = document.createElement("p");
    this.said.className = "picker-said";
    this.open = document.createElement("button");
    this.open.type = "button";
    this.open.className = "tool picker-open";
    this.open.hidden = true;
    this.open.addEventListener("click", () => this.takeAll());
    sheet.append(this.heading, this.input, this.list, this.said, this.open);
    this.element.append(sheet);

    this.input.addEventListener("input", () => void this.refresh());
    this.input.addEventListener("keydown", (event) => this.key(event));
    this.element.addEventListener("pointerdown", (event) => {
      if (event.target === this.element) this.close();
    });
  }

  /** Open a sefer in a new tab. */
  openTab(chosen: (slug: string) => void): void {
    this.beside = null;
    this.picked = [];
    this.chosen = (slugs) => {
      const first = slugs[0];
      if (first) chosen(first);
    };
    this.heading.textContent = say("openSefer");
    this.show();
  }

  /** Open the mefarshim on the sefer already open: to read beside it, or to tick. */
  openBeside(open: Beside): void {
    this.beside = open.slug;
    this.picked = [];
    this.chosen = open.chosen;
    this.mefarshim = open.mefarshim;
    this.tick = open.tick;
    this.heading.textContent = `${say("mefarshimOf")} · ${open.title}`;
    this.show();
  }

  /**
   * The tick-list changed under the door, so redraw it from the truth.
   *
   * Rust answers a tick with the **whole** list (`choose_mefaresh`), and this is
   * where it lands. The picker used to keep the copy it was opened with, so a
   * tick-box un-ticked itself the moment anything redrew the list.
   */
  refreshMefarshim(slug: string, now: Mefarshim): void {
    if (this.beside !== slug) return;
    this.mefarshim = now;
    if (this.isOpen) void this.refresh();
  }

  private show(): void {
    this.element.hidden = false;
    this.input.value = "";
    this.input.focus();
    void this.refresh();
  }

  close(): void {
    this.element.hidden = true;
  }

  get isOpen(): boolean {
    return !this.element.hidden;
  }

  private async refresh(): Promise<void> {
    const query = this.input.value.trim();
    if (query.length === 0 && this.beside) {
      // The list arrives woven. `choices`, `following` and `listed` did it here
      // — four sections, three Hebrew headings, an ordering rule and a
      // no-sefer-twice rule, in 277 lines of TypeScript beside a Rust module
      // with twenty-five tests about this same list. It is
      // `girsa_app::mefarshim::listed` now; this draws it.
      // Which rows are one sefer drawn twice, worked out over the **whole**
      // list before any of it is drawn: a duplicate is a fact about a pair, so
      // neither row can see it alone.
      const twice = sameSeferTwice(
        this.mefarshim.listed.flatMap((e) =>
          e.kind === "sefer" ? [{ slug: e.choice.slug, title: sefer(e.choice) }] : [],
        ),
      );
      this.fill(
        this.mefarshim.listed.map((entry) => listedRow(entry, twice)),
        say("nothingBeside"),
      );
      // Both groups tick, so both groups count. Reporting only the mefarshim
      // would tell a reader who has ticked the Arukh HaShulchan that they have
      // ticked nobody.
      this.said.textContent = ticked(
        this.mefarshim.touched,
        this.mefarshim.listed.filter((row) => row.kind === "sefer" && row.choice.chosen).length,
      );
      return;
    }
    this.said.textContent = "";
    if (query.length === 0) {
      // **What is open, first.** The open set is not the tab strip — a tab
      // holding a Gemara, its Rashi and its Tosafos is one entry in the strip
      // and three seforim that are open — so once a tab is an arrangement the
      // strip stops being an inventory of what you have, and this is the surface
      // that tells that truth. Borrowed from the sibling application, where the
      // same absence produced seven complaints
      // (`Ksav/decisions/2026-08-11-marking-up-the-ui-inventory.md`).
      //
      // Most recently read first, because the thing a keyboard route is for is
      // *the sefer I was just in*.
      await this.draws.run(
        async () => ({ open: await api.openSet(), recent: await api.recent() }),
        ({ open, recent }) => {
          const rows: Row[] = [];
          if (open.length > 0) {
            rows.push(headingRow(say("whatIsOpen"), open.length));
            rows.push(...open.map(openRow));
          }
          const already = new Set(open.map((o) => o.slug));
          const rest = recent.filter((c) => !already.has(c.slug));
          if (rest.length > 0) {
            rows.push(headingRow(say("recentlyRead"), rest.length));
            rows.push(...rest.map(cardRow));
          }
          this.fill(rows, say("startTyping"));
        },
      );
      return;
    }
    // Behind `Latest`: one `api.search` goes out per keystroke and they do not
    // come back in order, so a slow answer to `ברכ` could land after — and on
    // top of — the answer to `ברכות`.
    await this.draws.run(
      () => api.search(query),
      (cards) => this.fill(cards.map(cardRow), say("noSuchSefer")),
    );
  }

  private fill(rows: Row[], empty: string): void {
    this.list.replaceChildren();
    this.rows = [];
    if (rows.length === 0) {
      const none = document.createElement("li");
      none.className = "picker-empty";
      none.textContent = empty;
      this.list.append(none);
      this.drawOpenAll();
      return;
    }
    for (const row of rows) {
      // A folder heading (W44). Not a row you can choose: it is the shelf the
      // seforim under it stand on, and the arrow keys skip it for that reason.
      if (row.heading) {
        const head = document.createElement("li");
        head.className = "picker-folder";
        head.style.setProperty("--depth", String(row.heading.depth));
        head.textContent = row.heading.count > 0
          ? `${row.title} · ${row.heading.count}`
          : row.title;
        this.list.append(head);
        continue;
      }
      const node = document.createElement("li");
      node.className = "picker-row";
      if (row.depth) node.style.setProperty("--depth", String(row.depth));
      const title = document.createElement("span");
      title.className = "picker-row-title";
      title.textContent = row.title;
      const aside = document.createElement("span");
      aside.className = "picker-row-aside";
      aside.textContent = row.aside;
      if (row.why) aside.title = row.why;
      // The tick-box, on the rows that can carry one (W43). Its own control and
      // not part of the row's click, because the two do different things: ticking
      // marks this mefaresh on the daf and leaves the list open, clicking the row
      // opens the sefer in the column beside you and closes it.
      if (row.tick) {
        const tick = row.tick;
        // Through `field`, like every other control here: a checkbox with no name
        // is one of thirty unlabelled boxes to a screen reader, and B14's guard
        // in `sources.test.mjs` fails the build over exactly this.
        const box = field(`${say("tickName")} ${row.title}`, {
          type: "checkbox",
          className: "picker-tick",
        });
        box.checked = tick.on;
        box.title = say("tickWhy");
        box.addEventListener("click", (event) => {
          // Or the row's own handler would open it beside as well.
          event.stopPropagation();
          this.tick(row.slug, box.checked);
        });
        box.addEventListener("pointerdown", (event) => event.stopPropagation());
        node.append(box);
      }
      // Pick for opening — the multi-select half. A separate control from the
      // tick, because they are different questions: *mark what this one says on
      // my daf* and *put it in a column beside me* are the two things the door
      // does, and a reader wants either without the other.
      //
      // **And it does not look like the tick.** It was a second checkbox, so
      // every row carried two identical unlabelled boxes side by side and the
      // only thing telling them apart was a tooltip. A reader would have to
      // discover which column is which by trying one — which is the shape of
      // *"i have no clue what okev does"*, built fresh. A box you tick and a
      // plus you press are two different gestures for two different jobs.
      if (this.beside) {
        const on = this.picked.includes(row.slug);
        const pick = glyph(on ? "✓" : "＋", `${say("openChosen")} — ${row.title}`, () => {});
        pick.className = "picker-pick" + (on ? " is-on" : "");
        pick.title = say("openChosenWhy");
        pick.setAttribute("aria-pressed", String(on));
        pick.addEventListener("click", (event) => {
          event.stopPropagation();
          const now = !this.picked.includes(row.slug);
          this.picked = now
            ? [...this.picked.filter((s) => s !== row.slug), row.slug]
            : this.picked.filter((s) => s !== row.slug);
          pick.textContent = now ? "✓" : "＋";
          pick.classList.toggle("is-on", now);
          pick.setAttribute("aria-pressed", String(now));
          this.drawOpenAll();
        });
        pick.addEventListener("pointerdown", (event) => event.stopPropagation());
        node.append(pick);
      }
      node.append(title, aside);
      node.addEventListener("pointerdown", () => this.take(row.slug));
      this.list.append(node);
      this.rows.push({ slug: row.slug, node });
    }
    this.cursor = 0;
    this.mark();
    this.drawOpenAll();
  }

  /** The *open these* button, which exists only once something is picked. */
  private drawOpenAll(): void {
    this.open.hidden = this.picked.length === 0;
    this.open.textContent = `${say("openChosen")} · ${this.picked.length}`;
    this.open.title = say("openChosenWhy");
  }

  private mark(): void {
    this.rows.forEach((row, i) => row.node.classList.toggle("is-cursor", i === this.cursor));
    this.rows[this.cursor]?.node.scrollIntoView({ block: "nearest" });
  }

  private key(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      this.close();
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      this.cursor = Math.min(this.rows.length - 1, Math.max(0, this.cursor + step));
      this.mark();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      // Enter opens everything picked, where anything is; otherwise the row the
      // cursor is on. One key, and it does what is in front of the reader.
      if (this.picked.length > 0) {
        this.takeAll();
        return;
      }
      const row = this.rows[this.cursor];
      if (row) this.take(row.slug);
    }
  }

  private take(slug: string): void {
    this.close();
    this.chosen([slug]);
  }

  private takeAll(): void {
    const picked = this.picked;
    this.picked = [];
    this.close();
    if (picked.length > 0) this.chosen(picked);
  }
}

interface Row {
  slug: string;
  title: string;
  aside: string;
  why?: string;
  /** Present on a row that can be ticked, with whether it is (W43). */
  tick?: { on: boolean };
  /** Present on a folder heading rather than a sefer (W44). */
  heading?: { depth: number; count: number };
  /** How far in a sefer under a folder is drawn. */
  depth?: number;
}

/** One entry of the grouped list: a heading, or a sefer under one. */
function listedRow(entry: Listed, twice: Set<string>): Row {
  if (entry.kind === "folder") {
    return {
      slug: "",
      title: entry.title,
      aside: "",
      heading: { depth: entry.depth, count: entry.count },
    };
  }
  return companionRow(entry.choice, twice.has(entry.choice.slug));
}

/** A heading over a group of rows — not a row you can choose. */
function headingRow(title: string, count: number): Row {
  return { slug: "", title, aside: "", heading: { depth: 0, count } };
}

/** One sefer that is already open. */
function openRow(sefer: OpenSefer): Row {
  return {
    slug: sefer.slug,
    title: sefer.title,
    aside: sefer.here ? say("readingNow") : say("isOpen"),
  };
}

function cardRow(card: Card): Row {
  return {
    slug: card.slug,
    title: sefer(card),
    aside: [card.author, card.era].filter(Boolean).join(" · "),
  };
}

/**
 * What a row says this sefer **is** to the one you are reading.
 *
 * # It used to say `פירוש` to three different things
 *
 * The row read one bool, `declared`, which Rust set from
 * `stands.is_some()` — true for a commentary on this sefer, true for a sefer
 * running alongside it, and true for the sefer **this one is a commentary on**,
 * because `companions` offers that too and offering it is right. So opening
 * Onkelos listed Bereshis, under the word `פירוש`:
 *
 * > *"bereishis is counted as a peirush on onkelos."*
 *
 * The relation has a direction now (`girsa_app::shelf::Related`) and the words
 * for each of the three are Rust's, beside the enum — `said` and `why` arrive on
 * the row. A count of links is not a relationship and still says so.
 */
function companionRow(companion: Choice, twice = false): Row {
  // Where two rows would read as the same sefer, both say which corpus they
  // came from. Not a merge — see `sameSeferTwice` — a label, so that a
  // duplicate reads as two copies rather than as a bug.
  const from = twice ? ` · ${sourceSaid(companion.source)}` : "";
  return {
    slug: companion.slug,
    title: sefer(companion),
    aside: `${relatedSaid(companion)}${from}`,
    why: relatedWhy(companion),
    tick: companion.tickable ? { on: companion.chosen } : undefined,
  };
}

/** Which corpus a sefer's text came from. */
function sourceSaid(source: Source): string {
  if (source === "sefaria") return say("fromSefaria");
  if (source === "otzaria") return say("fromOtzaria");
  return say("fromMine");
}

/**
 * What the row says this sefer is, out of the **name** Rust sent.
 *
 * `Related::said()` used to compose this in Rust and send the words, which is
 * why an English window drew `פירוש`. The name crosses now — `on`, `base`,
 * `alongside`, or nothing at all — and the words are `say.ts`'s, like every
 * other word in this window.
 *
 * Nothing at all is the interesting case: the graph places this sefer's
 * comments on lines of what you are reading and the catalogue declares no
 * relationship. That used to read `מפרש` beside a declared commentary's
 * `פירוש` — two words a reader takes for synonyms, carrying the one
 * distinction in the list they cannot see. It says what the claim rests on.
 */
function relatedSaid(companion: Choice): string {
  if (companion.stands === "on") return say("relatedOn");
  if (companion.stands === "base") return say("relatedBase");
  if (companion.stands === "alongside") return say("relatedAlongside");
  return companion.links > 0 ? say("onlyLinked") : say("onlyLinked");
}

function relatedWhy(companion: Choice): string {
  if (companion.stands === "on") return say("relatedOnWhy");
  if (companion.stands === "base") return say("relatedBaseWhy");
  if (companion.stands === "alongside") return say("relatedAlongsideWhy");
  return companion.links > 0
    ? `${companion.links} ${say("linksCounted")} · ${say("onlyLinkedWhy")}`
    : say("onlyLinkedWhy");
}

/** Re-exported so `main.ts` can name the type without importing the module
 * twice. */
export type { Related };
