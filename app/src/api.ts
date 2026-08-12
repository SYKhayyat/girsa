// The line between the window and everything that decides anything.
//
// Every function here is one Tauri command. None of them work anything out:
// where a pane lands, which seforim may sit beside which, and what the nikud
// toggle takes off are all answered in Rust, where they are tested. See
// `app/src-tauri/src/lib.rs`.
//
// # Running in a plain browser
//
// With no Tauri around — `npm run dev` opened in Chrome — the same calls read
// static JSON out of `public/dev/`, written by
// `cargo run -p girsa-app --example dev-fixtures`. That is how the Hebrew and
// the nikud get looked at on a second rendering engine without building an
// installer, which trap W9 in BUILDER.md asks for and a screenshot from one OS
// does not answer.

import type { Glyph } from "./glyphs.ts";
import type { Language } from "./names.ts";

export type PaneId = number;

export interface Card {
  slug: string;
  he_title: string;
  en_title: string;
  categories: string[];
  author: string | null;
  era: string | null;
  source: "sefaria" | "otzaria" | "mine";
  /** Whether this sefer is a scan (W25) — which of the two reading modes it
   * opens into, and a thing a shelf row should say. */
  scan: boolean;
}

/** One shelf, and everything under it — see `girsa_app::taxonomy`. */
export interface Branch {
  key: string;
  title: string;
  /** Seforim standing on this shelf itself. */
  here: number;
  /** On it and everything under it. */
  count: number;
  /** The reader made this shelf. */
  mine: boolean;
  /** It is not where, or not what, it shipped as. */
  edited: boolean;
  children: Branch[];
  /** Not a shelf: the seforim standing on its parent, gathered so a level is all
   * folders or all seforim (W42). Carries its parent's key, so it must not be
   * renamed, pinned or dragged. */
  loose?: boolean;
}

/** What came of dropping files on the window. Both halves are reported. */
export interface Dropped {
  added: Card[];
  refused: Refusal[];
}

/** A file the window would not take, and why. */
export interface Refusal {
  file: string;
  why: string;
}

/** A stretch of words and how it is set — see `girsa_app::display::runs`. */
export interface Run {
  text: string;
  /** Absent means `plain`, which nearly every run is — it is left off the wire.
   * See `girsa_app::display::Run::style`. */
  style?: "plain" | "opening" | "quiet" | "break";
  /** These are the words that answered the search (W39). Beside the style and
   * not one of its values, because a hit inside a dibur hamatchil is both. */
  hit?: boolean;
}

/** One correction on a line, as the page shows it (spec.md §7, W20). */
export interface FixMark {
  id: string;
  /** A repair or a claim: a scanning error, or somebody's emendation. */
  kind: "ocr" | "girsa";
  was: string;
  now: string;
  who: string;
  /** Whether it is in the words on the page, or noted beside them. */
  applied: boolean;
  source?: string;
  note?: string;
}

export interface Line {
  id: string;
  address: string;
  /** What kind of line this is. `note`, `item`, `row` and `quote` come from a
   * .ksav of your own (W29) and are drawn as themselves rather than as prose.
   * Absent means `text`, which nearly every line is. */
  kind?: "heading" | "text" | "note" | "item" | "row" | "quote" | "page";
  runs: Run[];
  /** The corrections on this line. Absent on nearly every line there is. */
  fixed?: FixMark[];
  /** What the line says on disk, where a correction changed it. */
  printed?: string;
}

/** A correction, and the line it landed on — redrawn, so the window replaces
 * one line rather than rebuilding the sefer under the reader. */
export interface Fixed {
  line: Line;
  said: string;
}

/** One of your corrections, as the list shows it. */
export interface PatchRow {
  id: string;
  segment: string;
  work: string;
  /** The sefer, in the window's language (W41). */
  title: string;
  address: string;
  kind: "ocr" | "girsa";
  was: string;
  now: string;
  who: string;
  when: number;
  note?: string;
  source?: string;
}

/** How much of the correction layer is applied to what you read. */
export type Showing = "as_printed" | "fixed" | "fixed_with_variants";

/** One link on a line, with your repairs over it (spec.md §8.3, W23).
 *
 * Everything a repair UI has to show its work with: which end, what the corpus
 * said, how it was found, how much to believe it, and which of that was you. */
/** One kind of link, and what a reader is shown for it. */
export interface LinkKind {
  key: string;
  title: string;
}

export interface LinkRow {
  /** What names this edge in your layer — handed back to repair it. */
  edge: string;
  kind: string;
  /** What the corpus shipped, where your layer changed it. */
  was: string | null;
  outgoing: boolean;
  at: string;
  work: string;
  /** The sefer at the other end, in the window's language (W41). */
  title: string;
  /** The first words at the other end (W37). Absent unless that sefer is already
   * open — the panel does not read forty seforim to decorate a list. */
  preview?: string;
  address: string;
  said: string;
  method: string;
  confidence: number;
  /** The label the corpus used, verbatim — blank for 40% of them. */
  label: string;
  confirmed: boolean;
  rejected: boolean;
  mine: boolean;
  /** Which words of the line this link is about (spec.md §8.4). */
  span: [number, number] | null;
  /** `pinned` — you said; `dibur` — the commentary says. */
  span_from: string | null;
  changed: string[];
  who: string | null;
  /** Whether it may be shown as a statement about the texts. */
  curated: boolean;
}

export interface Links {
  links: LinkRow[];
  /** No companions cache, so the incoming half is missing — said, never
   * swallowed. */
  incoming_unknown: boolean;
  /**
   * The kinds of link, **labelled**, in the order they are offered.
   *
   * From `girsa_app::links::kinds`. `linksview.ts` used to hold the Hebrew in a
   * lookup table with a `?? kind` fallback, so a tenth edge type printed an
   * English slug into a Hebrew interface and nothing said so.
   */
  types: LinkKind[];
  /** Your lenses (spec.md §8.5): saved filters, not hardcoded lists. */
  lenses: { key: string; title: string }[];
  lens: string | null;
}

/** A sefer written out with your corrections in it (spec.md §7.4, W22). */
export interface Written {
  path: string;
  segments: number;
  corrections: number;
  stale: number;
  noted: number;
  said: string;
}

/** One candidate from the OCR queue (spec.md §7.3, W21). A question, not a
 * correction: it says which word, which word it looks like, how often each was
 * seen, and where to go and look. */
export interface SuspectRow {
  id: string;
  rare: string;
  common: string;
  rare_count: number;
  common_count: number;
  /** `ד/ר`, where the letters are a pair that look alike in print. */
  confusion: string | null;
  how: "letter" | "added" | "dropped" | "swapped";
  at: string | null;
  work: string | null;
  /** The sefer, in the window's language (W41). Absent when the candidate names
   * a sefer that is not on this shelf. */
  title: string | null;
  address: string | null;
}

/** Where a candidate's word sits on the page, and what to put in the box. */
export interface Standing {
  at: string;
  from_char: number;
  to_char: number;
  printed: string;
  /** `null` on a pointed word — rebuilding nikud for different letters would
   * be inventing text, so the reader types it. */
  suggestion: string | null;
}

export interface Text {
  work: Card;
  lines: Line[];
  has_nikud: boolean;
}

/**
 * How the corpus places one sefer against another — `girsa_app::shelf::Related`.
 *
 * Three, not a bool. `declared: boolean` was one flag over three different
 * claims, and Onkelos declares Bereshis as its base text, so opening Onkelos put
 * Bereshis in the list under the word `פירוש`.
 */
export type Related = "on" | "base" | "alongside";

