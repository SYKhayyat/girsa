// The links on the line you are standing on, and what you can say about them.
//
// spec.md §8.3, W23. The panel exists because the data is wrong in known ways —
// 40% of it carries no type — and the person who can see that is the one
// reading both texts. So every row **shows its work**: which end, what the
// corpus said, how it was found, how much to believe it, and which of that was
// you.
//
// Nothing here decides anything. Whether a link may be shown as a statement
// about the texts (`curated`) is answered in Rust, because it is a rule about
// evidence and not about a stylesheet.

import { api, type LinkKind, type LinkRow, type Links } from "./api.ts";

/**
 * What a kind of link is called, out of the labelled list Rust sent.
 *
 * There used to be a lookup table here with a `?? kind` fallback, so **a tenth
 * edge type printed an English slug into a Hebrew interface** and nothing said
 * so. `girsa_app::links::kinds` is the list now — twenty lines below where this
 * file already draws the lenses from a labelled list and asks nothing about what
 * a lens is.
 *
 * A key the list does not carry cannot happen: it is built from
 * `EdgeType::ALL`. If it does, the key is shown, which is at least a bug a
 * reader can report.
 */
function said(kinds: LinkKind[], key: string): string {
  return kinds.find((kind) => kind.key === key)?.title ?? key;
}
import { sayTrouble } from "./trouble.ts";
import { button, choice } from "./controls.ts";
import { Latest } from "./latest.ts";
import { say } from "./say.ts";
import { dock, undock, wideAs } from "./dock.ts";

export class LinksView {
  readonly element: HTMLElement;
  private readonly list: HTMLElement;
  private readonly note: HTMLElement;
  private at: string | null = null;
  /** Which lens is on, or none — all of them. */
  private lens: string | null = null;
  /** The highlight the panel was opened on, when it was opened on one. */
  private span: [number, number] | null = null;
  private goTo: ((work: string, at: string) => Promise<void>) | null = null;
  /** Where the reader is standing, for *reanchor to here* and *draw from here*. */
  private here: (() => string | null) | null = null;

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "links";

    const head = document.createElement("header");
    head.className = "links-head";
    const title = document.createElement("span");
    title.className = "links-title";
    title.textContent = say("linksTitle");
    this.note = document.createElement("span");
    this.note.className = "links-note";
    const close = button(say("close"), say("esc"), () => this.close());
    head.append(title, this.note, close);

