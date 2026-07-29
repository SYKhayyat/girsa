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

export type PaneId = number;

export interface Card {
  slug: string;
  he_title: string;
  en_title: string;
  categories: string[];
  author: string | null;
  era: string | null;
  source: "sefaria" | "otzaria" | "mine";
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
}

export interface Line {
  id: string;
  address: string;
  kind: "heading" | "text";
  runs: Run[];
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

/** Whether Ksav is there (spec.md §10.6). Asked of it, not assumed. */
export type Presence =
  | { state: "live"; version: string }
  | { state: "not_running" }
  | { state: "stale"; why: string };

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
  positions: Record<string, string>;
  works: number;
  trouble: string | null;
}

type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

let invoke: Invoke | null = null;

/** Whether the real shell is behind us, or the browser fixtures are. */
export function isShell(): boolean {
  return invoke !== null;
}

export async function connect(): Promise<void> {
  if ("__TAURI_INTERNALS__" in window) {
    const api = await import("@tauri-apps/api/core");
    invoke = api.invoke as Invoke;
  }
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
  openSefer: (slug: string) => call<Text>("open_sefer", { slug }),
  openTab: (slug: string) => call<PaneId>("open_tab", { slug }),
  split: (pane: PaneId, axis: "vertical" | "horizontal", slug: string, follow: boolean) =>
    call<PaneId | null>("split", { pane, axis, slug, follow }),
  closePane: (pane: PaneId) => call<void>("close_pane", { pane }),
  focus: (pane: PaneId) => call<void>("focus", { pane }),
  setFollows: (pane: PaneId, leader: PaneId | null) =>
    call<void>("set_follows", { pane, leader }),
  setRatio: (pane: PaneId, ratio: number) => call<void>("set_ratio", { pane, ratio }),
  setNikud: (on: boolean) => call<void>("set_nikud", { on }),
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
  /** Hits in seforim the catalogue does not have. Above zero, the three
   * derived facets are short by this many, and the panel says so. */
  uncatalogued: number;
  total: number;
}

export type Dimension = "sefer" | "shelf" | "era" | "author" | "link";

export interface Hit {
  id: string;
  address: string;
  work: string;
  he_title: string;
  runs: Run[];
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
    fixtureState = await json<AppState>("/dev/state.json").catch(() => ({
      workspace: { tabs: [], active: 0 },
      nikud: true,
      text_size: 100,
      positions: {},
      works: 0,
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
    case "buffer_open":
      return {
        name: String(args?.name ?? ""),
        text: "",
        path: "כתיבה פועלת בחלון בלבד",
      } as T;
    case "ksav_presence":
      return { state: "not_running" } as T;
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
