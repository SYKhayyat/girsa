// How a line became halacha, where a ruling came from, and where two rishonim
// read one gemara apart.
//
// spec.md §8, W28. The walk has existed since W28 and so has a terminal tool
// that prints it; **nothing in the window drew either**, so the whole of §8 was
// a feature a reader could only see by leaving the application. This is that
// panel.
//
// Nothing here decides which hops are real. `girsa_app::chaining` turns
// `girsa-link`'s walk into rows and the rows arrive already judged — whether a
// chain is a transmission, what its weakest link claims, whether the corpus
// said anything at all. A panel that made those calls itself would be a second
// opinion about the mesorah, kept in a stylesheet.

import { api, type Chain, type Forked, type Hop, type LeftOut, type Seen, type Side } from "./api.ts";
import { about, button, shut } from "./controls.ts";
import { dock, undock, wideAs } from "./dock.ts";
import { Latest } from "./latest.ts";
import { cssEscape } from "./pane.ts";
import { fill, say } from "./say.ts";
import { sayTrouble } from "./trouble.ts";

/** Which walk is on the screen. */
type Way = "forward" | "back" | "forks";

/**
 * The link types that assert something, drawn plainly, against the one that
 * does not.
 *
 * `references` is 49% of this graph and says only that two places are connected
 * somehow. A row that drew it like `quotes` would be presenting a shrug as
 * evidence, which is the whole reason `Hop.transmission` is computed in Rust
 * rather than guessed at here.
 */
const A_SHRUG = "references";

export class ChainView {
  readonly element: HTMLElement;
  private readonly list: HTMLElement;
  private readonly note: HTMLElement;
  private readonly tabs: HTMLElement;
  private at: string | null = null;
  private way: Way = "forward";
  private goTo: ((work: string, at: string) => Promise<void>) | null = null;
  private readonly draws = new Latest();

  get isOpen(): boolean {
    return this.element.classList.contains("is-open");
  }

  constructor() {
    this.element = document.createElement("section");
    this.element.className = "chain";

    const head = document.createElement("header");
    head.className = "chain-head";
    const title = document.createElement("span");
    title.className = "chain-title";
    title.textContent = say("chainTitle");
    this.note = document.createElement("span");
    this.note.className = "chain-note";
    head.append(title, this.note, shut(() => this.close()));

    this.tabs = document.createElement("nav");
    this.tabs.className = "chain-ways";
    this.list = document.createElement("div");
    this.list.className = "chain-list";
    this.element.append(head, about(say("chainAbout")), this.tabs, this.list);
    this.drawTabs();
  }

  onOpen(goTo: (work: string, at: string) => Promise<void>): void {
    this.goTo = goTo;
  }

  async toggle(at: string | null): Promise<void> {
    if (this.isOpen) {
      this.close();
      return;
    }
    await this.show(at);
  }

  async show(at: string | null): Promise<void> {
    if (!at) return;
    this.at = at;
    // Docked beside the reading, never over it — the answer every other panel
    // in this window gives, and the one a reader asked for by name.
    this.element.classList.add("is-open");
    dock("chain", wideAs("--chain-wide"));
    await this.draw();
  }

  close(): void {
    this.element.classList.remove("is-open");
    undock("chain");
  }

  private drawTabs(): void {
    this.tabs.replaceChildren();
    const ways: [Way, string, string][] = [
      ["forward", say("chainForward"), say("chainForwardWhy")],
      ["back", say("chainBack"), say("chainBackWhy")],
      ["forks", say("chainForks"), say("chainForksWhy")],
    ];
    for (const [way, label, why] of ways) {
      const tab = button(label, why, () => {
        if (this.way === way) return;
        this.way = way;
        this.drawTabs();
        void this.draw();
      });
      tab.classList.add("chain-way");
      tab.classList.toggle("is-on", this.way === way);
      this.tabs.append(tab);
    }
  }

  private async draw(): Promise<void> {
    const at = this.at;
    if (!at) return;
    const way = this.way;
    this.note.textContent = say("chainWalking");
    await this.draws.attempt(
      async () =>
        way === "forks" ? await api.chainForks(at) : await api.chainWalk(at, way),
      (answer) => {
        if (way === "forks") this.drawForks(answer as Forked);
        else this.drawWalk(answer as Chain);
      },
      (e) => {
        this.list.replaceChildren();
        sayTrouble(this.note, e, "chain");
      },
    );
  }