    this.list = document.createElement("div");
    this.list.className = "links-list";
    this.element.append(head, this.list);
  }

  onOpen(goTo: (work: string, at: string) => Promise<void>): void {
    this.goTo = goTo;
  }

  /** The window says where the reader is, because which pane is focused is the
   * window's business. */
  onHere(here: () => string | null): void {
    this.here = here;
  }

  /** …and where they have highlighted, for pinning a link onto those words
   * (spec.md §8.4). */
  onPinTo(span: () => [number, number] | null): void {
    this.pinTo = span;
  }

  private pinTo: (() => [number, number] | null) | null = null;
  /** One answer at a time: a lens clicked twice used to draw whichever read
   * finished last. See `latest.ts`. */
  private readonly draws = new Latest();

  async toggle(at: string | null, span?: [number, number] | null): Promise<void> {
    if (this.isOpen) {
      this.close();
      return;
    }
    await this.show(at, span);
  }

  async show(at: string | null, span?: [number, number] | null): Promise<void> {
    if (!at) return;
    this.at = at;
    this.span = span ?? null;
  // Docked, not laid over the reading. The reading is made **narrower** and this
  // stands beside it — the same answer the bookcase and the search already give,
  // and the reader's complaint that produced it was about a panel exactly like
  // this one: *"it is weirdly over the text, so i cant see it or the text."*
    this.element.classList.add("is-open");
    dock("links", wideAs("--links-wide"));
    this.note.textContent = say("linksReading");
    this.list.replaceChildren();
    await this.draw();
  }

  close(): void {
    this.element.classList.remove("is-open");
    undock("links");
  }

  private async draw(): Promise<void> {
    const at = this.at;
    if (!at) return;
    await this.draws.attempt(
      () => api.links(at, this.lens ?? undefined, this.span ?? undefined),
      (found) => this.drawFound(found),
      (e) => sayTrouble(this.note, e, "read_links"),
    );
  }

  private drawFound(found: Links): void {
    this.list.replaceChildren();
    const shown = found.links.filter((link) => !link.rejected);
    const words = this.span ? ` ${say("linksOnWords")}` : "";
    this.note.textContent =
      shown.length === 0
        ? `${say("linksNone")}${words}`
        : `${shown.length} ${say("links")}${words}`;
    this.list.append(this.lensRow(found));
    if (found.incoming_unknown) {
      // Two different statements, and a short list says the wrong one.
      //
      // `incoming_unknown` is `!girsa_link::inbound::built(root)` — the
      // **inbound** cache, which `girsa-link-types` writes. This line named
      // `girsa-companions`, which writes `companions.jsonl` and is the shelf's
      // neighbour list: a reader who did as they were told sat through a
      // four-million-edge walk and came back to the same sentence. `search.ts`
      // reports the same cold cache and names it correctly, which is how one of
      // the two was provably wrong without running either.
      const warn = document.createElement("p");
      warn.className = "links-warn";
      warn.textContent = say("linksNoInbound");
      this.list.append(warn);
    }
    this.list.append(...found.links.map((link) => this.row(link, found.types)));
  }

  /** The lenses, as a row of buttons (spec.md §8.5). They are yours: the five
   * that ship are five rows of a file, and this draws whatever is on it. */
  private lensRow(found: Links): HTMLElement {
    const row = document.createElement("div");
    row.className = "lenses";
    const all = this.lensButton(null, say("linksAll"));
    row.append(all);
    for (const lens of found.lenses) row.append(this.lensButton(lens.key, lens.title));
    return row;
  }

  private lensButton(key: string | null, title: string): HTMLElement {
    const button = document.createElement("button");
    button.className = "lens" + (this.lens === key ? " is-on" : "");
    button.textContent = title;
    button.addEventListener("click", () => {
      this.lens = key;
      void this.draw();
    });
    return button;
  }

  private row(link: LinkRow, types: LinkKind[]): HTMLElement {
    const row = document.createElement("div");
    row.className = "link" + (link.rejected ? " is-rejected" : "");
    if (link.mine) row.classList.add("is-mine");

    const kind = document.createElement("span");
    kind.className = "link-kind" + (link.curated ? "" : " is-uncurated");
    kind.textContent = said(types, link.kind);
    kind.title = link.curated ? say("linksCurated") : say("linksUncurated");

    const where = document.createElement("button");
    where.type = "button";
    where.className = "link-where";
    // **The sefer, then the place.** The row used to lead with a bare arrow
    // glyph — `←` for outgoing, `→` for incoming — which is a direction nobody
    // can read without a legend, in a panel the reader described as *"hard on
    // the eyes"*. The direction is a word now, and it is quiet, because the
    // thing a reader wants first is *what does this say*.
    const arrow = document.createElement("span");
    arrow.className = "link-arrow";
    arrow.textContent = link.outgoing ? say("linksOut") : say("linksIn");
    const place = document.createElement("span");
    place.className = "link-place";
    place.textContent = link.said;
    where.append(arrow, place);
    where.title = say("linksOpen");
    where.addEventListener("click", () => void this.goTo?.(link.work, link.at));

    // W37. *"kishuri i cant tell what is going on. it is hard to read."*
    //
    // It was all there and all at once: a badge, an arrow, a citation, a
    // confidence percentage, the method that found it, the corpus's own label in
    // quotes, what the type used to be, what you had changed, who you are — and
    // five controls. Every one of those is worth having and none of them is what
    // a reader wants first, which is **what does this say**.
    const head = document.createElement("div");
    head.className = "link-head";
    head.append(kind, where);
    row.append(head);

    // The first words at the other end, where that sefer is already open — most
    // often the one case that matters, the commentary in the column beside you.
    if (link.preview) {
      const words = document.createElement("p");
      words.className = "link-preview";
      words.textContent = link.preview;
      row.append(words);
    }

    // Its work, and the five repair controls: kept, and out of the way. W23's
    // point is that a link layer has to show its work; one click away is showing
    // it, and in front of the text is not.
    const shown = document.createElement("details");
    shown.className = "link-shown";
    const summary = document.createElement("summary");
    summary.textContent = provenanceSaid(link);
    summary.title = say("linksShowWork");
    const work = document.createElement("span");
    work.className = "link-work";
    work.textContent = provenance(link, types).join(" · ");
    shown.append(summary, work, this.actions(link, types));
    row.append(shown);
    return row;
  }

  private actions(link: LinkRow, types: LinkKind[]): HTMLElement {
    const box = document.createElement("span");
    box.className = "link-actions";

    if (link.rejected) {
      box.append(button(say("linksUnreject"), say("linksUnrejectWhy"), () => this.repair(link, "undo")));
      return box;
    }

    if (!link.confirmed) {
      box.append(
        button(say("linksConfirm"), say("linksConfirmWhy"), () => this.repair(link, "confirm")),
      );
    }
    box.append(button(say("linksReject"), say("linksRejectWhy"), () => this.repair(link, "reject")));

    const retype = choice(say("linksKind"));
    retype.className = "link-retype";
    retype.title = say("linksKindWhy");
    const keep = document.createElement("option");
    keep.textContent = say("linksKindPick");
    keep.value = "";
    retype.append(keep);
    for (const type of types) {
      const option = document.createElement("option");
      option.value = type.key;
      option.textContent = type.title;
      option.selected = type.key === link.kind;
      retype.append(option);
    }
    retype.addEventListener("change", () => {
      if (retype.value) void this.repair(link, "retype", retype.value);
    });
    box.append(retype);

    // Reanchoring: onto the line the reader is standing on, which is the only
    // segment the window can name without asking a second question.
    box.append(
      button(say("linksMoveHere"), say("linksMoveHereWhy"), async () => {
        const here = this.here?.();
        if (!here) return;
        try {
          await api.linkReanchor(link.edge, link.outgoing ? "to" : "from", here);
          await this.draw();
        } catch (e) {
          sayTrouble(this.note, e, "repair_link");
        }
      }),
    );
    // Pinning: onto the words the reader has highlighted right now, which is
    // the only span the window can name without asking a second question.
    box.append(
      button(say("linksPin"), say("linksPinWhy"), async () => {
        const span = this.pinTo?.();
        if (!span || !this.at) {
          this.note.textContent = say("linksPinFirst");
          return;
        }
        try {
          await api.linkPin(link.edge, this.at, span[0], span[1]);
          await this.draw();
        } catch (e) {
          sayTrouble(this.note, e, "repair_link");
        }
      }),
    );
    if (link.changed.length > 0 && !link.mine) {
      box.append(button(say("linksUndo"), say("linksUndoWhy"), () => this.repair(link, "undo")));
    }
    return box;
  }

  private async repair(link: LinkRow, does: string, value?: string): Promise<void> {
    try {
      await api.linkRepair(link.edge, does, value);
      await this.draw();
    } catch (e) {
      sayTrouble(this.note, e, "repair_link");
    }
  }

}

