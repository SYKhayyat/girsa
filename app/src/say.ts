// What the window says, in the language the window is in.
//
// > *"there is no way to change UI into english - only seforim names. there
// > should be 2 seperate commands."*
//
// There were not two commands and there was not one either: the toggle in the
// toolbar was `setLanguage`, which changes **which of a sefer's two titles is
// printed** (`names.ts`), and every button, heading, tooltip and sentence in the
// window was a Hebrew string literal typed in place. So a reader who asked for
// English got `Berakhot` in a window that still said `לצד`, `עוקב`, `הגדרות`.
//
// # Two settings, because they are two questions
//
// `girsa_app::session::Session` holds both — `language` for the seforim,
// `interface` for the window — and they are independent on purpose. A reader who
// learns in Hebrew and wants the buttons in English is ordinary; so is the
// reverse. `names.ts` owns the first. This owns the second.
//
// # One table, and a test that reads it
//
// Every string is a row with both languages, so adding a Hebrew word without its
// English is a type error rather than a hole a reader finds. `test/say.test.mjs`
// checks that neither column has a blank in it and that nothing outside this
// file hands `textContent` a bare Hebrew literal — the same shape of guard
// `sources.test.mjs` already uses for the sibling's name.
//
// The keys read as what the thing *is*, not as its Hebrew, so that a row can be
// reworded in either language without the call site moving.

import { KSAV, withPrefix, type Language } from "./names.ts";

/** One string, in both languages. */
type Both = readonly [he: string, en: string];

/**
 * Everything the window says.
 *
 * Grouped the way the window is: chrome, then one block per panel. Ordered so
 * the diff of a new panel is one block rather than thirty scattered lines.
 */
