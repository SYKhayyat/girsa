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
  searchPlaceholder: ["חפש בכל המדף…", "search the whole shelf…"] as Both,
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
  linkFacetUnbuilt: [
    "לא נבנה — הרץ girsa-link-types ובנה אינדקס מחדש",
    "not built — run girsa-link-types and rebuild the index",
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
  codeRungApplied: [
    "החל — לחזרה, חפש שוב בלי ההצעה",
    "applied — search again without the offer to go back",
  ] as Both,
  codeWillNotSerialize: [
    "המקור לא נארז כראוי — העתק שוב, ואם זה חוזר דווח על השורה",
    "the source would not pack — copy again, and if it keeps happening report the line",
  ] as Both,

  linked: ["מקושר", "linked"] as Both,

  // --- the links panel -----------------------------------------------------
  linksTitle: ["קישורים", "Links"] as Both,
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
  linksNoInbound: [
    "אין מטמון נכנס — הקישורים אל השורה הזאת אינם מוצגים. הרץ girsa-link-types.",
    "no inbound cache — links into this line are not shown. Run girsa-link-types.",
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
  writingSaved: ["נשמר", "saved"] as Both,
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
  ksavStale: [
    "{ksav} רשום אך אינו עונה — ייתכן שנסגר שלא כשורה",
    "{ksav} is registered but not answering — it may have closed badly",
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
  suspectsInQueue: ["{count} הבאים בתור", "{count} next in the queue"] as Both,
  yoursNoteAbout: [
    "{paragraphs} פסקאות · {places} מקומות",
    "{paragraphs} paragraphs · {places} places",
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
  doingRepairLink: ["תיקון הקישור", "repairing the link"] as Both,
  doingReadLane: ["קריאת נתיב המשמעות", "reading the lane"] as Both,
  doingWriteNote: ["כתיבת הרשומה", "writing the note"] as Both,
  doingFix: ["התיקון", "the correction"] as Both,
  doingExport: ["הכתיבה לקובץ", "writing to the file"] as Both,
  doingMark: ["הסימון", "the mark"] as Both,
  doingKeepQuery: ["שמירת החיפוש", "keeping the query"] as Both,
  doingCopySource: ["העתקת המקור", "copying the source"] as Both,
  doingReadSuspects: ["קריאת החשודים", "reading the queue"] as Both,
  doingSomething: ["הפעולה", "the action"] as Both,
  codeNoIndex: ["אין אינדקס חיפוש — יש לבנות אותו: girsa-index build", "there is no search index — build one: girsa-index build"] as Both,
  codeNoShelf: ["אין מדף כאן — ייתכן שהייבוא לא רץ", "there is no shelf here — the import may not have run"] as Both,
  codeNoSefer: ["אין ספר בשם הזה במדף", "no sefer on the shelf is called that"] as Both,
  codeWillNotOpen: ["הספר רשום במדף ואינו נפתח — פרטים בהצבה על ההודעה", "the sefer is on the shelf and will not open — details on hover"] as Both,
  codePoisoned: ["המצב הפנימי נפגם — יש לפתוח את החלון מחדש", "the internal state is broken — reopen the window"] as Both,
  codeShelfLoop: ["לא ניתן להכניס מדף לתוך עצמו", "a shelf cannot go inside itself"] as Both,
  codeLaneOff: ["הלשון הסמוכה כבויה — אפשר להדליק אותה בהגדרות", "the adjacent lane is off — you can turn it on in the settings"] as Both,
  codeNoSuchPage: ["אין עמוד כזה בסריקה", "there is no such page in the scan"] as Both,
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
  window.location.reload();
}

/** What the window calls something, in the language it is in. */
export function say(word: Word): string {
  const both = WORDS[word];
  return speaking === "hebrew" ? both[0] : both[1];
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
 */
export function fill(word: Word, holes: Record<string, string | number>): string {
  let said = say(word);
  for (const [name, value] of Object.entries(holes)) {
    said = said.split(`{${name}}`).join(String(value));
  }
  return said;
}

/** The same, in a language given rather than the one set — for a test. */
export function fillIn(
  word: Word,
  language: Language,
  holes: Record<string, string | number>,
): string {
  let said = sayIn(word, language);
  for (const [name, value] of Object.entries(holes)) {
    said = said.split(`{${name}}`).join(String(value));
  }
  return said;
}

/** Every key, for the guard that checks neither column has a hole in it. */
export function everyWord(): Word[] {
  return Object.keys(WORDS) as Word[];
}