export interface Companion {
  slug: string;
  he_title: string;
  en_title: string;
  /** `null` where only the edge count joins them. */
  stands: Related | null;
  links: number;
}

/** One mefaresh in the tick-list (W43). */
export interface Mefaresh {
  slug: string;
  he_title: string;
  en_title: string;
  chosen: boolean;
  /** The folder it stands in (W44). Absent for one drawn loose. */
  shelf?: string;
}

/** The tick-list, and which lines carry a marker given what is ticked. */
export interface Mefarshim {
  works: Mefaresh[];
  /** Seforim that keep this one's order without commenting on it — the Shulchan
   * Arukh under the Tur, the Arukh HaShulchan under the Shulchan Arukh. Its own
   * list, drawn as its own group, and never folded into `works`: *a mefaresh on
   * this* and *a sefer that follows this* are different claims and the reader is
   * the one who knows which they wanted. Ticked and marked exactly like a
   * mefaresh; only the heading differs. Empty for most seforim. */
  alongside: Mefaresh[];
  /** Rishonim, acharonim, and the authors with more than one sefer among them
   * (W44). Empty when there is nothing worth grouping. Over `works` only —
   * `alongside` is drawn flat. */
  folders: Branch[];
  marked: string[];
  /** How many lines of this sefer anybody comments on, so *you have ticked
   * nobody* never reads as *nobody wrote*. */
  touched: number;
  /**
   * The list behind the door, woven in Rust: headings and seforim, in reading
   * order, each sefer once (W44).
   *
   * The four arrays above are what it is woven **from**, and they stay because
   * the picker and the tick-list each want a different one. What used to be
   * here was the weave — `mefarshim.ts`'s `choices`, `following` and `listed`,
   * 277 lines deciding four sections, three Hebrew headings and an ordering
   * rule, beside a Rust module with twenty-five tests about this same list.
   */
  listed: Listed[];
  /** Why the list is empty, when it is empty because the link graph has never
   * been walked here. `null` when there is a cache and it holds nothing —
   * which is a different statement, and used to be the same silence. */
  unbuilt: string | null;
}

/** One row of the mefarshim list: a sefer you can open, tick, or both. */
export interface Choice {
  slug: string;
  he_title: string;
  en_title: string;
  /** How the corpus places this sefer against the one you are reading. */
  stands: Related | null;
  /** What that relationship is called, worded in Rust beside the enum. */
  said?: string;
  /** And what the claim rests on, for the hover. */
  why?: string;
  /** How many edges join the two, where that is all there is. */
  links: number;
  /**
   * Whether ticking it could mark a line — whether the graph has it commenting
   * somewhere in this sefer.
   *
   * **Not the same question as `declared`.** A tick-box that can never mark
   * anything is worse than no box.
   */
  tickable: boolean;
  chosen: boolean;
  /** The folder it stands in (W44). Absent for one drawn above the folders. */
  shelf: string | null;
}

/** A heading, or a sefer — one row of the list behind the door. */
export type Listed =
  | { kind: "folder"; title: string; depth: number; count: number }
  | { kind: "sefer"; choice: Choice };

/** One mefaresh's words on one line. */
export interface Said {
  work: string;
  he_title: string;
  en_title: string;
  address: string;
  lines: Line[];
}

export interface Comments {
  said: Said[];
  /** Somebody commented here that you have not ticked. */
  others: boolean;
}

/** One shortcut, as the settings panel shows it (B13). */
export interface Shortcut {
  id: string;
  he: string;
  en: string;
  /** What it answers to now — the reader's binding, or the shipped default. */
  bound: string | null;
  /** What it shipped bound to, so *reset* has something to reset to. */
  shipped: string;
}

/**
 * How a mekor is printed when you copy one (spec.md §5).
 *
 * **Two spellings, and both are right.** `girsa_cite::CiteStyle` derives
 * `serde(rename_all = "snake_case")`, so what Rust *sends* is `hebrew_full`;
 * `CiteStyle::name()` returns `hebrew-full`, and that is what
 * `setCiteStyle` *takes*. The two live in a pinned sibling crate and this
 * declares both rather than pretending one of them away — the field was
 * `cite: string`, which is how the asymmetry went eleven months unremarked.
 */
export type CiteStyle = "hebrew_full" | "hebrew_short" | "english";

/** The spelling the setter takes. See [`CiteStyle`]. */
export type CiteStyleName = "hebrew-full" | "hebrew-short" | "english";

/** The whole settings surface, in one call (B13). */
export interface Settings {
  pointing: Pointing;
  text_size: number;
  /** Which language the **seforim** are named in. */
  language: Language;
  /** And which language the **window** speaks. Two settings, two commands. */
  interface: Language;
  cite: CiteStyle;
  showing: Showing;
  theme: "system" | "light" | "dark";
  hebrew_font: string;
  latin_font: string;
  /** In hundredths of a line, so a session compares equal after a round trip. */
  line_height: number;
  /** In characters. Zero is *no limit*, which is a real answer. */
  column_ch: number;
  /**
   * The narrowest and widest a pane may be, in tenths of a per cent.
   *
   * From `girsa_app::workspace`, which is where the rule is. `layout.ts` used
   * to hold its own — `Math.min(85, Math.max(15, share))` — and Rust held a
   * different one, so what a drag allowed and what a session file could hold
   * were two answers.
   */
  share_bounds: [number, number];
  shortcuts: Shortcut[];
  /** Families we can name. A webview cannot list what is installed, so the field
   * takes any text and this is a convenience. */
  fonts: string[];
}

export type Place =
  | { kind: "at"; ids: string[] }
  | { kind: "no_place" }
  | { kind: "unrelated" };

export type Relation =
  | { declared: { follower_is_commentary: boolean } }
  | "linked"
  | "unrelated";

export interface Move {
  pane: PaneId;
  place: Place;
  relation: Relation;
  /** For a pane holding a scan, the page to turn to (W25). Counted in Rust,
   * because a page number worked out here from a segment id would be the
   * window deriving an address from an ordinal. */
  page?: number;
}

export type Layout =
  | { kind: "leaf"; pane: PaneId }
  | {
      kind: "split";
      axis: "vertical" | "horizontal";
      ratio: number;
      first: Layout;
      second: Layout;
    };

export interface Pane {
  id: PaneId;
  slug: string;
  at?: string;
  follows?: PaneId;
}

export interface Tab {
  layout: Layout;
  panes: Pane[];
  focused: PaneId;
}

export interface Workspace {
  tabs: Tab[];
  active: number;
}

/** What one Ctrl+C put down, and where it points (spec.md §10.2, W15). */
export interface Copied {
  /** The citation as printed, so the confirmation names the place. */
  display: string;
  /** The ref the document will store — not the printed string. */
  reference: string;
  lines: number;
  put: { plain: boolean; html: boolean; packet: boolean; trouble: string | null };
}

/** Whether Ksav is there (spec.md §10.6). Asked of it, not assumed.
 *
 *  Declared in `presence.ts`, beside the three sentences that describe the three
 *  states, and re-exported here so the bridge stays the one import for callers.
 *  Two declarations of one type is the shape of every bug this project's rules
 *  are written to prevent. */
import type { Presence } from "./presence.ts";
import { say } from "./say.ts";
export type { Presence };

/**
 * Where something asked Girsa to open — over the loopback, or a `girsa://`
 * link clicked in a document. The ref is turned into a segment id in Rust,
 * because which segments an address names is a question about the corpus.
 *
 * **Called `Asked` and not `Landing`** because `Landing` is also what a
 * citation search comes back as, 725 lines below, and TypeScript merges two
 * interfaces of one name rather than refusing them. So `Landing` was silently
 * the union of both — `ref`, `slug`, `id`, `said`, `places`, `near` — and
 * every use of it type-checked against fields the other one sends.
 */
