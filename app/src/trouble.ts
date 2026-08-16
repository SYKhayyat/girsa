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

import { KSAV } from "./names.ts";
import { fill, ksavAs, say } from "./say.ts";

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
  // The six below arrived with the sweep for `textContent = String(e)`. This
  // file's own header claimed **every** such assignment went through here;
  // sixteen did not, and eight of those were `say(String(e), true)` in
  // `main.ts` — each under a comment arguing that the raw string carried a
  // distinction worth keeping. The distinctions were real; the raw English was
  // never how to keep them, and each one needed a name for what was being
  // attempted before it could be said in Hebrew.
  | "fix"
  | "export"
  // Putting the section on paper (`printview.ts`). Its own name because its
  // commonest failure is not the export's: the sefer is open and the line the
  // reader is standing on is not in it any more, after a correction re-cut it.
  | "print"
  // Reading or changing the named arrangements (`desksview.ts`).
  | "desks"
  // Asking whether there is a newer Girsa (`girsa_app::newer`). Its own name
  // because its commonest failure is the one every other action here cannot
  // have: the machine is offline, which is an ordinary way to run this.
  | "update"
  | "mark"
  | "keep_query"
  | "copy_scan"
  | "read_suspects"
  // Following a mekor the reader typed themselves (W19). Its own name because
  // its commonest failure is a real and specific one — you can cite a sefer you
  // have not imported — and *reading the links* would name the wrong thing.
  | "open_ref"
  // Walking the transmission chain (W28). Its own name because its commonest
  // failure is specific and not about links at all: the catalogue could not be
  // read, so nothing knows when any sefer was written.
  | "chain"
  // Reading a sefer's table of contents (A3). Its own name because its
  // commonest failure is one a reader can act on — the sefer is not open — and
  // *reading the sefer* would name the page they are already looking at.
  | "contents"
  | "general";

export interface Trouble {
  /** The reader's sentence, in the window's language. Always present, always
   * about what the reader can do. */
  said: string;
  /** The developer's string, for the details affordance. Never the message. */
  detail: string;
}