  private drawWalk(chain: Chain): void {
    this.list.replaceChildren();
    this.note.textContent = chain.title ? `${chain.title} · ${chain.address}` : "";

    if (chain.hops.length === 0) {
      this.list.append(this.nothing(say("chainNothing")));
      this.list.append(this.leftOut(chain.left_out));
      return;
    }

    // A tree, not a list of chains: the same three seforim would otherwise be
    // redrawn under every leaf below them, which is how a walk eight rows wide
    // becomes two hundred rows and reads as noise.
    const children = new Map<number, number[]>();
    chain.hops.forEach((hop, i) => {
      const under = hop.parent ?? -1;
      const kin = children.get(under) ?? [];
      kin.push(i);
      children.set(under, kin);
    });
    this.branch(chain, children, -1);
    void this.quote(chain.hops);

    // The honest count. Not *how many chains* — how many of them assert
    // something at every hop, which is the number that says whether this is a
    // mesorah or a pile of maybes.
    const tally = document.createElement("p");
    tally.className = "chain-tally";
    tally.textContent = fill("chainTally", {
      chains: chain.chains,
      carried: chain.transmissions,
    });
    this.list.append(tally, this.leftOut(chain.left_out));
  }

  private branch(chain: Chain, children: Map<number, number[]>, under: number): void {
    for (const i of children.get(under) ?? []) {
      const hop = chain.hops[i];
      if (!hop) continue;
      this.list.append(this.hopRow(hop));
      this.branch(chain, children, i);
    }
  }

  /**
   * Put the words on every hop.
   *
   * > *"i feel like chain and links should quote the actual line and give a
   * > link, no?"*
   *
   * They should, and neither did. A chain was a column of sefer names and
   * dates — a genealogy — and the question a person is asking it is *what did
   * each of them say*, which took eight clicks to answer and lost your place
   * doing it.
   *
   * One request per sefer, because that is what a sefer costs to read: a walk
   * eight hops deep across five seforim is five reads, not eight. The rows are
   * already on the page and are filled in as the answers land, so the panel
   * does not wait on any of it.
   */
  private async quote(hops: Hop[]): Promise<void> {
    const by = new Map<string, string[]>();
    for (const hop of hops) {
      const ats = by.get(hop.work) ?? [];
      ats.push(hop.at);
      by.set(hop.work, ats);
    }
    await Promise.all(
      [...by].map(async ([work, ats]) => {
        let said;
        try {
          said = await api.linkWords(work, ats);
        } catch {
          // A sefer that will not open leaves its hops naming the sefer and the
          // place, which is what every hop said before this.
          return;
        }
        for (const words of said) {
          const on = this.list.querySelector<HTMLElement>(
            `[data-at="${cssEscape(words.at)}"] .chain-said`,
          );
          if (on) on.textContent = words.opening;
        }
      }),
    );
  }

  private hopRow(hop: Hop): HTMLElement {
    const row = document.createElement("div");
    row.className = "chain-hop";
    // So `quote` can find this row again when the words arrive. Two hops onto
    // one segment is possible — the same sefer reached down two branches — and
    // `querySelector` finding the first of them is the right answer, because
    // they say the same thing.
    row.dataset.at = hop.at;
    // The tree's shape, as an indent the stylesheet owns rather than as spaces.
    row.style.setProperty("--depth", String(hop.depth));
    row.classList.toggle("is-transmission", hop.transmission);
    row.classList.toggle("is-shrug", hop.edge_type === A_SHRUG);

    const open = button(hop.title, hop.address, () => {
      void this.goTo?.(hop.work, hop.at);
    });
    open.classList.add("chain-where");

    const kind = document.createElement("span");
    kind.className = "chain-kind";
    kind.textContent = hop.edge_type;
    // What the corpus actually said, where it said anything. Three quarters of
    // this graph carries no label at all, and *the corpus said nothing* is a
    // different fact from *the corpus said `related`*.
    kind.title = hop.label ? fill("chainCorpusSaid", { label: hop.label }) : say("chainNoLabel");

    const when = document.createElement("span");
    when.className = "chain-when";
    when.textContent = hop.written ?? hop.era ?? say("chainNoDate");
    if (!hop.written && !hop.era) when.classList.add("is-empty");

    row.append(open, kind, when);

    // The words themselves, filled in by `quote` when the sefer is read. An
    // empty element on the page rather than one appended later: a row that
    // grows a line under it after the reader has started reading is a row that
    // moves everything below it.
    const said = document.createElement("p");
    said.className = "chain-said";
    row.append(said);

    if (hop.mine) {
      const mine = document.createElement("span");
      mine.className = "chain-mine";
      mine.textContent = say("chainMine");
      mine.title = say("chainMineWhy");
      row.append(mine);
    }
    // Named only where the chain is *not* a transmission: saying which link is
    // the weak one matters when there is one, and is noise when there is not.
    if (!hop.transmission && hop.weakest) {
      const weak = document.createElement("span");
      weak.className = "chain-weakest";
      weak.textContent = fill("chainWeakest", { kind: hop.weakest });
      row.append(weak);
    }
    return row;
  }

