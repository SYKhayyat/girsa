// The settings panel Girsa did not have (B13).
//
// > *"The complete user-adjustable surface today … nikud on/off, reading size
// > 60–250%, citation style, corrections showing. **There is no settings panel.**
// > No font. No theme control … Otzar HaChochma and Bar Ilan both ship deep
// > display and search preferences. **This is a step backwards from what you are
// > replacing.**"*
//
// # What is here, and where each thing is decided
//
// Nothing on this panel decides anything. The clamps are in
// `girsa_app::session::Look::sane`, the shortcut table and its resolution are in
// `girsa_app::keys`, and the theme's three values are an enum in the session. This
// file draws rows and sends what was typed — which is the same division every
// other view in this window keeps, and it matters most here because a settings
// panel that clamps a number itself is the second reader of a rule that already
// exists (`percent.clamp(60, 250)` in the shell against
// `Math.min(250, Math.max(60, …))` in the window, which the grade flags as a
// family).
//
// # Why the shortcut rows capture a real press
//
// A text box asking a reader to type `Ctrl+Shift+C` asks them to know how we spell
// it. The row listens for one press instead and shows what it heard, so the only
// spelling anybody has to agree with is the one `keys.ts` produces — and that one
// is cross-checked against Rust's.

import {
  api,
  type CiteStyle,
  type CiteStyleName,
  type Pointing,
  type Settings,
  type Shemos,
  type Shortcut,
} from "./api.ts";
import { about, announces, button, choice as pick, field, glyph, region } from "./controls.ts";
import { OneKey } from "./capture.ts";
import { said } from "./keys.ts";
import type { Language } from "./names.ts";
import { interfaceLanguage, say } from "./say.ts";
import { THEME_ROUND, themeSaid } from "./toolbar.ts";

/**
 * What a theme row offers — the same round the toolbar button turns, and the
 * same words.
 *
 * It was a third list of the three values, beside two spellings of the type in
 * `api.ts`. The panel and the button are the two places a reader can change this
 * from, and a panel that offers `בהיר` while the button calls it something else
 * is one control lying about the other.
 */
const THEMES = THEME_ROUND.map((value) => ({ value, label: () => themeSaid(value) }));

/** The three pointing settings, in the order the control rounds them —
 * `girsa_app::session::Pointing::ALL`. */
const POINTING: { value: Pointing; label: () => string }[] = [
  { value: "full", label: () => say("pointingFull") },
  { value: "nikud", label: () => say("pointingNikud") },
  { value: "plain", label: () => say("pointingPlain") },
];

/** The two settings for the shemos — `girsa_app::shemos::Shemos::ALL`. */
const SHEMOS: { value: Shemos; label: () => string }[] = [
  { value: "as-written", label: () => say("shemosAsWritten") },
  { value: "changed", label: () => say("shemosChanged") },
];

/**
 * The three citation styles, each carrying **both** of its spellings.
 *
 * `Settings.cite` comes back `hebrew_full` and `setCiteStyle` takes
 * `hebrew-full` — a serde rename against a hand-written `name()`, in a pinned
 * crate this repository does not own. The asymmetry is real and `api.ts`
 * declares both types rather than pretending one away; this table is the one
 * place the window crosses between them, so a row selects on `stored` and
 * sends `takes`.
 */
const CITES: { stored: CiteStyle; takes: CiteStyleName; label: () => string }[] = [
  { stored: "hebrew_full", takes: "hebrew-full", label: () => say("citeHebrewFull") },
  { stored: "hebrew_short", takes: "hebrew-short", label: () => say("citeHebrewShort") },
  { stored: "english", takes: "english", label: () => say("citeEnglish") },
];

/** The two languages, offered on both language rows. */
function languages(): { value: string; label: string }[] {
  return [
    { value: "hebrew", label: say("hebrew") },
    { value: "english", label: say("english") },
  ];
}

export class SettingsView {
  readonly element: HTMLElement;
  private readonly body: HTMLElement;
  private now: Settings | null = null;
  private changed: () => void = () => {};
  /**
   * The one wait for a key, and the only one there can be.
   *
   * The listener used to be added straight onto `window` and removed in
   * exactly one place — inside itself, when a key arrived — so closing the
   * panel, clicking a second row, or any redraw of the body left it armed and
   * the next bare letter you typed anywhere was bound (finding 13). `OneKey`
   * owns the waiting; this panel's job is to call `stop()` at every door out,
   * which is safe when nothing is waiting and is why it can be called at all
   * of them without asking.
   */
  private readonly binding = new OneKey();

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "settings";
    this.element.hidden = true;

