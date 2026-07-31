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
