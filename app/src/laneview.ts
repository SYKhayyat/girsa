// The semantic lane, drawn (spec.md §9.9, BUILDER.md W30).
//
// Two things live here: the **panel**, where the lane is turned on, pointed at a
// model and told what to embed; and the **column**, which is how an adjacent
// result reaches a reader.
//
// Nothing here decides anything. Which state the lane is in, what its coverage
// sentence says, whether a result set is adjacent — all of it is answered in
// `girsa-lane` and composed in Rust, and this file draws it and sends the clicks
// back. That division matters more here than anywhere else in the window,
// because the one sentence a reader has to be able to trust — *what is in this
// index and what is not* — must have exactly one author.
//
// Three rules the drawing keeps, each of them §9.9 in a line of CSS or a line of
// text:
//
//   · **adjacent, always.** The column carries the label on every draw, in the
//     wording Rust hands over, and it is never in the same list as a literal
//     hit. There is no code path in this file that appends a `Near` to a `Hit`.
//   · **coverage, always.** Found, empty, refused or off — the sentence is
//     drawn. A partial lane that reads as a complete one is the defect §9.9
//     exists to prevent.
//   · **off is not a mode that returns nothing.** With the lane off the column
//     is not drawn at all, and the panel says what turning it on would need.

import {
  api,
  pickFolder,
  whenLaneWorks,
  type LaneAnswer,
  type LaneProgress,
  type LaneState,
} from "./api.ts";
import { field } from "./controls.ts";
import { clearTrouble, sayTrouble, trouble } from "./trouble.ts";
import { fill, say } from "./say.ts";
import { dock, undock, wideAs } from "./dock.ts";

/** Open a sefer at a segment — the same handler the search list is given. */
type Opened = (slug: string, id: string | null, marked?: string[]) => void;

/** `742923190` → `708 MB`. The number a reader is agreeing to spend. */
function megabytes(bytes: number): string {
  return `${Math.round(bytes / 1_048_576)} MB`;
}

/**
 * The adjacent column, under the literal results.
 *
 * Drawn from a `LaneAnswer` and nothing else, so there is no way for it to show
 * a row without the label and the coverage sentence above it.
 */
export class LaneColumn {
  readonly element: HTMLElement;
  private opened: Opened = () => {};

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "lane";
    this.element.hidden = true;
  }

  /** Ask the lane about what was typed, and draw whatever comes back.
   *
   * Called after the literal search has already drawn: the lane is slower —
   * it runs a model over the query — and the literal results must never wait
   * on it. If the lane is off, this draws nothing and takes no time. */
  async show(typed: string, opened: Opened): Promise<void> {
    this.opened = opened;
    const state = await api.laneState().catch(() => null);
    if (!state || state.state === "off") {
      this.element.hidden = true;
      return;
    }
    this.element.hidden = false;
    this.element.replaceChildren(spinner());
    const answer = await api.laneAsk(typed).catch(() => null);
    if (!answer) {
      this.element.hidden = true;
      return;
    }
    this.draw(answer, state);
  }

  hide(): void {
    this.element.hidden = true;
  }

  private draw(answer: LaneAnswer, state: LaneState): void {
    const box = document.createElement("div");
    box.className = "lane-box";

    // The label, first and every time. This is the whole of *drawn as adjacent,
    // always* — the sentence comes from `girsa_lane::ADJACENT` so the window,
    // the CLI and the MCP surface cannot word it three ways.
    const label = document.createElement("p");
    label.className = "lane-label";
    label.textContent = answer.label;
    box.append(label);

    // What the model was measured to do, and over how many se'ifim. Drawn
    // whether or not anything was found, and before the coverage line, because
    // they are two different admissions: this one is about what the lane is
    // known to be bad at, and the next is about how much of the shelf it has
    // seen. Both sentences are composed in Rust.
    const measured = document.createElement("p");
    measured.className = "lane-measured";
    measured.textContent = answer.measured;
    // And the specific one, where it applies. `measured` says the lane is poor
    // at questions under *every* answer, which is where to start and not where
    // to stop: a reader who has just typed one is reading a general caveat with
    // ten plausible-looking rows under it. Composed in Rust like the other
    // four, so the window, the terminal and an agent cannot word it three ways.
    if (answer.asking) {
      const asking = document.createElement("p");
      asking.className = "lane-asking";
      asking.textContent = answer.asking;
      box.append(asking);
    }
    box.append(measured);

    // What is in the index and what is not. Drawn whether or not anything was
    // found, because it is a statement about the answer and not a suggestion.
    const coverage = document.createElement("p");
    coverage.className = "lane-coverage";
    coverage.textContent = answer.coverage;
    box.append(coverage);

    // How the ranking was got. Third of the three admissions, and the only one
    // that is about the *retrieval* rather than about the corpus: above a size
    // the lane ranks from a shortlist of signatures instead of reading every
    // vector, which is fast and is not the same answer. Drawn only when true —
    // a permanent disclaimer is one nobody reads.
    if (answer.shortlisted) {
      const shortlisted = document.createElement("p");
      shortlisted.className = "lane-shortlisted";
      shortlisted.textContent = answer.shortlisted;
      box.append(shortlisted);
    }

    if (state.state === "adrift" && state.said) {
      const adrift = document.createElement("p");
      adrift.className = "lane-adrift";
      adrift.textContent = state.said;
      box.append(adrift);
    }

    if (answer.refused) {
      const why = document.createElement("p");
      why.className = "lane-refused";
      why.textContent = answer.refused;
      box.append(why);
    } else if (answer.near.length === 0) {
      const none = document.createElement("p");
      none.className = "lane-refused";
      none.textContent = say("laneNothingNear");
      box.append(none);
    }

    for (const near of answer.near) {
      const row = document.createElement("button");
      row.className = "lane-hit";
      const where = document.createElement("span");
      where.className = "lane-where";
      where.textContent = `${near.title} ${near.address}`;
      // The cosine, shown. A reader deciding whether to follow one of these is
      // entitled to know how near it actually was — and the numbers cluster, so
      // hiding them would make a 0.64 look like a 0.83.
      const how = document.createElement("span");
      how.className = "lane-near";
      how.textContent = near.nearness.toFixed(2);
      const text = document.createElement("p");
      text.className = "lane-text";
      text.textContent = near.text;
      row.append(where, how, text);
      row.addEventListener("click", () => this.opened(near.work, near.id));
      box.append(row);
    }

    this.element.replaceChildren(box);
  }
}