  private drawForks(forked: Forked): void {
    this.list.replaceChildren();
    this.note.textContent = forked.title ? `${forked.title} · ${forked.address}` : "";

    if (forked.forks.length === 0) {
      this.list.append(this.nothing(say("chainNoForks")));
      this.list.append(this.leftOut(forked.left_out));
      return;
    }
    // Said once, above the list, because it is true of every row in it: the
    // graph has no `disputes` edge anywhere, so nothing in the data says two
    // seforim disagree. What it says is that two of them read one line and a
    // later one had to deal with both.
    const caveat = document.createElement("p");
    caveat.className = "chain-caveat";
    caveat.textContent = say("chainForkCaveat");
    this.list.append(caveat);

    for (const fork of forked.forks) {
      const row = document.createElement("div");
      row.className = "chain-fork";
      row.classList.toggle("is-joined", fork.joined);
      row.append(this.sideRow(fork.a), this.sideRow(fork.b));

      const said = document.createElement("p");
      said.className = "chain-witnesses";
      // How near the nearest one is, not just how many. A sefer that quotes
      // both sides itself is evidence the two readings were argued out; one
      // that reaches them through three others is a much weaker claim wearing
      // the same word, and the count alone cannot tell them apart.
      const nearest = fork.witnesses[0]?.steps ?? 0;
      said.textContent = fork.joined
        ? say("chainForkJoined")
        : nearest <= 1
          ? fill("chainForkWitnesses", { n: fork.witnesses.length })
          : fill("chainForkFarWitnesses", { n: fork.witnesses.length, steps: nearest });
      row.append(said);
      for (const witness of fork.witnesses) {
        const one = this.sideRow(witness);
        one.classList.add("is-witness");
        if (witness.steps > 1) {
          const how = document.createElement("span");
          how.className = "chain-steps";
          how.textContent = fill("chainSteps", { n: witness.steps });
          one.append(how);
        }
        row.append(one);
      }
      this.list.append(row);
    }
    this.list.append(this.leftOut(forked.left_out));
  }

  private sideRow(side: Side | Seen): HTMLElement {
    const row = document.createElement("div");
    row.className = "chain-side";
    const open = button(side.title, side.address, () => {
      void this.goTo?.(side.work, side.at);
    });
    open.classList.add("chain-where");
    const when = document.createElement("span");
    when.className = "chain-when";
    when.textContent = side.written ?? side.era ?? say("chainNoDate");
    row.append(open, when);
    return row;
  }

  private nothing(words: string): HTMLElement {
    const out = document.createElement("p");
    out.className = "chain-empty";
    out.textContent = words;
    return out;
  }

  /**
   * What the walk would not follow.
   *
   * Part of the answer and not a footnote: *nine of the eleven seforim that
   * read this line could not be dated* changes what the chain above it means,
   * and a reader who cannot see that number is reading a chain that looks
   * complete.
   */
  private leftOut(left: LeftOut): HTMLElement {
    const out = document.createElement("p");
    out.className = "chain-refused";
    if (left.nothing) {
      out.textContent = say("chainFollowedAll");
      return out;
    }
    const said: string[] = [];
    if (left.undated > 0) said.push(fill("chainUndated", { n: left.undated }));
    if (left.wrong_way > 0) said.push(fill("chainWrongWay", { n: left.wrong_way }));
    if (left.contemporary > 0) said.push(fill("chainContemporary", { n: left.contemporary }));
    if (left.rejected > 0) said.push(fill("chainRejected", { n: left.rejected }));
    if (left.over_budget > 0) said.push(fill("chainOverBudget", { n: left.over_budget }));
    if (left.incoming_unknown > 0) {
      // The one that is not a count of edges but of blind spots: half of every
      // link is stored at its far end, so a sefer whose incoming half was never
      // built is a place the walk may have missed a hop entirely.
      said.push(fill("chainNoInbound", { n: left.incoming_unknown }));
    }
    out.textContent = `${say("chainLeftOut")} ${said.join(" · ")}`;
    return out;
  }
}