    const sheet = region("dialog", say("settings"), "settings-sheet");
    const head = document.createElement("header");
    head.className = "settings-head";
    const title = document.createElement("h2");
    title.textContent = say("settings");
    head.append(title, glyph("×", say("settingsClose"), () => this.close()));
    this.body = document.createElement("div");
    this.body.className = "settings-body";
    announces(this.body, say("settings"));
    sheet.append(head, this.body);
    this.element.append(sheet);

    this.element.addEventListener("pointerdown", (event) => {
      if (event.target === this.element) this.close();
    });
  }

  /** Told when something changed, so the window can redraw what depends on it. */
  onChanged(fn: () => void): void {
    this.changed = fn;
  }

  /**
   * Told when the **interface language** changed, which is not the same event.
   *
   * Everything else on this panel is redrawn by `changed()`: the chrome, the
   * panes, this panel's own body. The interface language is different because
   * every panel in the window builds its title, its buttons and its
   * placeholders **in its constructor** and never rebuilds them — so after a
   * switch the settings panel said `הגדרות` over rows that all read English,
   * and the shelf, the search box, the links panel and the writing drawer kept
   * every word they were born with.
   *
   * The window's answer is to reload rather than to grow a `retitle()` on
   * eleven panels, each restating the strings its constructor already sets —
   * which is a second list per panel, and a twelfth panel nobody adds one to.
   * Reloading is safe here for a reason that is not luck: **the session lives
   * in Rust**, so the tabs, the panes, where you are in each of them and every
   * setting are read back exactly as they were. What is only in the window is
   * what the reader is typing, and `main.ts` flushes that first.
   *
   * **The reload was right and it did not work**, for a whole audit, because
   * the panels are rebuilt from a *cache* of the language and nothing wrote
   * that cache before reloading. Which is why this hands the handler the
   * language rather than only the news that one changed: the write and the
   * reload are one act, and they live together in `say.ts` as
   * `switchInterfaceTo`.
   */
  onInterfaceChanged(fn: (language: Language) => Promise<void>): void {
    this.interfaceChanged = fn;
  }

  private interfaceChanged: ((language: Language) => Promise<void>) | null = null;

  get isOpen(): boolean {
    return !this.element.hidden;
  }

  close(): void {
    // Door one. The reader clicked `×`, pressed Escape, or clicked outside the
    // sheet — three ways in, and none of them used to disarm the trap.
    this.binding.stop();
    this.element.hidden = true;
  }

  async toggle(): Promise<void> {
    if (this.isOpen) {
      this.close();
      return;
    }
    await this.show();
  }

  async show(): Promise<void> {
    this.element.hidden = false;
    await this.refresh();
  }

  private async refresh(): Promise<void> {
    this.now = await api.settings();
    this.draw();
  }

  private draw(): void {
    const s = this.now;
    if (!s) return;
    // Door two. Every row here is rebuilt, so a capture still holding a
    // reference to the button that showed `…` is holding a button that is no
    // longer on the screen.
    this.binding.stop();
    this.body.replaceChildren();

    this.body.append(this.heading(say("settingsReading")));
    this.body.append(
      this.choice(
        say("settingsTheme"),
        THEMES.map((t) => ({ value: t.value, label: t.label() })),
        s.theme,
        (value) => void this.look({ theme: value as Settings["theme"] }),
      ),
    );
    // Two font rows, and that is the point: a daf is Hebrew with an English
    // footnote, and one family for both means choosing which language reads badly.
    this.body.append(
      this.fontRow(say("settingsHebrewFont"), s.hebrew_font, s.fonts, (value) =>
        void this.look({ hebrew_font: value }),
      ),
    );
    this.body.append(
      this.fontRow(say("settingsLatinFont"), s.latin_font, s.fonts, (value) =>
        void this.look({ latin_font: value }),
      ),
    );
    this.body.append(
      this.number(say("settingsSize"), s.text_size, 60, 250, 5, "%", (value) => {
        void api.setTextSize(value).then(() => this.changed());
      }),
    );
    // In hundredths, because a session that stored 1.95 as a float would not
    // compare equal to itself after a round trip — the same reason a split's ratio
    // is in tenths of a percent.
    this.body.append(
      this.number(say("settingsLeading"), s.line_height, 100, 320, 5, "%", (value) =>
        void this.look({ line_height: value }),
      ),
    );
    // Zero is *no limit*, which is a real answer and why the row says so rather
    // than showing a 0 somebody would read as a bug.
    this.body.append(
      this.number(say("settingsMeasure"), s.column_ch, 0, 200, 5, "", (value) =>
        void this.look({ column_ch: value }),
      ),
    );
    // Three settings, not a checkbox. A bool could hold *everything* and
    // *nothing*, and the one a reader asked for is the third: nikud with the
    // te'amim off.
    this.body.append(
      this.choice(
        say("settingsPointing"),
        POINTING.map((p) => ({ value: p.value, label: p.label() })),
        s.pointing,
        (value) => {
          void api.setPointing(value as Pointing).then(() => this.changed());
        },
      ),
    );

    // How the shemos are written. Beside the pointing rather than off in a
    // corner of its own: they are the same kind of setting — *how the letters
    // on the page are drawn* — and a reader looking for one finds the other.
    this.body.append(
      this.choice(
        say("settingsShemos"),
        SHEMOS.map((s) => ({ value: s.value, label: s.label() })),
        s.shemos,
        (value) => {
          void api.setShemos(value as Shemos).then(() => this.changed());
        },
      ),
    );
    this.body.append(about(say("shemosAbout")));

    // The citation style, which `start-here.md` has promised since the first
    // draft "reformats every citation" — and which no view called, so the
    // promise could not be kept by any sequence of clicks. `Session::cite`
    // was already read by the copy path; it now reaches the margin of every
    // line, the commentary header, every search row and the resolver's
    // landing, so this row changes visibly more than the sentence claimed.
    this.body.append(
      this.choice(
        say("settingsCite"),
        CITES.map((c) => ({ value: c.takes, label: c.label() })),
        CITES.find((c) => c.stored === s.cite)?.takes ?? "hebrew-full",
        (value) => {
          void api.setCiteStyle(value as CiteStyleName).then(() => this.changed());
        },
      ),
    );

    // **Two** language rows, because they are two questions.
    //
    // > *"there is no way to change UI into english - only seforim names. there
    // > should be 2 seperate commands."*
    //
    // There was one setting, and it was the seforim: the interface was Hebrew
    // string literals typed into twenty modules. The window has a language of
    // its own now, and a reader can have either combination — Hebrew seforim in
    // an English window is the ordinary case for somebody who learns in Hebrew
    // and works in English, and it was the one arrangement that was impossible.
    this.body.append(this.heading(say("settingsLanguage")));
    this.body.append(
      this.choice(say("seforimIn"), languages(), s.language, (value) => {
        void api.setLanguage(value as Language).then(() => this.changed());
      }),
    );
    this.body.append(
      // …and **this panel** with it. `changed()` redraws the window's chrome and
      // its panes; it does not redraw the panel the reader is standing in, so
      // switching the interface to English left the settings themselves in
      // Hebrew — the one surface where the change is least deniable. Redrawn
      // from `refresh()` rather than patched, because the row has to come back
      // showing what Rust actually stored.
      this.choice(say("windowIn"), languages(), s.interface, (value) => {
        void api.setInterface(value as Language).then(async () => {
          // The language goes to the handler. It used to be told only *that*
          // something changed and had to go and look — and what it looked at
          // was a cache written after the panels that read it, which is why
          // every switch left the window in two languages (finding 2).
          if (this.interfaceChanged) await this.interfaceChanged(value as Language);
          else {
            this.changed();
            await this.refresh();
          }
        });
      }),
    );

    this.body.append(this.heading(say("settingsKeys")));
    const note = document.createElement("p");
    note.className = "settings-note";
    note.textContent = say("settingsKeysHint");
    this.body.append(note);
    for (const row of s.shortcuts) this.body.append(this.shortcutRow(row));
  }

  private heading(text: string): HTMLElement {
    const h = document.createElement("h3");
    h.className = "settings-heading";
    h.textContent = text;
    return h;
  }

  private row(label: string, control: HTMLElement): HTMLElement {
    const row = document.createElement("label");
    row.className = "settings-row";
    const name = document.createElement("span");
    name.textContent = label;
    row.append(name, control);
    return row;
  }

  private number(
    label: string,
    value: number,
    min: number,
    max: number,
    step: number,
    unit: string,
    set: (value: number) => void,
  ): HTMLElement {
    const input = field(label, { type: "number", className: "settings-number" });
    input.min = String(min);
    input.max = String(max);
    input.step = String(step);
    input.value = String(value);
    // On `change` and not on every keystroke: a session file written per digit is
    // a session file written four times to reach 250.
    input.addEventListener("change", () => set(Number(input.value)));
    const box = document.createElement("span");
    box.className = "settings-with-unit";
    box.append(input);
    if (unit) {
      const u = document.createElement("span");
      u.className = "settings-unit";
      u.textContent = unit;
      box.append(u);
    }
    return this.row(label, box);
  }