function spinner(): HTMLElement {
  const line = document.createElement("p");
  line.className = "lane-coverage";
  line.textContent = "…";
  return line;
}

/**
 * The panel: turn it on, point it at a model, choose what goes in it.
 *
 * The order of the controls is the order of the decisions, and the fetch button
 * is **last and hidden** until a reader turns fetching on — spec.md §14 says
 * Girsa never *needs* the network, and the default path down this panel never
 * touches it.
 */
export class LanePanel {
  readonly element: HTMLElement;
  private readonly body: HTMLElement;
  private readonly progress: HTMLElement;
  private open = false;
  private state: LaneState | null = null;
  /** Which sefer is open in the pane behind the panel, so *add this one* has
   * something to add. Set by the window. */
  private here: { slug: string; title: string } | null = null;
  private working = false;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "lane-panel";
    this.element.hidden = true;

    const sheet = document.createElement("div");
    sheet.className = "lane-sheet";

    const bar = document.createElement("div");
    bar.className = "lane-bar";
    const title = document.createElement("p");
    title.className = "lane-title";
    title.textContent = say("laneTitle");
    const close = document.createElement("button");
    close.className = "tool";
    close.textContent = say("close");
    close.addEventListener("click", () => this.close());
    bar.append(title, close);

    this.body = document.createElement("div");
    this.body.className = "lane-body";
    this.progress = document.createElement("p");
    this.progress.className = "lane-progress";
    this.progress.hidden = true;

    sheet.append(bar, this.body, this.progress);
    this.element.append(sheet);
    this.element.addEventListener("pointerdown", (event) => {
      if (event.target === this.element) this.close();
    });