const WORDS = {
  // --- the toolbar and the tab strip ---------------------------------------
  openSefer: ["פתח ספר", "Open a sefer"] as Both,
  openSeferKey: ["Ctrl+O", "Ctrl+O"] as Both,
  // A sefer into the tab you are already in, beside what you are reading —
  // rather than into a tab of its own, which is what a click has always meant.
  openHere: [
    "פתח כאן, לצד מה שאני קורא (Ctrl+Enter)",
    "open here, beside what I am reading (Ctrl+Enter)",
  ] as Both,
  browseShelf: ["מדף", "Shelf"] as Both,
  browseShelfWhy: ["עיין במדף (Ctrl+B)", "browse the bookcase (Ctrl+B)"] as Both,
  search: ["חפש", "Search"] as Both,
  searchWhy: ["חפש בכל המדף (Ctrl+F)", "search the whole shelf (Ctrl+F)"] as Both,
  write: ["כתוב", "Write"] as Both,
  writeWhy: ["פתח את הכתיבה (Ctrl+E)", "open the writing drawer (Ctrl+E)"] as Both,
  lane: ["לשון סמוכה", "Adjacent"] as Both,
  laneWhy: [
    "הלשון הסמוכה — מציאה לפי עניין (Ctrl+Shift+L)",
    "the adjacent lane — found by meaning (Ctrl+Shift+L)",
  ] as Both,
  queue: ["טעויות", "Suspects"] as Both,
  queueWhy: ["תור שגיאות הסריקה (Ctrl+J)", "the scanning-error queue (Ctrl+J)"] as Both,
  settings: ["הגדרות", "Settings"] as Both,
  settingsWhy: ["הגדרות הקריאה (Ctrl+,)", "reading settings (Ctrl+,)"] as Both,
  newTab: ["＋", "＋"] as Both,
  newTabWhy: [
    "לשונית חדשה — גם לספר שכבר פתוח",
    "a new tab — for a sefer already open too",
  ] as Both,
  closeTab: ["סגור", "Close"] as Both,
  renameTab: ["שנה שם ללשונית", "Rename this tab"] as Both,
  seforimCount: ["ספרים", "seforim"] as Both,
  inBrowser: ["דפדפן, נתוני דוגמה", "browser — sample data"] as Both,
  smaller: ["א−", "A−"] as Both,
  bigger: ["א+", "A+"] as Both,
  smallerWhy: ["הקטן את הקריאה (Ctrl+-)", "smaller reading (Ctrl+-)"] as Both,
  biggerWhy: ["הגדל את הקריאה (Ctrl+=)", "larger reading (Ctrl+=)"] as Both,
  /** The three pointing settings, named after what they draw. The control
   * prints the **next** one, because that is what clicking it does. */
  /** How the shemos are written. Two settings, and the second is the one a
   * reader asked for by name: a page that can be thrown away. */
  /** The daf yomi button, and what it says when the sefer is not here. */
  /** The find bar — Ctrl+F, in the sefer in front of you. */
  /** Printing (Ctrl+P): the section in front of you, on paper. */
  /** Checking for a newer Girsa — a button, never a background check. */
  update: ["בדוק אם יש גרסה חדשה", "Check for a newer version"] as Both,
  updateWhy: [
    "בודק פעם אחת מול GitHub. שום דבר לא נבדק מאליו.",
    "Asks GitHub once. Nothing is checked on its own.",
  ] as Both,
  updateChecking: ["בודק…", "checking…"] as Both,
  updateNewest: ["זו הגרסה האחרונה ({running})", "This is the newest ({running})"] as Both,
  updateFound: ["יש גרסה חדשה: {latest} (כאן {running})", "There is a newer one: {latest} (this is {running})"] as Both,
  updateGet: ["פתח את דף ההורדה", "Open the download page"] as Both,
  updateAbout: [
    "אין עדכון אוטומטי, ובכוונה: התקנה של קובץ מהרשת דורשת אימות חתימה, וזה חלק מתהליך ההוצאה ולא מהיישום.",
    "Nothing installs itself, deliberately: running a file off the internet needs a signature verified, and that belongs to the release process rather than to the application.",
  ] as Both,
  /** Named arrangements (Ctrl+Shift+D). */
  desks: ["שולחנות", "Arrangements"] as Both,
  desksWhy: ["סדרי ספרים ששמרת בשם", "Layouts you saved under a name"] as Both,
  desksNone: ["עדיין לא שמרת שולחן", "You have not named an arrangement yet"] as Both,
  desksCount: ["שולחנות: {desks}", "Arrangements: {desks}"] as Both,
  desksName: ["שם השולחן", "The arrangement's name"] as Both,
  desksNamePlaceholder: ["למשל: סוגיית הכל שוחטין", "e.g. hilchos shechitah"] as Both,
  desksKeep: ["שמור כפי שהוא", "Keep it as it is"] as Both,
  desksKeepWhy: [
    "שמור את סדר הספרים שעל המסך תחת השם הזה",
    "Save the layout on screen under this name",
  ] as Both,
  desksOpenWhy: ["שב לשולחן הזה", "Sit down at this one"] as Both,
  desksForget: ["הסר", "forget"] as Both,
  desksHolds: [
    "לשוניות: {tabs} · ספרים: {seforim}",
    "Tabs: {tabs} · Seforim: {seforim}",
  ] as Both,
  desksAbout: [
    "מעבר לשולחן אחר שומר קודם את הסדר שעל המסך, כך שאין מה להפסיד.",
    "Moving to another arrangement saves the one on screen first, so there is nothing to lose.",
  ] as Both,
  /** The links panel, opened out: the whole line rather than its first words. */
  linksReadHere: ["הראה את הלשון", "Show what it says"] as Both,
  linksReadHereWhy: [
    "פתח את השורה כאן, בלי לצאת מהדף",
    "Open the line here, without leaving the page",
  ] as Both,
  /** The mefarshim printed on the page with the sefer — one press. The button's
   * face is their names, so this is only the tooltip. */
  openTheUsualWhy: [
    "פתח את המפרשים שעל הדף",
    "Open the mefarshim printed on the page",
  ] as Both,
  print: ["הדפס", "Print"] as Both,
  printWhy: ["הדפס את הסימן או את העמוד שאתה עומד בו", "Print the section you are standing in"] as Both,
  printNothingOpen: ["אין ספר פתוח להדפיס", "no sefer is open to print"] as Both,
  printReady: ["מוכן להדפסה", "ready to print"] as Both,
  findHere: ["חפש בספר הזה", "Find in this sefer"] as Both,
  findHereWhy: ["חפש מילה או ביטוי בספר הפתוח", "Find a word or a phrase in the open sefer"] as Both,
  findHerePlaceholder: ["מילה או ביטוי", "a word or a phrase"] as Both,
  findHereNext: ["הבא", "next"] as Both,
  findHerePrevious: ["הקודם", "previous"] as Both,
  findHereClose: ["סגור", "close"] as Both,
  findHereNone: ["אין", "none"] as Both,
  /** Said beside the count when the list was cut and the count was not. */
  findHereCut: ["(מוצגים הראשונים)", "(first shown)"] as Both,
  findHereNothingOpen: ["אין ספר פתוח לחפש בו", "no sefer is open to find in"] as Both,
  /** Why the mareh makom mode is grey on the find bar's chip row, and where it
   * is not grey. See `findhere.ts` — the engine returns `Answer::Cited` here
   * and the bar has nowhere to put a landing. */
  findHereNoCitation: [
    "מראה מקום הוא קפיצה למקום אחר — חפש בכל המדף (Ctrl+Shift+F)",
    "a mareh makom is a jump somewhere else — search the shelf (Ctrl+Shift+F)",
  ] as Both,
  dafYomi: ["דף היומי", "Daf Yomi"] as Both,
  dafYomiWhy: ["פתח את דף היומי", "Open today's daf"] as Both,
  dafYomiNotHere: ["המסכת אינה על המדף", "that masechta is not on this shelf"] as Both,
  /** The daf turns over at nightfall and the button turns it over at midnight,
   * so the next one is named beside it rather than guessed at. */
  tomorrow: ["מחר", "tomorrow"] as Both,
  settingsDayTurns: ["מאיזו שעה נחשב היום הבא", "When the day turns over"] as Both,
  dayTurnsAbout: [
    "הדף מתחלף בצאת הכוכבים, וצאת הכוכבים תלוי במקום שאתה נמצא בו — וגרסא אינה שואלת היכן אתה. השעה כאן היא הערכה ולא חשבון. דף המחר מופיע ממילא לצד דף היום.",
    "The daf turns over at nightfall, and nightfall depends on where you are standing — which Girsa does not ask. This hour is an approximation and not a calculation. Tomorrow's daf is named beside today's either way.",
  ] as Both,
  settingsShemos: ["שמות הקודש", "Holy names"] as Both,
  shemosAsWritten: ["ככתבם", "as written"] as Both,
  shemosChanged: ["בשינוי אות", "with a letter changed"] as Both,
  shemosAbout: [
    "בשינוי אות — יקוק, אלקים, קל — כדי שאפשר יהיה לגנוז דף מודפס. חל על הדף, על תוצאות החיפוש, על ההעתקה ועל הייצוא. שמות שאין להם שינוי של אות אחת נשארים ככתבם.",
    "With a letter changed — יקוק, אלקים, קל — so a printed page may be discarded. Applies to the page, to search results, to what you copy and to an export. Shemos with no one-letter convention are left as written.",
  ] as Both,
  pointingFull: ["ניקוד וטעמים", "nikud and te'amim"] as Both,
  pointingNikud: ["ניקוד בלי טעמים", "nikud, no te'amim"] as Both,
  pointingPlain: ["בלי ניקוד", "no nikud"] as Both,
  pointingWhy: [
    "כמה מן הניקוד להראות (Alt+N) — לחיצה עוברת לבא",
    "how much of the pointing to draw (Alt+N) — click for the next",
  ] as Both,
  seforimIn: ["שמות הספרים", "Sefer names"] as Both,
  windowIn: ["שפת החלון", "Interface"] as Both,
  /**
   * The three citation styles, each shown by an example of itself.
   *
   * A row reading *full / short / English* asks the reader to guess what
   * three words mean before they can choose between them; the same row
   * carrying `אורח חיים סימן א׳ סעיף א׳` shows the answer. The examples are
   * `girsa_cite::CiteStyle`'s own — the doc comment on each variant — so the
   * label and the formatter cannot drift apart without somebody noticing.
   *
   * The Hebrew examples stay Hebrew in the English column, because that is
   * what the setting produces: a reader working in an English window still
   * gets `אורח חיים א׳, א׳` when they copy under `hebrew-short`, and a row
   * that transliterated the example would be a lie about the output.
   */
  citeHebrewFull: [
    "עברית מלאה — אורח חיים סימן א׳ סעיף א׳",
    "Hebrew, full — אורח חיים סימן א׳ סעיף א׳",
  ] as Both,
  citeHebrewShort: [
    "עברית מקוצרת — אורח חיים א׳, א׳",
    "Hebrew, short — אורח חיים א׳, א׳",
  ] as Both,
  citeEnglish: ["אנגלית — Orach Chayim 1:1", "English — Orach Chayim 1:1"] as Both,
  hebrew: ["עברית", "Hebrew"] as Both,
  english: ["English", "English"] as Both,
  sendToKsav: ["שלח", "Send"] as Both,
  sendToKsavWhy: [
    "Ctrl+Shift+C — שלח את הבחירה למסמך הפתוח",
    "Ctrl+Shift+C — send the selection to the open document",
  ] as Both,

  // --- a reading pane ------------------------------------------------------
  beside: ["לצד", "Beside"] as Both,
  links: ["קישורים", "Links"] as Both,
  linksWhy: [
    "מה מקושר לשורה שאתה עומד בה (Ctrl+L)",
    "what is linked to the line you are standing on (Ctrl+L)",
  ] as Both,
  exportSefer: ["ייצא", "Export"] as Both,
  exportWhy: [
    "כתוב את הספר לקובץ, עם התיקונים שלך — ובחר לאן",
    "write this sefer to a file with your corrections — and choose where",
  ] as Both,
  closePane: ["סגור", "Close"] as Both,
  closePaneWhy: ["סגור את הטור (Ctrl+W)", "close this column (Ctrl+W)"] as Both,
  /**
   * The line between two panes, and what it can do.
   *
   * > *"Tabs should be splittable in any way and movable, like we want in
   * > ksav."*
   *
   * The names are the arrangement each control **gives** you, not the one you
   * are in — the same convention the three toolbar state buttons follow, for
   * the reason written over `pointingWhy`.
   */
  dividerWhy: [
    "גרור כדי לשנות את הרוחב, או בחצים. הכפתורים מסובבים את החלוקה ומחליפים בין הצדדים",
    "drag to move it, or use the arrows. The buttons turn the split and change the sides over",
  ] as Both,
  splitStacked: ["שים זה מעל זה", "put them one above the other"] as Both,
  splitBeside: ["שים זה לצד זה", "put them side by side"] as Both,
  swapSplit: ["החלף בין הצדדים", "change the sides over"] as Both,
  /** The scroll link, named after what it does rather than after a participle. */
  linkScroll: ["גלילה משותפת", "Linked scroll"] as Both,
  unlinkScroll: ["גלילה נפרדת", "Own scroll"] as Both,
  /**
   * The two tooltips, named after the state they **describe**, which is not the
   * state the button beside them is labelled with.
   *
   * They were `linkScrollWhy` and `unlinkScrollWhy`, and each was shown under
   * the *other* one's label — because the label is the next state and the
   * tooltip is the current one. Both were right and the names were a trap: the
   * next person to line them up by their keys reintroduces finding 12.
   */
  scrollNowSharedWhy: [
    "הטור הזה זז עם הטור שהוא עוקב אחריו. לחץ כדי לנתק",
    "this column moves with the one it follows. Click to unlink",
  ] as Both,
  scrollNowOwnWhy: [
    "הטור הזה גולל לבדו. לחץ כדי לקשור אותו לטור שלצדו",
    "this column scrolls on its own. Click to link it to the one beside it",
  ] as Both,
  following: ["גלילה עם", "scrolling with"] as Both,
  nothingHere: ["אין כאן", "nothing here"] as Both,
  nothingHereWhy: [
    "אין בספר הזה מה שיושב על השורה הזאת",
    "nothing in this sefer sits on that line",
  ] as Both,
  markWhy: [
    "מפרשים שסימנת כתבו על השורה הזאת — לחץ",
    "mefarshim you ticked wrote on this line — click",
  ] as Both,
  /** How many of the ticked wrote on this line, for the hover on a marker that
   * carries a number rather than a diamond. */
  markHowMany: [
    "{n} מהמפרשים שסימנת כתבו על השורה הזאת — לחץ",
    "{n} of the mefarshim you ticked wrote on this line — click",
  ] as Both,
  /**
   * Said once in the header, in place of a marker on every line of the sefer.
   *
   * > *"Ticking a targum marks every line. 1,533 of Bereishis' 1,533."*
   *
   * A claim that holds on every line is a fact about the sefer, not about the
   * line. `marking` in `mefarshim.ts` decides; these are what it says.
   */
  markEveryLineOne: [
    "מפרש שסימנת כתב על כל שורה כאן — לחץ על שורה",
    "a mefaresh you ticked wrote on every line here — click a line",
  ] as Both,
  markEveryLine: [
    "{n} מהמפרשים שסימנת כתבו על כל שורה כאן — לחץ על שורה",
    "{n} of the mefarshim you ticked wrote on every line here — click a line",
  ] as Both,

  // --- the shelf -----------------------------------------------------------
  theShelf: ["המדף", "The shelf"] as Both,
  newShelf: ["מדף חדש", "New shelf"] as Both,
  newShelfWhy: ["פתח מדף תחת המדף המסומן", "make a shelf under the chosen one"] as Both,
  resetShelf: ["החזר לסדר המקורי", "Back to how it shipped"] as Both,
  resetShelfWhy: ["בטל את כל השינויים בסידור", "undo every change to the arrangement"] as Both,
  close: ["סגור", "Close"] as Both,
  /** The two buttons on a question. See `controls.ask` — these arrived when
   * the four `window.prompt` calls did not. */
  askOk: ["אישור", "OK"] as Both,
  askCancel: ["ביטול", "Cancel"] as Both,
  askNoteHint: [
    "Ctrl+Enter כדי לשמור",
    "Ctrl+Enter to keep it",
  ] as Both,
  esc: ["Esc", "Esc"] as Both,
  minimize: ["צמצם", "Minimise"] as Both,
  minimizeWhy: [
    "צמצם לפס צר. לחיצה עליו פותחת שוב",
    "shrink to a strip. Click the strip to open it again",
  ] as Both,
  reopen: ["פתח שוב", "Open again"] as Both,
  shelfHint: [
    "גרור ספר למדף אחר · לחיצה כפולה על שם מדף כדי לשנותו · גרור קובץ לחלון כדי להוסיף ספר משלך",
    "drag a sefer to another shelf · double-click a shelf's name to rename it · drop a file on the window to add your own",
  ] as Both,
  shelfReadOnly: ["דפדפן — הסידור כאן לקריאה בלבד", "browser — the arrangement is read-only"] as Both,
  shelfEmpty: ["אין ספרים במדף הזה", "nothing stands on this shelf"] as Both,
  shelfBelow: ["הספרים כאן יושבים במדפים שתחתיו", "the seforim here are on the shelves under it"] as Both,
  shelfOf: ["מתוך", "of"] as Both,
  /**
   * What the number beside a shelf's name counts — three claims, three
   * sentences.
   *
   * > *"`תנ״ך · 66` is a parent whose children are indented 14 px; it reads as
   * > a category with 66 seforim and nothing under it."*
   *
   * The number was always the total under the shelf, and the row never said so.
   * On תנ״ך that total is 66 and the shelf itself holds nothing, so the row
   * promised a list of sixty-six and clicking it produced an empty column. Same
   * number, three different things it can mean; `countedOn` in `shelf.ts`
   * decides which, and these are what it says.
   */
  shelfCountHere: ["ספרים על המדף הזה", "seforim on this shelf"] as Both,
  shelfCountUnder: [
    "ספרים במדפים שתחתיו — על המדף הזה עצמו אין ספרים",
    "seforim on the shelves under it — nothing stands on this shelf itself",
  ] as Both,
  shelfCountBoth: [
    "ספרים על המדף הזה ועל המדפים שתחתיו",
    "seforim on this shelf and on the shelves under it",
  ] as Both,
  /** The heading over a shelf whose seforim all stand on the shelves under it,
   * and the heading over the list of those shelves. */
  shelfUnderCount: ["במדפים שתחתיו", "on the shelves under it"] as Both,
  shelfUnderHeading: ["המדפים שתחתיו", "The shelves under it"] as Both,
  mine: ["שלי", "mine"] as Both,
  pinToTop: ["העלה לראש הרשימה", "move to the top of the list"] as Both,
  looseSeforim: ["הספרים שעומדים על המדף הזה עצמו", "the seforim standing on this shelf itself"] as Both,
  editedShelf: ["שינית את המדף הזה", "you changed this shelf"] as Both,
  shelfName: ["שם המדף", "Shelf name"] as Both,
  newShelfNamed: ["שם המדף החדש", "What is the new shelf called?"] as Both,
  newShelfDefault: ["מדף חדש", "New shelf"] as Both,
  resetAsk: [
    "להחזיר את כל המדפים לסדר שהגיע עם הספרייה? הספרים שלך יישארו.",
    "Put every shelf back the way the library shipped? Your own seforim stay.",
  ] as Both,

  // --- the picker and the mefarshim door -----------------------------------
  filterList: ["סינון הרשימה", "Filter the list"] as Both,
  whatIsOpen: ["פתוחים עכשיו", "Open now"] as Both,
  recentlyRead: ["נקראו לאחרונה", "Read recently"] as Both,
  readingNow: ["כאן אתה קורא", "you are here"] as Both,
  /** *Open*, the state — not `open`, which is the button that opens something. */
  isOpen: ["פתוח", "open"] as Both,
  startTyping: ["התחל להקליד שם של ספר", "start typing a sefer's name"] as Both,
  noSuchSefer: ["אין ספר בשם הזה", "no sefer is called that"] as Both,
  nothingBeside: ["אין ספר שהחיבור מעיד עליו — חפש אחד", "the corpus places nothing here — search for one"] as Both,
  mefarshimOf: ["מפרשים", "Mefarshim"] as Both,
  tickWhy: [
    "סמן כדי לראות מה כתב על השורות של הספר",
    "tick to see what it says on this sefer's lines",
  ] as Both,
  tickName: ["סמן את", "Tick"] as Both,
  openChosen: ["פתח את המסומנים", "Open the ticked ones"] as Both,
  openChosenWhy: [
    "פתח כל מפרש שסימנת בטור משלו, לצד הספר",
    "open every ticked mefaresh in a column of its own, beside the sefer",
  ] as Both,
  doorNone: ["אין מפרשים מוצהרים על הספר הזה", "no declared mefarshim on this sefer"] as Both,
  doorSome: ["מפרשים על הספר הזה", "mefarshim on this sefer"] as Both,
  doorWhy: ["פתח ספר בטור שלצדו (Ctrl+\\)", "open a sefer in the column beside it (Ctrl+\\)"] as Both,
  tickSomebody: [
    "סמן מפרשים ברשימה כדי לראות מה כתבו על השורה",
    "tick mefarshim in the list to see what they wrote on a line",
  ] as Both,
  othersWroteHere: ["כתבו כאן מפרשים שלא סימנת", "mefarshim you have not ticked wrote here"] as Both,
  nobodyWroteHere: ["אין מפרש שכתב על השורה הזאת", "no mefaresh wrote on this line"] as Both,
  noMefarshimAtAll: [
    "אין מפרשים על הספר הזה בגרסה שלך",
    "no mefarshim on this sefer in your copy",
  ] as Both,
  /** What else is behind the door that its count does not promise — the
   * seforim running in this one's order, the sefer this one comments on, and
   * the ones joined by edges alone. Three headings in the list, so three
   * phrases in the tooltip. */
  doorAlso: ["ועוד", "also"] as Both,
  doorAlongside: ["על סדר הספר", "in this sefer's order"] as Both,
  doorBase: ["הספר שעליו נכתב", "the sefer it comments on"] as Both,
  doorLinked: ["מקושרים", "joined by links"] as Both,
  mefarshimOn: ["מפרשים על", "mefarshim on"] as Both,
  lines: ["שורות", "lines"] as Both,
  tickedNobody: ["לא סימנת אף אחד", "you have ticked nobody"] as Both,
  tickedN: ["סימנת", "you have ticked"] as Both,
  linksCounted: ["קישורים", "links"] as Both,
  /**
   * What a row says this sefer **is** to the one you are reading.
   *
   * `girsa_app::shelf::Related`'s three names, said here. They used to be
   * `Related::said()` and `Related::why()` — a Hebrew label and an English
   * hover sentence, both composed in Rust — so an English window drew `פירוש`
   * on every declared commentary and a Hebrew window put *the corpus declares
   * this a commentary on what you are reading* behind the hover. Both
   * languages wrong, in opposite directions, on the same row. The name crosses
   * the wire now; the words are here, like every other word.
   */
  relatedOn: ["פירוש", "commentary"] as Both,
  relatedOnWhy: [
    "הקטלוג מצהיר שזה פירוש על מה שאתה קורא",
    "the catalogue declares this a commentary on what you are reading",
  ] as Both,
  relatedBase: ["הספר עצמו", "the sefer itself"] as Both,
  relatedBaseWhy: [
    "מה שאתה קורא הוא פירוש על הספר הזה",
    "what you are reading is a commentary on this sefer",
  ] as Both,
  relatedAlongside: ["על סדר הספר", "in this sefer's order"] as Both,
  relatedAlongsideWhy: [
    "ספר בפני עצמו, ההולך על סדר מה שאתה קורא",
    "its own sefer, following the order of what you are reading",
  ] as Both,
  /**
   * The rows the graph found and the catalogue never declared.
   *
   * This said `מפרש` where a declared commentary said `פירוש` — two words a
   * reader takes for synonyms, carrying the one distinction in the list they
   * cannot see. The label says what the claim **rests on** now, which is the
   * same move the links panel makes for every row it draws.
   */
  onlyLinked: ["לפי הקישורים", "by the links"] as Both,
  onlyLinkedWhy: [
    "הגרף מציב את דברי הספר הזה על שורות של מה שאתה קורא — הקטלוג אינו מצהיר כלום",
    "the link graph places this sefer's comments on lines of what you are reading — the catalogue declares nothing",
  ] as Both,
  /** Which corpus a row came from, shown **only** where two rows in the same
   * list would otherwise read as one sefer. See `picker.ts`. */
  fromSefaria: ["ספריא", "Sefaria"] as Both,
  fromOtzaria: ["אוצריא", "Otzaria"] as Both,
  fromMine: ["שלך", "yours"] as Both,

  // --- searching -----------------------------------------------------------
  searchBox: ["חיפוש בכל המדף", "Search the whole shelf"] as Both,
  /**
   * **The box says the second thing it does, with an example of it.**
   *
   * > *"The guide talks about a search box where you can write טור סימן א. I
   * > can't find anything like that."*
   *
   * There is no such box and there was never going to be one: this box reads a
   * mareh makom as a place and goes there, which is the better answer than a
   * second box beside it. But the promise lived in `start-here.md` and in
   * `searchNothingAsked` — a line printed on an *empty* panel, which a reader
   * who typed something has already scrolled past — and the box itself said
   * only *search the whole shelf*. A reader who came looking for the feature by
   * name found a search box, which is what it said it was.
   *
   * The example is the guide's own, so somebody holding the two side by side
   * sees the same string twice.
   */
  searchPlaceholder: [
    "חפש בכל המדף — או מראה מקום: טור או״ח סימן א",
    "search the whole shelf — or a mareh makom: שבת לא.",
  ] as Both,
  keepQuery: ["שמור", "Keep"] as Both,
  keepQueryWhy: ["שמור את השאלה הזאת", "keep this question"] as Both,
  results: ["תוצאות", "Results"] as Both,
  narrowResults: ["צמצום התוצאות", "Narrowing the results"] as Both,
  whatWasFound: ["מה נמצא", "What was found"] as Both,
  page: ["עמוד", "page"] as Both,
  pageOf: ["מתוך", "of"] as Both,
  more: ["עוד", "More"] as Both,
  undo: ["בטל", "Undo"] as Both,
  readThem: ["קרא אותם", "Read them"] as Both,
  wholeShelf: ["כל המדף", "the whole shelf"] as Both,
  /** What the panel says when it opens showing the search you ran last time. */
  previously: [
    "התוצאות של החיפוש הקודם — הקש Enter כדי לחפש שוב",
    "the results of your last search — press Enter to search again",
  ] as Both,
  scope: ["היכן לחפש", "Where to look"] as Both,
  scopeWhy: [
    "הוסף מדפים וספרים, או הוצא אותם. נשמר בין חיפוש לחיפוש",
    "add shelves and seforim, or take them out. Kept between searches",
  ] as Both,
  scopeEverything: ["כל המדף — לא צומצם כלום", "the whole shelf — nothing narrowed"] as Both,
  scopeAdd: ["הוסף", "Add"] as Both,
  scopeTake: ["הוצא", "Take out"] as Both,
  scopeDrop: ["הסר את השורה הזאת", "remove this row"] as Both,
  scopeFindSefer: ["חפש ספר להוסיף", "Find a sefer to add"] as Both,
  /** The one line that says what the whole panel comes to. Two numbers, because
   * a list of eight labels does not tell a reader whether they are about to
   * search four seforim or four thousand. */
  scopeCounted: ["מחפש ב: {n} מתוך {all} ספרים", "Searching: {n} of {all} seforim"] as Both,
  /** The row a box is on, and what a click on it does. */
  scopeTick: ["סמן — חפש כאן", "tick — search here"] as Both,
  scopeUntick: ["בטל — אל תחפש כאן", "untick — do not search here"] as Both,
  /** Both the *check everything* and the *clear all* a reader asked for: with
   * nothing ticked the search runs over every sefer, so the two are one click. */
  scopeClear: ["סמן הכל", "Tick everything"] as Both,
  scopeClearWhy: [
    "נקה את כל מה שסומן — חיפוש בכל המדף",
    "clear every tick — search the whole shelf",
  ] as Both,
  /** Opening a shelf down to the seforim standing on it, so a single sefer can
   * be ticked without knowing its name. */
  scopeOpenShelf: ["הצג את הספרים שעל המדף הזה", "show the seforim on this shelf"] as Both,
  scopeLoading: ["רגע…", "one moment…"] as Both,
  scanBadge: ["סריקה", "scan"] as Both,
  scanGuessedWhy: [
    "נקרא במכונה — יש לבדוק מול הצילום",
    "read by a machine — check it against the photograph",
  ] as Both,
  scanEmbeddedWhy: ["המילים מתוך הקובץ עצמו", "the words come from the file itself"] as Both,
  facetShelf: ["מדף", "Shelf"] as Both,
  facetEra: ["תקופה", "Era"] as Both,
  facetAuthor: ["מחבר", "Author"] as Both,
  facetSefer: ["ספר", "Sefer"] as Both,
  facetLink: ["קישור", "Link"] as Both,
  facetTag: ["תג", "Tag"] as Both,
  andMore: ["ועוד", "and"] as Both,
  narrowTo: ["צמצם ל־", "narrow to"] as Both,
  takeOut: ["הוצא את", "take out"] as Both,
  /** The same missing cache as [`linksNoInbound`], one panel over, and it was
   * worded the same way. Same answer: the fact here, the command on the hover. */
  linkFacetUnbuilt: [
    "אי אפשר לצמצם לפי קישור — לא נבנה מה שסופר אותם",
    "cannot narrow by link — what counts them was never built",
  ] as Both,
  linkFacetUnbuiltWhy: [
    "שלב 5 בהקמת הספרייה: girsa-link-types corpus personal, ואחריו בניית האינדקס מחדש",
    "step 5 of setting up the library: girsa-link-types corpus personal, then rebuild the index",
  ] as Both,
  uncatalogued: [
    "תוצאות בספרים שאינם בקטלוג — המדפים שלמעלה חסרים אותן",
    "results in seforim the catalogue does not have — the shelves above are short by that many",
  ] as Both,
  nameTheQuery: ["איך לקרוא לשאילתה?", "What should this question be called?"] as Both,
  nothingToKeep: ["אין מה לשמור — תיבת החיפוש ריקה", "nothing to keep — the search box is empty"] as Both,

  // --- the chips (spec.md §9.5, finding 7) ---------------------------------
  //
  // Every one of these used to be an English string literal in
  // `girsa-search/src/chips.rs`, because the chip's *name* was also its API key
  // and could not be translated without changing the protocol. It is two fields
  // now — `Chip.key` is the wire, `Choice.label` is the wire's own English, and
  // what a reader sees comes from here like every other word in this window.
  //
  // A fully Hebrew window used to open Ctrl+F on:
  //
  //     torat emet ▾   whole shelf ▾   the word ▾   anywhere in a segment ▾
  //     nothing to search for
  chipMode: ["איך לחפש", "How to search"] as Both,
  chipMatch: ["מה נחשב מילה", "What counts as the word"] as Both,
  chipTogether: ["איך המילים עומדות", "How the words stand"] as Both,
  chipInstrument: ["איזה כלי", "Which instrument"] as Both,
  modeToratEmet: ["כלשונו", "as written"] as Both,
  modeSmart: ["מרחיב", "widening"] as Both,
  modeRegex: ["תבנית", "pattern"] as Both,
  modeCitation: ["מראה מקום", "a mareh makom"] as Both,
  // *Instruments* is opaque in either language, and it is not one thing: it is
  // gematria, rashei tevos, sofei tevos, atbash and dilug. Named after what a
  // reader is doing with them.
  modeInstruments: ["חשבונות ורמזים", "gematria and remazim"] as Both,
  matchWord: ["המילה עצמה", "the word itself"] as Both,
  matchContains: ["מילה שיש בה האותיות", "a word containing these letters"] as Both,
  matchLetters: ["האותיות האלה, לפי הסדר", "these letters, in this order"] as Both,
  togetherAnywhere: ["בכל מקום בקטע", "anywhere in a segment"] as Both,
  togetherPhrase: ["זו אחר זו", "one after the other"] as Both,
  /** `{words}` is filled in with the distance the chip is set to. */
  togetherNear: [
    "בתוך {words} מילים זו מזו",
    "within {words} words of each other",
  ] as Both,
  instrumentGematria: ["גימטריא", "gematria"] as Both,
  instrumentRashei: ["ראשי תיבות", "rashei tevot"] as Both,
  instrumentSofei: ["סופי תיבות", "sofei tevot"] as Both,
  instrumentAtbash: ["אתב״ש", "atbash"] as Both,
  instrumentDilug: ["דילוג", "dilug"] as Both,
  /** The era facet's largest row: the seforim whose era nobody recorded. */
  noEraRecorded: ["לא נרשמה תקופה", "no era recorded"] as Both,

  // --- what the header says, and what a zero says --------------------------
  //
  // The header was composed in Rust, in English, and echoed the query back with
  // its **final letters folded** — `מאימתי קורינ את שמע` — which reads as a
  // typo the reader did not make. It is composed here now, from the chip row
  // that actually ran and from what the reader actually typed.
  askedFor: ["חיפשת", "Searched for"] as Both,
  /** A result row has three landings and used to advertise none of them, so
   * *"Ctrl+Enter is supposed to open in the same tab"* was true of the guide
   * and not of the row. The row says it now, on hover, in the order the
   * modifiers are held: nothing, Ctrl, Shift. */
  findOpenWhy: [
    "פתח — Ctrl בלשונית שאני קורא בה, Shift בלשונית משלו",
    "Open — Ctrl beside what I am reading, Shift in a tab of its own",
  ] as Both,
  /** A search with no hits used to be a bare `0` over an entirely blank panel. */
  foundNothing: ["לא נמצא כלום", "nothing was found"] as Both,
  foundNothingWhy: [
    "אפשר להרחיב את החיפוש בשורת הכפתורים שלמעלה, או להוסיף מדפים ב״היכן לחפש״",
    "widen the search with the chips above, or add shelves under “Where to look”",
  ] as Both,
  // The relaxation ladder (spec.md §9.6). Seven rungs, offered on a zero with
  // their counts worked out before the click and **nothing applied**. Their
  // names cross the wire; their words are here, because the offers were the one
  // thing on a zero-hit panel that was not blank and they were in English.
  rungNikud: ["בלי ניקוד", "drop nikud"] as Both,
  rungPrefixes: ["בלי אותיות השימוש", "peel the prefixes"] as Both,
  rungSpellings: ["כתיב מלא וחסר", "full and defective spelling"] as Both,
  rungGershayim: ["בלי גרשיים", "drop the gershayim"] as Both,
  rungAbbreviations: ["פתיחת ראשי תיבות", "expand abbreviations"] as Both,
  rungRoot: ["לפי השורש", "match the root"] as Both,
  rungProximity: ["בכל הקטע", "widen to the same passage"] as Both,

  /** Before anything has been typed: not a refusal, and not a zero. */
  searchNothingAsked: [
    "הקש מה לחפש — או מראה מקום, כדי ללכת לשם",
    "type what to look for — or a mareh makom, to go there",
  ] as Both,
  codeNoClipboard: [
    "אין לוח העתקה זמין — נסה שוב, ואם זה חוזר סגור ופתח את החלון",
    "no clipboard is available — try again, and if it keeps happening close and reopen the window",
  ] as Both,
  codeClipboardRefused: [
    "ההעתקה נדחתה על ידי המערכת — יישום אחר מחזיק בלוח. נסה שוב",
    "the system refused the copy — another application is holding the clipboard. Try again",
  ] as Both,
  codeNothingChosen: [
    "לא נבחר כלום — סמן קודם את מה שאתה מתכוון אליו",
    "nothing is chosen — highlight what you mean first",
  ] as Both,
  codeOffline: ["אין חיבור לרשת", "there is no network here"] as Both,
  codeRungApplied: [
    "החל — לחזרה, חפש שוב בלי ההצעה",
    "applied — search again without the offer to go back",
  ] as Both,
  codeAlreadyThere: [
    "כבר יש שם משהו — לא הוחלף",
    "something is already there — nothing was replaced",
  ] as Both,
  codeAlreadyWorking: [
    "הנתיב כבר עובד — עצור אותו קודם, או המתן",
    "the lane is already working — stop it first, or wait for it",
  ] as Both,
  codeNotKept: [
    "לסידור שעל המסך אין שם, ואין לאן להחזיר אותו",
    "the arrangement on screen has no name, so there is nowhere to put it back",
  ] as Both,
  codeWillNotSerialize: [
    "המקור לא נארז כראוי — העתק שוב, ואם זה חוזר דווח על השורה",
    "the source would not pack — copy again, and if it keeps happening report the line",
  ] as Both,

  linked: ["מקושר", "linked"] as Both,
  /** Two columns keeping step because the reader put them side by side, not
   * because the corpus joins them. Said out loud so a column that moves is
   * never read as a column making a claim. */
  roughly: ["בקירוב", "roughly"] as Both,
  roughlyWhy: [
    "אין קשר בין שני הספרים, אז שתי העמודות נשארות באותו מקום יחסי — ולא מול השורה",
    "nothing joins these two seforim, so the two columns keep the same place in each — not the same line",
  ] as Both,

  // --- the links panel -----------------------------------------------------
  linksTitle: ["קישורים", "Links"] as Both,
  // What the panel answers, on the screen rather than on a tooltip. The reader,
  // asked what he could see in it: *"All of it — I don't know what I'm looking
  // at."* And plainer: *"idk what links is."*
  linksAbout: [
    "כל מקום בספרייה שקשור לשורה שאתה עומד עליה — מי מצטט אותה ולאן היא מפנה. כל שורה אומרת מאין הקישור בא וכמה לסמוך עליו.",
    "every place in the library joined to the line you are standing on — who quotes it and what it points to. Each row says where the link came from and how much to trust it.",
  ] as Both,
  linksNone: ["אין קישורים", "no links"] as Both,
  linksOnWords: ["על המילים שסימנת", "on the words you highlighted"] as Both,
  linksReading: ["קורא…", "reading…"] as Both,
  linksAll: ["הכל", "All"] as Both,
  linksOpen: ["פתח את המקום", "open the place"] as Both,
  linksCurated: ["טענה על הטקסטים", "a claim about the texts"] as Both,
  linksUncurated: [
    "הקורפוס לא אמר איזה קשר — לא מוצג כעובדה",
    "the corpus did not say what kind of link — not shown as fact",
  ] as Both,
  linksShowWork: ["מאיפה הקישור, ומה עשית לו", "where the link came from, and what you did to it"] as Both,
  linksConfirm: ["אשר", "Confirm"] as Both,
  linksConfirmWhy: ["בדקתי — הקישור נכון", "I checked — this link is right"] as Both,
  linksReject: ["דחה", "Reject"] as Both,
  linksRejectWhy: ["בדקתי — הקישור שגוי", "I checked — this link is wrong"] as Both,
  linksUnreject: ["בטל דחייה", "Undo the rejection"] as Both,
  linksUnrejectWhy: ["החזר את הקישור", "put the link back"] as Both,
  linksKind: ["סוג הקישור", "Kind of link"] as Both,
  /** The pin, in its two states. It reads as what it **is**, and its tooltip
   * says what clicking it does — the same shape every toggle in this toolbar
   * uses. */
  /** The word in front of the lens row, which turns five nouns into a filter. */
  linksShow: ["הצג:", "Show:"] as Both,
  linksAllWhy: ["כל הקישורים — בלי סינון", "every link — nothing filtered out"] as Both,
  linksLensKeeps: ["משאיר:", "keeps:"] as Both,
  linksLensEras: ["תקופות", "eras"] as Both,
  linksLensMine: ["רק מה שנגעת בו", "only what you have touched"] as Both,
  linksLensAtLeast: ["ודאות {n}% ומעלה", "{n}% certainty and up"] as Both,
  linksDrawOpen: ["צייר קשר משלך", "Draw a link of your own"] as Both,
  linksFollowing: ["עוקב", "following"] as Both,
  linksFollowingWhy: [
    "הקישורים של השורה שאתה עומד בה — לחץ כדי לקבע כאן",
    "the links on the line you are standing on — click to stay on this one",
  ] as Both,
  linksStaying: ["מקובע", "pinned"] as Both,
  linksStayingWhy: [
    "מקובע לשורה הזאת — לחץ כדי לעקוב שוב",
    "pinned to this line — click to follow the reading again",
  ] as Both,
  linksKindWhy: ["קבע את סוג הקשר", "say what kind of link this is"] as Both,
  linksKindPick: ["סוג…", "kind…"] as Both,
  linksMoveHere: ["העבר לכאן", "Move it here"] as Both,
  linksMoveHereWhy: [
    "העבר את הקצה הזה לשורה שאתה עומד בה",
    "move this end of the link to the line you are standing on",
  ] as Both,
  linksPin: ["על מילים אלו", "On these words"] as Both,
  linksPinWhy: ["קבע שהקישור מדבר על מה שסימנת", "say the link is about the words you highlighted"] as Both,
  linksPinFirst: ["סמן קודם את המילים", "highlight the words first"] as Both,
  /** Drawing a link of your own, which is the one thing this panel could do in
   * Rust and could not do in the window. */
  linksDraw: ["צור קשר לכאן", "Draw a link to here"] as Both,
  linksDrawWhy: [
    "צור קשר חדש מן השורה שהחלון נפתח עליה אל השורה שאתה עומד בה",
    "draw a new link from the line this panel opened on to the line you are standing on",
  ] as Both,
  linksDrawFirst: [
    "עמוד בשורה אחרת, והקשר ייווצר אליה",
    "stand on a different line, and the link will be drawn to it",
  ] as Both,
  linksDrawKindFirst: ["בחר תחילה סוג קשר", "choose a kind of link first"] as Both,
  linksDrew: ["נוצר קשר", "link drawn"] as Both,
  /** Which of your chaburah folders hold the line you are on — the one thing
   * `yours` knows that no other call in this window does. */
  linksInFolders: ["בתיקיות שלך", "In your folders"] as Both,
  linksUndo: ["בטל", "Undo"] as Both,
  linksUndoWhy: ["בטל את מה שאמרת על הקישור", "undo what you said about this link"] as Both,
  linksOut: ["מכאן אל", "from here to"] as Both,
  linksIn: ["אל כאן מן", "to here from"] as Both,
  onWords: ["על מילים", "on words"] as Both,
  onWordsYours: ["על מילים (שלך)", "on words (yours)"] as Both,
  wasKind: ["היה", "was"] as Both,
  /**
   * **What is missing, in the words of the thing that is missing.**
   *
   * > *"the sentence of what I cannot see is hard to decipher."*
   *
   * It read: *no inbound cache — links into this line are not shown. Run
   * girsa-link-types.* Three implementation nouns and a command name, printed
   * at a person who came to read a sefer. A reader does not have a cache, has
   * no way to know that *inbound* is the half of the graph that points **at**
   * this line rather than out of it, and cannot run anything from inside the
   * window.
   *
   * So the sentence says the fact — half the list is here and which half — in
   * the terms the list itself is drawn in (`linksOut` is *from here to*,
   * `linksIn` is *to here from*), and the command moves behind the hover, which
   * is where [`othersWroteHere`] put the same kind of footnote. The step number
   * is the one on the setup screen, so the two screens agree about which step
   * it is.
   */
  linksNoInbound: [
    "מוצגים רק הקישורים היוצאים מן השורה הזאת. מי שמצביע עליה — מפרשים עליה, וספרים המצטטים אותה — חסר מן הרשימה.",
    "Only the links leading out of this line are listed. What points at it — mefarshim on it, seforim quoting it — is missing from the list.",
  ] as Both,
  linksNoInboundWhy: [
    "המטמון שקורא את הקישורים לאחור לא נבנה — שלב 5 בהקמת הספרייה: girsa-link-types corpus personal",
    "the cache that reads the links backwards was never built — step 5 of setting up the library: girsa-link-types corpus personal",
  ] as Both,

  // --- the writing drawer --------------------------------------------------
  documentName: ["שם המסמך", "Document name"] as Both,
  heading1: ["כותרת", "Heading"] as Both,
  quote: ["ציטוט", "Quote"] as Both,
  editorNote: ["הערה", "Note"] as Both,
  insertSource: ["מקור", "Source"] as Both,
  insertSourceWhy: ["הכנס את הבחירה שבספר", "insert what you highlighted in the sefer"] as Both,
  writingBox: ["מה שאתה כותב", "What you are writing"] as Both,
  /**
   * What the empty drawer says.
   *
   * It said nothing at all, which on a dark theme is a black rectangle with no
   * frame and no caret until you click it: *"typing works. Nothing tells you
   * that."* The sentence names the one thing a reader cannot guess — that the
   * buttons overhead put markup into this box — because a drawer that looked
   * like somewhere to write would still not say what it is for.
   */
  writingHint: [
    "כתוב כאן. הכפתורים שלמעלה מכניסים סימון — כותרת, ציטוט, הערת עורך, מראה מקום.",
    "Write here. The buttons above insert markup — a heading, a quotation, an editor's note, a mekor.",
  ] as Both,
  /** What the drawer says after a save. The path is on the hover: a reader
   * wants to know it was written, not where the application keeps its files. */
  desksReplace: ["כבר יש שולחן בשם {name}. להחליף את הסידור השמור בו?", "you already have a desk called {name}. Replace the arrangement it holds?"] as Both,
  desksReplaceYes: ["להחליף", "Replace"] as Both,
  desksNameThisFirst: ["לתת שם לסידור שעל המסך?", "name the arrangement on screen?"] as Both,
  desksNameThisWhy: [
    "הסידור הנוכחי לא נשמר בשום שולחן, ופתיחת שולחן אחר תחליף אותו. השאר ריק כדי להמשיך בלי לשמור.",
    "the arrangement you have now belongs to no desk, and sitting down at another one replaces it. Leave it empty to go ahead without keeping it.",
  ] as Both,
  /** Taking a correction back. Was a Hebrew string composed in `lib.rs`. */
  unfixed: ["הוחזר כפי שנדפס", "back to what is printed"] as Both,
  /** How many corrections went into an export — *label: number*, because
   * agreement at one is `girsa_plain::said`'s job and the window does not do
   * it by hand. See `93f4979`. */
  wroteFixes: ["תיקונים", "corrections"] as Both,
  writingSaved: ["נשמר", "saved"] as Both,
  // The rename that would land on a document you already have. Named rather
  // than generic: *something went wrong* to somebody about to lose a week of
  // writing is the silence this whole arrangement replaced.
  writingNameTaken: ["כבר יש לך משהו בשם {name}. להחליף אותו?", "you already have something called {name}. Replace it?"] as Both,
  writingNameTakenWhy: [
    "המסמך שכתוב שם יימחק והמסמך הנוכחי יישמר במקומו. המסמך בשם הישן יישאר.",
    "the document under that name is replaced by this one. The one under the old name stays where it is.",
  ] as Both,
  writingNameTakenYes: ["להחליף", "Replace"] as Both,
  // The highlight is in the *other* pane. Named rather than silent, because
  // the alternative the window used to take was to copy the line the reader was
  // standing on here — the wrong passage, with a right-looking citation on it.
  highlightElsewhere: [
    "ההדגשה נמצאת בעמודה אחרת — לחץ בה ואז העתק",
    "the highlight is in the other column — click in it, then copy",
  ] as Both,
  nothingChosen: ["לא נבחר כלום בספר", "nothing is highlighted in the sefer"] as Both,
  saveACopy: ["שמור עותק…", "Save a copy…"] as Both,
  saveACopyWhy: [
    "כתוב עותק של המסמך לתיקייה שתבחר",
    "write a copy of this document into a folder you choose",
  ] as Both,
  chooseFolder: ["בחר תיקייה", "Choose a folder"] as Both,
  wrote: ["נכתב", "written"] as Both,

  // --- sentences with a hole in them ---------------------------------------
  //
  // Every row below was a template literal typed into a module, and every one
  // of them printed Hebrew into an English window. They were written outside
  // the table because the table held strings and these are sentences with a
  // number, a name or a filename in the middle — see `fill`, which is what
  // makes a hole a thing the table can hold.
  //
  // The word order is deliberately not the same in the two columns. *3 of 8
  // pages read* and *3 מתוך 8 עמודים נקראו* do not put the numbers in the same
  // places, which is exactly what a sentence spliced together at a call site
  // can never do.

  /** What was being attempted, for `trouble.ts`'s `DOING` table. */
  doingReachKsav: ["הקשר עם {ksav}", "reaching {ksav}"] as Both,
  doingSendToKsav: ["השליחה {ksav}", "sending to {ksav}"] as Both,

  /** Refusals whose sentence has to name what was being attempted. The rest
   * of `Code` needs no hole and sits with `codeNoIndex` above. */
  codeNoSuch: [
    "{doing} נכשלה — נתבקש דבר שאינו קיים",
    "{doing} failed — something that does not exist was asked for",
  ] as Both,
  codeReadOnly: [
    "{doing} נכשלה — אין אפשרות לכתוב לשכבה האישית",
    "{doing} failed — the personal layer will not take a write",
  ] as Both,
  codeNoDesk: ["{ksav} אינו מחובר", "{ksav} is not connected"] as Both,
  codePostNotRunning: ["{ksav} אינו פועל", "{ksav} is not running"] as Both,
  codePostUnreachable: [
    "{doing} לא נענתה בזמן — ייתכן שהיישום נסגר שלא כשורה",
    "{doing} was not answered in time — the application may have closed badly",
  ] as Both,
  codePostRefused: [
    "{doing} נדחתה על ידי היישום שמעבר",
    "{doing} was refused by the application on the other side",
  ] as Both,

  /** The failures nobody in this product owns, matched by their own words. */
  familyRefused: [
    "{doing} נדחתה — אין מי שמאזין בצד השני",
    "{doing} was refused — nothing is listening on the other side",
  ] as Both,
  familyNoPermission: [
    "{doing} נמנעה — אין הרשאה לקובץ",
    "{doing} was prevented — there is no permission for the file",
  ] as Both,
  familyNoFile: [
    "{doing} נכשלה — הקובץ אינו נמצא במקום שנרשם",
    "{doing} failed — the file is not where it was recorded",
  ] as Both,
  familyBadAnswer: [
    "{doing} נכשלה — התשובה לא נקראה כראוי",
    "{doing} failed — the answer did not read properly",
  ] as Both,
  /** Unrecognised: name what was being done, and point at the one place to
   * look. Never the machine's string on its own. */
  troubleUnknown: [
    "{doing} נכשלה · פרטים בהצבה על ההודעה",
    "{doing} failed · details on hovering the message",
  ] as Both,
  // A8. The reader, on the chip: *"it says ksav is registered but not
  // answering. i have no clue if that is right."*
  //
  // It is right, and it said so in a way nobody could act on. `Presence::Stale`
  // means *there is a `ksav-endpoint.json` and nothing answered on the port it
  // names* — which is almost always the sibling having left its marker behind
  // when it closed, the same defect Girsa fixed on its own side in `run()`. So
  // the sentence names the likely cause and the remedy, and the transport's own
  // English stays on the hover where a developer's string belongs.
  //
  // **And short.** Driven in the real window, the first draft of this sentence
  // was 119 characters and the chip is a note in a toolbar: it wrapped to two
  // lines and took 450px of the top of the window. A status line that pushes
  // the tools aside is a worse answer than the one it replaced. The cause and
  // the remedy fit in half of that; the transport's own string is still on the
  // hover, where a developer's string belongs.
  ksavStale: [
    "{ksav} נסגר והשאיר רישום — פתח אותו שוב",
    "{ksav} closed and left a registration behind — open it again",
  ] as Both,

  /** The writing drawer and the send, which name the sibling. */
  writingOpenInKsav: ["פתח {ksav}", "Open in {ksav}"] as Both,
  writingOpenInKsavWhy: [
    "פתח את המסמך {ksav} עצמו",
    "open the document in {ksav} itself",
  ] as Both,
  writingHandedOver: ["נמסר {ksav}", "handed to {ksav}"] as Both,
  sendToKsavNamed: ["שלח {ksav}", "Send to {ksav}"] as Both,
  sentToKsavNamed: ["נשלח {ksav} — {what}", "sent to {ksav} — {what}"] as Both,

  /** The scan pane, which counts pages in two different orders. */
  scanNotInThis: ["{asked} אינו בסריקה הזאת", "{asked} is not in this scan"] as Both,
  scanPageOfFile: [
    "עמוד {page} מתוך {pages} בקובץ",
    "page {page} of {pages} in the file",
  ] as Both,
  scanPageNumbered: ["עמוד {page} בקובץ", "page {page} in the file"] as Both,
  scanNoTextNoEngine: [
    "עמוד {page} — אין בו טקסט, ואין מנוע OCR מותקן",
    "page {page} — no text on it, and no OCR engine installed",
  ] as Both,
  scanReadBy: ["נקרא — {by}", "read — {by}"] as Both,
  scanPagesRead: [
    "{read} מתוך {pages} עמודים נקראו",
    "{read} of {pages} pages read",
  ] as Both,
  scanAnchorShape: [
    "{text}: עוגן נכתב עמוד=דף",
    "{text}: an anchor is written page=daf",
  ] as Both,

  /** The adjacent lane, bringing a model down and choosing what it covers. */
  laneProgress: ["{what} · {done}", "{what} · {done}"] as Both,
  laneProgressOf: ["{what} · {done} מתוך {of}", "{what} · {done} of {of}"] as Both,
  laneBringOne: ["הבא את {name}", "Bring {name}"] as Both,
  laneTakeOut: ["הוצא {title}", "Take out {title}"] as Both,
  lanePutIn: ["הכנס {title}", "Put in {title}"] as Both,
  laneAndMore: ["ועוד {count}", "and {count} more"] as Both,
  laneOtherModel: [
    "{slug} — הווקטורים נעשו במודל אחר ואינם נקראים",
    "{slug} — the vectors were made with a different model and are not read",
  ] as Both,

  /** The scanning-error queue, and the corrections overlay. */
  suspectsInQueue: ["בתור: {count}", "In the queue: {count}"] as Both,
  yoursNoteAbout: [
    "פסקאות: {paragraphs} · מקומות: {places}",
    "Paragraphs: {paragraphs} · Places: {places}",
  ] as Both,
  yoursForgetNoteWhy: [
    "הקובץ, הספר והשורה בקטלוג",
    "the file, the sefer and the catalogue line",
  ] as Both,

  // --- settings ------------------------------------------------------------
  settingsReading: ["הקריאה", "Reading"] as Both,
  settingsTheme: ["ערכת צבעים", "Colours"] as Both,
  settingsHebrewFont: ["גופן עברי", "Hebrew font"] as Both,
  settingsLatinFont: ["גופן לטיני", "Latin font"] as Both,
  settingsSize: ["גודל הקריאה", "Reading size"] as Both,
  settingsMefarshimSize: ["גודל המפרשים", "Mefarshim size"] as Both,
  settingsLeading: ["רווח בין השורות", "Line spacing"] as Both,
  settingsMeasure: ["רוחב הטור (אותיות, 0 = בלי הגבלה)", "Column width (characters, 0 = no limit)"] as Both,
  settingsPointing: ["ניקוד", "Pointing"] as Both,
  settingsCite: ["ציון מקורות", "Citations"] as Both,
  settingsLanguage: ["שפה", "Language"] as Both,
  settingsKeys: ["מקשים", "Keys"] as Both,
  settingsKeysHint: [
    "לחץ על המקש הרצוי, או על ↺ כדי להחזיר",
    "press the key you want, or ↺ to put it back",
  ] as Both,
  settingsClose: ["סגור את ההגדרות", "close the settings"] as Both,
  themeWhy: [
    "ערכת הצבעים של החלון — כמו המערכת, בהיר או כהה. הכפתור אומר מה תקבל בלחיצה",
    "the window's colours — follow the system, light, or dark. The button says what clicking gets you",
  ] as Both,
  themeSystem: ["כמו המערכת", "Follow the system"] as Both,
  themeLight: ["בהיר", "Light"] as Both,
  themeDark: ["כהה", "Dark"] as Both,
  fontDefault: ["כמו שהוגדר בעיצוב", "as the design has it"] as Both,
  putBack: ["החזר ל", "put back to"] as Both,

  // --- what the window says back -------------------------------------------
  messages: ["הודעות", "Messages"] as Both,
  theReading: ["הקריאה", "The reading"] as Both,
  tabs: ["לשוניות", "Tabs"] as Both,
  tools: ["כלים", "Tools"] as Both,
  opened: ["נפתח", "opened"] as Both,
  copied: ["הועתק", "copied"] as Both,
  copiedLines: ["שורות", "lines"] as Both,
  sent: ["נשלח", "sent"] as Both,
  noLineHere: ["אין כאן שורה לכתוב עליה", "there is no line here to write about"] as Both,
  whatDoYouSay: ["מה יש לך לומר?", "What do you have to say?"] as Both,
  written: ["נכתב", "written"] as Both,
  bookmark: ["סימנייה", "bookmark"] as Both,
  marked: ["סומן", "marked"] as Both,
  kept: ["נשמר", "kept"] as Both,
  highlightFirst: ["סמן את המילה שצריכה תיקון", "highlight the word that needs correcting"] as Both,
  fixed: ["תוקן", "corrected"] as Both,
  variantNoted: ["נרשמה גרסה", "variant noted"] as Both,
  staleFixes: ["תיקונים לא חלו", "corrections did not land"] as Both,
  emptyHint: [
    "Ctrl+O — פתח ספר · Ctrl+B — עיין במדף · Ctrl+F — חפש · Ctrl+K — תקן",
    "Ctrl+O — open a sefer · Ctrl+B — browse · Ctrl+F — search · Ctrl+K — correct",
  ] as Both,
  /** The first screen when there is no corpus at all — finding 19. */
  noCorpusHint: [
    "אין כאן ספרים עדיין. אם כבר הורדת אוצר ספרים, אפשר להראות לגרסא איפה הוא.",
    "There are no seforim here yet. If you have already downloaded a corpus, you can show Girsa where it is.",
  ] as Both,
  chooseCorpus: ["בחר תיקיית ספרים", "Choose a folder of seforim"] as Both,
  /**
   * What it takes to get seforim, said out loud on the first screen.
   *
   * The installer carries the window and the five tools; it does not carry
   * 15 GB of Torah, and one leg of the road — the `.txt` libraries — is a
   * download this project does not automate. A folder picker on a machine with
   * no corpus is a question with no answer available, so the steps are on the
   * screen.
   *
   * **These are the same commands the README's table gives**, and they have
   * drifted from it before: the archive was named here without the platform it
   * is named with, and step 3 said `<otzaria>` for a week after the tools began
   * taking a list. Nothing checks that these agree, so when one moves the other
   * has to be moved by hand.
   */
  corpusStepsTitle: ["איך מביאים ספרים", "How to bring in seforim"] as Both,
  corpusStepFetch: [
    "1. הורד את ספריא — כ־3.4 ג׳יגה: girsa-fetch corpus\\sefaria",
    "1. Fetch Sefaria — about 3.4 GB: girsa-fetch corpus\\sefaria",
  ] as Both,
  corpusStepOtzaria: [
    "2. הורד ספריית טקסטים — למשל אוצריא: github.com/Sivan22/otzaria-library",
    "2. Get a library of .txt seforim — e.g. Otzaria: github.com/Sivan22/otzaria-library",
  ] as Both,
  corpusStepImport: [
    "3. בנה את המדף: girsa-import corpus <ספרייה>...",
    "3. Build the shelf: girsa-import corpus <library>...",
  ] as Both,
  /**
   * The two steps this screen did not have, and the reason it needs them.
   *
   * The list went 1 · fetch, 2 · Otzaria, 3 · import, 4 · index — and a reader
   * who did all four had a library with **no link graph at all**. No
   * `corpus/links/`, so no mefarshim on any sefer, so the מפרשים button reads
   * `לצד` on every daf and the panel `docs/start-here.md` step 2 is about
   * cannot be opened. The window's own message for it — *I have not been told*
   * — is the honest sentence for a missing cache and is indistinguishable, to
   * somebody who was never told to build one, from a sefer nobody wrote on.
   *
   * `girsa-link-import` writes the edges and `girsa-link-types` writes the
   * caches that read them backwards. Both are needed before a mefaresh appears,
   * and neither was on this screen or in the README's table.
   */
  corpusStepLinks: [
    "4. הקשרים בין הספרים: girsa-link-import corpus <ספרייה>...",
    "4. The links between them: girsa-link-import corpus <library>...",
  ] as Both,
  corpusStepLinkTypes: [
    "5. המטמונים שקוראים אותם לאחור: girsa-link-types corpus personal",
    "5. The caches that read them backwards: girsa-link-types corpus personal",
  ] as Both,
  /**
   * The step that is invisible when it is missing, which is why it is here.
   *
   * `girsa-companions` decides **which seforim are worth opening beside which**.
   * Without it the shelf offers the commentaries a *schema* declares — and
   * nothing in a `.txt` library declares a base text, so for those seforim it
   * offers none. The Encyclopedia Talmudit's own footnote volume sat unoffered
   * beside the entry it annotates with all 5,657 of its edges per letter
   * already imported, because this ran five days before the seforim did.
   *
   * Nothing refuses without it, which is exactly how a step stops being run.
   */
  corpusStepCompanions: [
    "6. אילו ספרים נפתחים זה לצד זה: girsa-companions corpus",
    "6. Which seforim open beside which: girsa-companions corpus",
  ] as Both,
  corpusStepIndex: [
    "7. לחיפוש — כ־4 ג׳יגה: girsa-index build index corpus personal",
    "7. For search — about 4 GB: girsa-index build index corpus personal",
  ] as Both,
  corpusStepsWhere: [
    "את הכלים מורידים מדף השחרור — girsa-tools-windows-x86_64.zip",
    "the tools are on the release page — girsa-tools-windows-x86_64.zip",
  ] as Both,
  chooseCorpusWhy: [
    "התיקייה שהייבוא כתב אליה — זו שיש בה works/index.jsonl",
    "the folder the import wrote to — the one with works/index.jsonl in it",
  ] as Both,
  /** What the wall of paths becomes: one hover, for whoever is debugging an
   * installation. */
  whereItLooked: ["איפה חיפשנו", "where it looked"] as Both,
  addedSeforim: ["נוסף", "added"] as Both,
  refusedSeforim: ["ולא נוסף", "and not added"] as Both,
  nothingAdded: ["לא נוסף כלום", "nothing was added"] as Both,
  readingFiles: ["קורא", "reading"] as Both,
  files: ["קבצים", "files"] as Both,

  // --- corrections ---------------------------------------------------------
  showingFixed: ["מתוקן", "corrected"] as Both,
  showingAsPrinted: ["כפי שנדפס", "as printed"] as Both,
  showingVariants: ["עם גרסאות", "with variants"] as Both,
  /** What the toast says after the round moves. **Not** the bare state word:
   * the button beside it is a promise about the next click and the toast is a
   * report of the last one, and for a while both were `מתוקן`. */
  showingNow: ["מוצג עכשיו: {what}", "now showing: {what}"] as Both,
  showingWhy: [
    "מתוקן — טעויות דפוס מתוקנות, גרסאות נרשמות בלבד · כפי שנדפס — הטקסט המקורי · עם גרסאות — גם ההגהות מוחלות (Ctrl+Shift+K)",
    "corrected — typos repaired, variants only noted · as printed — the original · with variants — emendations applied too (Ctrl+Shift+K)",
  ] as Both,

  // --- the deeper panels ---------------------------------------------------
  browserWriting: ["הכתיבה פועלת בחלון בלבד", "the writing drawer only works in the window"] as Both,
  browserCopy: ["העתקת מקור פועלת בחלון בלבד", "copying a source only works in the window"] as Both,
  browserScans: ["סריקות נפתחות בחלון בלבד", "scans only open in the window"] as Both,
  browserFixes: ["תיקונים פועלים בחלון בלבד", "corrections only work in the window"] as Both,
  browserLayer: ["השכבה שלך פועלת בחלון בלבד", "your own layer only works in the window"] as Both,
  browserBuffer: ["כתיבה פועלת בחלון בלבד", "writing only works in the window"] as Both,
  browserSearch: ["החיפוש פועל בחלון בלבד — הדפדפן קורא קבצי דוגמה סטטיים, ואין בהם אינדקס", "search only works in the window — the browser reads static sample files and there is no index in them"] as Both,
  browserLane: ["הלשון הסמוכה פועלת בחלון בלבד — הדפדפן קורא קבצי דוגמה סטטיים", "the adjacent lane only works in the window — the browser reads static sample files"] as Both,
  fixTitle: ["תיקון", "Correction"] as Both,
  fixAsPrinted: ["כפי שנדפס", "as printed"] as Both,
  fixTheWords: ["התיקון", "The correction"] as Both,
  fixKindOcr: ["טעות דפוס", "a misprint"] as Both,
  fixKindOcrWhy: ["השגיאה של הסורק — מתוקנת בגוף הטקסט", "the scanner's mistake — repaired in the body of the text"] as Both,
  fixKindGirsa: ["גרסה", "a variant"] as Both,
  fixKindGirsaWhy: ["כך גורס מישהו — נרשם ואינו מוחל", "somebody reads it this way — noted, not applied"] as Both,
  fixKeys: ["Enter — שמור · Esc — בטל", "Enter — save · Esc — cancel"] as Both,
  fixWasFixed: ["תוקן", "corrected"] as Both,
  fixRevert: ["החזר", "Revert"] as Both,
  fixRevertWhy: ["בטל את התיקון — הטקסט חוזר כפי שנדפס", "undo the correction — the text goes back to how it was printed"] as Both,
  laneNothingNear: ["אין דבר סמוך לזה במה שנכנס ללשון", "nothing in the lane is adjacent to this"] as Both,
  laneTitle: ["הלשון הסמוכה", "The adjacent lane"] as Both,
  laneDone: ["נגמר", "done"] as Both,
  laneNoShelf: ["אין מדף כאן", "there is no shelf here"] as Both,
  laneAbout: ["כתוב שורה כמו שאתה זוכר אותה, והלשון תמצא מקומות סמוכים לה בעניין — לא במילים. ", "write a line the way you remember it and the lane finds places adjacent to it in meaning — not in words. "] as Both,
  laneNotSearch: ["אין זו חיפוש ואינה פוסקת.", "it is not a search and it does not pasken."] as Both,
  laneOn: ["הדלק", "Turn it on"] as Both,
  laneOff: ["כבה", "Turn it off"] as Both,
  laneChooseModel: ["בחר תיקיית מודל…", "Choose a model folder…"] as Both,
  laneModelFolder: ["תיקיית המודל", "The model folder"] as Both,
  laneModelPath: ["נתיב אל המודל", "Path to the model"] as Both,
  laneAllowFetch: ["הרשה ל־Girsa להביא מודל מן הרשת", "let Girsa fetch a model over the network"] as Both,
  laneStarting: ["מתחיל…", "starting…"] as Both,
  laneTakeAll: ["הוצא את כל הספרייה", "Take the whole library out"] as Both,
  laneAddAll: ["הכנס את כל הספרייה", "Put the whole library in"] as Both,
  laneEmbed: ["הכנס לאינדקס", "Embed"] as Both,
  laneStop: ["עצור", "Stop"] as Both,
  fixNotApplied: [" (לא הוחל)", " (not applied)"] as Both,
  scanPrev: ["העמוד הקודם", "The page before"] as Both,
  scanNext: ["העמוד הבא", "The page after"] as Both,
  scanPageOrDaf: ["עמוד או דף", "Page or daf"] as Both,
  scanPageInFile: ["עמוד בקובץ", "page in the file"] as Both,
  scanGoToDaf: ["קפוץ לדף", "Go to a daf"] as Both,
  scanToDaf: ["לדף…", "to daf…"] as Both,
  scanNoSuchDaf: ["הדף הזה אינו בסריקה", "this daf is not in the scan"] as Both,
  scanNoDafHere: ["אין כאן דף", "no daf here"] as Both,
  scanNothingPrinted: ["לא נדפס על העמוד הזה דבר שאפשר לציין", "nothing is printed on this page that a mekor could name"] as Both,
  scanUnmapped: ["לא מופה — אין מראה מקום", "not mapped — no mareh makom"] as Both,
  scanSayOnce: ["אמור איזה עמוד הוא איזה דף, פעם אחת", "say which page is which daf, once"] as Both,
  scanRead: ["קרא", "Read"] as Both,
  scanReadWhy: ["קרא את המילים שבסריקה — אפשר להפסיק בכל רגע", "read the words in the scan — you can stop at any moment"] as Both,
  /** Correcting the engine on the photograph itself (W21). `scan_fix` was a
   * command with no door: a word plainly wrong on the page had nothing to
   * click. */
  scanFix: ["תקן מילה", "Correct a word"] as Both,
  scanFixWhy: [
    "הראה את כל המילים שהמנוע קרא — לחיצה על אחת מתקנת אותה",
    "show every word the engine read — click one to correct it",
  ] as Both,
  scanFixWord: ["מה כתוב כאן", "what it says here"] as Both,
  /** Highlighting on the photograph (W24 meeting W26). A page is one segment,
   * so a highlight on one could only ever be the whole page — what is written
   * down now is the ink, which is what survives a re-read. */
  scanMark: ["סמן מילים", "Highlight words"] as Both,
  scanMarkWhy: [
    "לחץ על המילה הראשונה ואחר כך על האחרונה — מה שנרשם הוא המקום על התצלום",
    "click the first word and then the last — what is written down is the place on the photograph",
  ] as Both,
  scanMarkMoved: [
    "סומן על: {was} · כעת שם: {says}",
    "marked on: {was} · now reads: {says}",
  ] as Both,
  // The transmission chain (spec.md §8, W28). The walk and the terminal tool
  // have existed since W28; nothing drew them, so the whole tier was a feature
  // you had to leave the window to see.
  // --- the table of contents (A3) ------------------------------------------
  // The arrow at the foot of an open mefarshim block (A14). Its own way out,
  // because the way in — clicking the line — is only reachable from the top,
  // and a long comment is longer than the window.
  saidShut: ["סגור כאן", "close here"] as Both,
  // The reader's own claim that two seforim keep the same order (A6).
  pairAlongside: ["על סדר הספר", "keeps the same order"] as Both,
  pairAlongsideWhy: [
    "אמור שהספר הזה הולך על סדר הספר שאתה קורא — כשהקישורים לא מספיקים כדי לדעת",
    "say this sefer keeps the order of the one you are reading — where the links do not show it",
  ] as Both,
  pairAlongsideOff: [
    "בטל: הספר הזה אינו על סדר הספר שאתה קורא",
    "take it back: this sefer does not keep the order of the one you are reading",
  ] as Both,
  // Moving a pane into another tab (A12).
  moveToTab: ["העבר ללשונית", "move to a tab"] as Both,
  moveToTabWhy: [
    "העבר את הטור הזה ללשונית אחרת, עם המקום שאתה עומד בו",
    "move this column into another tab, with the place you are at in it",
  ] as Both,
  moveToNewTab: ["ללשונית חדשה", "to a new tab"] as Both,
  moveStay: ["השאר כאן", "stay here"] as Both,
  // A15: pressed in a sefer with no marks in it. Said, rather than the key
  // doing nothing — a shortcut that is silent is one a reader stops pressing.
  noPlaceMarked: [
    "לא סימנת כאן מקום. Ctrl+Shift+H מסמן את המקום שאתה עומד בו.",
    "you have not marked a place here. Ctrl+Shift+H marks where you are standing.",
  ] as Both,
  tocTitle: ["תוכן הספר", "Contents"] as Both,
  tocWhy: [
    "תוכן הספר — סימנים, פרקים ודפים, לקפוץ ביניהם (Ctrl+Shift+T)",
    "the sefer's contents — simanim, perakim and dapim, to jump between (Ctrl+Shift+T)",
  ] as Both,
  tocAbout: [
    "כל מקום בספר הזה, לפי הכתובת שלו. השורה שאתה עומד בה מסומנת. סנן כדי למצוא סימן בלי לגלול.",
    "every place in this sefer, by its address. The one you are standing in is marked. Filter to find a siman without scrolling.",
  ] as Both,
  tocFilter: ["סנן", "filter"] as Both,
  tocReading: ["קורא…", "reading…"] as Both,
  tocNone: ["אין לספר הזה חלוקה להראות", "this sefer has no structure to show"] as Both,
  chainTitle: ["שלשלת המסירה", "The chain"] as Both,
  // The same answer as `linksAbout`, for the same reason: three good sentences
  // already existed here and all three were tooltips on the three buttons.
  chainAbout: [
    "הדרך שהשורה הזאת עשתה: קדימה — מי הביא אותה ומי פסק כמותה; אחורה — מאין באה; ולחוד, שני ראשונים שקראו אותה בשתי דרכים.",
    "the road this line travelled: forward — who quoted it and who ruled from it; back — where it came from; and, on its own, two rishonim who read it two ways.",
  ] as Both,
  chainWhy: [
    "מן השורה הזאת: לאן הגיעה, ומאין באה (Ctrl+Shift+M)",
    "from this line: where it went, and where it came from (Ctrl+Shift+M)",
  ] as Both,
  chainWalking: ["הולך…", "walking…"] as Both,
  chainForward: ["איך נעשה הלכה", "To halacha"] as Both,
  chainForwardWhy: [
    "מן השורה הזאת ולהלן — מי הביא אותה, ומי פסק כמותה",
    "from this line onward — who quoted it, and who ruled from it",
  ] as Both,
  chainBack: ["מאין בא", "Where it came from"] as Both,
  chainBackWhy: [
    "אחורה, אל המקור שממנו נלקח",
    "back, to the source it came from",
  ] as Both,
  chainForks: ["שתי גרסאות", "Two readings"] as Both,
  chainForksWhy: [
    "שני ראשונים שקראו שורה אחת בשתי דרכים, ומי שנצרך לשניהם",
    "two rishonim who read one line two ways, and who had to deal with both",
  ] as Both,
  chainNothing: ["אין כאן מה ללכת אחריו", "nothing this walk could follow"] as Both,
  chainNoForks: ["לא נמצאו כאן שתי גרסאות", "no two readings found here"] as Both,
  chainTally: [
    "שלשלות: {chains} · מהן מסירה לכל אורכן: {carried}",
    "Chains: {chains} · a transmission all the way: {carried}",
  ] as Both,
  /** Said once above the fork list, because it is true of every row in it. */
  chainForkCaveat: [
    "אין בגרף שום קשר שאומר שני ספרים חולקים — מה שיש הוא ששניהם קראו שורה אחת ושמאוחר מהם נצרך לשניהם. מקום להסתכל בו, לא מסקנה.",
    "nothing in the graph says two seforim disagree — what it says is that both read one line and a later one had to deal with both. A place to look, not a finding.",
  ] as Both,
  chainForkJoined: [
    "קשר מחבר את השניים במישרין — אחד מהם משיב על חבירו",
    "a link joins the two directly — one is answering the other",
  ] as Both,
  chainForkWitnesses: [
    "מאוחרים שנצרכו לשני הצדדים: {n}",
    "Later seforim that deal with both sides: {n}",
  ] as Both,
  /** The same count, when not one of them quotes both sides itself. A weaker
   * claim wearing the same word, and the count alone cannot tell them apart. */
  chainForkFarWitnesses: [
    "מאוחרים שנצרכו לשני הצדדים: {n} — הקרוב שבהם במרחק {steps}",
    "Later seforim that deal with both sides: {n} — the nearest at {steps}",
  ] as Both,
  chainSteps: ["צעדים: {n}", "Hops: {n}"] as Both,
  chainNoDate: ["בלי תאריך", "no date"] as Both,
  // --- what a kind of link is called ---------------------------------------
  //
  // > *"The latin text in links makes it hard to read. In general, it reads
  // > like an AI. Same with the chain."*
  //
  // The chain printed `girsa_link::EdgeType::as_str` — `comments-on`,
  // `parallel-to`, `references` — set left to right, in a right-to-left panel,
  // beside Hebrew sefer names. Those are wire keys. They are the right thing to
  // store, to index and to put in a shard, and they were never meant to be
  // read by a person in either language.
  //
  // The links panel already drew them from `girsa_app::links::kinds`, which
  // labels them — in Hebrew only, because when it was written the window had no
  // other language. It has two now, so the words are here, where every other
  // word the window says lives, and `linkKind` below is the one door to them.
  linkKindCommentsOn: ["מפרש", "comments on"] as Both,
  linkKindQuotes: ["מצטט", "quotes"] as Both,
  linkKindParaphrases: ["מביא", "brings"] as Both,
  linkKindCodifies: ["פוסק", "rules from"] as Both,
  linkKindDisputes: ["חולק", "disputes"] as Both,
  linkKindEmends: ["מגיה", "emends"] as Both,
  linkKindParallelTo: ["מקביל", "parallel to"] as Both,
  linkKindTranslates: ["מתרגם", "translates"] as Both,
  /** Half this graph, and it asserts nothing — see `chain.rs`'s own tally. */
  linkKindReferences: ["קשור", "connected to"] as Both,
  chainNoLabel: ["הקורפוס לא אמר כלום", "the corpus said nothing"] as Both,
  chainCorpusSaid: ["הקורפוס אמר: {label}", "the corpus said: {label}"] as Both,
  chainMine: ["שלך", "yours"] as Both,
  chainMineWhy: [
    "את הקשר הזה ציירת או אישרת בעצמך",
    "you drew or confirmed this link yourself",
  ] as Both,
  chainWeakest: ["החוליה החלשה: {kind}", "weakest link: {kind}"] as Both,
  chainFollowedAll: ["שום דבר לא נשאר בחוץ", "nothing was left out"] as Both,
  chainLeftOut: ["לא הלך אחרי:", "did not follow:"] as Both,
  chainUndated: ["{n} בלי תאריך", "{n} undated"] as Both,
  chainWrongWay: ["{n} לצד השני", "{n} the other way"] as Both,
  chainContemporary: ["{n} בני זמן אחד", "{n} contemporary"] as Both,
  chainRejected: ["{n} שדחית", "{n} you rejected"] as Both,
  chainOverBudget: ["{n} מעבר למכסה", "{n} over the limit"] as Both,
  chainNoInbound: [
    "ספרים שחצי הקשרים הנכנס אליהם לא נבנה: {n}",
    "Seforim whose incoming links were never built: {n}",
  ] as Both,
  /** A candidate from the OCR queue whose word the page no longer has — the
   * page was read again by something better since the queue was built. Said
   * out loud rather than opening a correction box on nothing. */
  scanSuspectNotHere: ["המילה הזאת כבר לא בעמוד הזה", "that word is not on this page any more"] as Both,
  scanPages: ["דפים", "Pages"] as Both,
  scanPagesWhy: ["אמור איזה עמוד הוא איזה דף", "say which page is which daf"] as Both,
  scanScheme: ["איך העמודים נקראים", "How the pages are named"] as Both,
  scanSchemeAmud: ["עמוד לכל דף בקובץ (ב. ב: ג.)", "a page per amud (2a 2b 3a)"] as Both,
  scanSchemeDaf: ["דף שלם בכל עמוד (ב. וב: יחד)", "a whole daf per page (2a and 2b together)"] as Both,
  scanSchemeNumbered: ["מספר לכל עמוד", "a number per page"] as Both,
  scanAnchors: ["עמוד=מקום, שורה לכל אחד", "page=place, one per line"] as Both,
  scanOfWhich: ["של איזה ספר", "Of which sefer"] as Both,
  scanOfWhichWhy: ["אם זו סריקה של ספר שעל המדף, המקורות ייכתבו בשמו", "if this is a scan of a sefer on the shelf, mekoros are written in its name"] as Both,
  save: ["שמור", "Save"] as Both,
  scanSaveMapping: ["שמור את המיפוי", "save the mapping"] as Both,
  scanForget: ["בטל מיפוי", "Forget the mapping"] as Both,
  scanForgetWhy: ["מוטב בלי מראה מקום מאשר מראה מקום שגוי", "better no mareh makom than a wrong one"] as Both,
  suspectsTitle: ["טעויות סריקה", "Scanning errors"] as Both,
  suspectsNoQueue: ["אין תור. הרץ: cargo run --release -p girsa-search --bin girsa-suspects -- index personal", "no queue. Run: cargo run --release -p girsa-search --bin girsa-suspects -- index personal"] as Both,
  suspectsCounts: ["כמה קטעים מכילים כל מילה", "how many segments hold each word"] as Both,
  suspectsConfusion: ["אותיות שנראות דומה בדפוס", "letters that look alike in print"] as Both,
  open: ["פתח", "Open"] as Both,
  suspectsOpenWhy: ["פתח את המקום, עם המילה מסומנת", "open the place, with the word marked"] as Both,
  suspectsNotAnError: ["לא טעות", "Not an error"] as Both,
  suspectsNotAnErrorWhy: ["אינה שגיאה — לא תוצע שוב", "not a mistake — it will not be offered again"] as Both,
  suspectsSwapped: ["אות שהוחלפה", "a letter swapped"] as Both,
  suspectsAdded: ["אות מיותרת", "a letter too many"] as Both,
  suspectsDropped: ["אות חסרה", "a letter missing"] as Both,
  suspectsTransposed: ["אותיות שהתחלפו", "two letters transposed"] as Both,
  doingOpenFile: ["פתיחת הקובץ", "opening the file"] as Both,
  doingReadPage: ["קריאת העמוד", "reading the page"] as Both,
  doingReadLinks: ["קריאת הקישורים", "reading the links"] as Both,
  doingOpenRef: ["פתיחת המראה מקום", "opening the mareh makom"] as Both,
  doingChain: ["מעקב אחר שלשלת המסירה", "following the chain"] as Both,
  doingContents: ["קריאת תוכן הספר", "reading the sefer's contents"] as Both,
  doingRepairLink: ["תיקון הקישור", "repairing the link"] as Both,
  doingReadLane: ["קריאת נתיב המשמעות", "reading the lane"] as Both,
  doingWriteNote: ["כתיבת הרשומה", "writing the note"] as Both,
  doingFix: ["התיקון", "the correction"] as Both,
  doingExport: ["הכתיבה לקובץ", "writing to the file"] as Both,
  doingPrint: ["בהדפסה", "printing"] as Both,
  doingDesks: ["בשולחנות", "reading the arrangements"] as Both,
  doingUpdate: ["בבדיקת גרסה", "checking for a newer version"] as Both,
  doingMark: ["הסימון", "the mark"] as Both,
  doingKeepQuery: ["שמירת החיפוש", "keeping the query"] as Both,
  doingCopySource: ["העתקת המקור", "copying the source"] as Both,
  doingReadSuspects: ["קריאת החשודים", "reading the queue"] as Both,
  doingSomething: ["הפעולה", "the action"] as Both,
  codeNoIndex: ["אין אינדקס חיפוש — יש לבנות אותו: girsa-index build", "there is no search index — build one: girsa-index build"] as Both,
  codeNoShelf: ["אין מדף כאן — ייתכן שהייבוא לא רץ", "there is no shelf here — the import may not have run"] as Both,
  codeNotACorpus: [
    "אין ספרים בתיקייה הזאת — יש לבחור את התיקייה שהייבוא כתב אליה",
    "there are no seforim in that folder — choose the one the import wrote to",
  ] as Both,
  codeNoSefer: ["אין ספר בשם הזה במדף", "no sefer on the shelf is called that"] as Both,
  codeWillNotOpen: ["הספר רשום במדף ואינו נפתח — פרטים בהצבה על ההודעה", "the sefer is on the shelf and will not open — details on hover"] as Both,
  codePoisoned: ["המצב הפנימי נפגם — יש לפתוח את החלון מחדש", "the internal state is broken — reopen the window"] as Both,
  codeShelfLoop: ["לא ניתן להכניס מדף לתוך עצמו", "a shelf cannot go inside itself"] as Both,
  codeLaneOff: ["הלשון הסמוכה כבויה — אפשר להדליק אותה בהגדרות", "the adjacent lane is off — you can turn it on in the settings"] as Both,
  codeNoSuchPage: ["אין עמוד כזה בסריקה", "there is no such page in the scan"] as Both,
  codeNoEngine: [
    "אין מנוע קריאת-תמונות — התקן את tesseract ואת המודל העברי",
    "no OCR engine is installed — install tesseract and the Hebrew model",
  ] as Both,
  yoursNotes: ["הערות", "Notes"] as Both,
  yoursMarks: ["סימונים", "Marks"] as Both,
  yoursQueries: ["שאילתות", "Questions"] as Both,
  yoursFolders: ["תיקיות", "Folders"] as Both,
  yoursTags: ["תגיות", "Tags"] as Both,
  yoursExport: ["ייצוא", "Export"] as Both,
  yoursNothingWritten: ["עוד לא כתבת", "you have not written anything yet"] as Both,
  yoursOpenAsSefer: ["פתח כספר", "Open as a sefer"] as Both,
  yoursEdit: ["ערוך", "Edit"] as Both,
  yoursDelete: ["מחק", "Delete"] as Both,
  yoursParagraph: ["פסקה", "Paragraph"] as Both,
  yoursNewParagraph: ["פסקה חדשה אחרי זו", "a new paragraph after this one"] as Both,
  yoursDropParagraph: ["הסר פסקה", "remove the paragraph"] as Both,
  yoursParagraphAtEnd: ["פסקה בסוף", "a paragraph at the end"] as Both,
  yoursNoMarks: ["אין סימניות", "no marks"] as Both,
  yoursStale: ["המילים שסומנו אינן בשורה — הסימון לא מוצג", "the marked words are not in the line — the mark is not drawn"] as Both,
  yoursMoved: ["השורה זזה, והסימון נמצא מחדש לפי המילים", "the line moved, and the mark was found again by its words"] as Both,
  yoursNoQueries: ["לא שמרת שאילתות", "you have kept no questions"] as Both,
  yoursAskAgain: ["שאל שוב", "Ask again"] as Both,
  yoursNoFolders: ["אין תיקיות", "no folders"] as Both,
  yoursRemove: ["הסר", "Remove"] as Both,
  yoursNoTags: ["אין תגיות", "no tags"] as Both,
  /**
   * A tag as a way in (W27, spec.md §11).
   *
   * The tags drawer counted them and a click did nothing, which made the tally
   * a report about your layer rather than a route through it. A tag is the one
   * thing in here that crosses the four kinds — the same word is on a note, on
   * a highlight, on a saved question and on a chaburah folder — so picking one
   * shows all four at once rather than filtering the drawer you happen to be
   * standing in.
   */
  yoursTagPick: [
    "הראה את כל מה שנושא את התגית הזאת",
    "show everything carrying this tag",
  ] as Both,
  yoursTagged: ["עם התגית {tag}", "carrying {tag}"] as Both,
  yoursTagClear: ["הסר סינון", "Clear"] as Both,
  yoursTagNothing: [
    "שום דבר אינו נושא את התגית הזאת עוד",
    "nothing carries this tag any more",
  ] as Both,
  /** The corrections you have made, which had no list anywhere in the window
   * until this tab: `api.fixes` was wired to a live command nothing called, so
   * a correction made yesterday could not be found again, let alone undone. */
  yoursFixes: ["תיקונים", "Corrections"] as Both,
  yoursNoFixes: ["לא תיקנת עדיין כלום", "you have not corrected anything yet"] as Both,
  yoursExportWhy: ["הכל, כקבצים פשוטים", "everything, as plain files"] as Both,
  yoursForgetFolderWhy: [
    "התיקייה בלבד — מה שהיה בתוכה לא נוגעים בו",
    "the folder only — what was in it is untouched",
  ] as Both,
  /**
   * What each drawer counts, as a plural noun.
   *
   * Five of these were Hebrew string literals in `yoursview.ts` —
   * `${notes.length} הערות` — which is the one thing `say.ts` exists to
   * prevent, and which the tag row twenty lines below already argued against
   * in a comment while four of its neighbours did it anyway. An English window
   * counted `12 הערות`.
   */
  countNotes: ["הערות", "notes"] as Both,
  countMarks: ["סימניות", "marks"] as Both,
  countQueries: ["שאילתות", "questions"] as Both,
  countFolders: ["תיקיות", "folders"] as Both,
  countTags: ["תגיות", "tags"] as Both,
  countFixes: ["תיקונים", "corrections"] as Both,
  scanGoWhy: [
    "כתוב דף — ב. או ב ע\"ב — או הדבק מראה מקום",
    "type a daf — 2a, or 2 amud bet — or paste a mareh makom",
  ] as Both,
  scanAnchorsWhy: [
    "שורה לכל עוגן: עמוד=דף. `43=-` אומר שמכאן אין אלו עמודי הספר — לוחות, מפתח",
    "one line per anchor: page=daf. `43=-` says that from here these are not the sefer's pages — plates, an index",
  ] as Both,
  scanOfWhichHint: ["צילום של… (bavli/berakhot)", "a photograph of… (bavli/berakhot)"] as Both,
} as const;

