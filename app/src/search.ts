// The search bar: the query, the chips under it, where it looks, the results,
// and the facets beside them (spec.md §9.5, §9.6, §9.8 — BUILDER.md W14).
//
// Nothing here decides anything. Which chips exist, what they can be set to,
// what a facet row means and what clicking one narrows by are all answered in
// `girsa-search`, where they are tested; this file draws them and sends the
// clicks back. That division is the same one the rest of the window keeps, and
// it matters most here: a webview that worked out its own chip row would be a
// second opinion about what the engine can do, and the first thing a reader
// would notice is a chip that lies.
//
// Three things it is careful about, all of them §9's governing rule — *the
// engine never changes your query without you knowing*:
//
//   · the header says what was searched for, and it comes from the search that
//     ran rather than from the box;
//   · a zero shows the ladder with counts and applies **nothing** until it is
//     clicked;
//   · a refusal is shown as a refusal, with its reason, and never as an empty
//     list of results.
//
// # Two bugs this file is the site of, and they are both about time
//
// **Whichever answer came back last won.** Every draw here is `await` then
// `replaceChildren`, which is correct exactly once. Opening the panel re-ran the
// *previous* query — the box keeps its text between openings — so a reader who
// opened it and immediately typed something new had two searches in flight, and
// the old, broader, slower one landed second:
//
// > *"when i search, it gets the right search for a second, then that drops out
// > and it has a list of totally different things."*
//
// Every ask goes through `Latest` now, and a stale answer is dropped rather than
// drawn.
//
// **And the panel re-ran that query silently.** Opening a search panel is not
// asking a question. It shows the last answer, says that is what it is showing,
// and waits — *"then it goes to the last searched item without telling you."*

import {
  api,
  type Chip,
  type Dimension,
  type FacetRow,
  type Found,
  type Run,
  type ScopeView,
} from "./api.ts";
import { LaneColumn } from "./laneview.ts";
import { announces, button, field, glyph, region, shut } from "./controls.ts";
import { dock, isDocked, minimise, undock } from "./dock.ts";
import { Latest } from "./latest.ts";
import { chipRow, chipSaid } from "./chips.ts";
import { say, type Word } from "./say.ts";
import { ScopePanel } from "./scopeview.ts";
import { sayTrouble, trouble } from "./trouble.ts";

/** Open a sefer at a segment — or, with no id, at wherever it was left.
 *
 * The second is W26's *read them*: a scan with no words has no segment worth
 * landing on, and the pane it opens is the one carrying the control that reads
 * it. */
type Opened = (slug: string, id: string | null, marked?: string[], where?: Where) => void;

/**
 * Which of the three landings a click on a result asked for.
 *
 * > *"ctrl-enter is supposed to open in the same tab, i think. It does not."*
 *
 * It did — in the **picker**, where `Ctrl+Enter` has meant *beside what I am
 * reading* since the door was written, and where the label says so. The results
 * list had one gesture and no modifiers at all, so the key a reader had learned
 * one screen over did nothing here.
 *
 * > *"There is no way to open one sefer in two tabs via search, at least."*
 *
 * And that is the third. Opening a sefer that is already open **goes to it**,
 * which is `Workspace::open`'s ruling and the right default — but it left no
 * gesture at all for *a second view of this*, which is the ordinary thing to
 * want of a search result in a sefer you already have open somewhere else.
 */
type Where = "tab" | "here" | "fresh";

/** What a click on a result asked for, from the keys held down with it. */
export function landingOf(held: { ctrl: boolean; shift: boolean }): Where {
  if (held.ctrl) return "here";
  if (held.shift) return "fresh";
  return "tab";
}