    void whenLaneWorks((progress) => this.told(progress));
  }

  get isOpen(): boolean {
    return this.open;
  }

  /** What the pane behind the panel is holding, so it can be added by name. */
  standing(here: { slug: string; title: string } | null): void {
    this.here = here;
  }

  async show(): Promise<void> {
    this.open = true;
    this.element.hidden = false;
    // Docked, not laid over the reading. This is **the** panel the reader named:
    // *"i opened halashon smucha and it is weirdly over the text, so i cant see
    // it or the text."* Docking answered that complaint for the bookcase and the
    // search and left this one a scrim with a card floating in the middle of it.
    dock("lane", wideAs("--lane-wide"));
    await this.refresh();
  }

  close(): void {
    this.open = false;
    this.element.hidden = true;
    undock("lane");
  }

  async toggle(): Promise<void> {
    if (this.open) this.close();
    else await this.show();
  }

  private async refresh(): Promise<void> {
    this.state = await api.laneState().catch(() => null);
    this.draw();
  }

  private told(progress: LaneProgress): void {
    if (progress.doing === "done") {
      this.working = false;
      this.progress.hidden = false;
      // The download's own failure — a network error, a disk error — which is
      // somebody else's `Display` and belongs on the hover rather than in the
      // line. Third of the three fields on the wire that were being printed
      // rather than read; finding 19 was the first.
      if (progress.trouble) {
        sayTrouble(this.progress, progress.trouble, "general");
      } else {
        clearTrouble(this.progress);
        this.progress.textContent = say("laneDone");
      }
      void this.refresh();
      return;
    }
    this.working = true;
    this.progress.hidden = false;
    // Megabytes while a model is coming down, a plain count while it is being
    // read. Two rows in the table rather than one with a fragment spliced on
    // the end, because *3 of 8* and *3 מתוך 8* do not put the numbers in the
    // same places, and a trailing ` מתוך {of}` can only ever suit one of them.
    const bringing = progress.doing === "bring";
    const done = bringing ? megabytes(progress.done) : String(progress.done);
    this.progress.textContent =
      progress.of > 0
        ? fill("laneProgressOf", {
            what: progress.what,
            done,
            of: bringing ? megabytes(progress.of) : progress.of,
          })
        : fill("laneProgress", { what: progress.what, done });
  }

  private draw(): void {
    this.body.replaceChildren();
    if (!this.state) {
      const none = document.createElement("p");
      none.className = "lane-refused";
      none.textContent = say("laneNoShelf");
      this.body.append(none);
      return;
    }
    const state = this.state;

    // 1 · What it is for, and what it is not. Two sentences, because a reader
    // meeting this panel for the first time will otherwise try it as a search
    // box — and it was measured to be poor at questions and good at a line you
    // half remember (see `girsa_lane::model`).
    const what = document.createElement("p");
    what.className = "lane-what";
    what.textContent =
      say("laneAbout") +
      say("laneNotSearch");
    this.body.append(what);

    // 2 · On or off. One control, and off is the default.
    const onoff = document.createElement("button");
    onoff.className = "tool lane-onoff";
    onoff.textContent = state.state === "off" ? say("laneOn") : say("laneOff");
    onoff.addEventListener("click", async () => {
      const next = await api.laneSet(state.state === "off").catch((e) => {
        this.trouble(e);
        return null;
      });
      if (next) {
        this.state = next;
        this.draw();
      }
    });
    this.body.append(onoff);

    // The state, in the words Rust composed.
    if (state.said) {
      const said = document.createElement("p");
      said.className = state.state === "adrift" ? "lane-adrift" : "lane-said";
      said.textContent = state.said;
      this.body.append(said);
    }

    // 3 · The model. Pointing at one you already have is the default path and
    // always works, whatever the fetch setting says.
    const model = document.createElement("div");
    model.className = "lane-row";
    const pick = document.createElement("button");
    pick.className = "tool";
    pick.textContent = say("laneChooseModel");
    pick.addEventListener("click", async () => {
      const dir = await pickFolder(say("laneModelFolder"));
      if (!dir) return;
      const next = await api.laneSet(true, dir).catch((e) => {
        this.trouble(e);
        return null;
      });
      if (next) {
        this.state = next;
        this.draw();
      }
    });
    model.append(pick);
    if (state.model) {
      const where = document.createElement("span");
      where.className = "lane-where";
      where.dir = "ltr";
      where.textContent = state.model;
      model.append(where);
    }
    this.body.append(model);

    // 4 · The fetch button, and the setting that reveals it.
    //
    // Off in a fresh install. With it off, the button is not drawn — and the
    // command behind it refuses as well, so this is a drawing of a rule rather
    // than the rule itself (`girsa_lane::bring`).
    const allow = document.createElement("label");
    allow.className = "lane-allow";
    const box = field(say("laneModelPath"));
    box.type = "checkbox";
    box.checked = state.may_fetch;
    box.addEventListener("change", async () => {
      const next = await api.laneAllowFetch(box.checked).catch((e) => {
        this.trouble(e);
        return null;
      });
      if (next) {
        this.state = next;
        this.draw();
      }
    });
    const allowSaid = document.createElement("span");
    allowSaid.textContent = say("laneAllowFetch");
    allow.append(box, allowSaid);
    this.body.append(allow);

    if (state.may_fetch) {
      const offer = state.offer;
      // The terms, before the button does anything. They are not Girsa's to
      // grant on the reader's behalf.
      const terms = document.createElement("p");
      terms.className = "lane-terms";
      terms.textContent = `${offer.name} · ${offer.by} · ${offer.licence} · ${megabytes(offer.bytes)}`;
      const about = document.createElement("a");
      about.className = "lane-about";
      about.href = offer.about;
      about.textContent = offer.about;
      about.dir = "ltr";
      const why = document.createElement("p");
      why.className = "lane-what";
      why.textContent = offer.what;

      const bring = document.createElement("button");
      bring.className = "tool lane-bring";
      bring.textContent = fill("laneBringOne", { name: offer.name });
      bring.disabled = this.working;
      bring.addEventListener("click", async () => {
        bring.disabled = true;
        this.progress.hidden = false;
        this.progress.textContent = say("laneStarting");
        await api.laneBring().catch((e) => this.trouble(e));
      });
      this.body.append(terms, about, why, bring);
    }

    // 5 · What goes in it. Any granularity: this sefer, or the whole library.
    const chose = document.createElement("div");
    chose.className = "lane-row";
    const all = document.createElement("button");
    all.className = "tool";
    all.textContent = state.everything ? say("laneTakeAll") : say("laneAddAll");
    all.addEventListener("click", async () => {
      const next = await api.laneChoose(null, !state.everything, true).catch((e) => {
        this.trouble(e);
        return null;
      });
      if (next) {
        this.state = next;
        this.draw();
      }
    });
    chose.append(all);
    if (this.here && !state.everything) {
      const here = this.here;
      const inside = state.chosen.some((c) => c.slug === here.slug);
      const one = document.createElement("button");
      one.className = "tool";
      one.textContent = inside
        ? fill("laneTakeOut", { title: here.title })
        : fill("lanePutIn", { title: here.title });
      one.addEventListener("click", async () => {
        const next = await api.laneChoose(here.slug, !inside).catch((e) => {
          this.trouble(e);
          return null;
        });
        if (next) {
          this.state = next;
          this.draw();
        }
      });
      chose.append(one);
    }
    this.body.append(chose);

    // 6 · Coverage. The sentence, and then the seforim behind it.
    const coverage = document.createElement("p");
    coverage.className = "lane-coverage";
    coverage.textContent = state.coverage;
    this.body.append(coverage);

    for (const covered of state.chosen.slice(0, 12)) {
      const row = document.createElement("p");
      row.className = "lane-covered";
      row.textContent = `${covered.title} — ${covered.embedded}/${covered.wanted}`;
      this.body.append(row);
    }
    if (state.chosen.length > 12) {
      const more = document.createElement("p");
      more.className = "lane-covered";
      more.textContent = fill("laneAndMore", { count: state.chosen.length - 12 });
      this.body.append(more);
    }
    for (const slug of state.other_model) {
      const row = document.createElement("p");
      row.className = "lane-adrift";
      row.textContent = fill("laneOtherModel", { slug });
      this.body.append(row);
    }

    // 7 · Run it. In the background, resumable, and stoppable — and the reader
    // can go on learning while it runs (§9.9, and W26's rule for the same
    // reason).
    if (state.state === "on") {
      const run = document.createElement("div");
      run.className = "lane-row";
      const go = document.createElement("button");
      go.className = "tool lane-go";
      go.textContent = say("laneEmbed");
      go.disabled = this.working;
      go.addEventListener("click", async () => {
        this.working = true;
        this.progress.hidden = false;
        this.progress.textContent = say("laneStarting");
        await api.laneEmbed().catch((e) => this.trouble(e));
      });
      const stop = document.createElement("button");
      stop.className = "tool";
      stop.textContent = say("laneStop");
      stop.addEventListener("click", async () => {
        await api.laneStop().catch(() => undefined);
      });
      run.append(go, stop);
      this.body.append(run);
    }
  }

  /**
   * Say what went wrong, in the reader's words.
   *
   * Seven call sites reached this with `String(e)` — `trouble.ts`'s header
   * claims *"**Every** `textContent = String(e)` in this application goes
   * through here"*, and these were the seven that did not: the raw string was
   * handed to a private method, so the guard in `sources.test.mjs` — which
   * requires the `String(e)` and the assignment in **one expression** — could
   * not see them. They were in different functions.
   *
   * It takes the caught value now rather than a string, which is the fix that
   * makes the class unrepeatable here: there is no way to pass this a raw
   * message without going through `trouble()` first.
   */
  private trouble(e: unknown): void {
    const said = trouble(e, "read_lane");
    this.progress.hidden = false;
    this.progress.textContent = said.said;
    this.progress.title = said.detail;
  }
}
