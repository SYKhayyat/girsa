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

import { api, type LensRow, type LinkKind, type LinkRow, type Links, type Yours } from "./api.ts";

import { sayTrouble } from "./trouble.ts";
import { about, button, choice, shut } from "./controls.ts";
import { Latest } from "./latest.ts";
import { fill, linkKind, say } from "./say.ts";
import { dock, undock, wideAs } from "./dock.ts";

/**
 * What a kind of link is called.
 *
 * There used to be a lookup table here with a `?? kind` fallback, so **a tenth
 * edge type printed an English slug into a Hebrew interface** and nothing said
 * so. Then it was `girsa_app::links::kinds`, which labels every key — in Hebrew
 * only, because when it was written the window spoke one language.
 *
 * `say.ts` owns what the window says in either of them, so it goes first; the
 * Rust list is the fallback, and the key itself is the fallback for that. Three
 * deep sounds like a lot for one word, and each rung answers a different way of
 * being wrong: a language this table has not been translated into, a kind
 * `say.ts` has not heard of, and a kind nothing has.
 */
export function kindSaid(kinds: LinkKind[], key: string): string {
  return linkKind(key) ?? kinds.find((kind) => kind.key === key)?.title ?? key;
}

/** One sefer's links, in the order the panel had them. */
interface Group {
  work: string;
  title: string;
  links: LinkRow[];
}

/** How many rows a sefer may have and still be drawn open. */
const FEW = 4;

/**
 * What one lens keeps, in a sentence.
 *
 * Built from the lens's own four fields rather than from a table of
 * explanations, so a lens the reader edits — and every one of them is editable
 * — describes itself correctly without anybody writing a second sentence about
 * it. A lens that constrains nothing says so, which is a real answer: `שלי` is
 * *only what you have touched* and nothing else.
 */
function lensSays(lens: LensRow): string {
  const parts: string[] = [];
  if (lens.types.length > 0) {
    parts.push(lens.types.map((key) => linkKind(key) ?? key).join(", "));
  }
  if (lens.eras.length > 0) parts.push(`${say("linksLensEras")}: ${lens.eras.join(", ")}`);
  if (lens.mine) parts.push(say("linksLensMine"));
  if (lens.at_least > 0) {
    parts.push(fill("linksLensAtLeast", { n: Math.round(lens.at_least * 100) }));
  }
  return parts.length === 0 ? say("linksAllWhy") : `${say("linksLensKeeps")} ${parts.join(" · ")}`;
}

/**
 * The links, by sefer, in the order each sefer first appeared.
 *
 * Order preserved rather than sorted: the panel already puts the links in an
 * order — the lens, then what the engine returned — and re-sorting by title
 * here would be a second opinion about which sefer matters most, made by a
 * function that knows nothing about the reader.
 */
