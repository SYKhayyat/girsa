// Your own layer, in a drawer (spec.md §11, W27).
//
// **What is not in here matters more than what is.** There is no "notes on
// this line" list, because a note's connection to a sugya is a link like any
// other and the links panel already draws it — beside Rashi, sorted by the
// same rule, marked שלי. Building a second panel for it would be the first
// place the claim stopped being true.
//
// What is here is what a note, a mark, a saved query and a chaburah folder
// have in common: they are yours, they are files, and they are a list.

import {
  api,
  type FolderRow,
  type MarkRow,
  type NoteRow,
  type PatchRow,
  type QueryRow,
  type TagRow,
} from "./api.ts";
import { sayTrouble } from "./trouble.ts";
import { area, glyph } from "./controls.ts";
import { fill, say } from "./say.ts";
import { dock, undock, wideAs } from "./dock.ts";

type Panel = "notes" | "marks" | "queries" | "folders" | "tags" | "fixes";

const PANELS: [Panel, string][] = [
  ["notes", say("yoursNotes")],
  ["marks", say("yoursMarks")],
  ["queries", say("yoursQueries")],
  ["folders", say("yoursFolders")],
  ["tags", say("yoursTags")],
  // Sixth, and the last of the five things your layer holds to get a list. A
  // correction *is* yours — it is a patch in your own layer, `unfix` takes it
  // back — and until this tab the only place one could be seen was the line it
  // was made on, which means a correction made yesterday in a sefer you have
  // since closed was findable by remembering where you were.
  ["fixes", say("yoursFixes")],
];

export class YoursView {
  readonly element: HTMLElement;
  private readonly tabs: HTMLElement;
  private readonly list: HTMLElement;
  private readonly note: HTMLElement;
  private panel: Panel = "notes";
  private goTo: ((work: string, at: string) => Promise<void>) | null = null;
  private ask: ((typed: string) => Promise<void>) | null = null;
  private changed: (() => Promise<void>) | null = null;
  /** Which note is open for editing, if any. */
  private editing: string | null = null;
  /**
   * The tag being followed, if any (W27).
   *
   * Beside `panel` rather than a seventh value of it, because a tag is not a
   * kind of thing you own — it is a word that crosses all four kinds at once.
   * Made a panel, it would have had to pick one drawer to filter, and the
   * whole use of a tag is that the note, the highlight, the saved question and
   * the folder you put `ברכות` on are the answer together.
   */
  private tag: string | null = null;

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "yours";

    const head = document.createElement("header");
    head.className = "yours-head";
    const title = document.createElement("span");
    title.className = "yours-title";
    title.textContent = say("mine");
    this.note = document.createElement("span");
    this.note.className = "yours-note";

    const out = document.createElement("button");
    out.className = "tool";
    out.textContent = say("yoursExport");
    out.title = say("yoursExportWhy");
    out.addEventListener("click", () => void this.exportLayer());

    const close = document.createElement("button");
    close.className = "tool";
    close.textContent = say("close");
    close.title = say("esc");
    close.addEventListener("click", () => this.close());
    head.append(title, this.note, out, close);

    this.tabs = document.createElement("div");
    this.tabs.className = "yours-tabs";
    for (const [key, said] of PANELS) {
      const button = document.createElement("button");
      button.className = "lens";
      button.dataset.panel = key;
      button.textContent = said;
      button.addEventListener("click", () => {
        this.panel = key;
        this.editing = null;
        // A tab is how you get out of a tag. There is no separate way back,
        // because a reader who has finished with `ברכות` wants a drawer, not
        // an empty one with the filter still on.
        this.tag = null;
        void this.draw();
      });
      this.tabs.append(button);
    }

