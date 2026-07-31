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
