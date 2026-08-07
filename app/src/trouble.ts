// A caught error, as a sentence a reader can act on.
//
// This module exists because of one class of defect, found three times over in
// this project family and reported as three bugs: **a mechanism that is right,
// reporting itself in the wrong vocabulary.**
//
// The instance that started it was on the desktop app's first screen, top-left of
// an otherwise entirely Hebrew window:
//
//     could not reach ksav: connection timed out — כסב        7191 ספרים
//
// Nothing underneath that is broken. Ksav's server had been killed rather than
// closed, its endpoint file outlived its listener, and `presence()` correctly
// returned `Stale` rather than `Live` — which is exactly what `Presence::Stale`
// is for. What the reader got was `PostError::Unreachable`'s `Display` string,
// interpolated into a chip by `${ksav.why}`: the developer's sentence, in
// English, where the reader's sentence belonged.
//
// The same shape was in nine other places in this application, all of them
// `textContent = String(e)`. So the fix is not nine strings, it is one function
// with a rule:
//
//   **Every user-visible failure names (a) what failed in the reader's words,
//   (b) the thing they can act on, and (c) exactly one place to look.**
//
// The developer's string is not thrown away — it goes on `title`, which is the
// details affordance this codebase already uses for raw compiler output. It is
// never the message.

import { KSAV, withPrefix } from "./names.ts";

/**
 * What was being attempted. Needed because "what failed in the reader's words"
 * cannot be written without knowing what was being tried — a timeout reaching a
 * sibling application and a timeout reading a PDF are the same `io::Error` and
 * two entirely different things to be told.
 */
export type Doing =
  | "reach_ksav"
  | "send_to_ksav"
  | "open_pdf"
  | "read_page"
  | "read_links"
  | "repair_link"
  | "read_lane"
  | "write_note"
  | "general";

export interface Trouble {
  /** The Hebrew sentence. Always present, always about what the reader can do. */
  said: string;
  /** The developer's string, for the details affordance. Never the message. */
  detail: string;
}

/** What each attempt is called, when a sentence has to name it. */
const DOING: Record<Doing, string> = {
  reach_ksav: `הקשר עם ${KSAV}`,
  send_to_ksav: `השליחה ${withPrefix("ל", KSAV)}`,
  open_pdf: "פתיחת הקובץ",
  read_page: "קריאת העמוד",
  read_links: "קריאת הקישורים",
  repair_link: "תיקון הקישור",
  read_lane: "קריאת נתיב המשמעות",
  write_note: "כתיבת הרשומה",
  general: "הפעולה",
};

/**
 * The families of failure worth their own sentence.
 *
 * Matched on the shapes `girsa-post` and the shell actually produce, listed here
 * rather than guessed: `PostError`'s five variants, `std::io::ErrorKind`'s
 * common Display strings, and `serde_json`'s. Anything unmatched falls through to
 * a sentence that still names what failed and still puts the raw string one hover
 * away — an unrecognised error is a worse message, never a missing one.
 */

/**
 * The refusals this codebase makes on purpose, by the **name** Rust puts on
 * them rather than by the words.
 *
 * `girsa_app::trouble::Code`, written as `no-index: there is no index here`.
 * These fourteen used to be matched by regular expression against the English
 * prose of Rust's `Display` impls — which made every error string in the
 * repository load-bearing API, with the only test asserting any of them on
 * *this* side, against a hand-typed copy. Reword `"there is no index here"` and
 * both halves stayed green while the reader stopped being told what to run.
 *
 * `crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs` fails if a
 * code Rust can send has no line here.
 */
const CODED: Record<string, (doing: string) => string> = {
  "no-index": () => "אין אינדקס חיפוש — יש לבנות אותו: girsa-index build",
  "no-shelf": () => "אין מדף כאן — ייתכן שהייבוא לא רץ",
  "no-sefer": () => "אין ספר בשם הזה במדף",
  "will-not-open": () => "הספר רשום במדף ואינו נפתח — פרטים בהצבה על ההודעה",
  poisoned: () => "המצב הפנימי נפגם — יש לפתוח את החלון מחדש",
  "no-such": (doing) => `${doing} נכשלה — נתבקש דבר שאינו קיים`,
  cycle: () => "לא ניתן להכניס מדף לתוך עצמו",
  "read-only": (doing) => `${doing} נכשלה — אין אפשרות לכתוב לשכבה האישית`,
  "no-lane": () => "הלשון הסמוכה כבויה — אפשר להדליק אותה בהגדרות",
  "no-desk": () => `${KSAV} אינו מחובר`,
  "no-page": () => "אין עמוד כזה בסריקה",
};

