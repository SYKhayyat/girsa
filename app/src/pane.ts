// One column of reading.
//
// A pane holds a sefer, renders a window of it, and reports where the reader
// is. It does not decide where anything goes — it is told, by `main.ts`, which
// asks Rust.

import { api } from "./api.ts";
import type { FixMark, Line, MarkRow, PaneId, Place, Relation, Run, Said, Text } from "./api.ts";
import { glyph, toolStrip } from "./controls.ts";
import { everywhereSaid, marking } from "./mefarshim.ts";
import { alsoCalled, sefer } from "./names.ts";
import { fill, say } from "./say.ts";

/** How many lines are put on the page at once, and how many more at an edge. */
const WINDOW = 400;
const STEP = 300;

/**
 * The most lines the document holds at once — the window, plus room to reach
 * an edge twice before anything comes off the other end.
 *
 * > *"Mishnah Berurah, 17,418 segments, scrolled to its end: 400 lines in the
 * > document on opening, 17,418 after 240 jumps. `extend()` appends and
 * > prepends and nothing ever removes a line."*
 *
 * The ceiling has to clear `WINDOW + STEP`, or an extend would trim into the
 * lines it had just drawn; past that the number is a trade between how far a
 * reader can turn back without a round trip and how much page the browser is
 * laying out. Two steps of slack is a thousand lines either side of where they
 * are standing, which is more than anybody reads back through in one motion.
 */
const KEEP = WINDOW + 2 * STEP;

/** A stretch of the sefer, as indices into `lines`. */
export interface Drawn {
  from: number;
  to: number;
}

/**
 * Which lines should be on the page after the reader reaches an edge.
 *
 * **Both ends move.** The old rule moved one: `to` forward at the bottom,
 * `from` back at the top, and the far end stayed where it was from the moment
 * the sefer opened. That is a window in the sense that it starts small, and not
 * in the sense that it stays that way — a reader working through Mishnah
 * Berurah front to back finished holding all 17,418 lines, and every `paint()`
 * on the way there walked every line drawn so far.
 *
 * So growing one end pulls the other in behind it, and the arithmetic is here
 * rather than in the method that appends because *how big does this get* is a
 * question with an answer, and the answer should be checkable without a
 * browser. `pane.test.mjs` runs this two hundred and forty times over the real
 * number from the audit and asserts the span stops growing.
 *
 * `Math.max`/`Math.min` against the near end and not a plain subtraction: a
 * sefer shorter than the ceiling is never trimmed at all, and neither is a
 * window that has not yet grown to it.
 */
export function grown(have: Drawn, where: "up" | "down", total: number): Drawn {
  if (where === "down") {
    const to = Math.min(total, have.to + STEP);
    return { from: Math.max(have.from, to - KEEP), to };
  }
  const from = Math.max(0, have.from - STEP);
  return { from, to: Math.min(have.to, from + KEEP) };
}

/**
 * The next place of yours to go to, from where you are standing.
 *
 * > *"a way to leave a mark in a sefer — like here is my place, so it is
 * > visible and jumpable (many should be available)."*
 *
 * The marks became visible when `paint()` stopped skipping every span-less
 * mark. **Jumpable** was still only true of the *yours* panel — a reader with
 * four places in Mishnah Berurah had to open a drawer, find the row, and click
 * it, which is not what *here is my place* is for.
 *
 * The rule: **the next one after where you are, wrapping to the first.** Wrapping
 * rather than stopping, because a reader pressing the key at the end of the
 * sefer means *the next one* and there is one — at the top — and a key that
 * silently does nothing is indistinguishable from a key that is not bound.
 *
 * `null` only when there are no places at all, which is the one case where
 * nothing is the honest answer.
 */
export function nextPlace(places: number[], from: number): number | null {
  if (places.length === 0) return null;
  const order = [...places].sort((a, b) => a - b);
  return order.find((at) => at > from) ?? order[0] ?? null;
}

/**
 * A sefer is up to eighteen thousand segments (Mishnah Berurah), and putting
 * all of them in the document makes opening one feel like waiting. So a window
 * of lines around where the reader is goes in, and it grows when they reach an
 * edge — the scrollbar tells a small lie about the length of the sefer, which
 * is the same lie every reader in the world is already used to from a book.
 *
 * # And the pane is no longer *given* the whole sefer either
 *
 * It used to be: `open_sefer` serialized every segment and the pane sliced a
 * window out of what it held. Measured, that is **7.7 MB of JSON** for Mishnah
 * Berurah — built in Rust, pushed over IPC, parsed by the webview and kept in
 * its heap — so that four hundred lines could be drawn
 * (`examples/measure-opening.rs`).
 *
 * Now `lines` is as long as the sefer and mostly **holes**. What has been loaded
 * is filled in at its true index, so every index in this file still means the
 * same thing it always did; what has not is fetched when the reader reaches it.
 * A line the pane has never seen — a search hit, a link, a mefaresh's place — is
 * found by asking Rust where it is (`sefer_index_of`) and loading that window,
 * which is one round trip on a jump and none on a scroll.
 *
 * # And the window gives lines back
 *
 * It did not. `WINDOW` bounded the *first* render and nothing else, so the
 * sentence above — *a window of lines around where the reader is* — was true of
 * the first screen and became less true with every edge they reached. The
 * measurement is finding 22: 400 lines in the document on opening, **17,418**
 * after two hundred and forty jumps, 52,618 nodes.
 *
 * Nineteen megabytes is not the reason to care, and saying so is worth the
 * sentence: for the largest sefer on the shelf that is nothing, and the audit
 * says as much. The cost that matters is that `paint()` — highlights, and the
 * mefarshim marker on every line — walks the drawn lines, and the drawn lines
 * were everything ever drawn. An unbounded page makes the work of adding to it
 * grow with how long the reader has been sitting there, which is the one shape
 * of slow that a person cannot tell from *this application gets worse*.
 *
 * `grown` bounds it. What is **loaded** is deliberately not bounded with it:
 * `lines` and `byId` stay, so turning back to a line the reader has already
 * seen is still no round trip, which is the property the previous work order
 * bought by taking the whole sefer off the wire. A cache whose worst case is
 * the size of the thing it caches is a cache; a document that never stops
 * growing is a leak.
 */
