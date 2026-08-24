// One column of a scan — the second reading mode (spec.md §6.3, W25).
//
// The scan *is* the daf: the Rashi is in its column and the Tosfos in its,
// exactly as it was set, and there is no typesetting engine anywhere near it.
// So this draws a page of a PDF and nothing else. What page is which daf, what
// the page cites as, and where a daf is printed are all answered in Rust — see
// `girsa-scan` — because they are arithmetic on a declaration and can be
// tested, and a canvas cannot.
//
// # Why pdf.js and not the webview's own PDF viewer
//
// Tauri runs on Edge's engine on Windows and WebKit's on macOS, and both will
// render a PDF in an iframe — with their own toolbar, their own page numbering
// and their own idea of what a page is. That is two behaviours, neither of them
// ours, and neither able to say *which page is on the screen* — which is the
// one thing this pane exists to know. pdf.js is one renderer on both, drawing
// into a canvas we own; it is Apache-2.0, which is one half of this project's
// own licence.

import type { PDFDocumentProxy } from "pdfjs-dist";
import worker from "pdfjs-dist/build/pdf.worker.mjs?url";

import { api, assetUrl } from "./api.ts";
import { alsoCalled, sefer } from "./names.ts";
import type {
  Anchor,
  PageSaid,
  PaneId,
  PageWords,
  Reading,
  ScanMark,
  ScanOpen,
  Scheme,
} from "./api.ts";
import { glyphsOf } from "./glyphs.ts";
import { sayTrouble } from "./trouble.ts";
import { area, button, choice, field, toolStrip } from "./controls.ts";
import { fill, say } from "./say.ts";

/**
 * pdf.js, loaded the first time a scan is opened and not before.
 *
 * It is half a megabyte of renderer and a two-megabyte worker, and most
 * readings of most seforim never touch a PDF at all — so it is **bundled but
 * not started**: no network (the CSP allows scripts from this origin and
 * nowhere else, and Girsa does not go to the network to read — spec.md §14),
 * and no cost to opening a Gemara.
 */
let renderer: Promise<typeof import("pdfjs-dist")> | null = null;

function pdf(): Promise<typeof import("pdfjs-dist")> {
  renderer ??= import("pdfjs-dist").then((module) => {
    module.GlobalWorkerOptions.workerSrc = worker;
    return module;
  });
  return renderer;
}

/** How wide a page is drawn, in device pixels, before the browser scales it. */
const RENDER_WIDTH = 1400;

/** Under this, the engine says so itself, and the box is drawn as a doubt.
 * `girsa-scan` reports Tesseract's per-word confidence as a fraction; 0.8 is
 * where its own documentation stops calling a word reliable. */
const DOUBTFUL = 0.8;

/** 300 dpi against a 72-dpi PDF page: what an OCR engine is trained near, and
 * what `girsa-scan/src/engine.rs` measured tesseract at. */
const OCR_SCALE = 300 / 72;

/** A word without its nikud, for comparing with what a search asked for.
 *
 * The one piece of Hebrew handling in this file, and it is here only because a
 * mark is drawn thirty times a page — the normalizer that matters is
 * `girsa-hebrew`, compiled into both halves of this application. */
function bare(word: string): string {
  return word.replace(/[\u0591-\u05C7]/gu, "");
}

export class ScanView {
  readonly id: PaneId;
  readonly slug: string;
  readonly element: HTMLElement;
  private readonly body: HTMLElement;
  private readonly canvas: HTMLCanvasElement;
  private readonly title: HTMLElement;
  private readonly where: HTMLElement;
  private readonly note: HTMLElement;
  /** The header's buttons, as one box that wraps as one — see `toolStrip`. */
  private readonly tools: HTMLElement;
  private readonly pageBox: HTMLInputElement;
  private readonly goBox: HTMLInputElement;