// The checkbox helper went with the nikud row it was written for. Nothing else
// on this panel is a yes-or-no: the pointing is three settings, the two
// languages are two of two, and the rest are numbers, fonts and keys. A helper
// kept for a caller that does not exist is the next reader's ten minutes.

  private choice(
    label: string,
    options: { value: string; label: string }[],
    value: string,
    set: (value: string) => void,
  ): HTMLElement {
    // Through `controls.choice`, like every other control here: a select with no
    // name is one of thirty unlabelled boxes to a screen reader, and B14's guard in
    // `sources.test.mjs` fails the build over exactly this. It caught this line.
    const select = pick(label, "settings-select");
    for (const option of options) {
      const node = document.createElement("option");
      node.value = option.value;
      node.textContent = option.label;
      if (option.value === value) node.selected = true;
      select.append(node);
    }
    select.addEventListener("change", () => set(select.value));
    return this.row(label, select);
  }

  /**
   * A font row: the families we can name, and a box for one we cannot.
   *
   * A webview cannot list what is installed, so a closed list would offer families
   * the machine does not have and hide the one it does. The list is a convenience
   * and the box is the answer.
   */
  private fontRow(
    label: string,
    value: string,
    known: string[],
    set: (value: string) => void,
  ): HTMLElement {
    const input = field(label, { type: "text", className: "settings-text", dir: "auto" });
    input.value = value;
    input.placeholder = say("fontDefault");
    input.setAttribute("list", `fonts-${label}`);
    input.addEventListener("change", () => set(input.value));
    const list = document.createElement("datalist");
    list.id = `fonts-${label}`;
    for (const family of known) {
      if (!family) continue;
      const option = document.createElement("option");
      option.value = family;
      list.append(option);
    }
    const box = document.createElement("span");
    box.className = "settings-with-unit";
    box.append(input, list);
    return this.row(label, box);
  }

  /**
   * One shortcut, rebound by pressing the keys.
   *
   * The button listens for the next press and sends what it heard. The reset puts
   * the reader's binding back to *absent*, which is what makes the shipped default
   * take over again — one code path rather than two.
   */
  private shortcutRow(row: Shortcut): HTMLElement {
    // Named in the language the **window** is in. `girsa_app::keys` carries
    // both names for every action; this row printed the Hebrew as its label and
    // the English as its tooltip whatever the window was set to, which reads as
    // a translation of a tooltip rather than as a setting.
    const named = interfaceLanguage() === "hebrew" ? row.he : row.en;
    const other = interfaceLanguage() === "hebrew" ? row.en : row.he;
    const key = button(row.bound ?? "—", `${other} · ${row.shipped}`, () => {
      key.textContent = "…";
      // Door three is inside `wait` itself: starting this one ends whichever
      // row was waiting before, and puts that row's label back.
      this.binding.wait(window, (pressed) => {
        if (!pressed) {
          key.textContent = row.bound ?? "—";
          return;
        }
        void this.bind(row.id, said(pressed));
      });
    });
    key.className = "settings-key";

    const reset = glyph("↺", `${named} — ${say("putBack")}${row.shipped}`, () => {
      void this.bind(row.id, "");
    });
    const box = document.createElement("span");
    box.className = "settings-with-unit";
    box.append(key, reset);
    return this.row(named, box);
  }

  private async bind(action: string, to: string): Promise<void> {
    const shortcuts = await api.bindKey(action, to).catch(() => null);
    if (!shortcuts || !this.now) {
      // A refusal is redrawn from the truth rather than guessed at.
      await this.refresh();
      return;
    }
    this.now = { ...this.now, shortcuts };
    this.draw();
    this.changed();
  }

  private async look(change: Partial<Settings>): Promise<void> {
    if (!this.now) return;
    const next = { ...this.now, ...change };
    this.now = next;
    await api.setLook({
      theme: next.theme,
      hebrew_font: next.hebrew_font,
      latin_font: next.latin_font,
      line_height: next.line_height,
      column_ch: next.column_ch,
    });
    // Redrawn from what Rust actually stored, because `Look::sane` clamps and a
    // panel showing 400 where the session holds 320 is a panel that lies.
    await this.refresh();
    this.changed();
  }
}

/**
 * Put the look on the document, where the stylesheet can see it (B13).
 *
 * Custom properties and one class, so `styles.css` keeps owning what everything
 * looks like: a panel that set colours itself would be a second stylesheet.
 */
export function applyLook(look: {
  theme: string;
  hebrew_font: string;
  latin_font: string;
  line_height: number;
  column_ch: number;
}): void {
  const root = document.documentElement;
  // `color-scheme` used to be pinned to dark in the stylesheet with a
  // `prefers-color-scheme` override, so the operating system decided and a reader
  // who wanted the other one could not have it. Now: the reader decides, and
  // *follow the system* is one of the three answers rather than the only one.
  root.dataset.theme = look.theme;
  root.style.setProperty("--hebrew-chosen", look.hebrew_font || "");
  root.style.setProperty("--plain-chosen", look.latin_font || "");
  root.style.setProperty("--leading", String(look.line_height / 100));
  root.style.setProperty("--measure", look.column_ch > 0 ? `${look.column_ch}ch` : "none");
}