    this.list = document.createElement("div");
    this.list.className = "yours-list";
    this.element.append(head, this.tabs, this.list);
  }

  /** The window says where to go when a row is clicked. */
  onOpen(goTo: (work: string, at: string) => Promise<void>): void {
    this.goTo = goTo;
  }

  /** …and how to ask a question again. */
  onAsk(ask: (typed: string) => Promise<void>): void {
    this.ask = ask;
  }

  /** …and what to do when your layer changed under the panes. */
  onChanged(changed: () => Promise<void>): void {
    this.changed = changed;
  }

  async toggle(panel?: Panel): Promise<void> {
    if (this.isOpen && (!panel || panel === this.panel)) {
      this.close();
      return;
    }
    if (panel) this.panel = panel;
  // Docked, not laid over the reading. The reading is made **narrower** and this
  // stands beside it — the same answer the bookcase and the search already give,
  // and the reader's complaint that produced it was about a panel exactly like
  // this one: *"it is weirdly over the text, so i cant see it or the text."*
    this.element.classList.add("is-open");
    dock("yours", wideAs("--yours-wide"));
    await this.draw();
  }

  close(): void {
    this.element.classList.remove("is-open");
    this.editing = null;
    undock("yours");
  }

  /** Redraw, because something of yours changed elsewhere in the window. */
  async refresh(): Promise<void> {
    if (this.isOpen) await this.draw();
  }

  private async draw(): Promise<void> {
    for (const button of this.tabs.querySelectorAll<HTMLElement>(".lens")) {
      button.classList.toggle("is-on", button.dataset.panel === this.panel);
    }
    this.note.textContent = say("linksReading");
    this.list.replaceChildren();
    try {
      if (this.tag !== null) return await this.drawTagged(this.tag);
      switch (this.panel) {
        case "notes":
          return await this.drawNotes();
        case "marks":
          return await this.drawMarks();
        case "queries":
          return await this.drawQueries();
        case "folders":
          return await this.drawFolders();
        case "tags":
          return await this.drawTags();
        case "fixes":
          return await this.drawFixes();
      }
    } catch (e) {
      sayTrouble(this.note, e);
    }
  }

  private async drawNotes(): Promise<void> {
    const notes = await api.notes();
    this.note.textContent = notes.length === 0 ? say("yoursNothingWritten") : `${notes.length} ${say("countNotes")}`;
    for (const note of notes) {
      this.list.append(await this.noteRow(note));
    }
  }

  private async noteRow(note: NoteRow): Promise<HTMLElement> {
    const row = document.createElement("div");
    row.className = "yours-row";

    const title = document.createElement("button");
    title.className = "yours-where";
    title.textContent = note.title;
    title.title = say("yoursOpenAsSefer");
    title.addEventListener("click", () => void this.goTo?.(note.slug, ""));

    const opening = document.createElement("span");
    opening.className = "yours-said";
    opening.textContent = note.opening;

    const about = document.createElement("span");
    about.className = "yours-about";
    about.textContent = fill("yoursNoteAbout", {
      paragraphs: note.paragraphs,
      places: note.on.length,
    });

    const edit = document.createElement("button");
    edit.className = "tool";
    edit.textContent = this.editing === note.name ? say("close") : say("yoursEdit");
    edit.addEventListener("click", () => {
      this.editing = this.editing === note.name ? null : note.name;
      void this.draw();
    });

    const forget = document.createElement("button");
    forget.className = "tool";
    forget.textContent = say("yoursDelete");
    forget.title = say("yoursForgetNoteWhy");
    forget.addEventListener("click", () => {
      void (async () => {
        await api.noteForget(note.name);
        await this.changed?.();
        await this.draw();
      })();
    });

    row.append(title, opening, about, edit, forget);
    for (const tag of note.tags) row.append(chip(tag, (t) => void this.pickTag(t)));

    if (this.editing === note.name) {
      row.append(await this.editor(note));
    }
    return row;
  }

  /**
   * A note, paragraph by paragraph.
   *
   * **One box per paragraph, each carrying its own id**, because that is what
   * the ids are for: the window hands back *which paragraph* changed rather
   * than a wall of text to be re-split, so nothing is ever matched up by
   * position and no anchor moves. A single textarea over the whole note would
   * have quietly re-derived every id from where the newlines fell — T1, in
   * your own writing.
   */
  private async editor(note: NoteRow): Promise<HTMLElement> {
    const box = document.createElement("div");
    box.className = "yours-editor";
    const paras = await api.noteRead(note.name);
    for (const para of paras) {
      const line = document.createElement("div");
      line.className = "yours-para";

      const id = document.createElement("span");
      id.className = "yours-id";
      id.textContent = para.id.split("#")[1] ?? "";
      id.title = para.id;

      const words = area(say("yoursParagraph"), { className: "yours-words", value: para.text });
      words.rows = Math.max(2, Math.ceil(para.text.length / 60));
      words.addEventListener("blur", () => {
        if (words.value === para.text) return;
        void api.noteEdit(note.name, "set", para.id, words.value);
      });

      const after = glyph("+", say("yoursNewParagraph"), () => {
        void (async () => {
          await api.noteEdit(note.name, "after", para.id, "");
          await this.draw();
        })();
      });
      after.classList.add("tool");

      const drop = glyph("−", say("yoursDropParagraph"), () => {
        void (async () => {
          await api.noteEdit(note.name, "remove", para.id);
          await this.draw();
        })();
      });

      line.append(id, words, after, drop);
      box.append(line);
    }

    const add = document.createElement("button");
    add.className = "tool";
    add.textContent = say("yoursParagraphAtEnd");
    add.addEventListener("click", () => {
      void (async () => {
        await api.noteEdit(note.name, "append", undefined, "");
        await this.draw();
      })();
    });
    box.append(add);

    for (const at of note.on) {
      const where = document.createElement("button");
      where.className = "yours-where";
      where.textContent = at;
      where.title = say("linksOpen");
      where.addEventListener("click", () => void this.goTo?.(workOf(at), at));
      box.append(where);
    }
    return box;
  }

  private async drawMarks(): Promise<void> {
    const marks = await api.bookmarks();
    this.note.textContent =
      marks.length === 0 ? say("yoursNoMarks") : `${marks.length} ${say("countMarks")}`;
    for (const mark of marks) this.list.append(this.markRow(mark));
  }

  private markRow(mark: MarkRow): HTMLElement {
    const row = document.createElement("div");
    row.className = "yours-row" + (mark.stale ? " is-stale" : "");

    const where = document.createElement("button");
    where.className = "yours-where";
    where.textContent = mark.label ?? mark.was ?? mark.at;
    where.title = mark.at;
    where.addEventListener("click", () => void this.goTo?.(workOf(mark.at), mark.at));

    const said = document.createElement("span");
    said.className = "yours-said";
    // Three different sentences, and a mark that quietly said nothing would be
    // a highlight the reader thinks is still on the words it was made on.
    said.textContent = mark.stale
      ? say("yoursStale")
      : mark.moved
        ? say("yoursMoved")
        : mark.kind === "bookmark"
          ? say("bookmark")
          : mark.was;

    const forget = document.createElement("button");
    forget.className = "tool";
    forget.textContent = say("yoursDelete");
    forget.addEventListener("click", () => {
      void (async () => {
        await api.markForget(mark.id);
        await this.changed?.();
        await this.draw();
      })();
    });

    row.append(where, said, forget);
    for (const tag of mark.tags) row.append(chip(tag, (t) => void this.pickTag(t)));
    return row;
  }

  private async drawQueries(): Promise<void> {
    const queries = await api.queries();
    this.note.textContent =
      queries.length === 0 ? say("yoursNoQueries") : `${queries.length} ${say("countQueries")}`;
    for (const query of queries) this.list.append(this.queryRow(query));
  }

  private queryRow(query: QueryRow): HTMLElement {
    const row = document.createElement("div");
    row.className = "yours-row";

    const again = document.createElement("button");
    again.className = "yours-where";
    again.textContent = query.name;
    again.title = say("yoursAskAgain");
    again.addEventListener("click", () => {
      void (async () => {
        // The chips and the scope are set back in Rust; what comes back is the
        // line for the box. The window does not reconstruct a search.
        const typed = await api.queryRecall(query.name);
        await this.ask?.(typed);
      })();
    });

    const said = document.createElement("span");
    said.className = "yours-said";
    said.textContent = query.said;

    const forget = document.createElement("button");
    forget.className = "tool";
    forget.textContent = say("yoursDelete");
    forget.addEventListener("click", () => {
      void (async () => {
        await api.queryForget(query.name);
        await this.draw();
      })();
    });

    row.append(again, said, forget);
    for (const tag of query.tags) row.append(chip(tag, (t) => void this.pickTag(t)));
    return row;
  }

  private async drawFolders(): Promise<void> {
    const folders = await api.folders();
    this.note.textContent = folders.length === 0 ? say("yoursNoFolders") : `${folders.length} ${say("countFolders")}`;
    for (const folder of folders) this.list.append(this.folderRow(folder));
  }

  private folderRow(folder: FolderRow): HTMLElement {
    const row = document.createElement("div");
    row.className = "yours-row is-folder";

    const title = document.createElement("span");
    title.className = "yours-where";
    title.textContent = folder.title || folder.name;

    const count = document.createElement("span");
    count.className = "yours-about";
    count.textContent = `${folder.members.length}`;

    const forget = document.createElement("button");
    forget.className = "tool";
    forget.textContent = say("yoursDelete");
    forget.title = say("yoursForgetFolderWhy");
    forget.addEventListener("click", () => {
      void (async () => {
        await api.folderForget(folder.name);
        await this.draw();
      })();
    });
    row.append(title, count, forget);

    // In the order they were put in. A chaburah is a sequence, so this list is
    // never sorted.
    for (const member of folder.members) {
      const line = document.createElement("div");
      line.className = "yours-member";
      const open = document.createElement("button");
      open.className = "yours-where";
      open.textContent = member.said;
      if (member.work) {
        open.addEventListener("click", () => void this.goTo?.(member.work ?? "", member.at ?? ""));
      } else {
        open.disabled = true;
      }
      const out = document.createElement("button");
      out.className = "tool";
      out.textContent = say("yoursRemove");
      out.addEventListener("click", () => {
        void (async () => {
          await api.folderEdit(folder.name, "take-out", member.key);
          await this.draw();
        })();
      });
      line.append(open, out);
      row.append(line);
    }
    // A folder carries tags like the other three, and had no chips — so the one
    // kind of thing you own that is *already* a grouping was the one you could
    // not reach a tag from.
    for (const tag of folder.tags) row.append(chip(tag, (t) => void this.pickTag(t)));
    return row;
  }

  private async drawTags(): Promise<void> {
    const tags = await api.tags();
    this.note.textContent = tags.length === 0 ? say("yoursNoTags") : `${tags.length} ${say("countTags")}`;
    for (const tag of tags) this.list.append(this.tagRow(tag));
  }

  /**
   * Follow a tag: everything of yours carrying it, all four kinds at once.
   *
   * Filtered here rather than in Rust. Your layer is the small one — notes,
   * marks, saved questions and folders are four files a person wrote by hand,
   * not five million segments — and a `tagged` command would be a fifth answer
   * to *what carries this tag* beside the four lists that already exist, which
   * is how the counts in the tags drawer and the rows under them start
   * disagreeing.
   *
   * The four are asked for together rather than in turn: they are four
   * independent reads of four files, and doing them one after another is three
   * round trips of waiting for no reason.
   */
  private async drawTagged(tag: string): Promise<void> {
    const [notes, marks, queries, folders] = await Promise.all([
      api.notes(),
      api.bookmarks(),
      api.queries(),
      api.folders(),
    ]);
    const carries = (tags: string[]) => tags.includes(tag);

    this.list.append(this.tagHead(tag));
    const mine = {
      notes: notes.filter((row) => carries(row.tags)),
      marks: marks.filter((row) => carries(row.tags)),
      queries: queries.filter((row) => carries(row.tags)),
      folders: folders.filter((row) => carries(row.tags)),
    };
    const found =
      mine.notes.length + mine.marks.length + mine.queries.length + mine.folders.length;
    this.note.textContent = fill("yoursTagged", { tag });

    // A tag the tally still counts and nothing carries is a layer that changed
    // under an open drawer — said out loud, because an empty list under a tag
    // you just clicked otherwise reads as a click that failed.
    if (found === 0) {
      const none = document.createElement("p");
      none.className = "yours-note";
      none.textContent = say("yoursTagNothing");
      this.list.append(none);
      return;
    }
    // In the order the tabs are in, so a reader who knows the drawer knows
    // where to look inside the answer.
    for (const note of mine.notes) this.list.append(await this.noteRow(note));
    for (const mark of mine.marks) this.list.append(this.markRow(mark));
    for (const query of mine.queries) this.list.append(this.queryRow(query));
    for (const folder of mine.folders) this.list.append(this.folderRow(folder));
  }

  /** The tag you are standing in, and the way out of it. */
  private tagHead(tag: string): HTMLElement {
    const row = document.createElement("div");
    row.className = "yours-row is-tagged";
    const name = document.createElement("span");
    name.className = "yours-where";
    name.textContent = tag;
    const clear = document.createElement("button");
    clear.className = "tool";
    clear.textContent = say("yoursTagClear");
    clear.addEventListener("click", () => {
      this.tag = null;
      void this.draw();
    });
    row.append(name, clear);
    return row;
  }

  /** Stand in a tag. */
  private async pickTag(tag: string): Promise<void> {
    this.tag = tag;
    this.editing = null;
    await this.draw();
  }

  private tagRow(tag: TagRow): HTMLElement {
    const row = document.createElement("div");
    row.className = "yours-row";
    const name = document.createElement("button");
    name.className = "yours-where";
    name.textContent = tag.tag;
    name.title = say("yoursTagPick");
    name.addEventListener("click", () => void this.pickTag(tag.tag));
    const count = document.createElement("span");
    count.className = "yours-about";
    // What a tag is on, not only how many: a tag on one note and a tag on
    // forty highlights are different things.
    //
    // The list comes from Rust, kinds and Hebrew plurals and all. It was four
    // ternaries with four nouns typed here, which made a fifth taggable thing —
    // a scan, a link repair — an edit to this file.
    count.textContent = tag.carried
      .map((carried) => `${carried.count} ${carried.said}`)
      .join(" · ");
    row.append(name, count);
    return row;
  }

  /**
   * The corrections you have made (W20), which had no list until now.
   *
   * `api.fixes` was wired to a live `fixes` command and **no view called it**,
   * which the second sitting flagged as a backend feature with no door. The
   * door belongs here rather than in a panel of its own: a patch is a file in
   * your layer, it is undone by `unfix` the way a mark is forgotten by
   * `markForget`, and `PatchRow` already carries the sefer's title in the
   * window's language and its address — a shape built to be listed.
   */
  private async drawFixes(): Promise<void> {
    const fixes = await api.fixes();
    this.note.textContent =
      fixes.length === 0 ? say("yoursNoFixes") : `${fixes.length} ${say("countFixes")}`;
    for (const fix of fixes) this.list.append(this.fixRow(fix));
  }

  private fixRow(fix: PatchRow): HTMLElement {
    const row = document.createElement("div");
    row.className = "yours-row";

    const where = document.createElement("button");
    where.className = "yours-where";
    // The sefer and the place, in the reader's language — the same two things
    // a search row and a link row lead with, and for the same reason: nobody
    // recognises a correction by its segment id.
    where.textContent = `${fix.title} ${fix.address}`;
    where.title = fix.segment;
    where.addEventListener("click", () => void this.goTo?.(fix.work, fix.segment));

    // What it said and what it says. A row that showed only the correction
    // would be a claim with its evidence left out, which is the thing W20 is
    // about: a corrected text you cannot see the correction in is a text
    // somebody has quietly edited.
    const said = document.createElement("span");
    said.className = "yours-said";
    said.textContent = `${fix.was} ← ${fix.now}`;
    said.title = fix.kind === "ocr" ? say("fixed") : say("variantNoted");

    const forget = document.createElement("button");
    forget.className = "tool";
    forget.textContent = say("yoursDelete");
    forget.addEventListener("click", () => {
      void (async () => {
        try {
          await api.unfix(fix.segment, fix.id);
          // The panes are showing the corrected text, so they are wrong until
          // they are redrawn — the same reason forgetting a mark calls this.
          await this.changed?.();
          await this.draw();
        } catch (e) {
          sayTrouble(this.note, e, "fix");
        }
      })();
    });

    row.append(where, said, forget);
    return row;
  }

  private async exportLayer(): Promise<void> {
    try {
      this.note.textContent = await api.exportLayer();
    } catch (e) {
      sayTrouble(this.note, e, "write_note");
    }
  }
}

/** A tag, as a way in (W27). A button and not a span: it does something now, and
 * an affordance that does something has to look like one. */
function chip(tag: string, pick: (tag: string) => void): HTMLElement {
  const el = document.createElement("button");
  el.className = "yours-tag";
  el.textContent = tag;
  el.title = say("yoursTagPick");
  el.addEventListener("click", () => pick(tag));
  return el;
}

/** The sefer a segment id names — everything before the last `/`. */
function workOf(id: string): string {
  const body = id.replace(/^girsa:/, "").split("#")[0];
  const cut = body.lastIndexOf("/");
  return cut < 0 ? body : body.slice(0, cut);
}