export interface Asked {
  ref: string;
  slug: string;
  id: string;
}

/** A buffer: what you are writing, and the `.ksav` file it lives in. */
export interface Writing {
  name: string;
  text: string;
  path: string;
}

/**
 * How much of the pointing is drawn — `girsa_app::session::Pointing`.
 *
 * Three, and the middle one is what the reader asked for: *"there is no way to
 * have nikud and no trup."*
 */
export type Pointing = "full" | "nikud" | "plain";

export interface AppState {
  workspace: Workspace;
  pointing: Pointing;
  text_size: number;
  /** Which language the seforim are named in (W41). */
  language: Language;
  /** Which language the window itself speaks. */
  interface: Language;
  /** The resolved shortcut table (B13): spelling → action id. Sent with the state
   * because a `keydown` handler cannot await a round trip before deciding whether
   * to swallow the key. */
  keys: Record<string, string>;
  look: {
    theme: "system" | "light" | "dark";
    hebrew_font: string;
    latin_font: string;
    line_height: number;
    column_ch: number;
  };
  /**
   * The narrowest and widest a pane may be, in tenths of a per cent.
   *
   * From `girsa_app::workspace::SMALLEST_SHARE`/`LARGEST_SHARE`, which is where
   * the rule is. `layout.ts` used to hold its own and Rust held a different one,
   * so what a drag allowed and what a session file could hold were two answers.
   */
  share_bounds: [number, number];
  positions: Record<string, string>;
  works: number;
  trouble: string | null;
  /** How a mekor is printed when you copy one (spec.md §5). */
  cite: CiteStyle;
  /** Why the desk is not paired, or `null` when it is. Rust has sent this
   * since the desk existed and this interface did not declare it. */
  pairing: string | null;
  showing: Showing;
  /** How many corrections you have made. */
  fixes: number;
  /** How many OCR candidates are waiting to be looked at. */
  suspects: number;
}

/**
 * A scan opened into a pane (spec.md §6.3, W25).
 *
 * The window is given the file and the mapping and draws the page itself — the
 * scan *is* the daf, so there is nothing to typeset. Which daf a page is comes
 * one page at a time from `scanAt`, because that is arithmetic on a
 * declaration and it is not done here.
 */
export interface ScanOpen {
  work: Card;
  pages: number;
  /** The page to open on: where it was left last time, or its first. */
  at: number;
  /** The PDF on disk, for `assetUrl`. */
  file: string;
  /** Whether the once-per-sefer chore has been done. */
  paged: boolean;
  /** The sefer this is a scan of, where the reader has said. */
  of: string | null;
  scheme: Scheme;
  anchors: Anchor[];
  /** Why nothing here can be cited, where that is so. */
  trouble: string | null;
}

export type Scheme = "amud" | "daf" | "numbered";

export interface Anchor {
  page: number;
  /** Absent where the anchor says *these are not pages of the sefer*. */
  at?: string;
}

/** What one page of a scan is. */
export interface PageSaid {
  page: number;
  /** The whole mareh makom — `ברכות כג.`. Null for a page with nothing printed
   * on it that a mekor could name, where the window says *page 3 of the file*
   * rather than inventing a daf. */
  display: string | null;
  reference: string | null;
  /** The permanent id of the page — what a note anchors to, and what no
   * mapping ever moves. */
  id: string | null;
}

/**
 * How far a scan has been read (spec.md §6.3, W26).
 *
 * A scan arrives on the shelf with pages and **no words** — the importer will
 * not invent Hebrew it cannot read — and this is what says how much of that has
 * been repaired and by what.
 */
export interface Reading {
  slug: string;
  pages: number;
  read: number;
  /** The next page to read, or null when there is none. */
  next: number | null;
  /** The readers that have been over it. More than one is normal: a PDF can
   * carry its own text for the pages that were typeset and none for the
   * plates. */
  by: string[];
  /** The OCR engine installed, if one is. Null means *there is nothing here to
   * read a picture with* — a state with a name, not a button that does
   * nothing. */
  engine: string | null;
  /** Corrections whose ink the current reading has no word under. */
  stranded: number;
}

/** One word on a page, and the rectangle of the page its ink is on — in
 * fractions, never pixels, because pixels are a fact about the zoom. */
export interface WordBox {
  text: string;
  left: number;
  top: number;
  right: number;
  bottom: number;
  confidence: number;
}

/** What is on one page of a scan. */
export interface PageWords {
  page: number;
  by: string | null;
  guessed: boolean;
  words: WordBox[];
}

/** One scan, and how much of it a search cannot see. */
export interface Scanned {
  slug: string;
  title: string;
  pages: number;
  read: number;
}

/**
 * What a search over this shelf cannot see — spec.md §9.7's results header.
 *
 * *"4 PDFs on this shelf aren't searchable yet — [OCR now]"*. Null is nothing
 * to say. The sentence is composed in Rust — by `girsa_app::Unseen`, which is
 * the one composer for all of it — so that the header, the CLI, the MCP
 * server's `did_not_search` and the test cannot drift apart.
 *
 * The three counts below are the clauses broken out, for a surface that wants
 * to draw a button beside one of them rather than the whole sentence. `null`
 * means *there is no index at all*, which is a different answer from `0` and
 * the larger gap of the two. They were serialized by Rust and **absent from
 * this interface**, which is the shape of drift a hand-mirrored wire format
 * makes: five fields on one side, three on the other, and nothing that fails.
 */
export interface Gap {
  said: string;
  pages: number;
  scans: Scanned[];
  /** Notes written since the index was built. */
  notes: number | null;
  /** Corrections made since then. */
  fixes: number | null;
  /** Scans carrying word corrections the index has not seen. */
  corrected_scans: number | null;
}

// --- the semantic lane (spec.md §9.9, W30) -----------------------------------

/**
 * What a lane with nothing in it says.
 *
 * The one string in this file that is also a Rust constant —
 * `girsa_lane::coverage::NOTHING_YET`. The browser build has no Rust to ask, so
 * it has to be typed out; typing it out **twice**, which is what the stub did,
 * is a fourth copy of a sentence whose whole point is that there is one of it.
 * `crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs` compares
 * this line to the Rust and fails if they part company.
 */
export const NOTHING_YET = "nothing is in the semantic lane yet";

/** One sefer's standing in the lane. */
export interface LaneCovered {
  slug: string;
  title: string;
  wanted: number;
  embedded: number;
}

/** What `laneBring` would fetch, with its terms.
 *
 * Shown **before** the button does anything: the licence on a model that is
 * about to land on the reader's disk is not Girsa's to grant on their behalf. */
export interface ModelOffer {
  name: string;
  by: string;
  licence: string;
  about: string;
  what: string;
  bytes: number;
}

/** Where the lane stands. Three states, drawn as three states. */
export interface LaneState {
  state: "off" | "adrift" | "on";
  /** The header line. Null when the lane is off — there is no lane to be
   * partial about, so there is nothing to say. */
  said: string | null;
  /** What it covers and what it does not. **Always a sentence.** */
  coverage: string;
  model: string | null;
  /** Whether Girsa may go and get a model. False in a fresh install. */
  may_fetch: boolean;
  everything: boolean;
  chosen: LaneCovered[];
  outside: number;
  other_model: string[];
  offer: ModelOffer;
}

