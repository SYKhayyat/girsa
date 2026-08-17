// The chip row, drawn once for both searches.
//
// > *"the search should be the same as regular girsa search (with all the
// > options)."*
//
// It is the same row, from the same engine, rendered by the same function —
// the panel sets the panel's chips and the find bar sets its own, and neither
// of them decides what a chip is or what it may be set to. That is
// `girsa_search::chips`, and a webview that worked one out for itself would be
// a webview that could disagree with the thing doing the searching.
//
// This was `SearchView.chip`, private, with the panel's own `api.findChip` and
// `this.run()` written into it. Extracting it is what made a second search bar
// possible without a second idea of what the options are.

import type { Chip, Choice } from "./api.ts";
import { say, type Word } from "./say.ts";

/** What a chip row does when a reader picks something. */
export interface Picking {
  /** A choice was made. The caller sends it and re-runs whatever it runs. */
  chosen: (chip: string, key: string) => void | Promise<void>;
  /**
   * The scope chip is a doorway rather than a setting — it reports where the
   * search is looking and opens the panel that changes it. Absent means this
   * row has no doorway to offer, which is the find bar: its scope is the sefer
   * in front of you and is not a thing to be set.
   */
  scope?: () => void;
  /**
   * Choices this row cannot honour, and **why**, keyed `chip/choice`.
   *
   * The window may decline to offer what the engine offered; it may not invent
   * an option the engine did not. That distinction is already made one line
   * down, where the scope chip is dropped from a row with no doorway — this is
   * the same decision with a reason attached instead of a silence, because a
   * grey control that does not say why it is grey is a control a reader tries
   * twice.
   *
   * It exists for one entry. The find bar runs inside one sefer and a mareh
   * makom is a jump out of it: `sefer_find` matches `Answer::Cited(_)` and
   * hands back nothing, in so many words. Everything else on the row works
   * here, **including the instruments** — `Bar::by_instrument` reads
   * `chips.scope` and `Bar::over_the_text` refuses a scope wider than a few
   * seforim, so one sefer is the narrowest case a dilug can have rather than a
   * case it cannot.
   */
  cannot?: Record<string, string>;
}

/** Every chip, as a row. */
export function chipRow(chips: Chip[], picking: Picking): HTMLElement {
  const row = document.createElement("div");
  row.className = "find-chips";
  for (const chip of chips) {
    if (chip.key === "where" && !picking.scope) continue;
    row.append(chipOf(chip, picking));
  }
  return row;
}

/** One chip: what it is set to, and every other thing it could be set to. */
function chipOf(chip: Chip, picking: Picking): HTMLElement {
  const shown = chip.choices.find((c) => c.chosen) ?? chip.choices[0];
  const wrap = document.createElement("div");
  wrap.className = "find-chip";

  const face = document.createElement("button");
  face.type = "button";
  face.className = "find-chip-face";
  face.textContent = `${shown ? chipSaid(chip.key, shown) : chipName(chip.key)} ▾`;
  face.title = chipName(chip.key);
  const menu = document.createElement("div");
  menu.className = "find-chip-menu";
  menu.hidden = true;

  if (chip.key === "where") {
    face.classList.add("is-doorway");
    face.title = say("scopeWhy");
    face.addEventListener("click", () => picking.scope?.());
    wrap.append(face);
    return wrap;
  }

  for (const choice of chip.choices) {
    const item = document.createElement("button");
    item.type = "button";
    const why = whyNot(picking.cannot, chip.key, choice.key);
    item.className =
      "find-chip-item" + (choice.chosen ? " is-chosen" : "") + (why ? " is-impossible" : "");
    item.textContent = chipSaid(chip.key, choice);
    if (choice.sigil) {
      const sigil = document.createElement("span");
      sigil.className = "find-sigil";
      // The sigil is shown **on** the chip, which is how §9.5's *the power
      // syntax teaches itself* actually happens: you click it once and see what
      // you could have typed.
      sigil.textContent = choice.sigil;
      item.append(sigil);
    }
    if (why) {
      // `disabled` and not just a class: a control a reader cannot use must be
      // one the keyboard skips too, and `title` is where the reason goes,
      // because a disabled button has no hover state to hang one on.
      item.disabled = true;
      item.title = why;
    } else {
      item.addEventListener("click", () => {
        menu.hidden = true;
        void picking.chosen(chip.key, choice.key);
      });
    }
    menu.append(item);
  }

  face.addEventListener("click", () => {
    menu.hidden = !menu.hidden;
  });
  wrap.append(face, menu);
  return wrap;
}

/**
 * Why this row cannot honour one choice, or `undefined` if it can.
 *
 * A function rather than an index expression so that the spelling of the key —
 * `chip/choice`, the same one `CHOICE_WORDS` uses — is written once and can be
 * asked about by a test without building a menu.
 */
export function whyNot(
  cannot: Record<string, string> | undefined,
  chip: string,
  choice: string,
): string | undefined {
  return cannot?.[`${chip}/${choice}`];
}

/** What a chip is called. */
export function chipName(key: string): string {
  switch (key) {
    case "mode":
      return say("chipMode");
    case "where":
      return say("scope");
    case "match":
      return say("chipMatch");
    case "together":
      return say("chipTogether");
    case "instrument":
      return say("chipInstrument");
    default:
      // A chip this window has not been taught. Its own key is a worse label
      // than a translation and a better one than nothing, and it cannot be
      // silently blank.
      return key;
  }
}

/**
 * What one choice on one chip says.
 *
 * Falls back to the wire's `label` — which is right for the scope chip, whose
 * label is the **names of shelves and seforim**, the corpus's own words in
 * whatever language the corpus wrote them. Those must not be translated, and
 * they are the only labels here that are data rather than interface.
 */
export function chipSaid(chip: string, choice: Choice): string {
  if (chip === "where") return choice.label || say("wholeShelf");
  const word = CHOICE_WORDS[`${chip}/${choice.key}`];
  if (word) return say(word);
  // `Near5`, `Near12`, `Near17` — the distance the reader set, which is a
  // number in a sentence rather than a row in a table.
  const near = /^Near(\d+)$/u.exec(choice.key);
  if (chip === "together" && near) {
    return say("togetherNear").replace("{words}", near[1] ?? "");
  }
  return choice.label;
}

/** Chip key and choice key → the word for it. The keys are Rust's `as_str`. */
const CHOICE_WORDS: Record<string, Word> = {
  "mode/ToratEmet": "modeToratEmet",
  "mode/Smart": "modeSmart",
  "mode/Regex": "modeRegex",
  "mode/Citation": "modeCitation",
  "mode/Instruments": "modeInstruments",
  "match/Word": "matchWord",
  "match/Contains": "matchContains",
  "match/Letters": "matchLetters",
  "together/Anywhere": "togetherAnywhere",
  "together/Phrase": "togetherPhrase",
  "instrument/Gematria": "instrumentGematria",
  "instrument/Rashei": "instrumentRashei",
  "instrument/Sofei": "instrumentSofei",
  "instrument/Atbash": "instrumentAtbash",
  "instrument/Dilug": "instrumentDilug",
};