/** Every string the window can say. */
export type Word = keyof typeof WORDS;

/**
 * Where the last known interface language is cached.
 *
 * # Why a cache at all
 *
 * Every panel builds its title, its buttons and its placeholders **in its
 * constructor**, and the constructors run at module load — which is before
 * `main()` has asked Rust anything. So on a fresh load `say()` answered in the
 * default before the window had been told what language it was in, and the
 * shelf came up saying `המדף` over an English window. Reloading did not help,
 * because reloading runs the constructors again, earlier still.
 *
 * The session file in Rust remains the truth; this is what the window paints
 * with in the milliseconds before the truth arrives, and `speakInterface`
 * overwrites it the moment it does. It is the same shape as the theme cache
 * every application keeps to avoid a flash of the wrong colours.
 *
 * # And how it came to be one switch behind, in both directions
 *
 * This is the whole of finding 2. The panels are built at module load, from the
 * cache. The cache was written by `speakInterface`, which runs inside `main()`,
 * **after** those constructors — so on the reload that follows a language
 * switch, the constructors read the value from the switch *before* the one the
 * reader had just made. Measured, immediately after switching to English:
 *
 * ```
 * toolbar     nikud, no te'amim · with variants · A− · A+ · Settings
 * shelf       המדף · מדף חדש · החזר לסדר המקורי · צמצם · סגור
 * ```
 *
 * …and immediately after switching back to Hebrew, the same window with the two
 * halves swapped. It settled only on a restart, which reads less like a setting
 * and more like a broken build.
 *
 * The reload was never the problem — it is the right mechanism, and the
 * argument for it is in `settingsview.ts`. The problem was that **the write and
 * the reload were two statements in two files with nothing making them agree**,
 * which is the pattern the audit named under all eighteen of the first
 * complaints. They are one function now: `switchInterfaceTo`.
 */