/** The facets, in the order spec.md §9.8 lists them. */
const FACETS: { dimension: Dimension; label: () => string }[] = [
  { dimension: "shelf", label: () => say("facetShelf") },
  { dimension: "era", label: () => say("facetEra") },
  { dimension: "author", label: () => say("facetAuthor") },
  { dimension: "sefer", label: () => say("facetSefer") },
  { dimension: "link", label: () => say("facetLink") },
  // Your own tags, last, because they are yours and everything above is the
  // library's. A search over the corpus alone has none and the row is simply
  // absent, the way an author facet is absent from a sefer nobody attributed.
  { dimension: "tag", label: () => say("facetTag") },
];

/** How many rows of one facet before the rest are counted rather than listed. */
const ROWS = 8;

export class SearchView {
  readonly element: HTMLElement;
  private readonly box: HTMLInputElement;
  private chipRow: HTMLElement;
  private readonly head: HTMLElement;
  private readonly list: HTMLElement;
  private readonly rail: HTMLElement;
  /**
   * Where the search looks (§9.8), as a panel rather than as a rail computed
   * from an answer.
   *
   * > *"i dont know how to add some and minus some things from the search (some
   * > seforim or folders). often the tree to pick from … is not even visible -
   * > it flashes, then flashes off."*
   *
   * The tree was the facet rail, which is derived from a result set: it did not
   * exist until a search returned hits and was cleared at the start of the next
   * one. So the one control for saying *where to look* could only be used after
   * you had already looked, and disappeared while you looked again. The scope
   * itself is a thing that exists before any search and outlives every one.
   */
  private readonly scope = new ScopePanel();
  /** The semantic lane (spec.md §9.9, W30) — **beside** the literal results and
   * never among them. It is asked after they are drawn, so a literal search
   * never waits on a model, and it draws nothing at all when the lane is off. */
  private readonly lane = new LaneColumn();
  private open = false;
  private page = 1;
  /** The rung the reader clicked, if any. Cleared by anything that makes it a
   * different search — which is the undo of spec.md §9.6. */
  private rung: string | null = null;
  private opened: Opened = () => {};
  /** The query the results on screen actually answer, or null when they answer
   * nothing yet. Read by nothing but the *these are the previous results* line,
   * which is the whole point: the window has to be able to say what it is
   * showing. */
  private answering: string | null = null;
  /** One answer at a time. See `latest.ts`. */
  private readonly draws = new Latest();

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "find";
    this.element.hidden = true;

    const sheet = document.createElement("div");
    sheet.className = "find-sheet";
    // What the strip says when the panel is minimised — `content: attr(…)` in
    // the stylesheet, so the name is here, in the window's language, rather than
    // in the CSS where it could only ever be in one.
    sheet.dataset.name = say("search");
    sheet.addEventListener("click", (event) => {
      // **The strip itself**, not anything inside it. Every control is a child
      // of the sheet, so a bare `click` handler here fires for them too — and
      // the minimise button is one of them: it added `is-small`, the click
      // bubbled to this handler, and this handler took it straight off again.
      // Minimising did nothing at all, visibly, which is the exact family of bug
      // this whole pass is about. Minimised, the children are `display: none`,
      // so the sheet is the only thing left to hit.
      if (event.target === sheet && this.element.classList.contains("is-small")) {
        this.dock();
      }
    });