/** One adjacent result. Deliberately not a `Hit`: nothing in this file can
 * turn one into the other, and nothing draws them in the same list. */
/**
 * Which place a row is about.
 *
 * The six fields `girsa_app::Naming` works out, flattened onto every row that
 * names a segment: a search hit, a lane result, a patch, a suspect. There used
 * to be no such shape — four Rust composers each worked out a title, an address
 * and a date for themselves and disagreed about all three, so the search column
 * honoured the window's language and the lane column beside it did not.
 */
export interface At {
  id: string;
  work: string;
  /** What to call the sefer, in the window's language (W41). One title, because
   * a row carries a name to print rather than a sefer — Rust chose which. Falls
   * back to the slug: a row with no name is a row a reader cannot act on. */
  title: string;
  /** `58:1`. **Not a citation** — a mekor is `girsa_cite::cite`, which knows
   * this work's section words, and everything leaving the window as one goes
   * through Rust's `sending`. */
  address: string;
  /** `1565`, or `1488–1575`. `null` where the corpus cannot date the work. */
  written: string | null;
  /** The era, in Hebrew, where the years are not known. */
  era: string | null;
}

export interface Near extends At {
  text: string;
  nearness: number;
}

/** What the lane answered. All six fields are drawn. */
export interface LaneAnswer {
  /** The label these must be drawn under, worded once in Rust. */
  label: string;
  /**
   * What the lane was measured to do, and at what size — worded once in Rust.
   *
   * It works on a half-remembered statement, poorly on a question, over 240
   * se'ifim rather than over the whole shelf, and it does not pasken. Every
   * word of that used to be in an MCP tool description, where an agent could
   * read it and the reader could not.
   */
  measured: string;
  near: Near[];
  coverage: string;
  /** Why there is nothing. Never an empty list with no reason attached. */
  refused: string | null;
  /**
   * Set when the ranking came off a signature shortlist rather than from
   * reading every vector in every store asked — worded once in Rust, as
   * `girsa_lane::SHORTLISTED`.
   *
   * Null is the ordinary case for a small lane and means the answer is the
   * answer. It is not null for a lane over a large sefer, where reading every
   * vector per query was the fifteen gigabytes the 9 August report is about.
   */
  shortlisted: string | null;
}

/** How far a background job has got — bringing a model, or embedding. */
export interface LaneProgress {
  doing: "bring" | "embed" | "done";
  what: string;
  done: number;
  of: number;
  trouble: string | null;
}

type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

let invoke: Invoke | null = null;
/** Turns a path on disk into a URL the webview may load. Only ever used on
 * `personal/files`, which is the one directory the shell opens to it. */
let asset: ((path: string) => string) | null = null;

// --- your own layer (spec.md §11, W27) --------------------------------------

/** One of your notes, as a row. */
export interface NoteRow {
  slug: string;
  /** What it is asked for by — the slug without the `note/`. */
  name: string;
  title: string;
  opening: string;
  tags: string[];
  paragraphs: number;
  edited: number;
  /** What it is about, as segment ids. */
  on: string[];
}

/** One paragraph of a note, and its permanent name. */
export interface ParaRow {
  id: string;
  text: string;
}

/** One highlight or bookmark, and where it lands in the line as it is drawn. */
export interface MarkRow {
  id: string;
  kind: "highlight" | "bookmark";
  at: string;
  label: string | null;
  colour: string | null;
  was: string;
  tags: string[];
  /** The characters it is on — absent for a bookmark, and absent when stale. */
  span: [number, number] | null;
  /** Its words had to be looked for: the line moved under it. */
  moved: boolean;
  /** Its words are gone, or are there twice. Reported, never quietly dropped. */
  stale: boolean;
}

/** What you have on one line, less the notes — those come back from `links`. */
export interface Yours {
  notes: NoteRow[];
  marks: MarkRow[];
  folders: string[];
}

export interface QueryRow {
  name: string;
  typed: string;
  said: string;
  tags: string[];
}

export interface FolderMember {
  key: string;
  said: string;
  work: string | null;
  at: string | null;
}

export interface FolderRow {
  name: string;
  title: string;
  members: FolderMember[];
  tags: string[];
}

/** One kind of thing carrying one tag, and how many. */
export interface Carried {
  /** `note`, `mark`, `query`, `collection` — Rust's `Taggable`, spelled. */
  kind: string;
  count: number;
  /** What that kind is called, in Hebrew, in the plural. Sent rather than
   * typed out here: this file had four of them, so a fifth taggable noun was
   * an edit to a window that has never been told what a mark is. */
  said: string;
}

export interface TagRow {
  tag: string;
  total: number;
  /** What carries it, by kind — only the kinds that do. */
  carried: Carried[];
}

/** Whether the real shell is behind us, or the browser fixtures are. */
export function isShell(): boolean {
  return invoke !== null;
}

export async function connect(): Promise<void> {
  if ("__TAURI_INTERNALS__" in window) {
    const api = await import("@tauri-apps/api/core");
    invoke = api.invoke as Invoke;
    asset = api.convertFileSrc;
  }
}

/** The URL a scan is drawn from. Empty outside the shell, where there is no
 * asset protocol and a scan cannot be opened at all. */
export function assetUrl(path: string): string {
  return asset ? asset(path) : "";
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (invoke) return (await invoke(cmd, args)) as T;
  return fixture<T>(cmd, args);
}