const REMEMBERED = "girsa-interface";

function lastKnown(): Language {
  try {
    return localStorage.getItem(REMEMBERED) === "english" ? "english" : "hebrew";
  } catch {
    // A browser with storage disabled paints Hebrew for one frame and is then
    // corrected by `speakInterface`. Not worth a sentence on screen.
    return "hebrew";
  }
}

let speaking: Language = lastKnown();

/**
 * Set the language the **window** is in.
 *
 * Separate from `names.speak`, which sets the language the **seforim** are named
 * in. Both are called once per reload from the session, and neither reaches into
 * the other: a window in English naming its seforim in Hebrew is a setting a
 * reader can ask for, and used to be the only one available.
 */
export function speakInterface(language: Language): void {
  speaking = language;
  try {
    localStorage.setItem(REMEMBERED, language);
  } catch {
    // See `lastKnown`.
  }
  // Which language the window is in is a fact about this module; putting it on
  // the document is a side effect for the stylesheet. Split, because
  // `say.test.mjs` runs in node — the guard `sources.test.mjs` needs is a guard
  // over **every module**, and a module that cannot be imported without a DOM
  // cannot be checked.
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  // The chrome flips; **the reading does not**. An English interface around a
  // Gemara does not make the Gemara left-to-right, and turning the document over
  // would put every sefer's lines in the wrong order to prove a point about a
  // menu. `styles.css` reads this to turn the chrome and leaves `.pane` alone.
  root.classList.toggle("is-english-ui", language === "english");
  root.lang = language === "hebrew" ? "he" : "en";
  root.dir = language === "hebrew" ? "rtl" : "ltr";
}