    const bar = document.createElement("div");
    bar.className = "find-bar";
    // The one control the whole application is about, and it had no name at all:
    // an accessibility snapshot listed 29 controls and this was not one of them.
    this.box = field(say("searchBox"), {
      className: "find-box",
      dir: "rtl",
      placeholder: say("searchPlaceholder"),
    });
    this.box.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        // A new query is a new search: a rung applied to the last one does not
        // carry over silently.
        this.page = 1;
        this.rung = null;
        void this.run();
      }
    });
    // Keep the question, not the answer (spec.md §11, W27): the corpus grows
    // and your own seforim go on the shelf, so what is worth keeping is the
    // asking. The chips and the scope are read off the engine, not off this
    // box — see `query_keep`.
    const keep = button(say("keepQuery"), say("keepQueryWhy"), () => {
      void this.keep?.(this.box.value.trim());
    });
    keep.classList.add("find-keep");

    const close = shut(() => this.close());
    close.classList.add("find-close");
    // Minimise rather than close, for a reader working through a long list of
    // results who wants the daf wide for a minute — see `ShelfView.minimise`,
    // which is the same control on the other panel and the same complaint.
    const shrink = button(say("minimize"), say("minimizeWhy"), () => this.minimise());
    shrink.classList.add("find-minimise");
    bar.append(this.box, keep, shrink, close);

    this.chipRow = document.createElement("div");
    this.chipRow.className = "find-chips";
    this.head = document.createElement("div");
    this.head.className = "find-head";
    // What the search said, as a live region: a count that changes without being
    // announced is a count a screen reader never mentions.
    announces(this.head, say("whatWasFound"));

    const body = document.createElement("div");
    body.className = "find-body";
    // Landmarks, so a reader can jump between the results and the facets instead
    // of walking every row of one to reach the other.
    this.list = region("region", say("results"), "find-list");
    this.rail = region("region", say("narrowResults"), "find-rail");
    body.append(this.list, this.rail);

    // The scope asks to be re-run when it changes, because narrowing is a
    // different answer to the same question and the reader is looking at the
    // old one.
    this.scope.onChanged(() => {
      this.page = 1;
      void this.run();
    });

    sheet.append(bar, this.chipRow, this.scope.element, this.head, body, this.lane.element);
    this.element.append(sheet);
    this.element.addEventListener("pointerdown", (event) => {
      if (event.target === this.element) this.close();
    });
  }

  get isOpen(): boolean {
    return this.open;
  }

  /** Standing beside the reading rather than over it — so the reader is
   * reading, and the keyboard is theirs (finding 3). From `dock.ts`, which owns
   * the set, not from this panel's own class. */
  get isDocked(): boolean {
    return isDocked("search");
  }

  private keep: ((typed: string) => Promise<void>) | null = null;

  /** The window says what *keep this question* does — asking a reader for a
   * name is the window's business, not this view's. */
  onKeep(keep: (typed: string) => Promise<void>): void {
    this.keep = keep;
  }

  /** Ask something again, with the chips already set back by `query_recall`. */
  async askAgain(opened: Opened, typed: string): Promise<void> {
    this.opened = opened;
    this.open = true;
    this.element.hidden = false;
    this.element.classList.remove("is-small");
    this.box.value = typed;
    this.page = 1;
    this.rung = null;
    this.box.focus();
    await this.run();
  }

  /**
   * Open the panel.
   *
   * **It does not search.** The chips and the scope are drawn, because they are
   * what the search *will* do and a control you can only see after you have used
   * it is not a control; the results already on screen stay where they are, with
   * a line saying they are the previous search's. Re-running silently is how a
   * reader ends up looking at an answer to a question they did not just ask.
   */
  async show(opened: Opened): Promise<void> {
    this.opened = opened;
    this.open = true;
    this.element.hidden = false;
    this.element.classList.remove("is-small");
    this.box.focus();
    this.box.select();
    await this.drawControls();
  }

  close(): void {
    this.open = false;
    this.element.hidden = true;
    this.element.classList.remove("is-docked", "is-small");
    undock("search");
  }

  /** The strip has to say what it is, and it says it in the reader's language,
   * so it is written when the panel is drawn rather than once at construction. */
  private nameTheStrip(): void {
    const sheet = this.element.querySelector<HTMLElement>(".find-sheet");
    if (sheet) sheet.dataset.name = say("search");
  }

  /**
   * Go to a result and **keep the results** (W48).
   *
   * > *"same for search - be able to go there while keeping search open."*
   *
   * A reader working through results reads one, comes back, reads the next. When
   * the jump closed the panel, the second result cost the whole search again —
   * the query, the chips, the page, and the place in the list.
   *
   * So it docks instead of closing: the scrim goes, the sheet becomes a column
   * on the **reading's leading edge**, and the reading pane is made narrower to
   * fit rather than being covered.
   */
  private dock(): void {
    this.element.classList.add("is-docked");
    this.element.classList.remove("is-small");
    dock("search");
    minimise("search", false);
  }

  /** Shrink to a strip, keeping everything. Clicking the strip opens it again. */
  private minimise(): void {
    // A closed panel has nothing to minimise, and docking one would take a strip
    // of the reading away for a panel nobody can see.
    if (!this.isOpen) return;
    this.element.classList.add("is-docked", "is-small");
    this.nameTheStrip();
    dock("search");
    minimise("search", true);
  }

  async toggle(opened: Opened): Promise<void> {
    if (this.open) this.close();
    else await this.show(opened);
  }

  /**
   * Open with a phrase already in the box, as one thing (W18).
   *
   * The quotation marks are the sigil for *one after the other* (§9.5), so
   * this arrives as a phrase search and the chip says so — the reader sees
   * what was asked, not a query that behaves differently from how it reads.
   */
  async showPhrase(opened: Opened, phrase: string): Promise<void> {
    this.opened = opened;
    this.open = true;
    this.element.hidden = false;
    this.element.classList.remove("is-small");
    this.box.value = `"${phrase.trim()}"`;
    this.box.focus();
    await this.run();
  }

  /**
   * Draw everything that is not a result: the chips, the scope, and whatever the
   * header should say about what is on screen.
   *
   * Called on open, and by nothing else — a search draws these from its own
   * answer, which is the row the search actually ran under.
   */
  private async drawControls(): Promise<void> {
    // `find("")` asks the engine for the chip row without asking it to search.
    await this.draws.attempt(
      () => api.find("", 1),
      (empty) => {
        this.drawChips(empty.chips);
        this.head.classList.toggle("is-trouble", Boolean(empty.refused));
        if (empty.refused) {
          // The browser build **does** refuse, and the refusal used to be
          // thrown away here — so the panel opened as a full-height empty box
          // and said nothing until something was typed.
          //
          // Through `trouble()`, because a refusal carries a name and not a
          // sentence. The shell no longer refuses an *empty* query at all —
          // that was `nothing to search for`, in red, above a row of English
          // chips, before the reader had typed anything, which is a panel that
          // opens by telling you off.
          sayTrouble(this.head, empty.refused, "general");
        } else {
          this.head.replaceChildren();
          if (this.answering === null) {
            // Nothing asked yet, and nothing wrong. One line saying what the
            // box is for — including the thing nothing on screen used to teach:
            // that a mareh makom typed into it goes there.
            const hint = document.createElement("p");
            hint.className = "find-said is-hint";
            hint.textContent = say("searchNothingAsked");
            this.head.append(hint);
          }
          if (this.answering !== null) {
            // What is on screen, and whose question it answers. This is the
            // whole of *"it goes to the last searched item without telling
            // you"*: it still shows you, and now it tells you.
            const said = document.createElement("p");
            said.className = "find-said is-previous";
            said.textContent = `${say("previously")} — ${this.answering}`;
            this.head.append(said);
          }
          this.head.append(this.gapLine());
        }
      },
      () => undefined,
    );
    await this.scope.refresh();
  }

  /** Ask, and draw what came back. */
  private async run(): Promise<void> {
    const typed = this.box.value.trim();
    if (typed === "") {
      await this.drawControls();
      return;
    }
    const rung = this.rung;
    const page = this.page;
    const drew = await this.draws.run(
      () => (rung ? api.findRung(typed, page, rung) : api.find(typed, page)),
      (found) => {
        this.answering = typed;
        // A sigil sets a chip; the box shows what is left, so what is on screen
        // is what was searched for.
        this.drawChips(found.chips);
        this.drawHead(found);
        this.drawHits(found);
        this.drawRail(found);
      },
    );
    if (!drew) return;
    await this.scope.refresh();
    // The lane, **after** — a separate call, a separate list, and a separate
    // claim (spec.md §9.9). It is not awaited: the literal answer is already on
    // screen and running a model over the query must not hold it there. And it
    // is asked whatever the literal search found, because *these words are
    // nowhere and something like them is here* is the interesting case.
    void this.lane.show(typed, this.opened);
  }

  private drawChips(chips: Chip[]): void {
    // Drawn by `chips.ts`, which the find bar draws with too. One chip row:
    // the reader asked for the find inside a sefer to be *the same as regular
    // girsa search (with all the options)*, and two renderers of one row is how
    // the two would come to offer different ones.
    const row = chipRow(chips, {
      chosen: async (chip, key) => {
        await api.findChip(chip, key);
        this.page = 1;
        this.rung = null;
        await this.run();
      },
      scope: () => this.scope.toggle(),
    });
    this.chipRow.replaceWith(row);
    this.chipRow = row;
  }

  private drawHead(found: Found): void {
    this.head.replaceChildren();
    this.head.classList.toggle("is-trouble", Boolean(found.refused));
    if (found.refused) {
      // Through `trouble()`, not raw. A refusal this codebase makes carries a
      // **name** in front of the colon, and printing the whole thing put
      // `no-index: there is no shelf to search` on the first line of a Hebrew
      // panel — the code and the developer's sentence both.
      sayTrouble(this.head, found.refused, "general");
      return;
    }
    const said = document.createElement("p");
    said.className = "find-said";
    // Composed here, from the chip row that actually ran and from what the
    // reader actually typed. It used to be composed in Rust, in English, and it
    // echoed the query back with its final letters folded — `מאימתי קורינ את
    // שמע` — which reads as a typo the reader did not make. The box has what
    // they wrote; the chips have what it meant.
    said.textContent = this.whatWasAsked(found);
    const count = document.createElement("p");
    count.className = "find-count";
    count.textContent =
      found.pages > 1
        ? `${found.total} · ${say("page")} ${found.page} ${say("pageOf")} ${found.pages}`
        : `${found.total}`;
    this.head.append(said, count);
    // A zero used to be a bare `0` over an entirely blank panel — no sentence,
    // no suggestion, nothing to do next. The ladder below offers the widenings
    // that would have hits; this says what happened, for the case where it
    // offers none.
    if (found.total === 0) {
      const nothing = document.createElement("p");
      nothing.className = "find-said is-nothing";
      nothing.textContent = say("foundNothing");
      const how = document.createElement("p");
      how.className = "find-note";
      how.textContent = say("foundNothingWhy");
      this.head.append(nothing, how);
    }
    // What this search could not see (spec.md §9.7, W26). Drawn on every
    // result page, above the note and the offers, because it is a statement
    // about the answer and not a suggestion about the query.
    this.head.append(this.gapLine());
    if (found.note) {
      const note = document.createElement("p");
      note.className = "find-note";
      // The note is coded like every other sentence the shell sends: it used to
      // be a Hebrew string written out in `lib.rs`, which an English window
      // would have shown in Hebrew.
      note.textContent = trouble(found.note).said;
      this.head.append(note);
    }
    // One click back to the literal query (spec.md §9.6 — reversibly).
    if (this.rung) {
      const undo = document.createElement("button");
      undo.type = "button";
      undo.className = "find-offer";
      undo.textContent = say("undo");
      undo.addEventListener("click", async () => {
        this.rung = null;
        this.page = 1;
        await this.run();
      });
      this.head.append(undo);
    }
    // The ladder: counts worked out before the click, and nothing applied.
    for (const offer of found.offers) {
      const chip = document.createElement("button");
      chip.type = "button";
      chip.className = "find-offer";
      // The offers were the one thing on a zero-hit panel that was not blank,
      // and they were in English. `offer.rung` is the name the ladder travels
      // under; the words are in `say.ts`.
      chip.textContent = `${rungSaid(offer)} — ${offer.count}`;
      chip.addEventListener("click", async () => {
        // The click. Until here nothing has been applied — the count beside the
        // offer was worked out from this very query, before it was asked for.
        this.rung = offer.rung;
        this.page = 1;
        await this.run();
      });
      this.head.append(chip);
    }
  }

  /**
   * What was asked, in words, out of the chips and the box.
   *
   * Three facts and no more: what was typed, what counts as a word, and how the
   * words had to stand. Rust's own `header` is kept on the wire and is used only
   * where the window cannot compose one — a mode whose chips do not describe it,
   * which is Smart announcing a widening.
   */
  private whatWasAsked(found: Found): string {
    const typed = this.box.value.trim();
    const set = (chip: string): string | null => {
      const row = found.chips.find((c) => c.key === chip);
      const chosen = row?.choices.find((c) => c.chosen);
      return chosen ? chipSaid(chip, chosen) : null;
    };
    const mode = found.chips.find((c) => c.key === "mode")?.choices.find((c) => c.chosen)?.key;
    // Smart says what it *widened to*, which is a fact about the search that ran
    // and not about the chips it ran under. Regex, Citation and Instruments
    // describe themselves in the query itself.
    if (mode !== "ToratEmet") return found.header || `${say("askedFor")} ${typed}`;
    const how = set("match");
    const where = set("together");
    const parts = [typed, how, where].filter(Boolean);
    return `${say("askedFor")}: ${parts.join(" · ")}`;
  }

  /**
   * *4 PDFs on this shelf aren't searchable yet — [קרא אותם]*.
   *
   * The sentence is composed in Rust (`girsa_app::reading::Gap::said`) so the
   * header, the CLI and the test cannot drift; the button opens the first of
   * the scans, where the *read* control on its pane is. Empty until the answer
   * arrives, and empty forever when there is nothing to say — which is a
   * different silence from the one this exists to prevent.
   */
  private gapLine(): HTMLElement {
    const line = document.createElement("p");
    line.className = "find-gap";
    void api
      .scanGap()
      .then((gap) => {
        // The header may have been rebuilt while this was in flight, in which
        // case this paragraph is no longer in the document and appending to it
        // costs nothing and shows nothing. `isConnected` says so rather than
        // leaving a reader wondering why a sentence appeared twice.
        if (!gap || !line.isConnected) return;
        const said = document.createElement("span");
        said.textContent = gap.said;
        line.append(said);
        const open = document.createElement("button");
        open.type = "button";
        open.className = "find-offer";
        open.textContent = say("readThem");
        open.title = gap.scans.map((s) => `${s.title} — ${s.read}/${s.pages}`).join("\n");
        open.addEventListener("click", () => {
          const first = gap.scans[0];
          if (!first) return;
          this.dock();
          this.opened(first.slug, null);
        });
        line.append(open);
      })
      .catch(() => undefined);
    return line;
  }

  private drawHits(found: Found): void {
    this.list.replaceChildren();
    if (found.landing) {
      // **Above the hits, not instead of them.** In Citation mode there are no
      // hits and this is the whole answer. In every other mode the words were
      // searched for, the count in the header is honest, and this is an offer:
      // *what you typed also reads as a place — here it is.*
      //
      // `שבת לא.` used to be 92,384 word hits and no way at all to reach the
      // daf, because the one control that could was behind an `@` that nothing
      // on any screen taught. Switching the mode for the reader would be the
      // one thing spec.md §9 forbids; putting the place in front of them is
      // not.
      this.list.append(this.landing(found));
    }
    for (const hit of found.hits) {
      const row = document.createElement("button");
      row.type = "button";
      row.className = "find-hit";
      const where = document.createElement("span");
      where.className = "find-where";
      where.textContent =
        hit.page === null
          ? `${hit.title} ${hit.address}`
          : `${hit.title} — ${say("page")} ${hit.page}`;
      const text = document.createElement("p");
      text.className = "find-text";
      text.append(...runs(hit.runs));
      row.append(where);
      // The badge (spec.md §9.7). **Badge them, don't demote them**: the row is
      // where the score put it, and this says what kind of reading it is.
      if (hit.by !== null) {
        const badge = document.createElement("span");
        badge.className = hit.guessed ? "find-badge is-guessed" : "find-badge";
        badge.textContent = hit.guessed ? "OCR" : say("scanBadge");
        badge.title = hit.guessed
          ? `${say("scanGuessedWhy")} (${hit.by})`
          : say("scanEmbeddedWhy");
        row.append(badge);
      }
      row.append(text);
      // Three landings, and the keys are the picker's: plain is a tab, Ctrl is
      // beside what you are reading, Shift is a second view of its own. Said on
      // the row's title, because a modifier nobody is told about is a modifier
      // nobody presses.
      row.title = say("findOpenWhy");
      row.addEventListener("click", (event) => {
        // Docked, not closed (W48): the reader is going to want the next result.
        this.dock();
        this.opened(
          hit.work,
          hit.id,
          hit.marked,
          landingOf({ ctrl: event.ctrlKey || event.metaKey, shift: event.shiftKey }),
        );
      });
      this.list.append(row);
    }
    if (found.pages > found.page) {
      const more = button(say("more"), say("more"), () => {
        void (async () => {
          this.page += 1;
          await this.run();
        })();
      });
      more.classList.add("find-more");
      this.list.append(more);
    }
  }

  /** A mareh makom: where it lands, or the candidates it could be. */
  private landing(found: Found): HTMLElement {
    const box = document.createElement("div");
    box.className = "find-landing";
    const said = document.createElement("p");
    said.className = "find-said";
    said.textContent = found.landing?.said ?? "";
    box.append(said);
    for (const place of found.landing?.places ?? []) {
      const row = document.createElement("button");
      row.type = "button";
      row.className = "find-hit";
      row.textContent = place.said;
      row.title = place.reference;
      row.addEventListener("click", () => {
        this.dock();
        this.opened(place.work, place.id);
      });
      box.append(row);
    }
    // Offered, never taken. Every one of these is something the shelf could not
    // rule out, or a spelling close to what was typed.
    for (const near of found.landing?.near ?? []) {
      const note = document.createElement("p");
      note.className = "find-near";
      note.textContent = near;
      box.append(note);
    }
    return box;
  }

  private drawRail(found: Found): void {
    this.rail.replaceChildren();
    if (!found.facets || found.total === 0) return;

    for (const { dimension, label } of FACETS) {
      const group = document.createElement("div");
      group.className = "find-facet";
      const title = document.createElement("p");
      title.className = "find-facet-title";
      title.textContent = label();
      group.append(title);

      if (dimension === "link" && found.facets.link.state === "not_built") {
        const why = document.createElement("p");
        why.className = "find-facet-none";
        // Never a silent gap: *nothing is commented on* and *nobody worked out
        // what is commented on* are different statements (spec.md §9.7's rule,
        // one facet over).
        why.textContent = say("linkFacetUnbuilt");
        why.title = say("linkFacetUnbuiltWhy");
        group.append(why);
        this.rail.append(group);
        continue;
      }

      const rows: FacetRow[] =
        dimension === "link"
          ? found.facets.link.state === "counted"
            ? found.facets.link.rows
            : []
          : found.facets[dimension];
      if (rows.length === 0) continue;

      for (const row of rows.slice(0, ROWS)) {
        group.append(this.facetRow(dimension, row));
      }
      if (rows.length > ROWS) {
        const more = document.createElement("p");
        more.className = "find-facet-none";
        more.textContent = `${say("andMore")} ${rows.length - ROWS}`;
        group.append(more);
      }
      this.rail.append(group);
    }

    if (found.facets.uncatalogued > 0) {
      const note = document.createElement("p");
      note.className = "find-facet-none";
      note.textContent = `${found.facets.uncatalogued} ${say("uncatalogued")}`;
      this.rail.append(note);
    }
  }

  /** One facet row: its count, and the two clicks on it. */
  private facetRow(dimension: Dimension, row: FacetRow): HTMLElement {
    const line = document.createElement("div");
    line.className = "find-facet-row";
    line.style.paddingInlineStart = `${row.depth * 0.75}rem`;

    // A row with no label is an **absence**, and what to call an absence is the
    // window's question. The era facet's largest row used to be the sentence
    // `no era recorded`, composed in Rust, in English, in a Hebrew panel.
    const label = row.label || nameOfNothing(dimension);

    const narrow = document.createElement("button");
    narrow.type = "button";
    narrow.className = "find-facet-narrow";
    narrow.textContent = label;
    narrow.title = `${say("narrowTo")}${label}`;
    narrow.addEventListener("click", async () => {
      await api.findNarrow(dimension, row, false);
      this.page = 1;
      await this.run();
    });

    const count = document.createElement("span");
    count.className = "find-facet-count";
    count.textContent = String(row.count);

    // `−` is not a name. What it does is, and it says which row it does it to.
    const out = glyph("−", `${say("takeOut")} ${label}`, () => {
      void (async () => {
        await api.findNarrow(dimension, row, true);
        this.page = 1;
        await this.run();
      })();
    });
    out.classList.add("find-facet-out");

    line.append(narrow, count, out);
    return line;
  }
}