/** What Rust put in front of the colon, if it put anything. */
function codeOf(detail: string): string | undefined {
  const at = detail.indexOf(": ");
  if (at <= 0) return undefined;
  const name = detail.slice(0, at);
  return name in CODED ? name : undefined;
}

/**
 * The refusals this codebase does **not** own.
 *
 * An `os error 2`, a `connection refused`, a `serde_json` message, whatever a
 * `PostError` says. Matching somebody else's `Display` by its words is the only
 * thing available for these, and that is honest — unlike doing it to our own,
 * which is what `CODED` above ended.
 */
const FAMILIES: { match: RegExp; said: (doing: string) => string }[] = [
  {
    // `PostError::NotRunning` — "ksav is not running".
    match: /\bis not running\b/i,
    said: () => `${KSAV} אינו פועל`,
  },
  {
    // `PostError::Unreachable` — the endpoint file is there and nothing answers.
    // This is `Presence::Stale` working, which is why the sentence says what to
    // do rather than apologising.
    match: /could not reach|timed out|timeout/i,
    said: (doing) => `${doing} לא נענתה בזמן — ייתכן שהיישום נסגר שלא כשורה`,
  },
  {
    // `PostError::Refused` — it answered, and said no.
    match: /refused it\b/i,
    said: (doing) => `${doing} נדחתה על ידי היישום שמעבר`,
  },
  {
    match: /connection refused|actively refused/i,
    said: (doing) => `${doing} נדחתה — אין מי שמאזין בצד השני`,
  },
  {
    match: /permission denied|access is denied|os error 5\b/i,
    said: (doing) => `${doing} נמנעה — אין הרשאה לקובץ`,
  },
  {
    match: /no such file|not found|os error 2\b/i,
    said: (doing) => `${doing} נכשלה — הקובץ אינו נמצא במקום שנרשם`,
  },
  {
    match: /expected value|trailing characters|EOF while parsing/i,
    said: (doing) => `${doing} נכשלה — התשובה לא נקראה כראוי`,
  },

  // The fourteen refusals this codebase makes on purpose used to be matched
  // here, by their English words. They are `CODED` above now, by name.
];

/** Turn anything a `catch` can hold into a sentence and a detail. */
export function trouble(e: unknown, doing: Doing = "general"): Trouble {
  const detail = raw(e);
  const what = DOING[doing] ?? DOING.general;
  // The name first, and the words only if there is no name. A refusal this
  // codebase made carries one; a refusal from the operating system does not.
  const code = codeOf(detail);
  if (code) return { said: CODED[code](what), detail };
  for (const family of FAMILIES) {
    if (family.match.test(detail)) return { said: family.said(what), detail };
  }
  // Unrecognised. Name what was being done, say the machine had more to say, and
  // point at the one place to look. Never `String(e)` on its own.
  return { said: `${what} נכשלה · פרטים בהצבה על ההודעה`, detail };
}

/** The developer's string, however the error arrived. */
export function raw(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message || String(e);
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return String(e);
}

/**
 * Put a caught error on an element: the sentence in the text, the raw string on
 * `title`, and the trouble class so it looks like trouble.
 *
 * Every `textContent = String(e)` in this application goes through here.
 */
export function sayTrouble(el: HTMLElement, e: unknown, doing: Doing = "general"): void {
  const t = trouble(e, doing);
  el.textContent = t.said;
  el.title = t.detail;
  el.classList.add("is-trouble");
}

/**
 * Take a trouble back off an element.
 *
 * The `title` has to go with the text. A note that now says something cheerful
 * and still carries yesterday's transport error on its hover is the same family
 * of defect this module exists to close — a report that does not match what
 * happened.
 */
export function clearTrouble(el: HTMLElement): void {
  el.removeAttribute("title");
  el.classList.remove("is-trouble");
}