export class PaneView {
  readonly id: PaneId;
  readonly slug: string;
  readonly element: HTMLElement;
  private readonly body: HTMLElement;
  private readonly note: HTMLElement;
  /** The one sentence that replaces a marker on every line of the sefer. */
  private readonly everywhere: HTMLElement;
  private readonly title: HTMLElement;
  private readonly where: HTMLElement;
  /** The header's buttons, as one box that wraps as one — see `toolStrip`. */
  private readonly tools: HTMLElement;
  private text: Text | null = null;
  /** The sefer, as long as the sefer is, with holes where nothing is loaded. */
  private lines: (Line | undefined)[] = [];
  /** The first and last **drawn** line, as indices into `lines`. */
  private from = 0;
  private to = 0;
  /** Segment id → its index in `lines`, for the lines that have been loaded. */
  private byId = new Map<string, number>();
  /** A fetch already in flight, so a burst of scroll events asks once. */
  private fetching: Promise<void> | null = null;
  /** Whether an extend is already waiting on that fetch.
   *
   * `fetching` makes a burst of scroll events **one request**; this makes them
   * one *append*. Without it every waiter resumes when the lines land and each
   * one appends the same three hundred lines, because they all read the same
   * unmoved `this.to`. It did not fire against the fixtures, which answer in
   * microseconds — it would fire against an IPC round trip, which is where this
   * code actually runs. */
  private extending = false;
  private highlighted: string[] = [];
  /** Your highlights in this sefer, as Rust placed them (W27). Kept so the
   * lines that scroll into view later get painted too. */
  private marks: MarkRow[] = [];
  /** The lines a **ticked** mefaresh speaks on, and how many of them do (W43).
   * Rust's answer, not this one's: which lines those are is a fact about the
   * link graph. */
  private marked: Record<string, number> = {};
  /** How many mefarshim are ticked on this sefer. Nothing ticked means a click
   * on a line is just a click — the reader has not asked for anything, so the
   * pane must not start answering. */
  private ticked = 0;
  private onComments: ((at: string) => void) | null = null;
  /** Where a citation in your own writing goes when it is clicked (W19). */
  private onCiting: ((reference: string) => void) | null = null;
  /** Set while a following pane is being moved, so its own scroll handler does
   * not report the move back and start the two panes chasing each other. */
  private quiet = false;
  /** The frame a coalesced scroll is waiting for, or `null`. */
  private pending: number | null = null;

  constructor(
    id: PaneId,
    slug: string,
    private readonly onMove: (pane: PaneId, at: string) => void,
    private readonly onFocus: (pane: PaneId) => void,
  ) {
    this.id = id;
    this.slug = slug;
    this.element = el("section", "pane");
    this.element.dataset.pane = String(id);

    const header = el("header", "pane-head");
    this.title = el("span", "pane-title");
    this.where = el("span", "pane-where");
    this.note = el("span", "pane-note");
    // The buttons are one box, so the header runs out of room by wrapping them
    // rather than by squeezing the sefer's name to nothing — see `toolStrip`.
    this.tools = toolStrip();
    header.append(this.title, this.where, this.note, this.tools);
    this.element.append(header);

    // What a marker on every line would have said, said once. Under the header
    // rather than in it, because the header is already the thing finding 5 was
    // about — and empty for every sefer where the marker does its job, which is
    // most of them. `:empty` takes the strip away with it.
    this.everywhere = el("p", "pane-everywhere");
    this.element.append(this.everywhere);

    this.body = el("div", "pane-body");
    this.body.tabIndex = 0;
    this.element.append(this.body);

    // One run per frame, not one per scroll event.
    //
    // `{ passive: true }` keeps this off the scroll thread and does nothing
    // about what is inside it: `extend` writes up to 300 line elements and then
    // reads `scrollHeight` to put the reader back where they were, which is a
    // forced synchronous layout, and a trackpad fires scroll events faster than
    // the compositor draws. Coalescing to a frame does the work once for the
    // burst.
    this.body.addEventListener(
      "scroll",
      () => {
        if (this.pending !== null) return;
        this.pending = requestAnimationFrame(() => {
          this.pending = null;
          this.scrolled();
        });
      },
      { passive: true },
    );
    this.body.addEventListener("pointerdown", () => this.onFocus(this.id));
    this.body.addEventListener("focus", () => this.onFocus(this.id));
    this.body.addEventListener("click", (event) => this.clicked(event));
  }

  /** What to do when a reader clicks a line to see their mefarshim on it. */
  whenComments(fn: (at: string) => void): void {
    this.onComments = fn;
  }

  /** What to do when a reader clicks a citation in their own writing (W19). */
  whenCiting(fn: (reference: string) => void): void {
    this.onCiting = fn;
  }