export function grouped(links: LinkRow[]): Group[] {
  const by = new Map<string, Group>();
  for (const link of links) {
    const found = by.get(link.work);
    if (found) {
      found.links.push(link);
      continue;
    }
    by.set(link.work, { work: link.work, title: link.title, links: [link] });
  }
  return [...by.values()];
}

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
  /** The *stay on this line* control, and whether it is on. */
  private readonly pinned: HTMLButtonElement;
  private stay = false;

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
    // Follow the reader, or stay put. See `follow` below for why this is a
    // control and not simply the behaviour.
    this.pinned = button(say("linksFollowing"), say("linksFollowingWhy"), () => {
      this.stay = !this.stay;
      this.drawPin();
      if (!this.stay) void this.follow(this.here?.() ?? null);
    });
    this.pinned.classList.add("links-pin");
    this.drawPin();
    const close = shut(() => this.close());
    head.append(title, this.note, this.pinned, close);

    this.list = document.createElement("div");
    this.list.className = "links-list";
    this.element.append(head, about(say("linksAbout")), this.list);
  }

  onOpen(goTo: (work: string, at: string) => Promise<void>): void {
    this.goTo = goTo;
  }

  /**
   * The reader moved. Follow them, unless they said not to.
   *
   * > *"Links is based on where you were when you opened it, and does not
   * > change."*
   *
   * It did not, and the reason is in `drawRow`'s own header: *"`from` is the
   * line this panel opened on — it does not follow the reader, which is what
   * makes the gesture possible."* Drawing a link needs two ends, and the two
   * this window can name without asking are *where the panel opened* and *where
   * you are now*. Pinning the panel is what keeps them different.
   *
   * That is a real argument for one gesture, and it was paying for it with the
   * panel's whole ordinary use — a reader scrolls down a daf and the links stay
   * on the line they left. So the pin is a **control** now: off by default,
   * which is what a panel called *the links on this line* has to do, and on
   * when the reader is about to draw one.
   *
   * A no-op when nothing has changed, because a pane reports a position on
   * every scroll frame and each one of these is a read of that work's shards.
   */
  async follow(at: string | null): Promise<void> {
    if (!this.isOpen || this.stay || !at || at === this.at) return;
    this.at = at;
    // A highlight is a narrower question about *these words on this line*, and
    // the line under it has just changed. Keeping it would filter the new
    // line's links by the old line's character offsets.
    this.span = null;
    await this.draw();
  }

  /** What the pin reads, in both of its states. */
  private drawPin(): void {
    this.pinned.textContent = this.stay ? say("linksStaying") : say("linksFollowing");
    this.pinned.title = this.stay ? say("linksStayingWhy") : say("linksFollowingWhy");
    this.pinned.setAttribute("aria-pressed", String(this.stay));
    this.pinned.classList.toggle("is-on", this.stay);
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
      async () => ({
        links: await api.links(at, this.lens ?? undefined, this.span ?? undefined),
        // Which of **your** folders hold this line, which is the one thing
        // `yours` knows that nothing else in the window does: its notes come
        // back from `links` above (that is the argument at the top of
        // `api.ts`'s own-layer section) and its marks from `marksIn`, which is
        // what draws them on the page. A chaburah is a sequence of places, and
        // *is this line in one* had no answer anywhere.
        //
        // Swallowed on failure rather than allowed to blank the links: a strip
        // that could not be drawn is a strip missing, not a panel broken.
        yours: await api.yours(at).catch(() => null),
      }),
      (found) => this.drawFound(found.links, found.yours),
      (e) => sayTrouble(this.note, e, "read_links"),
    );
  }

  private drawFound(found: Links, yours: Yours | null): void {
    this.list.replaceChildren();
    if (yours && yours.folders.length > 0) {
      const strip = document.createElement("p");
      strip.className = "links-folders";
      strip.textContent = `${say("linksInFolders")}: ${yours.folders.join(" · ")}`;
      this.list.append(strip);
    }
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
      // The command, behind the hover. A reader cannot run it from in here and
      // whoever set the library up can — so it is not nothing, and it is not
      // the sentence either.
      warn.title = say("linksNoInboundWhy");
      this.list.append(warn);
    }
    for (const group of grouped(found.links)) {
      this.list.append(this.sefer(group, found.types));
    }
    // **Last, and folded.** It was directly under the lens row, and it is a
    // `<select>` of link kinds: a dropdown at the top of a list of links reads
    // as the control that filters the list, which is what a reader concluded —
    // *"Links does not seem to filter based on the dropdown."* It never did. It
    // says what kind a link **you draw** should be, and it is the one thing in
    // this panel that writes rather than reads, so it goes where a writing
    // control belongs: at the foot, behind its own name.
    const drawer = document.createElement("details");
    drawer.className = "links-drawer";
    const what = document.createElement("summary");
    what.textContent = say("linksDrawOpen");
    drawer.append(what, this.drawRow(found.types));
    this.list.append(drawer);
  }

  /**
   * One sefer's worth of rows, behind a summary that says how many there are.
   *
   * # Why grouping is the fix and not a tidy-up
   *
   * > *"why are there repeats. i just don't get it."*
   *
   * They are not repeats. On one se'if of Yoreh De'ah the Kaf HaChayim writes
   * ס״ק א׳ through ס״ק ע״ח, and the panel drew seventy-eight rows carrying the
   * same eight words and a different number. Nothing was duplicated; the
   * **sefer's name** was printed seventy-eight times down a column, which is
   * what a reader sees and reasonably calls a repeat.
   *
   * So the name is said once, with a count and the range of what it covers, and
   * the rows underneath carry the part that differs. 280 rows from 61 seforim
   * become 61 lines a person can read.
   *
   * Opening a group is what fetches the words — one sefer read, not sixty-one.
   * That is the same argument `LinkRow::preview` made for reading none of them,
   * held to the gesture that makes it affordable.
   */
  private sefer(group: Group, types: LinkKind[]): HTMLElement {
    const box = document.createElement("details");
    box.className = "link-sefer";
    const summary = document.createElement("summary");
    const title = document.createElement("span");
    title.className = "link-sefer-title";
    title.textContent = group.title;
    const count = document.createElement("span");
    count.className = "link-sefer-count";
    count.textContent = String(group.links.length);
    // The range this sefer covers, so a reader can see at a glance that the
    // seventy-eight rows are ס״ק א׳ to ס״ק ע״ח and not the same one repeated.
    const span = document.createElement("span");
    span.className = "link-sefer-span";
    const ends = [group.links[0]?.said, group.links[group.links.length - 1]?.said];
    span.textContent = ends[0] === ends[1] ? (ends[0] ?? "") : `${ends[0]} … ${ends[1]}`;
    summary.append(count, title, span);
    box.append(summary);

    const rows = document.createElement("div");
    rows.className = "link-sefer-rows";
    const drawn = group.links.map((link) => this.row(link, types));
    rows.append(...drawn);
    box.append(rows);

    // A sefer with a handful of rows is not a wall of anything, and making a
    // reader click to see three lines is a click for its own sake.
    if (group.links.length <= FEW) box.open = true;
    let asked = false;
    const fill = () => {
      if (asked || !box.open) return;
      asked = true;
      void this.words(group, drawn);
    };
    box.addEventListener("toggle", fill);
    fill();
    return box;
  }

  /**
   * Fetch what one sefer says at each of these places, and put it on the rows.
   *
   * Failure is silent and the rows keep what they had, which is the sefer and
   * the place. A panel that replaced sixty readable rows with an error because
   * one sefer would not open would be worse than the panel before this.
   */
  private async words(group: Group, rows: HTMLElement[]): Promise<void> {
    let said: Awaited<ReturnType<typeof api.linkWords>>;
    try {
      said = await api.linkWords(group.work, group.links.map((link) => link.at));
    } catch {
      return;
    }
    const by = new Map(said.map((one) => [one.at, one]));
    for (const [nth, link] of group.links.entries()) {
      const words = by.get(link.at);
      const row = rows[nth];
      if (!words || !row) continue;
      const opening = row.querySelector<HTMLElement>(".link-preview");
      if (opening) opening.textContent = words.opening;
      const whole = row.querySelector<HTMLElement>(".link-said");
      if (whole) whole.textContent = words.said;
      row.classList.add("has-words");
    }
  }

  /**
   * Draw a link of your own — the one repair this panel could make in Rust and
   * could not make in the window.
   *
   * `link_draw` has been a command since W23 and `api.linkDraw` has been wired
   * to it since; no view called either, and the comment on `onHere` above said
   * *"for reanchor to here **and draw from here**"* the whole time. A comment
   * defending a behaviour is not evidence that the behaviour exists.
   *
   * **Two ends and one question.** `from` is the line this panel opened on —
   * it does not follow the reader, which is what makes the gesture possible —
   * and `to` is the line they are standing on now. That is the same idiom
   * `linksMoveHere` uses twenty lines down, and for the same reason: those are
   * the only two segments the window can name without asking a second
   * question. Standing where the panel opened means there is no second end, so
   * the row says to move rather than drawing a link from a line to itself.
   */
  private drawRow(types: LinkKind[]): HTMLElement {
    const row = document.createElement("div");
    row.className = "link-draw";

    const kind = choice(say("linksKind"), "link-retype");
    kind.title = say("linksDrawWhy");
    const pick = document.createElement("option");
    pick.textContent = say("linksKindPick");
    pick.value = "";
    kind.append(pick);
    for (const type of types) {
      const option = document.createElement("option");
      option.value = type.key;
      option.textContent = type.title;
      kind.append(option);
    }

    row.append(
      kind,
      button(say("linksDraw"), say("linksDrawWhy"), async () => {
        const from = this.at;
        const to = this.here?.();
        if (!from) return;
        if (!to || to === from) {
          this.note.textContent = say("linksDrawFirst");
          return;
        }
        if (!kind.value) {
          this.note.textContent = say("linksDrawKindFirst");
          return;
        }
        try {
          await api.linkDraw(from, to, kind.value);
          await this.draw();
          this.note.textContent = say("linksDrew");
        } catch (e) {
          sayTrouble(this.note, e, "repair_link");
        }
      }),
    );
    return row;
  }

  /** The lenses, as a row of buttons (spec.md §8.5). They are yours: the five
   * that ship are five rows of a file, and this draws whatever is on it. */
  private lensRow(found: Links): HTMLElement {
    const row = document.createElement("div");
    row.className = "lenses";
    // What the row **is**, said once in front of it. Five bare Hebrew words in
    // a line under a panel heading are five words; a line that begins *show:*
    // is a filter. That is half of *"I also can't tell what the filters are."*
    const what = document.createElement("span");
    what.className = "lenses-what";
    what.textContent = say("linksShow");
    row.append(what, this.lensButton(null, say("linksAll"), say("linksAllWhy")));
    for (const lens of found.lenses) {
      row.append(this.lensButton(lens.key, lens.title, lensSays(lens)));
    }
    return row;
  }

  private lensButton(key: string | null, title: string, why: string): HTMLElement {
    const button = document.createElement("button");
    button.className = "lens" + (this.lens === key ? " is-on" : "");
    button.textContent = title;
    // And the other half: what this one keeps, out of the lens's own
    // definition. A lens is a saved filter and its definition is four fields —
    // nothing here is a guess at what somebody meant by `לומדות`.
    button.title = why;
    button.setAttribute("aria-label", `${title} — ${why}`);
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
    kind.textContent = kindSaid(types, link.kind);
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

    // **The words, on the row.** This used to be drawn only when the other sefer
    // was already open, which is the case where the reader could see the words
    // anyway. The element is always here now and is filled when the group it is
    // in is opened — see `LinksView.sefer` for why that is what makes reading
    // one sefer per gesture affordable.
    const words = document.createElement("p");
    words.className = "link-preview";
    words.textContent = link.preview ?? "";
    row.append(words);

    // And the whole line, one click down. A quote that is cut at ninety
    // characters answers *is this the comment I want*; it does not answer
    // *what does it say*, and the answer to that was five gestures away —
    // open the sefer, find the place, come back.
    const whole = document.createElement("details");
    whole.className = "link-open";
    const open = document.createElement("summary");
    open.textContent = say("linksReadHere");
    open.title = say("linksReadHereWhy");
    const said = document.createElement("p");
    said.className = "link-said";
    whole.append(open, said);
    row.append(whole);

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
  // Which way the arrow points, and whether anybody said so. `undeclared`
  // means the direction is the order of two CSV columns — the one fact that
  // makes a −0.15 confidence dent legible, which is why it is said here and
  // not left inside the number.
  if (link.direction === "undeclared") bits.push(say("linkUndeclared"));
  else if (link.direction === "declared") bits.push(say("linkDeclared"));
  // Which words, and who says so — the dibur hamatchil the commentary itself
  // declares, or a span you pinned (spec.md §8.4).
  if (link.span_from) bits.push(link.span_from === "pinned" ? say("onWordsYours") : say("onWords"));
  if (link.label) bits.push(`"${link.label}"`);
  if (link.was && link.was !== link.kind) bits.push(`${say("wasKind")}: ${kindSaid(kinds, link.was)}`);
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