  private open: ScanOpen | null = null;
  private doc: PDFDocumentProxy | null = null;
  private page = 1;
  private said: PageSaid | null = null;
  /** The words of the page on the screen, for drawing over it (W26). */
  private words: PageWords | null = null;
  /** What is marked: the normalized words a search asked for. */
  private marked: string[] = [];
  /** Set while the reader is correcting the engine's words, which makes every
   * box on the page clickable rather than only what a search asked for. */
  private correcting = false;
  /** The box being typed into, so a second click does not leave two. */
  private fixing: HTMLInputElement | null = null;
  /** Set while the reader is highlighting: every box is clickable, and a click
   * picks the ends of a run rather than opening a correction box. */
  private marking = false;
  /** The first word of a run, once one has been picked. */
  private from: number | null = null;
  /** Your highlights on this page, drawn from the ink they were made on. */
  private yours: ScanMark[] = [];
  private readonly overlay: HTMLElement;
  private readonly reading: HTMLElement;
  /** Set while the read-the-scan job is running; cleared to stop it. */
  private job = false;
  /** The render in flight, so a reader holding the arrow key down does not
   * queue thirty of them onto one canvas. */
  private drawing: Promise<void> = Promise.resolve();
  /** Set while this pane is being turned by the pane it follows, so its own
   * report does not go back and start the two chasing each other — the same
   * guard `PaneView` has for the same reason. */
  private quiet = false;

  constructor(
    id: PaneId,
    slug: string,
    private readonly onMove: (pane: PaneId, at: string) => void,
    private readonly onFocus: (pane: PaneId) => void,
  ) {
    this.id = id;
    this.slug = slug;
    this.element = el("section", "pane is-scan");
    this.element.dataset.pane = String(id);

    const header = el("header", "pane-head");
    this.title = el("span", "pane-title");
    this.where = el("span", "pane-where");
    this.note = el("span", "pane-note");
    // One box, wrapping as one — the same contract `PaneView` has, and for the
    // same reason: a header that runs out of room must not do it by erasing the
    // name of the sefer. See `toolStrip`.
    this.tools = toolStrip();
    header.append(this.title, this.where, this.note, this.tools);
    this.element.append(header);

    const bar = el("div", "scan-bar");
    const back = button("‹", say("scanPrev"), () => void this.turn(-1));
    const on = button("›", say("scanNext"), () => void this.turn(1));
    this.pageBox = field(say("scanPageOrDaf"), { className: "scan-page" });
    this.pageBox.type = "text";
    this.pageBox.inputMode = "numeric";
    this.pageBox.title = say("scanPageInFile");
    this.pageBox.addEventListener("change", () => {
      const asked = this.pageBox.value.trim();
      const wanted = Number(asked);
      // An empty box is a reader who changed their mind, not a reader asking
      // for page 1 — which is where `Number("") === 0` was sending them.
      if (asked === "" || !Number.isFinite(wanted)) this.paint();
      else void this.goTo(wanted);
    });

    this.goBox = field(say("scanGoToDaf"), {
      className: "scan-goto",
      placeholder: say("scanToDaf"),
    });
    this.goBox.title = say("scanGoWhy");
    this.goBox.addEventListener("keydown", (event) => {
      if (event.key === "Enter") void this.goToPlace(this.goBox.value);
    });

    // In right-to-left, the *next* page is to the left. The order here is the
    // order they appear on the screen, and `dir="rtl"` on the pane turns it
    // round — so `‹` sits where a reader's hand expects the next daf.
    this.reading = el("span", "scan-reading");
    bar.append(
      on,
      this.pageBox,
      back,
      this.goBox,
      this.mapButton(),
      this.readButton(),
      this.correctButton(),
      this.markButton(),
      this.reading,
    );
    this.element.append(bar);

    this.body = el("div", "pane-body scan-body");
    this.body.tabIndex = 0;
    // The canvas and the marks are stacked inside one box that is exactly the
    // size of the drawn page, so a rectangle given in fractions of the page can
    // be positioned in percentages and stay right at every zoom. A mark
    // positioned in pixels of the render would sit in the margin the moment the
    // window is resized.
    const sheet = el("div", "scan-sheet");
    this.canvas = document.createElement("canvas");
    this.canvas.className = "scan-canvas";
    this.overlay = el("div", "scan-marks");
    sheet.append(this.canvas, this.overlay);
    this.body.append(sheet);
    this.element.append(this.body);

    this.body.addEventListener("pointerdown", () => this.onFocus(this.id));
    this.body.addEventListener("focus", () => this.onFocus(this.id));
    this.body.addEventListener("keydown", (event) => {
      if (event.key === "ArrowLeft" || event.key === "PageDown") void this.turn(1);
      if (event.key === "ArrowRight" || event.key === "PageUp") void this.turn(-1);
    });
  }