  /**
   * A click on a line, in the one case where it means something.
   *
   * Silent unless the reader has ticked at least one mefaresh. That is not
   * timidity — a pane where every click makes something appear is a pane you
   * cannot click, and the reader who has ticked nobody has not asked a question.
   */
  private clicked(event: MouseEvent): void {
    // A reader dragging across words is quoting, not asking. Checked before
    // the citation as well as before the comments: selecting a line that has a
    // mekor in it must not jump the pane out from under the selection.
    if (window.getSelection()?.isCollapsed === false) return;
    const target = event.target;
    if (!(target instanceof Node)) return;
    // A citation in your own writing, clicked (W19). Before the mefarshim,
    // because these words asked a narrower question than the line did — and
    // unconditionally, because unlike a comment there is nothing to tick: the
    // reader wrote the citation, which is the whole of the request.
    if (target instanceof Element) {
      const cited = target.closest<HTMLElement>(".run-cite")?.dataset.cite;
      if (cited && this.onCiting) {
        this.onCiting(cited);
        return;
      }
    }
    if (this.ticked === 0 || !this.onComments) return;
    // Clicks inside an open comment belong to the comment.
    if (target instanceof Element && target.closest(".line-said")) return;
    const line = lineOf(target);
    const id = line?.dataset.id;
    if (id) this.onComments(id);
  }

  /** The buttons a pane's header carries. Added by the caller, which owns what
   * they do. */
  addControl(control: HTMLElement): void {
    this.tools.append(control);
  }

  /** The sefer's Hebrew title, once it has been read. */
  title_he = "";

  show(text: Text, at: string | null): void {
    this.text = text;
    this.title_he = sefer(text.work);
    this.lines = new Array<Line | undefined>(text.total);
    this.byId = new Map();
    this.take(text.from, text.lines);
    this.title.textContent = sefer(text.work);
    // The other name a hover away, rather than gone.
    this.title.title = alsoCalled(text.work);
    const start = at ? (this.byId.get(at) ?? text.from) : text.from;
    this.render(start);
    if (at) void this.goToId(at, false);
  }

  /** File a stretch of lines at its true index, and index it by id. */
  private take(from: number, lines: Line[]): void {
    lines.forEach((line, n) => {
      this.lines[from + n] = line;
      this.byId.set(line.id, from + n);
    });
  }

  /** Whether every line in a range is loaded. */
  private has(from: number, to: number): boolean {
    for (let i = Math.max(0, from); i < Math.min(to, this.lines.length); i += 1) {
      if (!this.lines[i]) return false;
    }
    return true;
  }

  /**
   * Load a stretch, once.
   *
   * Every scroll to an edge asks; `fetching` makes a burst of them one request,
   * because a trackpad fires scroll events faster than an IPC round trip
   * returns and twenty requests for the same three hundred lines is the shape of
   * slow this whole change is about.
   */
  private async load(from: number, count: number): Promise<void> {
    if (this.fetching) return this.fetching;
    const at = Math.max(0, from);
    const run = (async () => {
      try {
        const lines = await api.seferLines(this.slug, at, count);
        this.take(at, lines);
      } finally {
        this.fetching = null;
      }
    })();
    this.fetching = run;
    return run;
  }

  /** Put a window of lines centred on `index` into the document. */
  private render(index: number): void {
    this.from = Math.max(0, index - WINDOW / 2);
    this.to = Math.min(this.lines.length, this.from + WINDOW);
    this.body.replaceChildren(...this.drawn(this.from, this.to));
    this.paint();
  }

  /** The elements for a range — only the lines that are actually loaded.
   *
   * A hole draws nothing rather than a placeholder: the fetch that fills it is
   * already in flight, and a row of grey boxes flickering into text is a worse
   * thing to look at than a page that arrives. */
  private drawn(from: number, to: number): HTMLElement[] {
    const out: HTMLElement[] = [];
    for (let i = from; i < to; i += 1) {
      const line = this.lines[i];
      if (line) out.push(lineElement(line));
    }
    return out;
  }

  /**
   * Your highlights, drawn on the words (spec.md §11).
   *
   * **Where each one goes was decided in Rust** — `girsa_note::Mark::place`,
   * against the same string this pane was sent. Nothing here re-finds a
   * highlight's words, because that rule lives in one place
   * (`girsa_corpus::span`) and a second copy of it in TypeScript would put a
   * highlight in one place in the pane and another in the panel.
   *
   * A mark whose words have gone is **not drawn** and is not thrown away
   * either: it comes back `stale` and the שלי panel says so.
   */
  setMarks(marks: MarkRow[]): void {
    this.marks = marks;
    this.paint();
  }

  /**
   * Which lines your mefarshim speak on (W43).
   *
   * Only the ticked ones, and that is the decision the whole interaction rests
   * on: 2,749 of Berakhot's segments carry commentary from somebody, so marking
   * every line that has any would mark the daf and say nothing. Which lines those
   * are was worked out in Rust, from the link graph — this draws it.
   *
   * It draws **how many** of them speak, and sometimes draws nothing at all: a
   * targum comments on every posuk, so *one of yours is here* was true of 1,533
   * of Bereishis' 1,533 lines and the careful marker marked the whole sefer.
   * `marking` holds that decision; this asks it.
   */
  setMefarshim(marked: Record<string, number>, ticked: number): void {
    this.marked = marked;
    this.ticked = ticked;
    // A mefaresh unticked while their comments are open: the comments are no
    // longer an answer to anything the reader is asking.
    for (const open of this.body.querySelectorAll(".line-said")) open.remove();
    this.paint();
  }