// ─── the chips, in the reader's language (finding 7) ────────────────────────
//
// The engine sends a **key** per chip and a key per choice, plus the wire's own
// English `label`. The keys are the protocol and the labels are a fallback; what
// a reader sees is decided here, out of `say.ts`, like every other word in this
// window.
//
// It used to be one field. `Chip.name` was both what was drawn and what
// `find_chip` was called back with, so a fully Hebrew window opened Ctrl+F on
// `torat emet ▾ | whole shelf ▾ | the word ▾ | anywhere in a segment ▾` and
// there was no way to translate it that did not change the protocol.

/**
 * What a facet row with no label is called.
 *
 * Named per dimension because *no era recorded* and *no author recorded* are
 * not the same sentence. The old fallback handed back `nothingHere` — the
 * reading pane's *אין כאן*, a sentence about the sefer you are standing in —
 * for any dimension whose row had no label, which answered an attribution
 * question with a place-word. Anything else falls to *unlabelled*, which says
 * only what is true: this row has no name.
 */
function nameOfNothing(dimension: Dimension): string {
  if (dimension === "era") return say("noEraRecorded");
  if (dimension === "author") return say("noAuthorRecorded");
  return say("facetUnlabelled");
}

/** What one rung of the relaxation ladder is called (spec.md §9.6). */
function rungSaid(offer: { rung: string; label: string }): string {
  const word = RUNG_WORDS[offer.rung];
  return word ? say(word) : offer.label;
}

/** `Rung::name()` → the word for it. */
const RUNG_WORDS: Record<string, Word> = {
  nikud: "rungNikud",
  prefixes: "rungPrefixes",
  spellings: "rungSpellings",
  gershayim: "rungGershayim",
  abbreviations: "rungAbbreviations",
  root: "rungRoot",
  proximity: "rungProximity",
};


/** The runs of a segment, as elements — corpus text is never put in as markup. */
function runs(list: Run[]): Node[] {
  return list.map((run) => {
    // Absent means plain — see `girsa_app::display::Run::style`.
    const style = run.style ?? "plain";
    if (style === "break") return document.createElement("br");
    // The words that answered the query get a `<mark>` (W39) — the element, not
    // just a colour, so a screen reader says *highlighted* and a reader can see
    // at a glance which part of the line is the hit. Which words those are came
    // from the engine; this does not go looking.
    const node = document.createElement(run.hit ? "mark" : "span");
    node.className = run.hit ? `run run-${style} is-hit` : `run run-${style}`;
    node.textContent = run.text;
    return node;
  });
}

export type { ScopeView };