export const api = {
  state: () => call<AppState>("state"),
  search: (query: string) => call<Card[]>("search", { query }),
  recent: () => call<Card[]>("recent"),
  companions: (slug: string) => call<Companion[]>("companions", { slug }),
  mefarshim: (slug: string) => call<Mefarshim>("mefarshim", { slug }),
  /** Tick one, and get the **whole** list back as it stands now. It used to
   * answer with the marked lines only, and the window patched the rest of its
   * own copy — which is why ticking a sefer that was not in `works` left the
   * tick-count at zero and clicking a line did nothing. */
  chooseMefaresh: (slug: string, work: string, on: boolean) =>
    call<Mefarshim>("choose_mefaresh", { slug, work, on }),
  mefarshimAt: (slug: string, at: string) => call<Comments>("mefarshim_at", { slug, at }),
  openSefer: (slug: string) => call<Text>("open_sefer", { slug }),
  openTab: (slug: string) => call<PaneId>("open_tab", { slug }),
  split: (pane: PaneId, axis: "vertical" | "horizontal", slug: string, follow: boolean) =>
    call<PaneId | null>("split", { pane, axis, slug, follow }),
  closePane: (pane: PaneId) => call<void>("close_pane", { pane }),
  closeTab: (index: number) => call<void>("close_tab", { index }),
  focus: (pane: PaneId) => call<void>("focus", { pane }),
  setFollows: (pane: PaneId, leader: PaneId | null) =>
    call<void>("set_follows", { pane, leader }),
  setRatio: (pane: PaneId, ratio: number) => call<void>("set_ratio", { pane, ratio }),
  setPointing: (pointing: Pointing) => call<void>("set_pointing", { pointing }),
  setLanguage: (language: Language) => call<void>("set_language", { language }),
  /** What the **window** says, as against what the seforim are called. */
  setInterface: (language: Language) => call<void>("set_interface", { language }),
  settings: () => call<Settings>("settings"),
  setLook: (look: {
    theme: string;
    hebrew_font: string;
    latin_font: string;
    line_height: number;
    column_ch: number;
  }) => call<void>("set_look", { look }),
  bindKey: (action: string, to: string) => call<Shortcut[]>("bind_key", { action, to }),
  setTextSize: (percent: number) => call<void>("set_text_size", { percent }),
  moved: (pane: PaneId, at: string) => call<Move[]>("moved", { pane, at }),

  // --- the shelf (W10) ----------------------------------------------------
  //
  // The tree carries counts and no seforim: 7,189 cards is not a browse, it is
  // a dump. The works of one shelf are asked for when that shelf is opened.
  shelfTree: () => call<Branch[]>("shelf_tree"),
  shelfWorks: (key: string) => call<Card[]>("shelf_works", { key }),
  shelfPutWork: (slug: string, shelf: string) => call<void>("shelf_put_work", { slug, shelf }),
  shelfPutShelf: (key: string, parent: string) => call<void>("shelf_put_shelf", { key, parent }),
  shelfRename: (key: string, title: string) => call<void>("shelf_rename", { key, title }),
  shelfPin: (parent: string, key: string) => call<void>("shelf_pin", { parent, key }),
  shelfMake: (parent: string, title: string) => call<string>("shelf_make", { parent, title }),
  shelfReset: () => call<void>("shelf_reset"),
  addMine: (paths: string[]) => call<Dropped>("add_mine", { paths }),

  // --- scans (W25) --------------------------------------------------------
  //
  // The page→daf mapping is a declaration, and everything asked of it —
  // what is on this page, which page is that daf, what does this page cite as
  // — is answered in `girsa-scan`. The window turns pages and draws them.
  scan: (slug: string) => call<ScanOpen>("scan", { slug }),
  scanAt: (slug: string, page: number) => call<PageSaid>("scan_at", { slug, page }),
  scanMap: (slug: string, scheme: Scheme, anchors: Anchor[], of: string | null) =>
    call<ScanOpen>("scan_map", { slug, scheme, anchors, of }),
  scanForget: (slug: string) => call<ScanOpen>("scan_forget", { slug }),
  /** The *go to daf* box. `null` where the scan does not carry it — never the
   * nearest page it does. */
  scanPageOf: (slug: string, written: string) =>
    call<number | null>("scan_page_of", { slug, written }),
  /** Ctrl+C on a page: the mareh makom, with no quote behind it. */
  scanCopy: (slug: string, page: number) => call<Copied>("scan_copy", { slug, page }),

  // --- reading a scan (W26) -----------------------------------------------
  /** How far this scan has been read, and what with. */
  scanReading: (slug: string) => call<Reading>("scan_reading", { slug }),
  /** Hand over the glyphs of one page; the words are worked out in Rust. */
  scanReadPage: (
    slug: string,
    page: number,
    width: number,
    height: number,
    glyphs: Glyph[],
  ) => call<Reading>("scan_read_page", { slug, page, width, height, glyphs }),
  /** Hand over a picture of one page, for a page that carries no text. */
  scanOcrPage: (slug: string, page: number, width: number, height: number, png: number[]) =>
    call<Reading>("scan_ocr_page", { slug, page, width, height, png }),
  /** What is on a page, for drawing a highlight over the photograph. */
  scanWords: (slug: string, page: number) =>
    call<PageWords | null>("scan_words", { slug, page }),
  /** Correct a word by its ink, so the fix survives the page being read
   * again by something better. */
  scanFix: (slug: string, page: number, word: number, says: string) =>
    call<PageWords | null>("scan_fix", { slug, page, word, says }),
  /** The results header's *not searchable yet* line. */
  scanGap: () => call<Gap | null>("scan_gap"),

  // --- searching (W14) ----------------------------------------------------
  find: (query: string, page: number) => call<Found>("find", { query, page }),
  findChip: (chip: string, key: string) => call<void>("find_chip", { chip, key }),
  /** Click an offer: apply one rung, and say in the header that it was applied
   * (spec.md §9.6). Nothing is applied until this is called. */
  findRung: (query: string, page: number, rung: string) =>
    call<Found>("find_rung", { query, page, rung }),
  findNarrow: (dimension: Dimension, row: FacetRow, exclude: boolean) =>
    call<void>("find_narrow", { dimension, row, exclude }),
  findWholeShelf: () => call<void>("find_whole_shelf"),

  // --- where the search looks (the scope panel) ---------------------------
  //
  // The scope exists before any search and outlives every one, which is the
  // whole point: the facet rail is computed from a result set, so it only
  // appeared after a search had already run and was cleared at the start of the
  // next — *"often the tree to pick from … is not even visible - it flashes,
  // then flashes off."*
  findScope: () => call<ScopeView>("find_scope"),
  findScopeAdd: (dimension: Dimension, key: string, label: string, exclude: boolean) =>
    call<ScopeView>("find_scope_add", { dimension, key, label, exclude }),
  findScopeDrop: (at: number) => call<ScopeView>("find_scope_drop", { at }),

  // --- the semantic lane (spec.md §9.9, W30) ------------------------------
  //
  // A separate set of calls, on purpose. There is no argument to `find` that
  // turns on the lane and no field of `Found` that carries an adjacent result:
  // the one thing §9.9 asks for above everything else is that the two kinds of
  // answer never arrive in the same shape.
  laneState: () => call<LaneState>("lane_state"),
  laneAsk: (text: string, limit?: number) => call<LaneAnswer>("lane_ask", { text, limit }),
  laneSet: (on: boolean, model?: string) => call<LaneState>("lane_set", { on, model }),
  /** Let Girsa go and get a model — off in a fresh install, and its own
   * decision rather than a field on `laneSet`. */
  laneAllowFetch: (allow: boolean) => call<LaneState>("lane_allow_fetch", { allow }),
  laneBring: () => call<void>("lane_bring"),
  laneChoose: (slug: string | null, add: boolean, all = false) =>
    call<LaneState>("lane_choose", { slug, add, all }),
  laneEmbed: () => call<void>("lane_embed"),
  laneStop: () => call<void>("lane_stop"),

  // --- the Ksav loop (W15) ------------------------------------------------
  //
  // One call, three flavours. The offsets are characters of the text this
  // window was *given* — markup already off, nikud already applied — which is
  // the only way the two ends can agree where a highlight starts without the
  // webview knowing what a mark is.
  copy: (from: string, to: string, fromChar: number, toChar: number | null, note?: string) =>
    call<Copied>("copy", { from, to, fromChar, toChar, note: note ?? null }),
  setCiteStyle: (style: CiteStyleName) =>
    call<void>("set_cite_style", { style }),

  // --- the loopback (W16) -------------------------------------------------
  ksavPresence: () => call<Presence>("ksav_presence"),

  // --- the buffer (W17) ---------------------------------------------------
  //
  // The window holds the text while it is being typed and Rust holds the file.
  // What the window never does is *write markup*: `sourceMarkup` comes from
  // `girsa-ksav`, the writer Ksav itself compiles.
  buffers: () => call<string[]>("buffers"),
  bufferOpen: (name: string) => call<Writing>("buffer_open", { name }),
  bufferSave: (name: string, text: string) => call<string>("buffer_save", { name, text }),
  /** Write a copy of the document into a folder the reader chose. The working
   * buffer stays where `buffers()` can find it. */
  bufferWriteTo: (name: string, text: string, into: string) =>
    call<string>("buffer_write_to", { name, text, into }),
  sourceMarkup: (from: string, to: string, fromChar: number, toChar: number | null) =>
    call<string>("source_markup", { from, to, fromChar, toChar }),
  bufferToKsav: (name: string, text: string) => call<void>("buffer_to_ksav", { name, text }),
  /** Straight into the open document, no clipboard. Only offered when
   * presence says it would land. */
  sendToKsav: (from: string, to: string, fromChar: number, toChar: number | null, note?: string) =>
    call<Copied>("send_to_ksav", { from, to, fromChar, toChar, note: note ?? null }),

  // --- corrections (W20) --------------------------------------------------
  //
  // The same offsets a copy uses, because it is the same highlight. Nothing
  // here writes into the corpus: a correction is a patch in your own layer,
  // and `unfix` takes it back (spec.md §7.1).
  fix: (
    at: string,
    fromChar: number,
    toChar: number,
    now: string,
    kind: "ocr" | "girsa",
    note?: string,
  ) => call<Fixed>("fix", { at, fromChar, toChar, now, kind, note: note ?? null }),
  unfix: (at: string, patch: string) => call<Fixed>("unfix", { at, patch }),
  setShowing: (showing: Showing) => call<void>("set_showing", { showing }),
  fixes: (slug?: string) => call<PatchRow[]>("fixes", { slug: slug ?? null }),

  // --- the OCR queue (W21) ------------------------------------------------
  //
  // Written by `girsa-suspects`, a batch job that runs outside the window.
  // Nothing here applies anything: `suspectAt` says where the word is, and the
  // correction goes through `fix` like any other.
  suspects: (limit: number) => call<SuspectRow[]>("suspects", { limit }),
  suspectAt: (id: string, at: string) => call<Standing>("suspect_at", { id, at }),
  suspectDecide: (id: string, decision: "dismissed" | "fixed") =>
    call<void>("suspect_decide", { id, decision }),

  // --- exporting a fixed sefer (W22) --------------------------------------
  //
  // Base text + your patches → a file. Nothing is applied here that is not
  // already applied on the page: the export writes the sefer as it is being
  // read.
  /** `into` is a folder the reader chose. Absent means *where the last one
   * went*, so the second export does not ask again. */
  exportSefer: (slug: string, format: "txt" | "docx", into?: string) =>
    call<Written>("export_sefer", { slug, format, into: into ?? null }),

  // --- links, and repairing them (W23) ------------------------------------
  //
  // Repairs are overrides in your own layer; nothing here writes into the
  // shipped graph. Which link may be shown as curated fact is answered in
  // Rust, because it is a rule about evidence.
  links: (at: string, lens?: string, span?: [number, number]) =>
    call<Links>("links", {
      at,
      lens: lens ?? null,
      fromChar: span ? span[0] : null,
      toChar: span ? span[1] : null,
    }),
  linkPin: (edge: string, at: string, fromChar: number, toChar: number) =>
    call<void>("link_pin", { edge, at, fromChar, toChar }),
  linkRepair: (edge: string, does: string, value?: string) =>
    call<void>("link_repair", { edge, does, value: value ?? null }),
  linkReanchor: (edge: string, end: "from" | "to", to: string) =>
    call<void>("link_reanchor", { edge, end, to }),
  linkDraw: (from: string, to: string, kind: string) =>
    call<void>("link_draw", { from, to, kind }),

  // --- your own layer (spec.md §11, W27) ----------------------------------
  //
  // There is no call here for *my notes on this line*, and that is the point:
  // a note's connection to a sugya is a `girsa_link::Edge`, so it comes back
  // from `links()` above, in the same list as Rashi and sorted by the same
  // rule. What is left is the writing side, and the two kinds of thing that
  // are not edges — marks and folders.
  yours: (at: string) => call<Yours>("yours", { at }),
  notes: () => call<NoteRow[]>("notes"),
  noteWrite: (at: string, text: string, title?: string) =>
    call<NoteRow>("note_write", { at, text, title: title ?? null }),
  noteRead: (note: string) => call<ParaRow[]>("note_read", { note }),
  noteEdit: (note: string, does: string, value?: string, text?: string) =>
    call<ParaRow[]>("note_edit", {
      note,
      does,
      value: value ?? null,
      text: text ?? null,
    }),
  noteForget: (note: string) => call<boolean>("note_forget", { note }),

  markHere: (at: string, span?: [number, number], label?: string, colour?: string) =>
    call<MarkRow>("mark_here", {
      at,
      fromChar: span ? span[0] : null,
      toChar: span ? span[1] : null,
      label: label ?? null,
      colour: colour ?? null,
    }),
  markForget: (mark: string) => call<boolean>("mark_forget", { mark }),
  marksIn: (slug: string) => call<MarkRow[]>("marks_in", { slug }),
  bookmarks: () => call<MarkRow[]>("bookmarks"),

  queryKeep: (name: string, typed: string) => call<QueryRow>("query_keep", { name, typed }),
  queries: () => call<QueryRow[]>("queries"),
  queryRecall: (name: string) => call<string>("query_recall", { name }),
  queryForget: (name: string) => call<boolean>("query_forget", { name }),

  folders: () => call<FolderRow[]>("folders"),
  folderEdit: (name: string, does: "put" | "take-out", member: string, title?: string) =>
    call<number>("folder_edit", { name, does, member, title: title ?? null }),
  folderForget: (name: string) => call<boolean>("folder_forget", { name }),

  tags: () => call<TagRow[]>("tags"),
  exportLayer: (into?: string) => call<string>("export_layer", { into: into ?? null }),
};