  /**
   * What the ticked mefarshim say about one line, under that line.
   *
   * Under it, and not in a panel over the page. Eleven panels in this window are
   * `position: fixed` and the reader's complaint about the first one they met was
   * *"it is weirdly over the text, so i cant see it or the text"*. A comment on a
   * line belongs beside the line — and this way the answer cannot cover the
   * question.
   *
   * Clicking the same line again closes it, so the gesture that opened it is the
   * gesture that puts the daf back.
   */
  showSaid(at: string, said: Said[], message: string): void {
    const row = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(at)}"]`);
    if (!row) return;
    const already = row.nextElementSibling;
    if (already?.classList.contains("line-said")) {
      already.remove();
      return;
    }
    const box = el("div", "line-said");
    if (message) {
      const none = el("p", "said-none");
      none.textContent = message;
      box.append(none);
    }
    for (const one of said) {
      // `said-one`, not `said`. `.said` is the toast at the foot of the window
      // — `position: fixed; opacity: 0` until `announce` raises it — and a
      // comment block given that class was drawn at zero opacity, off the flow,
      // from the day both were written. `collision.test.mjs` is the guard.
      const block = el("div", "said-one");
      const who = el("p", "said-who");
      const named = sefer(one);
      who.textContent = one.address ? `${named} ${one.address}` : named;
      who.title = alsoCalled(one);
      block.append(who);
      for (const line of one.lines) {
        const words = el("p", "said-line");
        words.append(...line.runs.map(runElement));
        block.append(words);
      }
      box.append(block);
    }
    // **Shut it from where you finished reading it** (A14).
    //
    // > *"a way to collapse a mefarshim block from its bottom (a little
    // > arrow)."*
    //
    // The gesture that opens a block is clicking its line, and clicking that
    // line again closes it — which is right, and is only reachable from the
    // top. A Kaf HaChayim on one se'if is longer than the window, so a reader
    // who has read to the end of it has to scroll back past everything they
    // just read to put the daf back. This is the same act, at the other end.
    const shutIt = glyph("▴", say("saidShut"), () => {
      box.remove();
      // Back to the line it was about, or the reader is left looking at
      // whatever was underneath — which on a long block is a different se'if
      // and reads as the page having jumped.
      row.scrollIntoView({ block: "nearest" });
    });
    shutIt.className = "said-shut";
    box.append(shutIt);
    row.after(box);
  }

  private paint(): void {
    this.markMefarshim();
    for (const mark of this.marks) {
      const at = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(mark.at)}"]`);
      // **A bookmark names a place, not words**, so it has no span — `MarkRow`
      // says so in as many words: *"the characters it is on — absent for a
      // bookmark"*. This loop opened with `if (!mark.span) continue`, which is
      // correct about highlighting and meant that the one function which draws
      // marks on a page skipped every bookmark ever made. A reader could put his
      // place down, and the sefer looked exactly the same afterwards; the mark
      // existed only as a row in the *yours* panel, which is the one place you
      // are not looking when you are learning.
      //
      // Drawn in the gutter beside the line rather than over the words, for the
      // same reason `.line-fix` is: there are no words it is on, and a highlight
      // covering a se'if would be a claim about the text that nobody made.
      if (!mark.span) {
        if (!at || at.querySelector(`[data-place="${cssEscape(mark.id)}"]`)) continue;
        const flag = el("span", "line-place");
        flag.dataset.place = mark.id;
        flag.textContent = "⚑";
        flag.title = mark.label ?? say("bookmark");
        at.prepend(flag);
        continue;
      }
      const line = at;
      const words = line?.querySelector<HTMLElement>(".line-text");
      if (!words || words.querySelector(`[data-mark="${cssEscape(mark.id)}"]`)) continue;
      const range = charRange(words, mark.span[0], mark.span[1]);
      if (!range) continue;
      const painted = el("mark", "line-mark");
      painted.dataset.mark = mark.id;
      if (mark.colour) painted.style.setProperty("--mark", mark.colour);
      painted.title = mark.label ?? mark.was;
      try {
        range.surroundContents(painted);
      } catch {
        // A highlight that runs across two runs of different styling cannot be
        // wrapped in one element. Left unpainted rather than split into two
        // marks that would look like two highlights.
      }
    }
  }

  /**
   * The mark itself: a class on the line, so the CSS owns what it looks like.
   *
   * Except where there is nothing to distinguish. `marking` answers over the
   * whole sefer's lines rather than the drawn ones, so the marker does not
   * appear and vanish as the reader scrolls into a stretch that happens to be
   * uniform — *the marker says nothing here* is a fact about the sefer, and a
   * fact about the sefer is said once, in the header, where the reader is
   * already told what they are looking at.
   */
  private markMefarshim(): void {
    const how = marking(this.marked, this.lines.length);
    this.element.classList.toggle("is-marked-everywhere", how.kind === "everywhere");
    this.everywhere.textContent = how.kind === "everywhere" ? everywhereSaid(how.each) : "";
    for (const row of this.body.querySelectorAll<HTMLElement>(".line")) {
      const n = how.kind === "some" ? (this.marked[row.dataset.id ?? ""] ?? 0) : 0;
      row.classList.toggle("has-mefarshim", n > 0);
      // The number, and only where it is a number worth reading: one ticked
      // mefaresh speaking is the diamond the reader already knows, and a `1` in
      // the margin of a page where nothing says `2` is a worse diamond.
      //
      // On the address and not on the line, because `attr()` in a
      // `::after` reads the element the pseudo-element hangs off and not its
      // ancestors — the one thing about `attr()` that is easy to write wrong and
      // impossible to see, since a `content` that cannot resolve draws nothing.
      const address = row.querySelector<HTMLElement>(".line-address");
      if (address) {
        if (n > 1) address.dataset.said = String(n);
        else delete address.dataset.said;
      }
      // Set **and cleared**, because unticking a mefaresh has to take the hover
      // with it. The old line was `if (on && !row.title)`, which wrote the title
      // once and left it there for the rest of the session.
      row.title = n > 1 ? fill("markHowMany", { n }) : n === 1 ? say("markWhy") : "";
    }
  }

  /**
   * Grow the page at the edge the reader reached, and take the far end off.
   *
   * One shape for both directions, where there were two. They were the same
   * method written twice with `from`/`to`, `prepend`/`append` and `max`/`min`
   * swapped, and the half that put the reader back after a prepend existed only
   * in the upward copy — which was correct then and would have been the bug the
   * moment the downward copy also had to move what is above the fold.
   */
  private extend(where: "up" | "down"): void {
    if (!this.text) return;
    const want = grown({ from: this.from, to: this.to }, where, this.lines.length);
    // Already there: the end of the sefer downward, the start of it upward.
    if (want.from === this.from && want.to === this.to) return;

    const [at, until] = where === "down" ? [this.to, want.to] : [want.from, this.from];
    if (!this.has(at, until)) {
      // Not here yet. Fetch it and come back — one waiter, however many scroll
      // events arrived while it was in flight.
      if (this.extending) return;
      this.extending = true;
      void this.load(at, until - at).then(() => {
        this.extending = false;
        this.extend(where);
      });
      return;
    }

    if (where === "down") {
      this.body.append(...this.drawn(at, until));
      this.to = want.to;
      if (want.from !== this.from) {
        this.holdingPlace(() => this.dropAbove(want.from));
        this.from = want.from;
      }
    } else {
      this.holdingPlace(() => this.body.prepend(...this.drawn(at, until)));
      this.from = want.from;
      if (want.to !== this.to) {
        // Nothing to correct: what goes is below the fold, and the reader is at
        // the other end of a thousand lines.
        this.dropBelow(want.to);
        this.to = want.to;
      }
    }
    this.paint();
  }

  /**
   * Do something that changes how much is **above** the reader, and leave them
   * on the words they were looking at.
   *
   * Adding lines at the top pushes the page down and taking lines off the top
   * pulls it up: one correction, with the sign falling out of which happened.
   * Two `scrollHeight` reads are two forced layouts, so the callers only wrap
   * the move that actually changes anything.
   *
   * The browser's own answer to this is scroll anchoring, and this pane must
   * not have both: `.pane-body` declares `overflow-anchor: none`, because a
   * heuristic about which element to hold and an explicit correction of the
   * same shift add up to twice the shift. Nothing here noticed, which is the
   * point — the correction was written when only one end moved.
   */
  private holdingPlace(change: () => void): void {
    const before = this.body.scrollHeight;
    change();
    this.body.scrollTop += this.body.scrollHeight - before;
  }

  /** Take off every drawn line before `from`. */
  private dropAbove(from: number): void {
    for (;;) {
      const first = this.body.firstElementChild;
      if (!(first instanceof HTMLElement)) return;
      // A block of commentary hangs *under* its line, so one standing at the
      // top of the page is one whose line has already gone (W43).
      if (first.classList.contains("line-said")) {
        first.remove();
        continue;
      }
      if (!first.classList.contains("line")) return;
      const at = this.byId.get(first.dataset.id ?? "");
      // A line this pane cannot place is left alone rather than thrown away:
      // `from`/`to` are indices, and removing something whose index is unknown
      // would put the two out of step with the page.
      if (at === undefined || at >= from) return;
      first.remove();
    }
  }

  /** Take off every drawn line at or after `to`. */
  private dropBelow(to: number): void {
    for (;;) {
      const last = this.body.lastElementChild;
      if (!(last instanceof HTMLElement)) return;
      const line = last.classList.contains("line-said") ? last.previousElementSibling : last;
      if (!(line instanceof HTMLElement) || !line.classList.contains("line")) return;
      const at = this.byId.get(line.dataset.id ?? "");
      if (at === undefined || at < to) return;
      last.remove();
    }
  }

  private scrolled(): void {
    if (this.body.scrollTop < 400) this.extend("up");
    if (this.body.scrollHeight - this.body.scrollTop - this.body.clientHeight < 600) {
      this.extend("down");
    }
    if (this.quiet) return;
    const top = this.topLine();
    if (top) {
      this.where.textContent = addressOf(top);
      this.onMove(this.id, top.dataset.id ?? "");
    }
  }

  /**
   * The first line whose text is actually in view.
   *
   * `.line` and not any child, because an open block of commentary (W43) sits
   * between two lines and is not one. Returning it would report a position with
   * no segment id, which is how a following pane gets told to scroll to "".
   *
   * **Binary search, not a walk.** Lines are in document order and stack
   * vertically, so *is this line's bottom below the fold* is monotonic — false
   * for every line above the answer and true for every line at or below it.
   * This read up to 400 `getBoundingClientRect()`s per scroll event to find the
   * one that flips; it reads about nine.
   */
  private topLine(): HTMLElement | null {
    const top = this.body.getBoundingClientRect().top + 8;
    const lines = [...this.body.querySelectorAll<HTMLElement>(":scope > .line")];
    let low = 0;
    let high = lines.length;
    while (low < high) {
      const middle = (low + high) >> 1;
      const line = lines[middle];
      if (line && line.getBoundingClientRect().bottom > top) high = middle;
      else low = middle + 1;
    }
    return lines[low] ?? null;
  }

  /**
   * Go to the next place of yours in this sefer (A15).
   *
   * Indices, not ids, because *next* is a question about reading order and an
   * id does not carry one. A mark on a line this pane has never loaded is
   * looked up — `sefer_index_of`, the same call a search hit and a link use —
   * rather than being skipped, which would make the key work on the places you
   * have already scrolled past and not on the ones you have not.
   */
  async goToNextPlace(): Promise<boolean> {
    // Index **and** id together, because the answer is chosen by index and
    // reached by id. Keeping only the indices meant looking the id up again
    // afterwards, and looking it up again is what produced a `find` over a
    // promise — always truthy, so the first mark in the list was jumped to
    // whichever one was next.
    const places: { at: number; id: string }[] = [];
    for (const mark of this.marks) {
      const known = this.byId.get(mark.at) ?? (await api.seferIndexOf(this.slug, mark.at));
      if (known !== null && known !== undefined) places.push({ at: known, id: mark.at });
    }
    const next = nextPlace(
      places.map((place) => place.at),
      this.lineIndex(),
    );
    if (next === null) return false;
    const going = places.find((place) => place.at === next);
    if (!going) return false;
    await this.goToId(going.id, true);
    return true;
  }

  /** Move this pane because the pane it follows moved. */
  goTo(place: Place, relation: Relation): void {
    this.note.className = "pane-note";
    if (place.kind === "unrelated") {
      this.note.textContent = "";
      return;
    }
    if (place.kind === "no_place") {
      // The honest answer, and the reason this app does not scroll to the
      // nearest thing: there is no comment on this line, and the column stays
      // where it is rather than showing a comment on a different one.
      this.note.textContent = say("nothingHere");
      this.note.title = say("nothingHereWhy");
      this.note.classList.add("is-empty");
      this.highlight([]);
      return;
    }
    this.note.textContent =
      typeof relation === "object" ? "" : relation === "linked" ? say("linked") : "";
    const first = place.ids[0];
    if (first) void this.goToId(first, true);
  }

  /**
   * Go to a segment, loading it first if this pane has never seen it.
   *
   * The two-round-trip path, and it is the only one: *where is this segment*
   * (`sefer_index_of`) and then *give me the lines around it*. A hit in a sefer
   * the reader has open at a different place used to be a lookup in a map that
   * held the whole sefer; it is a question for the corpus now, which is where
   * every other question about the corpus goes.
   */
  private async goToId(id: string, mark: boolean): Promise<void> {
    if (this.byId.has(id)) {
      this.scrollTo([id], mark);
      return;
    }
    const at = await api.seferIndexOf(this.slug, id);
    // Not in this sefer at all. Nothing to scroll to, and nothing to say here:
    // the panel that offered the link is where a bad link is reported.
    if (at === null) return;
    await this.load(Math.max(0, at - WINDOW / 2), WINDOW);
    if (!this.byId.has(id)) return;
    this.render(at);
    this.scrollTo([id], mark);
  }

  private scrollTo(ids: string[], mark: boolean): void {
    if (!this.text || ids.length === 0) return;
    const index = this.byId.get(ids[0]);
    if (index === undefined) return;
    if (index < this.from + 5 || index >= this.to - 5) this.render(index);

    const target = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(ids[0])}"]`);
    if (!target) return;
    this.quiet = true;
    this.body.scrollTop += target.getBoundingClientRect().top - this.body.getBoundingClientRect().top - 8;
    this.where.textContent = addressOf(target);
    if (mark) this.highlight(ids);
    // One frame is not enough on a long list; the scroll event lands after.
    window.setTimeout(() => {
      this.quiet = false;
    }, 120);
  }

  private highlight(ids: string[]): void {
    for (const id of this.highlighted) {
      this.body
        .querySelector<HTMLElement>(`[data-id="${cssEscape(id)}"]`)
        ?.classList.remove("is-here");
    }
    this.highlighted = ids;
    for (const id of ids) {
      this.body
        .querySelector<HTMLElement>(`[data-id="${cssEscape(id)}"]`)
        ?.classList.add("is-here");
    }
  }

  setFocused(on: boolean): void {
    this.element.classList.toggle("is-focused", on);
  }

  /**
   * What the reader has highlighted, as segment ids and character offsets
   * (spec.md §10.2 — *highlight part of a passage; only that goes*).
   *
   * The offsets are counted over the **text of the line as it stands in the
   * document**, which is the text Rust sent: markup already turned into runs,
   * nikud already applied. So `girsa_app::sending` can slice its own copy of
   * the same string and get the same words, without either side having to
   * describe a selection to the other.
   *
   * The address label in the margin is not part of the line's text, and is
   * deliberately excluded — a reader dragging across a se'if is not asking to
   * quote its number.
   *
   * `null` when nothing here is selected: that is the whole-line case, and it
   * is the caller's to decide, because "what is the reader standing on" is a
   * different question.
   */
  selection(): { from: string; to: string; fromChar: number; toChar: number } | null {
    const chosen = window.getSelection();
    if (!chosen || chosen.isCollapsed || chosen.rangeCount === 0) return null;
    const range = chosen.getRangeAt(0);
    const from = lineOf(range.startContainer);
    const to = lineOf(range.endContainer);
    if (!from || !to) return null;
    if (!this.body.contains(from) || !this.body.contains(to)) return null;

    const fromChar = offsetIn(from, range.startContainer, range.startOffset);
    const toChar = offsetIn(to, range.endContainer, range.endOffset);
    if (fromChar === null || toChar === null) return null;
    return {
      from: from.dataset.id ?? "",
      to: to.dataset.id ?? "",
      fromChar,
      toChar,
    };
  }

  /** The line the reader is standing on — the whole-line case for a copy. */
  here(): string | null {
    return this.topLine()?.dataset.id ?? null;
  }

  /**
   * …and **where** that line is, counted in segments from the start of the
   * sefer (A3).
   *
   * The table of contents needs a number and not an id: *which siman am I in*
   * is *which entry began at or before me*, and an id would make the panel
   * search the sefer to find out. This pane already holds the answer —
   * `byId` is filled as lines arrive and `text.from` is where its window
   * begins — so handing it over costs a lookup rather than a scan.
   *
   * `0` when there is no line yet, which is the top of the sefer and is where
   * a pane with nothing drawn in it is.
   */
  lineIndex(): number {
    const here = this.here();
    return (here ? this.byId.get(here) : undefined) ?? 0;
  }

  /**
   * One line, redrawn — after a correction (W20).
   *
   * The line is replaced where it stands rather than the sefer being rebuilt:
   * a reader who has just fixed a typo is looking at the word, and a rebuild
   * would take the page out from under them. spec.md §7.5 is a requirement
   * about how this feels, and this is most of what it costs here.
   */
  replaceLine(line: Line): void {
    if (!this.text) return;
    const at = this.byId.get(line.id);
    if (at === undefined) return;
    this.lines[at] = line;
    const drawn = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(line.id)}"]`);
    drawn?.replaceWith(lineElement(line));
    // The line was redrawn from scratch, so anything painted on it went with
    // it. Where a highlight now lands is Rust's answer, not this one's — the
    // caller asks again after a correction.
    this.paint();
  }

  /**
   * What the reader has highlighted, for a correction.
   *
   * `null` unless the highlight is inside one line: a patch names one segment
   * (spec.md §7.1), and a highlight running across three of them is not one
   * correction — it is three, and which words in which of them is a question
   * this window is not entitled to answer.
   */
  fixSelection(): {
    at: string;
    fromChar: number;
    toChar: number;
    words: string;
    fixed: FixMark[];
    printed: string | null;
  } | null {
    const chosen = this.selection();
    if (!chosen || chosen.from !== chosen.to || !this.text) return null;
    const at = this.byId.get(chosen.from);
    if (at === undefined) return null;
    const line = this.lines[at];
    if (!line) return null;
    const letters = Array.from(line.runs.map((run) => run.text).join(""));
    const words = letters.slice(chosen.fromChar, chosen.toChar).join("");
    if (!words.trim()) return null;
    return {
      at: chosen.from,
      fromChar: chosen.fromChar,
      toChar: chosen.toChar,
      words,
      fixed: line.fixed ?? [],
      printed: line.printed ?? null,
    };
  }

  /** The corrections on the line the reader is standing on. */
  fixesHere(): { at: string; fixed: FixMark[]; printed: string | null } | null {
    const here = this.here();
    if (!here || !this.text) return null;
    const at = this.byId.get(here);
    if (at === undefined) return null;
    const line = this.lines[at];
    if (!line) return null;
    return { at: here, fixed: line.fixed ?? [], printed: line.printed ?? null };
  }

  /**
   * Point at one word of one line, and say where it ended up on the screen.
   *
   * What the OCR queue needs (W21): the reader arrives at a line they have
   * never seen, and the word in question has to be the one their eye lands on.
   * The offsets are the ones Rust worked out — this only draws them.
   */
  markWord(id: string, fromChar: number, toChar: number): DOMRect | null {
    const at = this.byId.get(id);
    if (at === undefined) return null;
    if (at < this.from + 5 || at >= this.to - 5) this.render(at);
    const line = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(id)}"]`);
    const words = line?.querySelector<HTMLElement>(".line-text");
    if (!line || !words) return null;

    this.quiet = true;
    this.body.scrollTop +=
      line.getBoundingClientRect().top - this.body.getBoundingClientRect().top - 8;
    window.setTimeout(() => {
      this.quiet = false;
    }, 120);

    // A range over the characters themselves, so the mark is on the word and
    // not on the line — and so its rectangle is where the box opens.
    const range = charRange(words, fromChar, toChar);
    if (!range) return null;
    const chosen = window.getSelection();
    chosen?.removeAllRanges();
    chosen?.addRange(range);
    return range.getBoundingClientRect();
  }

  /** A word about who this pane is following, in its header. */
  setFollowing(label: string): void {
    let chip = this.element.querySelector<HTMLElement>(".pane-follows");
    if (!chip) {
      chip = el("span", "pane-follows");
      // Before the buttons, not after them: the chip is a word about the
      // reading and the buttons are a block that wraps as a block.
      this.tools.before(chip);
    }
    chip.textContent = label;
  }
}

