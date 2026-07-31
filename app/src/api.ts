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
  refused: { file: string; why: string }[];
}

/** A stretch of words and how it is set — see `girsa_app::display::runs`. */
export interface Run {
  text: string;
  style: "plain" | "opening" | "quiet" | "break";
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
   * .ksav of your own (W29) and are drawn as themselves rather than as prose. */
  kind: "heading" | "text" | "note" | "item" | "row" | "quote" | "page";
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
  types: string[];
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

export interface Companion {
  slug: string;
  he_title: string;
  en_title: string;
  declared: boolean;
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
  /** Rishonim, acharonim, and the authors with more than one sefer among them
   * (W44). Empty when there is nothing worth grouping. */
  folders: Branch[];
  marked: string[];
  /** How many lines of this sefer anybody comments on, so *you have ticked
   * nobody* never reads as *nobody wrote*. */
  touched: number;
}

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
export type { Presence };

/** Where something asked Girsa to open — over the loopback, or a `girsa://`
 * link clicked in a document. The ref is turned into a segment id in Rust,
 * because which segments an address names is a question about the corpus. */
export interface Landing {
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

export interface AppState {
  workspace: Workspace;
  nikud: boolean;
  text_size: number;
  /** Which language the window is in (W41). Every sefer name follows it. */
  language: Language;
  positions: Record<string, string>;
  works: number;
  trouble: string | null;
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
 * to say. The sentence is composed in Rust so that the header, the CLI and the
 * test cannot drift apart.
 */
export interface Gap {
  said: string;
  pages: number;
  scans: Scanned[];
}

// --- the semantic lane (spec.md §9.9, W30) -----------------------------------

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
export interface Near {
  id: string;
  work: string;
  title: string;
  address: string;
  text: string;
  nearness: number;
}

/** What the lane answered. All four fields are drawn. */
export interface LaneAnswer {
  /** The label these must be drawn under, worded once in Rust. */
  label: string;
  near: Near[];
  coverage: string;
  /** Why there is nothing. Never an empty list with no reason attached. */
  refused: string | null;
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

export interface TagRow {
  tag: string;
  total: number;
  notes: number;
  marks: number;
  queries: number;
  collections: number;
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
  chooseMefaresh: (slug: string, work: string, on: boolean) =>
    call<string[]>("choose_mefaresh", { slug, work, on }),
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
  setNikud: (on: boolean) => call<void>("set_nikud", { on }),
  setLanguage: (language: Language) => call<void>("set_language", { language }),
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
  setCiteStyle: (style: "hebrew-full" | "hebrew-short" | "english") =>
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
  exportSefer: (slug: string, format: "txt" | "docx") =>
    call<Written>("export_sefer", { slug, format }),

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

export interface Hit {
  id: string;
  address: string;
  work: string;
  /** What to call the sefer, in the window's language (W41). One title, because
   * a hit carries a name to print rather than a sefer — Rust chose which. */
  title: string;
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
  places: { reference: string; id: string; work: string }[];
  near: string[];
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
export async function whenAskedToOpen(handler: (landing: Landing) => void): Promise<void> {
  if (!invoke) return;
  const { listen } = await import("@tauri-apps/api/event");
  await listen<Landing>("girsa://open", (event) => handler(event.payload));
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

async function fixture<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!fixtureState) {
    fixtureState = await json<AppState>("/dev/state.json").catch((): AppState => ({
      workspace: { tabs: [], active: 0 },
      nikud: true,
      text_size: 100,
      language: "hebrew",
      positions: {},
      works: 0,
      showing: "fixed",
      fixes: 0,
      suspects: 0,
      trouble:
        "running in a browser with no fixtures — build them with " +
        "`cargo run -p girsa-app --example dev-fixtures`",
    }));
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
      return { works: [], folders: [], marked: [], touched: 0 } as T;
    case "choose_mefaresh":
      return [] as unknown as T;
    case "mefarshim_at":
      return { said: [], others: false } as T;
    case "open_sefer": {
      const key = flatten(slug!);
      if (!texts.has(key)) texts.set(key, await json<Text>(`/dev/text-${key}.json`));
      const text = texts.get(key)!;
      if (fixtureState.nikud) return text as T;
      return {
        ...text,
        lines: text.lines.map((l) => ({
          ...l,
          runs: l.runs.map((r) => ({ ...r, text: withoutMarks(r.text) })),
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
          trouble: "העתקת מקור פועלת בחלון בלבד",
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
      throw new Error("סריקות נפתחות בחלון בלבד");
    case "scan_at":
      return { page: Number(args?.page ?? 1), display: null, reference: null, id: null } as T;
    case "scan_page_of":
      return null as T;
    // Corrections are the shell's: they are written into your own layer, and
    // a browser has none. Saying so beats a fix that looks like it landed.
    case "fix":
    case "unfix":
    case "export_sefer":
      throw new Error("תיקונים פועלים בחלון בלבד");
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
      throw new Error("השכבה שלך פועלת בחלון בלבד");
    case "buffer_open":
      return {
        name: String(args?.name ?? ""),
        text: "",
        path: "כתיבה פועלת בחלון בלבד",
      } as T;
    case "ksav_presence":
      return { state: "not_running" } as T;
    case "set_language":
      return undefined as T;
    case "set_nikud":
      fixtureState.nikud = Boolean(args?.on);
      return undefined as T;
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
          "החיפוש פועל בחלון בלבד — הדפדפן קורא קבצי דוגמה סטטיים, ואין בהם אינדקס",
        landing: null,
      } as T;
    // The semantic lane in a browser build: **off, and saying so.** It cannot
    // be anything else — there is no model and no personal layer out here — and
    // the one wrong answer that would matter is a lane that looked available.
    case "lane_state":
      return {
        state: "off",
        said: null,
        coverage: "nothing is in the semantic lane yet",
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
        coverage: "nothing is in the semantic lane yet",
        refused: "הלשון הסמוכה פועלת בחלון בלבד — הדפדפן קורא קבצי דוגמה סטטיים",
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
function withoutMarks(text: string): string {
  // The mark block, less the four code points inside it that separate words —
  // maqaf, paseq, sof pasuq and nun hafukha. Deleting a maqaf would join two
  // words into one on the page.
  return text.replace(/[֑-ׇֽֿׁׂׅׄ]/g, "");
}