// --- searching (W14) --------------------------------------------------------
//
// Every one of these is drawn, not decided. What the chips are, what they can
// be set to, which facet rows exist and what clicking one means are all worked
// out in `girsa-search` and sent here as they stand — see spec.md §9.5, which
// is about controls being objects rather than a syntax.

/** One option on a chip, and the sigil that sets it by typing. */
export interface Choice {
  key: string;
  label: string;
  sigil: string | null;
  chosen: boolean;
}

export interface Chip {
  name: string;
  choices: Choice[];
}

/** One row of a facet: a count, and what clicking it narrows by. */
export interface FacetRow {
  key: string;
  label: string;
  count: number;
  /** How far to indent it. Only shelves nest. */
  depth: number;
}

/** The link-type facet, which can be *not built* as well as empty — spec.md
 * §9.8. Two different statements, and a column of zeros says the wrong one. */
export type LinkFacet = { state: "counted"; rows: FacetRow[] } | { state: "not_built" };

export interface Facets {
  sefer: FacetRow[];
  shelf: FacetRow[];
  era: FacetRow[];
  author: FacetRow[];
  link: LinkFacet;
  /** Your own tags, counted over this result set (spec.md §11).
   *
   *  Empty on a corpus-only search, because the corpus has no tags. Tags used to
   *  be counted across the layer and shown with **no code path by which clicking
   *  one could narrow anything** — `Dimension` had no `Tag` variant — so this is
   *  the facet and `"tag"` below is the click. */
  tag: FacetRow[];
  /** Hits in seforim the catalogue does not have. Above zero, the three
   * derived facets are short by this many, and the panel says so. */
  uncatalogued: number;
  total: number;
}

export type Dimension = "sefer" | "shelf" | "era" | "author" | "link" | "tag";

/** One thing the reader added to, or subtracted from, where the search looks. */
export interface ScopeStep {
  label: string;
  exclude: boolean;
  seforim: number;
}

/** Where the search is looking, as a list the panel draws and edits. */
export interface ScopeView {
  /** The chip's own sentence, so the panel and the chip cannot word it twice. */
  said: string;
  steps: ScopeStep[];
  everything: boolean;
}