/** Which language the window is in now. */
export function interfaceLanguage(): Language {
  return speaking;
}

/** What the next load of this window will be built in — the cache, read back.
 * For the guard that holds it to agreeing with `interfaceLanguage()`. */
export function nextLoadSpeaks(): Language {
  return lastKnown();
}

/**
 * Put the window into a language, and rebuild it in that language.
 *
 * **One function, because they are one act.** Every panel builds its title, its
 * buttons and its placeholders in its constructor, so the only thing that
 * relabels all of them is a reload — and the only thing that makes the reload
 * come back in the right language is the cache having been written first. Those
 * two used to be one statement in `say.ts` and one in `main.ts`, run in the
 * wrong order, which is finding 2 in its entirety.
 *
 * The alternative — a `retitle()` on eleven panels, each restating the strings
 * its constructor already sets — is a second list per panel, and a twelfth
 * panel nobody adds one to. Reloading is safe here for a reason that is not
 * luck: **the session lives in Rust**, so the tabs, the panes, where you are in
 * each of them and every setting come back exactly as they were. What is only
 * in the window is what the reader is typing, and the caller flushes that
 * first.
 */
export function switchInterfaceTo(language: Language): void {
  // Sets `speaking` and writes the cache. The reload below is what actually
  // relabels the window, and it reads that cache before anything else runs.
  speakInterface(language);
  if (typeof window === "undefined") return;
  // **And come back to where the reader was standing.**
  //
  // > *"Setting the language closes settings immediately."*
  //
  // It did, and there was nothing wrong with the diagnosis in the settings
  // panel's own header — a language switch is a reload, because eleven panels
  // build their words in their constructors. What nobody had noticed is that
  // the reader was *inside one of those panels* when they asked, so the reload
  // that relabelled it also dismissed it, and the one control whose effect they
  // most wanted to check went away at the moment of checking.
  //
  // A note in storage rather than a query string: this is a Tauri window, not a
  // page with a URL a reader ever sees, and `main.ts` clears it as it reads it,
  // so a crash between the two costs one unexpected panel and not a panel that
  // reopens forever.
  try {
    localStorage.setItem(REOPEN, "settings");
  } catch {
    // See `lastKnown`. The switch still works; the reader lands on the window
    // instead of on the panel.
  }
  window.location.reload();
}

