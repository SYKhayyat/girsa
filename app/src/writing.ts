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

import { api, isShell, pickFolder, type Presence } from "./api.ts";

import { clearTrouble, sayTrouble } from "./trouble.ts";
import { area, button, field } from "./controls.ts";
import { fill, ksavAs, say } from "./say.ts";

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

    this.title = field(say("documentName"));
    this.title.className = "writing-name";
    this.title.spellcheck = false;
    this.title.addEventListener("change", () => void this.rename());

    this.note = document.createElement("span");
    this.note.className = "writing-note";

    head.append(this.title, this.note);
    head.append(
      button(say("heading1"), "#כותרת1[…]", () => this.wrap("#כותרת1[", "]\n")),
      button(say("quote"), "#ציטוט[…]", () => this.wrap("#ציטוט[", "]")),
      button(say("editorNote"), "#הערת_עורך[…]", () => this.wrap("#הערת_עורך[", "]")),
      button(say("insertSource"), say("insertSourceWhy"), () => void this.insertSource()),
    );

    // > *"send to ksav and export dont let you pick a folder."*
    //
    // The working buffer lives in `personal/ksav/`, which is what `buffers()`
    // lists and what makes *the last thing you were writing* findable; a buffer
    // that wandered off to wherever a dialog last pointed would be a document
    // the drawer could no longer offer. So this writes a **copy** where it is
    // asked to, which is what *save a copy* means everywhere else.
    head.append(button(say("saveACopy"), say("saveACopyWhy"), () => void this.saveACopy()));

    this.ksavButton = button(
      fill("writingOpenInKsav", { ksav: ksavAs("ב") }),
      fill("writingOpenInKsavWhy", { ksav: ksavAs("ב") }),
      () =>
      void this.handOver(),
    );
    head.append(this.ksavButton);
    head.append(button(say("close"), say("esc"), () => this.close()));

    // A placeholder, and a box that looks like a sheet — finding 10. The
    // drawer opened as 1360 × 306 of `rgba(0,0,0,0)` with no border and no
    // placeholder, which on a dark theme is a black rectangle: no frame, no
    // caret until you click, nothing saying this is where you type. The
    // stylesheet gives it paper and a rule; this gives it the sentence.
    this.box = area(say("writingBox"), {
      className: "writing-box",
      dir: "rtl",
      placeholder: say("writingHint"),
    });
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

  /** Write what is in the box to disk **now**, and wait for it.
   *
   * For the one thing that takes the window down under it: changing the
   * interface language reloads (see `SettingsView.onInterfaceChanged`), and
   * nothing the reader has typed may go with it. `save()` is otherwise called
   * on a 900 ms timer, which is exactly long enough to lose a sentence. */
  async flush(): Promise<void> {
    if (this.saving !== null) window.clearTimeout(this.saving);
    this.saving = null;
    await this.save();
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
    // Where the file is, on the hover. It used to be the drawer's only status
    // line — a grey absolute path, in a panel whose problem was that it told
    // the reader nothing they wanted to know.
    this.note.textContent = "";
    this.note.title = writing.path;
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
      this.note.title = await api.bufferSave(this.name, this.box.value);
      this.note.textContent = say("writingSaved");
      clearTrouble(this.note);
    } catch (e) {
      // A buffer that will not save has to say so *while you are writing*, not
      // when you come back tomorrow and find yesterday missing.
      sayTrouble(this.note, e, "write_note");
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
      this.note.textContent = say("nothingChosen");
      return;
    }
    const at = this.box.selectionStart;
    this.box.value = this.box.value.slice(0, at) + markup + this.box.value.slice(at);
    const caret = at + markup.length;
    this.box.setSelectionRange(caret, caret);
    this.box.focus();
    this.scheduleSave();
  }

  /** Write a copy of the document into a folder the reader chooses. */
  private async saveACopy(): Promise<void> {
    await this.save();
    const into = await pickFolder(say("chooseFolder"));
    if (into === null) return;
    try {
      const path = await api.bufferWriteTo(this.name, this.box.value, into);
      this.note.textContent = `${say("wrote")} — ${path}`;
      clearTrouble(this.note);
    } catch (e) {
      sayTrouble(this.note, e, "write_note");
    }
  }

  /** Hand the document to the real Ksav. */
  private async handOver(): Promise<void> {
    await this.save();
    try {
      await api.bufferToKsav(this.name, this.box.value);
      this.note.textContent = fill("writingHandedOver", { ksav: ksavAs("ל") });
      clearTrouble(this.note);
    } catch (e) {
      sayTrouble(this.note, e, "send_to_ksav");
    }
  }

}

/** `כ״ח בתמוז` is nicer and needs a calendar; the date the machine knows is
 * enough to tell one day's notes from another's. */
function today(): string {
  const now = new Date();
  const two = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;
}