  /** The buttons a pane's header carries, added by the caller that owns what
   * they do — the same contract `PaneView` has. */
  addControl(control: HTMLElement): void {
    this.tools.append(control);
  }

  /** The sefer's Hebrew title, once it has been read. */
  title_he = "";

  /** Open the scan and draw a page. */
  async show(open: ScanOpen, at: number): Promise<void> {
    this.open = open;
    this.title_he = sefer(open.work);
    this.title.textContent = sefer(open.work);
    this.title.title = alsoCalled(open.work);
    this.page = clamp(at, 1, open.pages);

    const url = assetUrl(open.file);
    if (!url) {
      this.note.textContent = say("browserScans");
      return;
    }
    try {
      this.doc = await (await pdf()).getDocument({ url }).promise;
    } catch (e) {
      // A file that has been moved or is not a PDF after all. Said in the
      // reader's words, with the library's own reason one hover away, rather than
      // left as an empty grey rectangle.
      sayTrouble(this.note, e, "open_pdf");
      return;
    }
    await this.draw();
  }

  /** Which page is on the screen — what Ctrl+C copies. */
  here(): number {
    return this.page;
  }

  setFocused(on: boolean): void {
    this.element.classList.toggle("is-focused", on);
  }

  setFollowing(label: string): void {
    let chip = this.element.querySelector<HTMLElement>(".pane-follows");
    if (!chip) {
      chip = el("span", "pane-follows");
      this.tools.before(chip);
    }
    chip.textContent = label;
  }

  /**
   * Turn to the page the pane beside this one says (W25, and W9's rule).
   *
   * `null` is *stay where you are*: either nothing relates the two seforim, or
   * they are the same sefer and this scan does not carry that daf. The second
   * is said out loud — a column that quietly stayed put while the header of the
   * pane beside it moved is a reader looking at the wrong daf.
   */
  turnTo(page: number | null, why: "at" | "no_place" | "unrelated"): void {
    this.quiet = true;
    if (page === null) {
      if (why === "no_place") {
        this.note.textContent = say("nothingHere");
        this.note.title = say("scanNoSuchDaf");
        this.note.classList.add("is-empty");
      }
      this.quiet = false;
      return;
    }
    void this.goTo(page).finally(() => {
      this.quiet = false;
    });
  }

  private async turn(by: number): Promise<void> {
    await this.goTo(this.page + by);
  }

  private async goTo(page: number): Promise<void> {
    if (!this.open) return;
    const wanted = clamp(page, 1, this.open.pages);
    if (wanted === this.page && this.said) {
      this.paint();
      return;
    }
    this.page = wanted;
    await this.draw();
  }

  /** The *go to daf* box: a daf as a reader writes one, or a pasted ref. */
  private async goToPlace(written: string): Promise<void> {
    const asked = written.trim();
    if (!asked) return;
    const page = await api.scanPageOf(this.slug, asked);
    if (page === null) {
      // Never the nearest page it does have — see `girsa_scan::Paging`.
      this.note.textContent = fill("scanNotInThis", { asked });
      this.note.classList.add("is-empty");
      return;
    }
    this.goBox.value = "";
    await this.goTo(page);
  }