export interface Hit extends At {
  runs: Run[];
  /** Which page of a scan this is, where it is one — so the row opens the
   * viewer at it rather than a reading pane at a line with no words in it. */
  page: number | null;
  /** Who read the words (spec.md §9.7's badge, W26): `null` for the corpus,
   * which was not read off anything; `embedded` where the file said what its
   * own words are; the engine's name where a machine guessed. */
  by: string | null;
  /** Whether that reader was an OCR engine. **Badge them, don't demote
   * them** — the row is where the score put it and this is printed beside it,
   * because OCR text is dirtier and a reader should know which kind of result
   * is in front of them. */
  guessed: boolean;
  /** The words of this hit that answered the query, as the search itself
   * worked them out — what a scan's page is highlighted by. */
  marked: string[];
}

/** A rung of the relaxation ladder, priced before the click (spec.md §9.6). */
export interface Offer {
  label: string;
  count: number;
  rung: string;
}

export interface Landing {
  said: string;
  places: PlaceRow[];
  near: string[];
}

/** One place a citation landed on. */
export interface PlaceRow {
  reference: string;
  id: string;
  work: string;
}

export interface Found {
  header: string;
  note: string | null;
  hits: Hit[];
  total: number;
  page: number;
  pages: number;
  facets: Facets | null;
  chips: Chip[];
  offers: Offer[];
  /** A refusal is an answer and it says why — never a shorter list of results
   * with nothing attached. */
  refused: string | null;
  landing: Landing | null;
}

/** Files dropped on the window, as they arrive from the shell. */
/** Something asked Girsa to show a place: Ksav over the loopback, or a
 * `girsa://` citation clicked in a document or a compiled PDF. */
export async function whenAskedToOpen(handler: (landing: Asked) => void): Promise<void> {
  if (!invoke) return;
  const { listen } = await import("@tauri-apps/api/event");
  await listen<Asked>("girsa://open", (event) => handler(event.payload));
}

/** Something asked Girsa to put a phrase in the search — Ksav, when no
 *  candidate fitted what somebody highlighted (spec.md §10.4). */
export async function whenAskedToSearch(handler: (phrase: string) => void): Promise<void> {
  if (!invoke) return;
  const { listen } = await import("@tauri-apps/api/event");
  await listen<string>("girsa://search", (event) => handler(event.payload));
}

/** Progress from a lane job — bringing a model in, or embedding (W30).
 *
 * Both jobs run on their own thread in the shell and report here, because §9.9
 * says embedding never blocks reading and a panel that froze while it worked
 * would be a strange way to keep that promise. */
export async function whenLaneWorks(
  handler: (progress: LaneProgress) => void,
): Promise<void> {
  if (!invoke) return;
  const { listen } = await import("@tauri-apps/api/event");
  await listen<LaneProgress>("lane-bring", (event) => handler(event.payload));
  await listen<LaneProgress>("lane-embed", (event) => handler(event.payload));
}

/** *Choose the directory your model is in.* Null if the reader cancelled, and
 * null outside the shell, where there is no dialog to open. */
export async function pickFolder(title: string): Promise<string | null> {
  if (!invoke) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, multiple: false, title });
  return typeof picked === "string" ? picked : null;
}

export async function whenFilesDropped(handler: (paths: string[]) => void): Promise<void> {
  if (!invoke) return;
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  await getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type === "drop") handler(event.payload.paths);
  });
}

// ---------------------------------------------------------------------------
// The browser fallback
// ---------------------------------------------------------------------------
//
// Read-only, and it keeps its workspace in memory: it exists so the page can be
// looked at, not so the app can be used without its shell.

let fixtureState: AppState | null = null;
const texts = new Map<string, Text>();

async function json<T>(path: string): Promise<T> {
  const response = await fetch(path);
  if (!response.ok) throw new Error(`${path}: ${response.status}`);
  return (await response.json()) as T;
}

/** Where the browser build keeps what the shell keeps in `session.json`.
 *
 * The shell's session is a file, so reloading the window restores every setting
 * — which is what lets a language switch simply reload (see
 * `SettingsView.onInterfaceChanged`). Out here the state was a module variable,
 * so a reload put it back to the fixture's defaults and the preview would have
 * shown the switch doing nothing. `sessionStorage` is the same promise in the
 * only terms a static-file build has. */
const KEPT = "girsa-dev-state";

function keep(): void {
  try {
    sessionStorage.setItem(KEPT, JSON.stringify(fixtureState));
  } catch {
    // A browser with storage disabled loses the setting on reload and nothing
    // else. Not worth a sentence on screen.
  }
}

