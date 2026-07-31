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

import { api, type Settings, type Shortcut } from "./api.ts";
import { announces, button, choice as pick, field, glyph, region } from "./controls.ts";
import { said } from "./keys.ts";

/** What a theme row offers. Three, said out loud — see `session::Theme`. */
const THEMES: { value: string; he: string; en: string }[] = [
  { value: "system", he: "כמו המערכת", en: "Follow the system" },
  { value: "light", he: "בהיר", en: "Light" },
  { value: "dark", he: "כהה", en: "Dark" },
];

export class SettingsView {
  readonly element: HTMLElement;
  private readonly body: HTMLElement;
  private now: Settings | null = null;
  private changed: () => void = () => {};

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "settings";
    this.element.hidden = true;

    const sheet = region("dialog", "הגדרות", "settings-sheet");
    const head = document.createElement("header");
    head.className = "settings-head";
    const title = document.createElement("h2");
    title.textContent = "הגדרות";
    head.append(title, glyph("×", "סגור את ההגדרות", () => this.close()));
    this.body = document.createElement("div");
    this.body.className = "settings-body";
    announces(this.body, "הגדרות");
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

  get isOpen(): boolean {
    return !this.element.hidden;
  }

  close(): void {
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
    this.body.replaceChildren();

    this.body.append(this.heading("הקריאה"));
    this.body.append(
      this.choice(
        "ערכת צבעים",
        THEMES.map((t) => ({ value: t.value, label: t.he })),
        s.theme,
        (value) => void this.look({ theme: value as Settings["theme"] }),
      ),
    );
    // Two font rows, and that is the point: a daf is Hebrew with an English
    // footnote, and one family for both means choosing which language reads badly.
    this.body.append(
      this.fontRow("גופן עברי", s.hebrew_font, s.fonts, (value) =>
        void this.look({ hebrew_font: value }),
      ),
    );
    this.body.append(
      this.fontRow("גופן לטיני", s.latin_font, s.fonts, (value) =>
        void this.look({ latin_font: value }),
      ),
    );
    this.body.append(
      this.number("גודל הקריאה", s.text_size, 60, 250, 5, "%", (value) => {
        void api.setTextSize(value).then(() => this.changed());
      }),
    );
    // In hundredths, because a session that stored 1.95 as a float would not
    // compare equal to itself after a round trip — the same reason a split's ratio
    // is in tenths of a percent.
    this.body.append(
      this.number("רווח בין השורות", s.line_height, 100, 320, 5, "%", (value) =>
        void this.look({ line_height: value }),
      ),
    );
    // Zero is *no limit*, which is a real answer and why the row says so rather
    // than showing a 0 somebody would read as a bug.
    this.body.append(
      this.number("רוחב הטור (אותיות, 0 = בלי הגבלה)", s.column_ch, 0, 200, 5, "", (value) =>
        void this.look({ column_ch: value }),
      ),
    );
    this.body.append(
      this.check("ניקוד", s.nikud, (on) => {
        void api.setNikud(on).then(() => this.changed());
      }),
    );

    this.body.append(this.heading("שפת התוכנה"));
    this.body.append(
      this.choice(
        "שמות הספרים",
        [
          { value: "hebrew", label: "עברית" },
          { value: "english", label: "English" },
        ],
        s.language,
        (value) => {
          void api.setLanguage(value as "hebrew" | "english").then(() => this.changed());
        },
      ),
    );

    this.body.append(this.heading("מקשים"));
    const note = document.createElement("p");
    note.className = "settings-note";
    note.textContent = "לחץ על המקש הרצוי, או על ↺ כדי להחזיר";
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

  private check(label: string, on: boolean, set: (on: boolean) => void): HTMLElement {
    const input = field(label, { type: "checkbox", className: "settings-check" });
    input.checked = on;
    input.addEventListener("change", () => set(input.checked));
    return this.row(label, input);
  }

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
    input.placeholder = "כמו שהוגדר בעיצוב";
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
    const key = button(row.bound ?? "—", `${row.en} · ${row.shipped}`, () => {
      key.textContent = "…";
      const listen = (event: KeyboardEvent) => {
        event.preventDefault();
        event.stopPropagation();
        // A bare modifier is somebody on their way to a combination, not a
        // binding. Keep listening.
        if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) return;
        window.removeEventListener("keydown", listen, true);
        if (event.key === "Escape") {
          void this.refresh();
          return;
        }
        void this.bind(row.id, said(event));
      };
      window.addEventListener("keydown", listen, true);
    });
    key.className = "settings-key";

    const reset = glyph("↺", `${row.he} — החזר ל-${row.shipped}`, () => {
      void this.bind(row.id, "");
    });
    const box = document.createElement("span");
    box.className = "settings-with-unit";
    box.append(key, reset);
    return this.row(row.he, box);
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