/** What each attempt is called, when a sentence has to name it. */
const DOING: Record<Doing, string> = {
  reach_ksav: fill("doingReachKsav", { ksav: KSAV }),
  send_to_ksav: fill("doingSendToKsav", { ksav: ksavAs("ל") }),
  open_pdf: say("doingOpenFile"),
  read_page: say("doingReadPage"),
  read_links: say("doingReadLinks"),
  repair_link: say("doingRepairLink"),
  read_lane: say("doingReadLane"),
  write_note: say("doingWriteNote"),
  fix: say("doingFix"),
  export: say("doingExport"),
  print: say("doingPrint"),
  desks: say("doingDesks"),
  update: say("doingUpdate"),
  mark: say("doingMark"),
  keep_query: say("doingKeepQuery"),
  copy_scan: say("doingCopySource"),
  read_suspects: say("doingReadSuspects"),
  open_ref: say("doingOpenRef"),
  chain: say("doingChain"),
  contents: say("doingContents"),
  general: say("doingSomething"),
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
  "no-index": () => say("codeNoIndex"),
  "no-shelf": () => say("codeNoShelf"),
  // Not `no-shelf`, and the distinction is the reader's next move: *no shelf*
  // is the state the window opened in, and *this folder will not do* is an
  // answer to a folder they just picked. Sending somebody who chose their
  // Downloads directory to `girsa-import` is a wrong instruction, not a vague
  // one.
  "not-a-corpus": () => say("codeNotACorpus"),
  "no-sefer": () => say("codeNoSefer"),
  "will-not-open": () => say("codeWillNotOpen"),
  poisoned: () => say("codePoisoned"),
  "no-such": (doing) => fill("codeNoSuch", { doing }),
  cycle: () => say("codeShelfLoop"),
  "read-only": (doing) => fill("codeReadOnly", { doing }),
  "no-lane": () => say("codeLaneOff"),
  "no-desk": () => fill("codeNoDesk", { ksav: KSAV }),
  "no-page": () => say("codeNoSuchPage"),

  // The three the shell used to compose itself, in English, in Rust. One of
  // them reached a Hebrew right-to-left toast as `the clipboard refused it:
  // Empty clipboard error, code = OSError(1418): Thread does not have a
  // clipboard open.` The underlying failure was real; the sentence around it
  // was the defect, and the machine's words are on the hover now.
  "no-clipboard": () => say("codeNoClipboard"),
  "clipboard-refused": () => say("codeClipboardRefused"),
  "will-not-serialize": () => say("codeWillNotSerialize"),

  // Not a failure: a step the reader has not taken. Saying *something went
  // wrong* to somebody who pressed Ctrl+C with nothing highlighted is a lie
  // about their own machine.
  "nothing-chosen": () => say("codeNothingChosen"),
  offline: () => say("codeOffline"),
  // Not a refusal either — the ladder announcing what it applied. It is in this
  // table because the window says it, and everything the window says comes from
  // one place. It used to be a Hebrew sentence written out in `lib.rs`, which
  // an English window would have shown in Hebrew.
  "rung-applied": () => say("codeRungApplied"),

  // `girsa_post::PostError::code()`. The three below used to be matched by
  // their English words, here *and* in Ksav's `diagnostics.ts`, with four
  // character-identical regexes across two repositories — which made every word
  // of a `Display` impl in a third repository load-bearing API between them.
  //
  // The fix is the one this table already is, applied to the one error type
  // that actually crosses. It had never been: this file's own header explains
  // why regexing your own prose is wrong and then does it to somebody else's,
  // which is the same mistake with the blame moved.
  //
  // `PostError::Io` and `::Json` are deliberately **not** coded. They are the
  // operating system's failure and serde's, forwarded, and the distinction a
  // reader needs — permission against not-found — lives only in their own
  // words. Those fall through to `FAMILIES` below, where matching somebody
  // else's prose is honest because there is nothing else to match.
  "post-not-running": () => fill("codePostNotRunning", { ksav: KSAV }),
  "post-unreachable": (doing) => fill("codePostUnreachable", { doing }),
  "post-refused": (doing) => fill("codePostRefused", { doing }),
};

/**
 * What Rust put in front of the colon, if it put anything.
 *
 * Exported because a name is not only how a refusal is *worded* — it is also
 * how the window knows which screen to draw. A window with no corpus has to
 * offer a folder picker and one whose personal layer will not write must not,
 * and *which refusal is this* is exactly the question the code answers. Reading
 * it here rather than adding a second boolean to the wire keeps to the rule the
 * wire already follows: Rust sends names.
 */
export function codeOf(detail: string): string | undefined {
  const at = detail.indexOf(": ");
  if (at <= 0) return undefined;
  const name = detail.slice(0, at);
  return name in CODED ? name : undefined;
}

/**
 * The refusals **nobody in this product owns**.
 *
 * An `os error 2`, a `connection refused`, a `serde_json` message. Matching
 * somebody else's `Display` by its words is the only thing available for these,
 * and that is honest — unlike doing it to a type we ship, which is what `CODED`
 * above ended.
 *
 * It said *"whatever a `PostError` says"* here, and that was the whole of the
 * mistake: `PostError` is not somebody else's, it is `girsa-post`'s, in the
 * shared repository both applications compile. Its three refusals are in `CODED`
 * now and their regexes are gone from this list.
 */
const FAMILIES: { match: RegExp; said: (doing: string) => string }[] = [
  {
    match: /connection refused|actively refused/i,
    said: (doing) => fill("familyRefused", { doing }),
  },
  {
    match: /permission denied|access is denied|os error 5\b/i,
    said: (doing) => fill("familyNoPermission", { doing }),
  },
  {
    match: /no such file|not found|os error 2\b/i,
    said: (doing) => fill("familyNoFile", { doing }),
  },
  {
    match: /expected value|trailing characters|EOF while parsing/i,
    said: (doing) => fill("familyBadAnswer", { doing }),
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
  return { said: fill("troubleUnknown", { doing: what }), detail };
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