function lineElement(line: Line): HTMLElement {
  // A line of your own .ksav knows what it is — a footnote, a list item, a row
  // of a table, a block quote — and is drawn as that rather than as one more
  // paragraph. Nothing else in the corpus has these kinds, so nothing else
  // changes shape (W29).
  // A missing kind is `text`, and a missing style is `plain`: both are left off
  // the wire because nearly every line and nearly every run is one of them, and
  // opening the largest sefer on the shelf hands this window megabytes of JSON
  // before it can draw a word. See `girsa_app::view::Line::kind`.
  const kind = line.kind ?? "text";
  const row = el("p", kind === "text" ? "line" : `line is-${kind}`);
  row.dataset.id = line.id;
  row.dataset.address = line.address;
  const label = el("span", "line-address");
  label.textContent = line.address;
  const words = el("span", "line-text");
  if (kind === "row") {
    // The cells arrive tab-separated, which is what a column boundary is in
    // every plain rendering of a table. Split here rather than on the Rust
    // side: the boundary is a fact about the text and the columns are a fact
    // about the page.
    const cells = line.runs
      .map((r) => r.text)
      .join("")
      .split("	");
    for (const cell of cells) {
      const box = el("span", "line-cell");
      box.textContent = cell;
      words.append(box);
    }
  } else if (line.opens) {
    // The siman's own title, which the corpus keeps **inside** the first se'if
    // — see `girsa_app::display::opens_a_siman`. Drawn as its own block above
    // the words instead of running into them, which is what a printed Shulchan
    // Arukh does and what this window did not: siman א of Yoreh De'ah read
    // *"who is fit to shlacht, and it has 14 se'ifim: everyone may shlacht…"*
    // as one sentence.
    //
    // **Wrapped, not moved.** Every run is still here, in order, so the
    // characters a mark, a link or a correction were anchored against are
    // exactly where they were — `spanIn` counts `.line-text` from its start,
    // and lifting the title out of it would shift every offset in the se'if by
    // the length of the title.
    const title = el("span", "line-opens");
    title.append(...line.runs.slice(0, line.opens).map(runElement));
    words.append(title, ...line.runs.slice(line.opens).map(runElement));
  } else {
    words.append(...line.runs.map(runElement));
  }
  row.append(label, words);
  if (line.fixed?.length) row.append(fixMark(line));
  return row;
}