  private async draw(): Promise<void> {
    const mine = this.drawing.then(async () => {
      if (!this.doc) return;
      const page = await this.doc.getPage(this.page);
      const unscaled = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale: RENDER_WIDTH / unscaled.width });
      this.canvas.width = Math.floor(viewport.width);
      this.canvas.height = Math.floor(viewport.height);
      const context = this.canvas.getContext("2d");
      if (!context) return;
      await page.render({ canvas: this.canvas, canvasContext: context, viewport }).promise;
      this.body.scrollTop = 0;
    });
    this.drawing = mine.catch(() => undefined);
    // The page this draw is *for*. The render is serialized through
    // `this.drawing`, but these three fetches are not — and a page turn that
    // happens while they are in flight owns the screen: the older answer must
    // neither paint its rectangles over the newer photograph nor report its
    // segment id to the panes following this one, which is how followers used
    // to be dragged backwards. The page number is the ticket; `goTo` moves it
    // before any stale answer can come back.
    const forPage = this.page;
    // The header is asked for in parallel: it is one small call, and a reader
    // turning pages should not wait for the render to find out where they are.
    const [said, words, yours] = await Promise.all([
      api.scanAt(this.slug, forPage),
      api.scanWords(this.slug, forPage).catch(() => null),
      // Your own layer, which a browser build does not have — an empty list
      // rather than a failure, the answer every other own-layer call gives.
      api.scanMarks(this.slug, forPage).catch((): ScanMark[] => []),
      mine,
    ]);
    if (forPage !== this.page) return;
    this.said = said;
    this.words = words;
    this.yours = yours;
    this.paint();
    this.drawMarks();
    // Where the reader is, so the scan reopens on the page they left it on and
    // so a pane following this one is told. The id is the page's permanent id,
    // which is what every other pane in this window reports.
    if (!this.quiet && said.id) this.onMove(this.id, said.id);
  }

  /** The header: what is printed on this page, or that nothing is. */
  private paint(): void {
    if (!this.open) return;
    this.pageBox.value = String(this.page);
    this.pageBox.title = fill("scanPageOfFile", { page: this.page, pages: this.open.pages });
    this.note.className = "pane-note";

    if (this.said?.display) {
      this.where.textContent = this.said.display;
      this.where.title = this.said.reference ?? "";
      this.note.textContent = "";
      return;
    }
    this.where.textContent = fill("scanPageNumbered", { page: this.page });
    this.where.title = this.said?.id ?? "";
    // Two different sentences, and only one of them is a chore the reader can
    // do something about.
    if (this.open.trouble) {
      // Read, not printed. It arrives as `ShelfError`'s English `Display` —
      // the same shape as the wall of paths in finding 19, on a smaller
      // surface: a sentence composed for a log, put in front of a reader
      // because the field happened to be a string.
      sayTrouble(this.note, this.open.trouble, "read_page");
    } else if (this.open.paged) {
      this.note.textContent = say("scanNoDafHere");
      this.note.title = say("scanNothingPrinted");
    } else {
      this.note.textContent = say("scanUnmapped");
      this.note.title = say("scanSayOnce");
    }
    this.note.classList.add("is-empty");
  }

  /**
   * Mark the words a search asked for, on the photograph (spec.md §6.3).
   *
   * The words are handed in already normalized by Rust, and matched against a
   * page's words the same way — a mark worked out here by searching the
   * rendered text for the typed string would find nothing on a menukad page,
   * which is most of them.
   */
  markWords(words: string[]): void {
    this.marked = words;
    this.drawMarks();
  }

  /**
   * The rectangles, in percentages of the page.
   *
   * Nothing is drawn for a page nobody has read — an empty overlay, not a
   * guess at where the word would be if it were there.
   */
  private drawMarks(): void {
    this.overlay.replaceChildren();
    this.overlay.classList.toggle("is-guessed", this.words?.guessed === true);
    // One class for both modes, because both make every box on the page
    // clickable and the overlay has to stop being transparent to the pointer
    // for either. What a click *does* is the difference, and that is in the
    // handler rather than in the stylesheet.
    this.overlay.classList.toggle("is-correcting", this.correcting || this.marking);
    // Your highlights, drawn from the rectangles they were made on and not from
    // any offset — so they are in the right place on a page that has been read
    // again since, which is the whole reason the ink is what was written down.
    for (const mark of this.yours) {
      for (const ink of mark.ink) {
        const box = el("div", "scan-yours");
        box.style.insetInlineStart = "";
        box.style.left = `${ink.left * 100}%`;
        box.style.top = `${ink.top * 100}%`;
        box.style.width = `${(ink.right - ink.left) * 100}%`;
        box.style.height = `${(ink.bottom - ink.top) * 100}%`;
        if (mark.colour) box.style.setProperty("--yours", mark.colour);
        // What it was made on, and what is under it now. The same sentence
        // where the page has not been re-read, and the difference is the thing
        // worth seeing where it has.
        box.title =
          mark.says && mark.says !== mark.was
            ? fill("scanMarkMoved", { was: mark.was, says: mark.says })
            : mark.label ?? mark.was;
        this.overlay.append(box);
      }
    }
    if (!this.words) return;
    // Correcting draws **every** word, because the reader is looking for the
    // one the engine got wrong and cannot ask for it by name — asking for it by
    // name is the thing they cannot do. Otherwise only what a search asked for.
    const wanted = this.correcting || this.marking ? null : new Set(this.marked);
    if (wanted && wanted.size === 0) return;
    this.words.words.forEach((word, index) => {
      if (wanted && !wanted.has(bare(word.text))) return;
      const box = el("div", "scan-mark");
      // Which word this box is, named rather than implied. The highlight boxes
      // above share this overlay, so "the nth child" was never the nth word the
      // moment a page had any of yours on it — and `correctWord` used to index
      // children anyway, showing the reader the wrong rectangle while prefilling
      // the right text.
      box.dataset.word = String(index);
      box.style.insetInlineStart = "";
      box.style.left = `${word.left * 100}%`;
      box.style.top = `${word.top * 100}%`;
      box.style.width = `${(word.right - word.left) * 100}%`;
      box.style.height = `${(word.bottom - word.top) * 100}%`;
      box.title = word.text;
      if (this.correcting) {
        // The engine's own doubt, drawn. A page is a thousand rectangles and a
        // reader hunting the wrong one is better served by being told where the
        // machine was least sure than by being left to compare all of them.
        if (word.confidence < DOUBTFUL) box.classList.add("is-doubtful");
        if (this.marking) {
          box.classList.toggle("is-picked", this.from === index);
          box.addEventListener("click", () => void this.pick(index));
        } else {
          box.addEventListener("click", () => this.correct(index, word.text, box));
        }
      }
      this.overlay.append(box);
    });
  }

  /**
   * Correct one word by its ink (W21), which is the only correction a
   * photograph can take.
   *
   * A reading pane corrects characters of text: `api.fix` takes offsets into
   * the line, and a reader highlights the word and types. There is no text over
   * a photograph to highlight, so `scan_fix` takes the word's **index on the
   * page** instead, and the correction is stored against the box the engine
   * drew — which is why it survives the page being read again by something
   * better, where an offset into a re-OCR'd line would not.
   *
   * The command has existed since W21 and `api.scanFix` has been wired to it
   * since; **no view called either**, so a reader looking at a word the engine
   * plainly got wrong had nothing to click. This is that click.
   */
  /**
   * Open the correction box on one word because the OCR queue sent us here
   * (W21 meeting W26).
   *
   * The queue has always ranked what tesseract got wrong — the index's term
   * dictionary holds a page's words like any other segment's — and the row was
   * a dead end anyway, because opening it went through the reading pane and a
   * scan does not have one. This is the other door: turn to the page, put every
   * box on the screen, and open the same field the reader would have got by
   * clicking that word themselves.
   *
   * `done` is run only when a correction was actually saved, which is what
   * keeps *fixed* in the queue honest: a reader who looks at the word, decides
   * the engine was right and presses Escape has decided nothing, and the
   * candidate is still waiting for them.
   */
  async correctWord(page: number, index: number, done: () => Promise<void>): Promise<void> {
    await this.goTo(page);
    this.correcting = true;
    this.drawMarks();
    // The box is found by the word's own ordinal, named on it at draw time —
    // not by its position among the overlay's children, which the highlight
    // boxes ahead of it had already made a lie.
    const box = this.overlay.querySelector<HTMLElement>(`[data-word="${index}"]`);
    const word = this.words?.words[index];
    if (!(box instanceof HTMLElement) || !word) {
      // The page has been read again since the queue was built and there is no
      // longer a word there. Said out loud rather than opening an empty box.
      this.note.textContent = say("scanSuspectNotHere");
      this.note.classList.add("is-empty");
      return;
    }
    box.classList.add("is-suspect");
    box.scrollIntoView({ block: "center", behavior: "auto" });
    this.correct(index, word.text, box, done);
  }

  private correct(
    index: number,
    was: string,
    box: HTMLElement,
    after?: () => Promise<void>,
  ): void {
    if (this.fixing) this.fixing.remove();
    // In the overlay, over the word, and not in a `window.prompt` — a browser
    // dialog is a different application's furniture and it cannot show the ink
    // the reader is correcting against.
    const typed = field(say("scanFixWord"), { className: "scan-fix" });
    typed.value = was;
    this.fixing = typed;
    const at = box.getBoundingClientRect();
    const page = this.overlay.getBoundingClientRect();
    typed.style.left = `${((at.left - page.left) / page.width) * 100}%`;
    typed.style.top = `${((at.bottom - page.top) / page.height) * 100}%`;
    typed.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        this.stopCorrecting();
        return;
      }
      if (event.key !== "Enter") return;
      event.preventDefault();
      const says = typed.value.trim();
      if (!says || says === was) {
        this.stopCorrecting();
        return;
      }
      void (async () => {
        try {
          this.words = await api.scanFix(this.slug, this.page, index, says);
          this.stopCorrecting();
          this.drawMarks();
          await after?.();
        } catch (e) {
          sayTrouble(this.note, e, "fix");
        }
      })();
    });
    this.overlay.append(typed);
    typed.focus();
    typed.select();
  }

  /**
   * Pick the ends of a run, one click each.
   *
   * Two clicks rather than a drag, and it is not a shortcut. There is no text
   * over a photograph to select, so a drag would have to hit-test its own path
   * across the boxes and guess at what a diagonal across two columns of a daf
   * means — and on a page laid out in two columns the guess is wrong often
   * enough to matter. Two clicks say exactly which words, and the first one
   * stays lit until the second lands.
   *
   * Clicking the same word twice marks that one word, which is the common case
   * and needs no special path: a run of one.
   */
  private async pick(index: number): Promise<void> {
    if (this.from === null) {
      this.from = index;
      this.drawMarks();
      return;
    }
    const [from, to] = this.from <= index ? [this.from, index] : [index, this.from];
    this.from = null;
    try {
      this.yours = await api.scanMark(this.slug, this.page, from, to);
      this.drawMarks();
    } catch (e) {
      sayTrouble(this.note, e, "mark");
    }
  }

  /** *Highlight words* — the toggle that makes a click pick instead of correct. */
  private markButton(): HTMLElement {
    return button(say("scanMark"), say("scanMarkWhy"), () => {
      this.marking = !this.marking;
      // The two modes are exclusive. Both make every box clickable and a click
      // cannot mean two things, so turning one on turns the other off rather
      // than leaving a reader to find out which one they are in by clicking.
      if (this.marking) this.correcting = false;
      this.from = null;
      this.stopCorrecting();
      this.drawMarks();
    });
  }

  private stopCorrecting(): void {
    this.fixing?.remove();
    this.fixing = null;
  }

  /** *Correct a word* — the toggle that makes every box on the page clickable. */
  private correctButton(): HTMLElement {
    return button(say("scanFix"), say("scanFixWhy"), () => {
      this.correcting = !this.correcting;
      if (this.correcting) this.marking = false;
      this.from = null;
      this.stopCorrecting();
      this.drawMarks();
    });
  }

  /**
   * *Read this scan* — spec.md §6.3's optional, background, resumable job.
   *
   * Off until it is asked for, one page at a time, and it stops the moment it
   * is pressed again. Between pages it yields to the window, so a reader can
   * turn the page, search, and copy a mekor while it runs: **never blocking
   * reading** is a shape, not a promise, and the shape is a loop that owns
   * nothing between iterations.
   *
   * What is left to do is not tracked here. It is the pages already written
   * down, asked of Rust each time round — so closing the window mid-job costs
   * the page it was on and nothing else.
   */
  private readButton(): HTMLElement {
    return button(say("scanRead"), say("scanReadWhy"), () => {
      if (this.job) {
        this.job = false;
        return;
      }
      void this.read();
    });
  }

  private async read(): Promise<void> {
    if (!this.doc) return;
    this.job = true;
    try {
      for (;;) {
        if (!this.job) break;
        const where = await api.scanReading(this.slug);
        this.sayReading(where);
        if (where.next === null) break;
        const done = await this.readOne(where.next, where.engine !== null);
        if (!done) break;
        // Back to the window between pages. Without this the loop holds the
        // one thread the webview has and the reader cannot turn a page.
        await new Promise((wake) => setTimeout(wake, 0));
      }
      this.sayReading(await api.scanReading(this.slug));
    } catch (e) {
      sayTrouble(this.reading, e, "read_page");
    } finally {
      this.job = false;
    }
  }

  /** One page: what the file says, or failing that what the picture shows. */
  private async readOne(page: number, hasEngine: boolean): Promise<boolean> {
    if (!this.doc) return false;
    const glyphs = await glyphsOf(this.doc, page);
    if (glyphs) {
      await api.scanReadPage(this.slug, page, glyphs.width, glyphs.height, glyphs.glyphs);
      return true;
    }
    // No text of its own. That is a page for an engine — and if there is none
    // installed the job stops here and says so, rather than marking the page
    // read and leaving a hole nothing will ever come back to.
    if (!hasEngine) {
      this.reading.textContent = fill("scanNoTextNoEngine", { page });
      return false;
    }
    const png = await this.rasterize(page);
    if (!png) return false;
    await api.scanOcrPage(this.slug, page, png.width, png.height, png.base64);
    return true;
  }

  /** A picture of a page, for an engine to look at.
   *
   * The bytes leave as **base64**, not as an array of numbers: spreading a
   * multi-megabyte PNG into JS numbers serialized millions of boxed values
   * over the IPC inside the job that promises never to block reading. */
  private async rasterize(
    page: number,
  ): Promise<{ base64: string; width: number; height: number } | null> {
    if (!this.doc) return null;
    const it = await this.doc.getPage(page);
    // 300 dpi against a 72-dpi page, which is what the evaluation in
    // `girsa-scan/src/engine.rs` measured at and what every OCR engine is
    // trained near. Rendering at what the screen happens to be would make the
    // reading depend on the window size.
    const viewport = it.getViewport({ scale: OCR_SCALE });
    const canvas = document.createElement("canvas");
    canvas.width = Math.floor(viewport.width);
    canvas.height = Math.floor(viewport.height);
    const context = canvas.getContext("2d");
    if (!context) return null;
    await it.render({ canvas, canvasContext: context, viewport }).promise;
    const blob = await new Promise<Blob | null>((give) => canvas.toBlob(give, "image/png"));
    if (!blob) return null;
    const bytes = new Uint8Array(await blob.arrayBuffer());
    let binary = "";
    // In chunks: `String.fromCharCode(...bytes)` has an argument-count ceiling
    // and this is exactly where a whole archive-sized page hits it.
    for (let at = 0; at < bytes.length; at += 0x8000) {
      binary += String.fromCharCode(...bytes.subarray(at, at + 0x8000));
    }
    return {
      base64: btoa(binary),
      width: canvas.width,
      height: canvas.height,
    };
  }

  private sayReading(where: Reading): void {
    if (where.read >= where.pages) {
      this.reading.textContent = where.by.length
        ? fill("scanReadBy", { by: where.by.join(", ") })
        : "";
      this.reading.classList.remove("is-empty");
      return;
    }
    this.reading.textContent = fill("scanPagesRead", { read: where.read, pages: where.pages });
    this.reading.classList.add("is-empty");
  }

  /** *Say which page is which daf* — the once-per-sefer chore. */
  private mapButton(): HTMLElement {
    return button(say("scanPages"), say("scanPagesWhy"), () => this.mapper());
  }

  /**
   * The chore, as a small form on the pane.
   *
   * Nothing here validates anything: an anchor that is not the kind of place
   * the scheme counts, or one that would put two pages on one daf, is refused
   * by `girsa-scan` with a message saying which — and that message is shown
   * verbatim. A second opinion in TypeScript is a second opinion that can
   * disagree with the one the mekoros are printed from.
   */
  private mapper(): void {
    if (!this.open) return;
    const old = this.element.querySelector(".scan-map");
    if (old) {
      old.remove();
      return;
    }

    const box = el("div", "scan-map");
    const scheme = choice(say("scanScheme"));
    scheme.className = "scan-scheme";
    for (const [value, label] of [
      ["amud", say("scanSchemeAmud")],
      ["daf", say("scanSchemeDaf")],
      ["numbered", say("scanSchemeNumbered")],
    ] as [Scheme, string][]) {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = label;
      option.selected = this.open.scheme === value;
      scheme.append(option);
    }

    const anchors = area(say("scanAnchors"));
    anchors.className = "scan-anchors";
    anchors.rows = 4;
    anchors.spellcheck = false;
    anchors.value =
      this.open.anchors.map((a) => `${a.page}=${a.at ?? "-"}`).join("\n") ||
      `${this.page}=`;
    anchors.title = say("scanAnchorsWhy");

    const of = field(say("scanOfWhich"));
    of.className = "scan-of";
    of.type = "text";
    of.placeholder = say("scanOfWhichHint");
    of.title = say("scanOfWhichWhy");
    of.value = this.open.of ?? "";

    const said = el("p", "scan-said");
    const save = button(say("save"), say("scanSaveMapping"), () => {
      const rows: Anchor[] = [];
      for (const line of anchors.value.split("\n")) {
        const text = line.trim();
        if (!text) continue;
        const [page, at] = splitOnce(text, "=");
        const n = Number(page);
        if (!Number.isFinite(n)) {
          said.textContent = fill("scanAnchorShape", { text });
          return;
        }
        rows.push(at.trim() === "-" ? { page: n } : { page: n, at: at.trim() });
      }
      void this.saveMap(scheme.value as Scheme, rows, of.value.trim() || null, said, box);
    });
    const forget = button(say("scanForget"), say("scanForgetWhy"), () => {
      void api
        .scanForget(this.slug)
        .then(async (open) => {
          this.open = open;
          box.remove();
          await this.draw();
        })
        .catch((e: unknown) => {
          sayTrouble(said, e, "read_page");
        });
    });

    box.append(scheme, anchors, of, save, forget, said);
    this.element.append(box);
    anchors.focus();
  }

  private async saveMap(
    scheme: Scheme,
    anchors: Anchor[],
    of: string | null,
    said: HTMLElement,
    box: HTMLElement,
  ): Promise<void> {
    try {
      this.open = await api.scanMap(this.slug, scheme, anchors, of);
      box.remove();
      await this.draw();
    } catch (e) {
      // The refusal `girsa-scan` made, in the reader's words. `girsa-scan`'s own
      // wording is on the hover, because it is written for a log.
      sayTrouble(said, e, "read_page");
    }
  }
}

function splitOnce(text: string, on: string): [string, string] {
  const at = text.indexOf(on);
  return at < 0 ? [text, ""] : [text.slice(0, at), text.slice(at + on.length)];
}

function clamp(n: number, low: number, high: number): number {
  return Math.min(Math.max(Math.round(n), low), Math.max(low, high));
}

function el(tag: string, className: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  return node;
}

// The fourth copy of this lived here. It is `controls::button` now — one
// implementation of the thing every screen is made of, and it takes the name as an
// argument rather than hoping the caller sets one (B14).