async function fixture<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!fixtureState) {
    const kept = (() => {
      try {
        const held = sessionStorage.getItem(KEPT);
        return held ? (JSON.parse(held) as AppState) : null;
      } catch {
        return null;
      }
    })();
    fixtureState = kept ?? (await json<AppState>("/dev/state.json").catch((): AppState => ({
      workspace: { tabs: [], active: 0 },
      pointing: "full",
      text_size: 100,
      // The same numbers `girsa_app::workspace` holds. This literal is the
      // browser's last resort when even the fixture will not load, so it is not
      // a second rule so much as a second copy of one — and the fixture that
      // normally answers here is generated from Rust.
      share_bounds: [150, 850],
      language: "hebrew",
      interface: "hebrew",
      keys: {},
      look: { theme: "system", hebrew_font: "", latin_font: "", line_height: 195, column_ch: 0 },
      positions: {},
      works: 0,
      cite: "hebrew_full",
      pairing: say("browserWriting"),
      showing: "fixed",
      fixes: 0,
      suspects: 0,
      trouble:
        "running in a browser with no fixtures — build them with " +
        "`cargo run -p girsa-app --example dev-fixtures`",
    })));
  }
  const slug = args?.slug as string | undefined;
  switch (cmd) {
    case "state":
      return fixtureState as T;
    case "recent":
      return json<Card[]>("/dev/recent.json").catch(() => [] as Card[]) as Promise<T>;
    case "search": {
      const all = await json<Card[]>("/dev/recent.json").catch(() => [] as Card[]);
      const q = String(args?.query ?? "");
      return all.filter(
        (c) => c.he_title.includes(q) || c.en_title.toLowerCase().includes(q.toLowerCase()),
      ) as T;
    }
    case "companions":
      return json<Companion[]>(`/dev/companions-${flatten(slug!)}.json`).catch(
        () => [] as Companion[],
      ) as Promise<T>;
    // The mefarshim tick-list in a browser with no link graph. Empty, and empty
    // is the truth here — there are no edges in a fixture — so the door says
    // *nobody comments on this* rather than pretending six people do.
    case "mefarshim":
    case "choose_mefaresh":
      return {
        works: [], alongside: [], folders: [], listed: [], marked: [],
        touched: 0, unbuilt: null,
      } as T;
    case "mefarshim_at":
      return { said: [], others: false } as T;
    case "open_sefer": {
      const key = flatten(slug!);
      if (!texts.has(key)) texts.set(key, await json<Text>(`/dev/text-${key}.json`));
      const text = texts.get(key)!;
      if (fixtureState.pointing === "full") return text as T;
      const pointing = fixtureState.pointing;
      return {
        ...text,
        lines: text.lines.map((l) => ({
          ...l,
          runs: l.runs.map((r) => ({ ...r, text: withoutMarks(r.text, pointing) })),
        })),
      } as T;
    }
    case "shelf_tree":
      return json<Branch[]>("/dev/tree.json").catch(() => [] as Branch[]) as Promise<T>;
    case "shelf_works": {
      const shelves = await json<Record<string, Card[]>>("/dev/shelf.json").catch(
        () => ({}) as Record<string, Card[]>,
      );
      return (shelves[String(args?.key)] ?? []) as T;
    }
    case "moved": {
      const places = await json<Record<string, Move[]>>("/dev/moves.json").catch(
        () => ({}) as Record<string, Move[]>,
      );
      return (places[String(args?.at)] ?? []) as T;
    }
    // The clipboard is the shell's: a browser can put text down, and it
    // cannot register `application/x-girsa-source+json` as a format a native
    // Ksav would find. Saying so beats a copy that looks like it worked.
    case "copy":
      return {
        display: "",
        reference: "",
        lines: 0,
        put: {
          plain: false,
          html: false,
          packet: false,
          trouble: say("browserCopy"),
        },
      } as T;
    case "buffers":
      return [] as T;
    // A scan is a file on the reader's own disk, reached through the shell's
    // asset protocol. A browser has neither the file nor the protocol, and
    // saying so beats an empty viewer that reads as a corrupt PDF.
    case "scan":
    case "scan_map":
    case "scan_forget":
    case "scan_copy":
      throw new Error(say("browserScans"));
    case "scan_at":
      return { page: Number(args?.page ?? 1), display: null, reference: null, id: null } as T;
    case "scan_page_of":
      return null as T;
    // Corrections are the shell's: they are written into your own layer, and
    // a browser has none. Saying so beats a fix that looks like it landed.
    case "fix":
    case "unfix":
    case "export_sefer":
      throw new Error(say("browserFixes"));
    case "fixes":
    case "suspects":
      return [] as T;
    case "links":
      return { links: [], incoming_unknown: false, types: [], lenses: [], lens: null } as T;
    // Your own layer is the shell's, for the reason corrections are: it is
    // written into `personal/`, and a browser has none. Reading it comes back
    // empty; writing to it says so rather than looking as though it landed.
    case "yours":
      return { notes: [], marks: [], folders: [] } as T;
    case "notes":
    case "note_read":
    case "marks_in":
    case "bookmarks":
    case "queries":
    case "folders":
    case "tags":
      return [] as T;
    case "note_write":
    case "note_edit":
    case "note_forget":
    case "mark_here":
    case "mark_forget":
    case "query_keep":
    case "query_recall":
    case "query_forget":
    case "folder_edit":
    case "folder_forget":
    case "export_layer":
      throw new Error(say("browserLayer"));
    case "buffer_open":
      return {
        name: String(args?.name ?? ""),
        text: "",
        path: say("browserBuffer"),
      } as T;
    case "ksav_presence":
      return { state: "not_running" } as T;
    case "set_language":
      fixtureState.language = (args?.language as Language) ?? "hebrew";
      keep();
      return undefined as T;
    case "set_interface":
      fixtureState.interface = (args?.language as Language) ?? "hebrew";
      keep();
      return undefined as T;
    case "set_look":
      fixtureState.look = { ...fixtureState.look, ...(args?.look as AppState["look"]) };
      keep();
      return undefined as T;
    case "bind_key":
      return [] as unknown as T;
    // Read off the same state the setters write, so the panel shows what it did
    // rather than redrawing itself back to the defaults on every change.
    case "settings":
      return {
        pointing: fixtureState.pointing,
        text_size: fixtureState.text_size,
        language: fixtureState.language,
        interface: fixtureState.interface,
        cite: fixtureState.cite,
        showing: fixtureState.showing,
        theme: fixtureState.look.theme,
        hebrew_font: fixtureState.look.hebrew_font,
        latin_font: fixtureState.look.latin_font,
        line_height: fixtureState.look.line_height,
        column_ch: fixtureState.look.column_ch,
        shortcuts: [],
        fonts: [],
        share_bounds: fixtureState.share_bounds,
      } as T;
    // The settings that are purely about **how the page looks**, kept in memory
    // so the browser build can be looked at with them changed.
    //
    // They used to fall through to `default: undefined`, so every one of them
    // did exactly nothing out here — the size buttons, the pointing, both
    // languages, the theme, the fonts. That is the same defect as the one the
    // reader met in the shell (`calc` throwing the size away), in the build
    // whose entire purpose is *looking at the window*: a control that does
    // nothing teaches whoever is looking that the control does nothing.
    //
    // Only these. Anything that writes to a layer, an index or a document still
    // refuses out loud, because out here there is nothing to write to.
    case "set_pointing":
      fixtureState.pointing = (args?.pointing as Pointing) ?? "full";
      keep();
      return undefined as T;
    case "set_text_size":
      fixtureState.text_size = Math.min(250, Math.max(60, Number(args?.percent ?? 100)));
      keep();
      return undefined as T;
    case "set_showing":
      fixtureState.showing = (args?.showing as Showing) ?? "fixed";
      keep();
      return undefined as T;
    // The scope is the shell's: it lives beside the index, and there is no
    // index out here. An empty one is the honest answer — the whole shelf —
    // rather than a panel that looks editable and forgets every click.
    case "find_scope":
    case "find_scope_add":
    case "find_scope_drop":
      return { said: "whole shelf", steps: [], everything: true } as T;
    // Search is the shell's. The fixtures are static JSON written by
    // `dev-fixtures`, and a search index is neither static nor small — so the
    // browser says which of the two it is looking at rather than showing an
    // empty result list that reads as a corpus with nothing in it.
    case "find":
      return {
        header: "",
        note: null,
        hits: [],
        total: 0,
        page: 1,
        pages: 0,
        facets: null,
        chips: [],
        offers: [],
        refused:
          say("browserSearch"),
        landing: null,
      } as T;
    // The semantic lane in a browser build: **off, and saying so.** It cannot
    // be anything else — there is no model and no personal layer out here — and
    // the one wrong answer that would matter is a lane that looked available.
    //
    // The sentence is `girsa_lane::coverage::NOTHING_YET`, which is Rust and
    // cannot be imported here. It was typed out twice below, a fourth copy of a
    // string whose entire purpose is that there is one of it; it is now one
    // constant, and `the_rules_this_repository_wrote_down.rs` compares it to
    // the Rust.
    case "lane_state":
      return {
        state: "off",
        said: null,
        coverage: NOTHING_YET,
        model: null,
        may_fetch: false,
        everything: false,
        chosen: [],
        outside: 0,
        other_model: [],
        offer: {
          name: "BEREL 3.0",
          by: "dicta-il",
          licence: "Apache-2.0",
          about: "https://huggingface.co/dicta-il/BEREL_2.0",
          what: "BERT Embeddings for Rabbinic-Encoded Language",
          bytes: 742_923_190,
        },
      } as T;
    case "lane_ask":
      return {
        label: "adjacent — found by meaning rather than by these words",
        near: [],
        coverage: NOTHING_YET,
        refused: say("browserLane"),
      } as T;
    default:
      return undefined as T;
  }
}

function flatten(slug: string): string {
  return slug.replace(/\//g, "_");
}

// Only ever used by the browser fixtures, and only because a static file cannot
// call into Rust. The shell strips marks in `girsa-app::display`, which is the
// one implementation the app itself uses.
function withoutMarks(text: string, pointing: Pointing): string {
  // The mark block, less the four code points inside it that separate words —
  // maqaf, paseq, sof pasuq and nun hafukha. Deleting a maqaf would join two
  // words into one on the page.
  //
  // Two ranges now, because there are three settings: `nikud` takes off the
  // te'amim alone (U+0591-U+05AF, plus meteg and the two dots), and `plain`
  // takes off everything. The Rust is `girsa_app::session::Pointing::draws`,
  // and this is the browser's copy of it for a build with no Rust to ask.
  if (pointing === "nikud") {
    return text.replace(/[֑-ֽׅ֯ׄ]/gu, "");
  }
  return text.replace(/[֑-ׇֽֿׁׂׅׄ]/g, "");
}