/**
 * The mark on a line a correction touched (spec.md §7.1).
 *
 * Two shapes, because they are two different statements: a scanning error that
 * has been repaired, and a variant that is only noted. A reader has to be able
 * to tell at a glance whether the words in front of them are the printed ones.
 */
function fixMark(line: Line): HTMLElement {
  const fixed = line.fixed ?? [];
  const applied = fixed.filter((f) => f.applied);
  const mark = el("span", applied.length > 0 ? "line-fix" : "line-fix is-noted");
  mark.textContent = applied.length > 0 ? "✓" : "≠";
  const said = fixed
    .map((f) => {
      const claim = f.kind === "ocr" ? say("fixWasFixed") : say("fixKindGirsa");
      const state = f.applied ? "" : say("fixNotApplied");
      const who = f.source ? ` · ${f.source}` : f.who ? ` · ${f.who}` : "";
      return `${claim}${state}: ${f.was} ← ${f.now}${who}`;
    })
    .join("\n");
  mark.title = line.printed
    ? `${said}\n${say("fixAsPrinted")}: ${line.printed}`
    : said;
  return mark;
}

/** One run of words. Built as elements, never as a string of HTML — the text
 * comes out of a file and nothing that came out of a file is put into the
 * document as markup. */
function runElement(run: Run): Node {
  const style = run.style ?? "plain";
  if (style === "break") return document.createElement("br");
  if (style === "plain" && !run.cite) return document.createTextNode(run.text);
  const node = document.createElement("span");
  // A citation in your own writing (W19). A `<span>` and not a `<button>`: this
  // sits inside the flow of a sentence, and a button there breaks the line box,
  // takes tab focus away from the reading pane, and — the one that matters —
  // stops a reader dragging a selection across it to quote the line. `charRange`
  // walks text nodes, so wrapping words in a span leaves highlighting alone.
  if (run.cite) {
    node.classList.add("run-cite");
    node.dataset.cite = run.cite;
    node.title = run.cite;
  }
  if (style === "opening") node.classList.add("run-opening");
  if (style === "quiet") node.classList.add("run-quiet");
  node.textContent = run.text;
  return node;
}