/**
 * Everything the corpus and your layer say about a link (W23), as a list.
 *
 * Behind a disclosure since W37, and unchanged in content: a link layer that
 * cannot show its work is a link layer you have to take on faith.
 */
function provenance(link: LinkRow, kinds: LinkKind[]): string[] {
  const bits = [`${Math.round(link.confidence * 100)}%`, link.method];
  // Which words, and who says so — the dibur hamatchil the commentary itself
  // declares, or a span you pinned (spec.md §8.4).
  if (link.span_from) bits.push(link.span_from === "pinned" ? say("onWordsYours") : say("onWords"));
  if (link.label) bits.push(`"${link.label}"`);
  if (link.was && link.was !== link.kind) bits.push(`${say("wasKind")}: ${said(kinds, link.was)}`);
  if (link.changed.length > 0) bits.push(link.changed.join(", "));
  if (link.who) bits.push(link.who);
  return bits;
}

/**
 * What the closed disclosure says.
 *
 * The two things worth seeing without opening it: whether **you** have touched
 * this link, and how sure the corpus was. A summary that only said `פרטים` would
 * make a reader open all twenty rows to find the one they had rejected.
 */
function provenanceSaid(link: LinkRow): string {
  const mine = link.changed.length > 0 ? `${link.changed.join(", ")} · ` : "";
  return `${mine}${Math.round(link.confidence * 100)}%`;
}
