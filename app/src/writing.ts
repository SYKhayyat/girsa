// A place to write, without leaving the library (spec.md §10.3, W17).
//
// A drawer along the foot of the window rather than a pane: the sefer you are
// writing about has to stay on the screen, or the thing you were going to say
// about it is the thing you go and look up again.
//
// # What this file is allowed to decide
//
// Where the caret is, and which button is pressed. **Not what a source looks
// like** — the markup for a quote comes from Rust, from `girsa-ksav`, the same
// writer Ksav compiles. spec.md §10.3: *lightweight means the UI, not the
// format.* A second renderer here would be two applications producing
// documents that differ depending on which end wrote them.

import { api, isShell, type Presence } from "./api.ts";

/** How long after the last keystroke the buffer is written to disk. */
const SAVE_AFTER_MS = 900;

export class WritingView {
  readonly element: HTMLElement;
  private readonly box: HTMLTextAreaElement;
  private readonly title: HTMLInputElement;
  private readonly note: HTMLElement;
  private readonly ksavButton: HTMLButtonElement;
  private name = "";
  private saving: number | null = null;
  /** Asked for by the window when a source is wanted: the drawer does not know
   * which pane is focused, and should not. */
  private askForSource: (() => Promise<string | null>) | null = null;

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "writing";

    const head = document.createElement("header");
    head.className = "writing-head";

    this.title = document.createElement("input");
    this.title.className = "writing-name";
    this.title.spellcheck = false;
    this.title.addEventListener("change", () => void this.rename());

    this.note = document.createElement("span");
    this.note.className = "writing-note";

    head.append(this.title, this.note);
    head.append(
      this.button("כותרת", "#כותרת1[…]", () => this.wrap("#כותרת1[", "]\n")),
      this.button("ציטוט", "#ציטוט[…]", () => this.wrap("#ציטוט[", "]")),
      this.button("הערה", "#הערת_עורך[…]", () => this.wrap("#הערת_עורך[", "]")),
      this.button("מקור", "הכנס את הבחירה שבספר", () => void this.insertSource()),
    );

    this.ksavButton = this.button("פתח בכסב", "פתח את המסמך בכסב עצמו", () =>
      void this.handOver(),
    );
    head.append(this.ksavButton);
    head.append(this.button("סגור", "Esc", () => this.close()));

    this.box = document.createElement("textarea");
    this.box.className = "writing-box";
    this.box.dir = "rtl";
    this.box.spellcheck = false;
    this.box.addEventListener("input", () => this.scheduleSave());

    this.element.append(head, this.box);
  }

  /** The window tells the drawer how to get a source, because *which pane is
   * focused* is the window's business. */
  onSourceWanted(ask: () => Promise<string | null>): void {
    this.askForSource = ask;
  }

  async toggle(): Promise<void> {
    if (this.isOpen) {
      this.close();
      return;
    }
    await this.show();
  }

  async show(): Promise<void> {
    if (!this.name) {
      // The last one you were writing, or a new one named for today. A buffer
      // called `היום` on three different days is three different documents,
      // which is not what anybody means by "the last one".
      const existing = await api.buffers().catch(() => []);
      this.name = existing[0] ?? today();
    }
    await this.load(this.name);
    this.element.classList.add("is-open");
    this.box.focus();
  }

  close(): void {
    void this.save();
    this.element.classList.remove("is-open");
  }

  /** Whether the "open in Ksav" button is offered — presence, again: never
   * offer what would fail (spec.md §10.6). */
  setKsav(presence: Presence): void {
    const live = presence.state === "live";
    this.ksavButton.hidden = !live;
  }

  private async load(name: string): Promise<void> {
    const writing = await api.bufferOpen(name);
    this.name = writing.name;
    this.title.value = writing.name;
    this.box.value = writing.text;
    this.note.textContent = writing.path;
  }

  private async rename(): Promise<void> {
    const wanted = this.title.value.trim();
    if (!wanted || wanted === this.name) return;
    // Saved under the new name, and the old file stays where it is: a rename
    // that quietly deleted the thing you had been writing is not a rename.
    this.name = wanted;
    await this.save();
    await this.load(wanted);
  }

  private scheduleSave(): void {
    if (this.saving !== null) window.clearTimeout(this.saving);
    this.saving = window.setTimeout(() => void this.save(), SAVE_AFTER_MS);
  }

  private async save(): Promise<void> {
    if (!this.name || !isShell()) return;
    try {
      this.note.textContent = await api.bufferSave(this.name, this.box.value);
      this.note.classList.remove("is-trouble");
    } catch (e) {
      // A buffer that will not save has to say so *while you are writing*, not
      // when you come back tomorrow and find yesterday missing.
      this.note.textContent = String(e);
      this.note.classList.add("is-trouble");
    }
  }

  /** Put the markup around the selection, or at the caret. */
  private wrap(before: string, after: string): void {
    const { selectionStart: from, selectionEnd: to, value } = this.box;
    const inside = value.slice(from, to);
    this.box.value = value.slice(0, from) + before + inside + after + value.slice(to);
    const caret = from + before.length + inside.length;
    this.box.setSelectionRange(caret, caret);
    this.box.focus();
    this.scheduleSave();
  }

  /** The passage you have highlighted in the sefer, as real Ksav markup. */
  private async insertSource(): Promise<void> {
    if (!this.askForSource) return;
    const markup = await this.askForSource();
    if (!markup) {
      this.note.textContent = "לא נבחר כלום בספר";
      return;
    }
    const at = this.box.selectionStart;
    this.box.value = this.box.value.slice(0, at) + markup + this.box.value.slice(at);
    const caret = at + markup.length;
    this.box.setSelectionRange(caret, caret);
    this.box.focus();
    this.scheduleSave();
  }

  /** Hand the document to the real Ksav. */
  private async handOver(): Promise<void> {
    await this.save();
    try {
      await api.bufferToKsav(this.name, this.box.value);
      this.note.textContent = "נמסר לכסב";
      this.note.classList.remove("is-trouble");
    } catch (e) {
      this.note.textContent = String(e);
      this.note.classList.add("is-trouble");
    }
  }

  private button(label: string, title: string, click: () => void): HTMLButtonElement {
    const node = document.createElement("button");
    node.className = "tool";
    node.textContent = label;
    node.title = title;
    node.addEventListener("click", click);
    return node;
  }
}

/** `כ״ח בתמוז` is nicer and needs a calendar; the date the machine knows is
 * enough to tell one day's notes from another's. */
function today(): string {
  const now = new Date();
  const two = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;
}