/**
 * A range over characters `from..to` of a line's words.
 *
 * Walked over the text nodes, because a line is runs and `<br>`s rather than
 * one string — the same reason `offsetIn` asks the document rather than adding
 * up lengths.
 */
function charRange(words: HTMLElement, from: number, to: number): Range | null {
  const walker = document.createTreeWalker(words, NodeFilter.SHOW_TEXT);
  const range = document.createRange();
  let seen = 0;
  let started = false;
  let node = walker.nextNode();
  while (node) {
    const length = node.textContent?.length ?? 0;
    if (!started && seen + length >= from) {
      range.setStart(node, from - seen);
      started = true;
    }
    if (started && seen + length >= to) {
      range.setEnd(node, to - seen);
      return range;
    }
    seen += length;
    node = walker.nextNode();
  }
  return started ? range : null;
}

function addressOf(line: HTMLElement): string {
  return line.dataset.address ?? "";
}

/** The `.line` a node sits inside, if it is inside one. */
function lineOf(node: Node): HTMLElement | null {
  const element = node.nodeType === Node.TEXT_NODE ? node.parentElement : (node as HTMLElement);
  return element?.closest<HTMLElement>(".line") ?? null;
}

/**
 * How many characters into a line's words a selection boundary is.
 *
 * Measured by asking the document, rather than by walking the runs and adding
 * up lengths: a range from the start of `.line-text` to the boundary knows
 * about every node between them, including the `<br>` a break run draws, and
 * cannot drift from what is on the screen.
 */
function offsetIn(line: HTMLElement, container: Node, offset: number): number | null {
  const words = line.querySelector<HTMLElement>(".line-text");
  if (!words) return null;
  if (!words.contains(container)) {
    // The boundary is in the margin label, or on the line element itself —
    // which is what a triple-click gives. Either way it means *this end of the
    // line*, and which end is told by where the other one is.
    return container === line && offset > 0 ? (words.textContent?.length ?? 0) : 0;
  }
  const upTo = document.createRange();
  upTo.setStart(words, 0);
  upTo.setEnd(container, offset);
  return upTo.toString().length;
}

function el(tag: string, className: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  return node;
}

/** A segment id carries `:` and `#`, which are CSS selector syntax. */
function cssEscape(value: string): string {
  return CSS.escape(value);
}
