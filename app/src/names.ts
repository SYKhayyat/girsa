// What the two applications are called, in Hebrew, in one place.
//
// Girsa called its sibling **כסב** in six places, one of them the toolbar of the
// first screen. `כסב` is kaf-samekh-bet: a letter-by-letter transliteration of
// the Latin "Ksav" back into Hebrew. It is not a word. The application is
// **כְּתָב** — kaf-tav-bet, the Hebrew word for *writing*, which is its name in
// its own README, its window title and its wordmark. In a project whose premise
// is that Hebrew writers deserve software that treats Hebrew properly, in the one
// place where one of the two applications names the other, the name was
// misspelled.
//
// It is here as a constant rather than typed out per site so that a seventh site
// cannot spell it a third way.

// ---------------------------------------------------------------------------
// What a sefer is called (W41)
//
// > *"hebrew and english ui. all seforim names in hebrew ui should be heb all in
// > english ui should be english."*
//
// Fifteen places drew a sefer's name and every one of them reached straight for
// `he_title`, so there was no switch to build — there were fifteen. Now there is
// one: `sefer()`, and a guard in `sources.test.mjs` that fails the build if any
// module outside this file touches `he_title` or `en_title` again.
//
// The language is held here rather than passed to each call because it is one
// setting for the window and threading it through nine files would be fifteen
// chances to forget. `titleIn` is the rule as a pure function of both, which is
// what the tests hold.

/** Which language the window is in. */
export type Language = "hebrew" | "english";

/** Anything the corpus names twice. */
export interface Named {
  he_title: string;
  en_title: string;
}

let speaking: Language = "hebrew";

/** Set the window's language, once, from the session. */
export function speak(language: Language): void {
  speaking = language;
  document.documentElement.lang = language === "hebrew" ? "he" : "en";
  // The reading itself stays RTL: an English *interface* around a Gemara does
  // not make the Gemara left-to-right, and flipping the document would put every
  // sefer's lines in the wrong order to prove a point about a menu.
  document.documentElement.classList.toggle("is-english", language === "english");
}

/** Which language the window is in now. */
export function speaking_(): Language {
  return speaking;
}

/**
 * Which of a sefer's two names to print, given a language.
 *
 * Falls back to the other when the one asked for is blank. The corpus has works
 * with one title and not the other, and a row with no name on it is worse than a
 * row named in the wrong language. Same rule as `Language::title_of` in Rust,
 * and the same reason: two rules would name a sefer one way in the pane header
 * and another in the tab above it.
 */
export function titleIn(named: Named, language: Language): string {
  const first = language === "hebrew" ? named.he_title : named.en_title;
  const second = language === "hebrew" ? named.en_title : named.he_title;
  return first.trim() ? first : second;
}

/** What to call a sefer, in the language the window is in. */
export function sefer(named: Named): string {
  return titleIn(named, speaking);
}

/** The *other* name, for a tooltip — so the one you did not choose is a hover
 * away rather than gone. */
export function alsoCalled(named: Named): string {
  return titleIn(named, speaking === "hebrew" ? "english" : "hebrew");
}

/** The sibling writing application, as it spells its own name. */
export const KSAV = "כְּתָב";

/** This application, as it spells its own name. */
export const GIRSA = "גִּרְסָא";

/**
 * `KSAV` with a one-letter prefix — *to* it, *from* it, *in* it.
 *
 * `כְּתָב` carries a dagesh and a sheva on its first letter, and gluing a prefix
 * straight onto that reads as one long word rather than a preposition and a
 * name. A maqaf keeps the name legible as a name, which is the whole reason it is
 * pointed here at all.
 */
export function withPrefix(prefix: string, name: string): string {
  return `${prefix}־${name}`;
}

/**
 * Which rows in a list are one sefer drawn twice.
 *
 * > *"Rabbeinu Chananel appears twice (`רבינו חננאל על בראשית`, `ר חננאל על
 * > בראשית`) — one from each corpus, undeduplicated, in the same list."*
 *
 * **This does not merge them, and that is deliberate.** Two catalogue entries
 * are two seforim until something states otherwise; deciding that two titles
 * name the same work is guessing at identity from a string, which is the one
 * thing this repository's rules say never to do with a reference. The two
 * files may hold different text, different structure, different completeness,
 * and a merge would silently pick one.
 *
 * So it answers a smaller question, where a wrong answer is cheap: *would a
 * reader read these two rows as the same sefer?* Where the answer is yes, both
 * rows say which corpus they came from, and a duplicate reads as two copies
 * rather than as a bug. A false positive costs a label nobody needed; a false
 * negative leaves the row exactly as it is today.
 *
 * # Why it is here
 *
 * `sources.test.mjs` put it here, and it was right to. It began in
 * `mefarshim.ts` reading `he_title` directly — which is the one thing no module
 * outside this file may do, because the reader may be in an English window and
 * the duplicate they are looking at is the one on their screen.
 *
 * It takes the title **as drawn** rather than the record, for the same reason:
 * the pair to find is the pair on the screen, and the caller has already
 * decided which of a sefer's two names that is. It is also why the honorifics
 * below come in both scripts.
 */
export function sameSeferTwice(rows: { slug: string; title: string }[]): Set<string> {
  const seen = new Map<string, string[]>();
  for (const row of rows) {
    const key = looksLike(row.title);
    if (!key) continue;
    seen.set(key, [...(seen.get(key) ?? []), row.slug]);
  }
  const twice = new Set<string>();
  for (const slugs of seen.values()) {
    if (slugs.length > 1) for (const slug of slugs) twice.add(slug);
  }
  return twice;
}

/**
 * A title, flattened to what a reader would take it for.
 *
 * Nikud and te'amim off, gershayim and quotation marks off, the honorifics the
 * two corpora spell differently off, case folded, whitespace collapsed. For
 * **comparison only** — nothing is displayed from this and nothing is merged
 * on it.
 */
function looksLike(title: string): string {
  return title
    .replace(/[֑-ׇ]/gu, "")
    .replace(/["'׳״]/gu, "")
    .toLowerCase()
    .split(/\s+/u)
    .filter((word) => word && !HONORIFIC.has(word))
    .join(" ")
    .trim();
}

/**
 * *Our teacher*, in front of a name, in every spelling the two corpora use.
 *
 * Sefaria writes `רבינו חננאל` and `Rabbeinu Chananel`; Otzaria writes
 * `ר חננאל`. Both scripts, because the comparison is on the title as drawn and
 * a reader in an English window sees the duplicate too.
 */
const HONORIFIC = new Set([
  "רבי", "רבינו", "רבנו", "ר", "רב", "הרב", "מרן",
  "rabbeinu", "rabbenu", "rabbi", "rav", "r", "rn",
]);
