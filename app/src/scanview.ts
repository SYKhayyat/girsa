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
import type { Anchor, PageSaid, PaneId, PageWords, Reading, ScanOpen, Scheme } from "./api.ts";
import { glyphsOf } from "./glyphs.ts";
import { sayTrouble } from "./trouble.ts";
import { area, button, choice, field } from "./controls.ts";
import { say } from "./say.ts";

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
  private readonly marks: HTMLElement;
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
    header.append(this.title, this.where, this.note);
    this.element.append(header);

    const bar = el("div", "scan-bar");
    const back = button("‹", say("scanPrev"), () => void this.turn(-1));
    const on = button("›", say("scanNext"), () => void this.turn(1));
    this.pageBox = field(say("scanPageOrDaf"), { className: "scan-page" });
    this.pageBox.type = "text";
    this.pageBox.inputMode = "numeric";
    this.pageBox.title = say("scanPageInFile");
    this.pageBox.addEventListener("change", () => {
      const wanted = Number(this.pageBox.value);
      if (Number.isFinite(wanted)) void this.goTo(wanted);
      else this.paint();
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
    bar.append(on, this.pageBox, back, this.goBox, this.mapButton(), this.readButton(), this.reading);
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
    this.marks = el("div", "scan-marks");
    sheet.append(this.canvas, this.marks);
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
    this.element.querySelector(".pane-head")?.append(control);
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
      this.element.querySelector(".pane-head")?.append(chip);
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
      this.note.textContent = `${asked} אינו בסריקה הזאת`;
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
    // The header is asked for in parallel: it is one small call, and a reader
    // turning pages should not wait for the render to find out where they are.
    const [said, words] = await Promise.all([
      api.scanAt(this.slug, this.page),
      api.scanWords(this.slug, this.page).catch(() => null),
      mine,
    ]);
    this.said = said;
    this.words = words;
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
    this.pageBox.title = `עמוד ${this.page} מתוך ${this.open.pages} בקובץ`;
    this.note.className = "pane-note";

    if (this.said?.display) {
      this.where.textContent = this.said.display;
      this.where.title = this.said.reference ?? "";
      this.note.textContent = "";
      return;
    }
    this.where.textContent = `עמוד ${this.page} בקובץ`;
    this.where.title = this.said?.id ?? "";
    // Two different sentences, and only one of them is a chore the reader can
    // do something about.
    if (this.open.trouble) {
      this.note.textContent = this.open.trouble;
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
    this.marks.replaceChildren();
    this.marks.classList.toggle("is-guessed", this.words?.guessed === true);
    if (!this.words || this.marked.length === 0) return;
    const wanted = new Set(this.marked);
    for (const word of this.words.words) {
      if (!wanted.has(bare(word.text))) continue;
      const box = el("div", "scan-mark");
      box.style.insetInlineStart = "";
      box.style.left = `${word.left * 100}%`;
      box.style.top = `${word.top * 100}%`;
      box.style.width = `${(word.right - word.left) * 100}%`;
      box.style.height = `${(word.bottom - word.top) * 100}%`;
      box.title = word.text;
      this.marks.append(box);
    }
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
      this.reading.textContent = `עמוד ${page} — אין בו טקסט, ואין מנוע OCR מותקן`;
      return false;
    }
    const png = await this.rasterize(page);
    if (!png) return false;
    await api.scanOcrPage(this.slug, page, png.width, png.height, png.bytes);
    return true;
  }

  /** A picture of a page, for an engine to look at. */
  private async rasterize(
    page: number,
  ): Promise<{ bytes: number[]; width: number; height: number } | null> {
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
    return {
      bytes: [...new Uint8Array(await blob.arrayBuffer())],
      width: canvas.width,
      height: canvas.height,
    };
  }

  private sayReading(where: Reading): void {
    if (where.read >= where.pages) {
      this.reading.textContent = where.by.length ? `נקרא — ${where.by.join(", ")}` : "";
      this.reading.classList.remove("is-empty");
      return;
    }
    this.reading.textContent = `${where.read} מתוך ${where.pages} עמודים נקראו`;
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
          said.textContent = `${text}: עוגן נכתב עמוד=דף`;
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