/** Where the reader was standing when the window reloaded itself. */
const REOPEN = "girsa-reopen";

/** What to open again after a reload the window asked for, once. */
export function reopenAfterReload(): string | null {
  try {
    const held = localStorage.getItem(REOPEN);
    localStorage.removeItem(REOPEN);
    return held;
  } catch {
    return null;
  }
}

/** What the window calls something, in the language it is in. */
export function say(word: Word): string {
  const both = WORDS[word];
  return speaking === "hebrew" ? both[0] : both[1];
}

/**
 * A kind of link, in the window's language.
 *
 * The wire keys are `girsa_link::EdgeType::as_str`, and they are the keys
 * `girsa_app::links::kinds` is built from — so this table cannot drift out of
 * step with the engine without a key falling off the end of it, which is what
 * `null` is for. The caller then shows what Rust labelled it, and if there is
 * no label either, the key: a slug on the screen is a bug somebody can report,
 * and a blank is not.
 */
const LINK_KINDS: Record<string, Word> = {
  "comments-on": "linkKindCommentsOn",
  quotes: "linkKindQuotes",
  paraphrases: "linkKindParaphrases",
  codifies: "linkKindCodifies",
  disputes: "linkKindDisputes",
  emends: "linkKindEmends",
  "parallel-to": "linkKindParallelTo",
  translates: "linkKindTranslates",
  references: "linkKindReferences",
};

