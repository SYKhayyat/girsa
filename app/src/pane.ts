// One column of reading.
//
// A pane holds a sefer, renders a window of it, and reports where the reader
// is. It does not decide where anything goes — it is told, by `main.ts`, which
// asks Rust.

import { api } from "./api.ts";
import type { FixMark, Line, MarkRow, PaneId, Place, Relation, Run, Said, Text } from "./api.ts";
import { alsoCalled, sefer } from "./names.ts";
import { say } from "./say.ts";

/** How many lines are put on the page at once, and how many more at an edge. */
const WINDOW = 400;
const STEP = 300;

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
 */
export class PaneView {
  readonly id: PaneId;
  readonly slug: string;
  readonly element: HTMLElement;
  private readonly body: HTMLElement;
  private readonly note: HTMLElement;
  private readonly title: HTMLElement;
  private readonly where: HTMLElement;
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
  /** The lines a **ticked** mefaresh speaks on (W43). Rust's answer, not this
   * one's: which lines those are is a fact about the link graph. */
  private marked = new Set<string>();
  /** How many mefarshim are ticked on this sefer. Nothing ticked means a click
   * on a line is just a click — the reader has not asked for anything, so the
   * pane must not start answering. */
  private ticked = 0;
  private onComments: ((at: string) => void) | null = null;
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
    header.append(this.title, this.where, this.note);
    this.element.append(header);

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

  /**
   * A click on a line, in the one case where it means something.
   *
   * Silent unless the reader has ticked at least one mefaresh. That is not
   * timidity — a pane where every click makes something appear is a pane you
   * cannot click, and the reader who has ticked nobody has not asked a question.
   */
  private clicked(event: MouseEvent): void {
    if (this.ticked === 0 || !this.onComments) return;
    // A reader dragging across words is quoting, not asking.
    if (window.getSelection()?.isCollapsed === false) return;
    const target = event.target;
    if (!(target instanceof Node)) return;
    // Clicks inside an open comment belong to the comment.
    if (target instanceof Element && target.closest(".line-said")) return;
    const line = lineOf(target);
    const id = line?.dataset.id;
    if (id) this.onComments(id);
  }

  /** The buttons a pane's header carries. Added by the caller, which owns what
   * they do. */
  addControl(control: HTMLElement): void {
    this.element.querySelector(".pane-head")?.append(control);
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
   */
  setMefarshim(marked: string[], ticked: number): void {
    this.marked = new Set(marked);
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
    row.after(box);
  }

  private paint(): void {
    this.markMefarshim();
    for (const mark of this.marks) {
      if (!mark.span) continue;
      const line = this.body.querySelector<HTMLElement>(`[data-id="${cssEscape(mark.at)}"]`);
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

  /** The mark itself: a class on the line, so the CSS owns what it looks like. */
  private markMefarshim(): void {
    for (const row of this.body.querySelectorAll<HTMLElement>(".line")) {
      const on = this.marked.has(row.dataset.id ?? "");
      row.classList.toggle("has-mefarshim", on);
      if (on && !row.title) row.title = say("markWhy");
    }
  }

  private extend(where: "up" | "down"): void {
    if (!this.text) return;
    if (where === "down" && this.to < this.lines.length) {
      const next = Math.min(this.lines.length, this.to + STEP);
      if (!this.has(this.to, next)) {
        // Not here yet. Fetch it and come back — one waiter, however many
        // scroll events arrived while it was in flight.
        if (this.extending) return;
        this.extending = true;
        void this.load(this.to, next - this.to).then(() => {
          this.extending = false;
          this.extend("down");
        });
        return;
      }
      this.body.append(...this.drawn(this.to, next));
      this.to = next;
      this.paint();
    } else if (where === "up" && this.from > 0) {
      const next = Math.max(0, this.from - STEP);
      if (!this.has(next, this.from)) {
        if (this.extending) return;
        this.extending = true;
        void this.load(next, this.from - next).then(() => {
          this.extending = false;
          this.extend("up");
        });
        return;
      }
      const before = this.body.scrollHeight;
      this.body.prepend(...this.drawn(next, this.from));
      this.from = next;
      this.paint();
      // Adding lines above moves everything down; put the reader back on the
      // words they were looking at.
      this.body.scrollTop += this.body.scrollHeight - before;
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
      this.element.querySelector(".pane-head")?.append(chip);
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
  mark.title = line.printed ? `${said}\nכפי שנדפס: ${line.printed}` : said;
  return mark;
}

/** One run of words. Built as elements, never as a string of HTML — the text
 * comes out of a file and nothing that came out of a file is put into the
 * document as markup. */
function runElement(run: Run): Node {
  const style = run.style ?? "plain";
  if (style === "break") return document.createElement("br");
  if (style === "plain") return document.createTextNode(run.text);
  const node = document.createElement("span");
  node.className = style === "opening" ? "run-opening" : "run-quiet";
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