/** What to call one kind of link, or `null` for a key this table has not heard
 * of. */
export function linkKind(key: string): string | null {
  const word = LINK_KINDS[key];
  return word ? say(word) : null;
}

/** The same, in a language given rather than the one set — for a test. */
export function sayIn(word: Word, language: Language): string {
  const both = WORDS[word];
  return language === "hebrew" ? both[0] : both[1];
}

/**
 * The sibling's name, ready to drop into a `{ksav}` hole.
 *
 * **The name itself is not translated.** `names.ts` argues that at length: the
 * application is `כְּתָב`, that is what it calls itself in its own README and
 * its wordmark, and a name is a name in either window. What *is* language is
 * the preposition. Hebrew glues *to* and *in* onto the front of a word, with a
 * maqaf so the name stays legible; English puts it in front as its own word,
 * which means the English sentence in the table already says `to {ksav}` and
 * this must not hand it a name with a `ל` on the front.
 *
 * So one hole, filled two ways, and every row keeps the same holes in both
 * columns — which is the invariant `say.test.mjs` holds.
 */
export function ksavAs(prefix: string): string {
  return speaking === "hebrew" ? withPrefix(prefix, KSAV) : KSAV;
}

/**
 * A sentence with a number or a name in it, filled.
 *
 * **Why this exists at all.** Forty-two sentences in this window were written
 * as `` `${notes.length} הערות` `` — a template literal, so the table could not
 * hold them and the guard, which matched double-quoted strings, could not see
 * them. An English window counted `12 הערות` and said `עמוד 4 מתוך 380 בקובץ`
 * over the photograph of a page. Every one of them was a sentence with a hole
 * in it, and the hole is why they were written outside the table.
 *
 * So the table takes holes. `{name}` in **both** columns, filled here — which
 * means the word order can differ between the languages, and it has to: *3 of
 * 8 pages read* and *3 מתוך 8 עמודים נקראו* do not put the numbers in the same
 * places, and a sentence spliced together at the call site can only ever have
 * one order.
 *
 * `say.test.mjs` checks that both columns of a row carry the same holes, so a
 * translation that dropped `{count}` is a failing build rather than a sentence
 * with a number missing out of the middle of it.
 *
 * # Why a count is written after a colon and not in front of a noun
 *
 * This substitutes and stops. It cannot make a noun agree with a number, and
 * Rust — which can, in `girsa_plain::said` — was the only half of the
 * application that did: `plural` and `counted` are named there and their own
 * header records that three composers wrote that ternary eleven times before
 * anybody gave it a name.
 *
 * So the window said **`1 arrangements`**, and it was one of about twenty-five
 * rows with a count in them. Teaching this function to agree would have meant
 * two rules, not one: English wants an `s`, and Hebrew inflects the numeral
 * itself for gender, has a dual, and does not put the number in the same place.
 * Guessing at that for two dozen nouns is what BUILDER rule 6 forbids.
 *
 * The rows say `שולחנות: {desks}` and `Arrangements: {desks}` instead. A label
 * with its count after it is correct at every number in both languages, needs
 * no agreement rule and no second row per noun, and reads the way a tally in a
 * panel is supposed to read. Where a sentence genuinely needs the singular —
 * the two markers in `mefarshim.ts` and `pane.ts` — there is a second row and
 * the call site chooses, which is the shape to copy if another one turns up.
 *
 * # Thousands
 *
 * A number is grouped — `1,204`, not `1204` — which is what
 * `girsa_plain::thousands` does on the Rust side and what this did not do at
 * all. Only numbers: a version string arriving in `{running}` is left exactly
 * as it came.
 */
export function fill(word: Word, holes: Record<string, string | number>): string {
  return put(say(word), holes);
}

/** A number as a reader reads it. `girsa_plain::thousands`, in the window. */
function shown(value: string | number): string {
  return typeof value === "number" ? value.toLocaleString("en-US") : value;
}

/** The holes of one already-chosen sentence, filled. */
function put(said: string, holes: Record<string, string | number>): string {
  for (const [name, value] of Object.entries(holes)) {
    said = said.split(`{${name}}`).join(shown(value));
  }
  return said;
}

/** The same, in a language given rather than the one set — for a test. */
export function fillIn(
  word: Word,
  language: Language,
  holes: Record<string, string | number>,
): string {
  return put(sayIn(word, language), holes);
}

/** Every key, for the guard that checks neither column has a hole in it. */
export function everyWord(): Word[] {
  return Object.keys(WORDS) as Word[];
}
