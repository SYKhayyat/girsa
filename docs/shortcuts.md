# Girsa — keyboard shortcuts · גִּרְסָא — מקשים

<!-- Generated: cargo run -p girsa-app --bin girsa-card > docs/shortcuts.md
     Do not edit by hand. The table is `crates/girsa-app/src/keys.rs`, which is
     what the window resolves a key press against — so this card is wrong only
     if the application is. -->

Every one of these can be changed: **Ctrl+,** opens the settings, and each row
there rebinds by pressing the keys you want. `↺` puts one back.

כל אחד מהם ניתן לשינוי: **Ctrl+,** פותח את ההגדרות, ובכל שורה שם אפשר
להקליד את המקש הרצוי. `↺` מחזיר לברירת המחדל.

| Keys · מקשים | What it does | מה זה עושה |
|---|---|---|
| `Ctrl+O` | Open a sefer | פתח ספר |
| `Ctrl+B` | Browse the shelf | עיין במדף |
| `Ctrl+F` | Find in this sefer | חפש בספר הזה |
| `Ctrl+Shift+F` | Search the whole shelf | חפש בכל המדף |
| `Ctrl+E` | Open the writing pane | פתח את הכתיבה |
| `Ctrl+\` | Mefarshim, or a sefer alongside | מפרשים / ספר לצד |
| `Ctrl+L` | Links on this line | קישורים על השורה |
| `Ctrl+Shift+L` | The adjacent language | הלשון הסמוכה |
| `Ctrl+Shift+M` | The transmission chain | שלשלת המסירה |
| `Ctrl+Shift+T` | The sefer's contents | תוכן הספר |
| `Ctrl+Shift+B` | The place I marked | המקום שסימנתי |
| `Ctrl+W` | Close this column | סגור את הטור |
| `Ctrl+Shift+C` | Send a source to Ksav | שלח מקור לכתב |
| `Ctrl+C` | Copy with its citation | העתק עם מקור |
| `Ctrl+N` | Write a note on this line | כתוב על השורה |
| `Ctrl+D` | Highlight these words | סמן את המילים |
| `Ctrl+Shift+H` | Mark without a colour | סמן בלי צבע |
| `Ctrl+M` | What I have written | מה שכתבתי |
| `Ctrl+K` | Correct this word | תקן את המילה |
| `Ctrl+Shift+K` | How much of the corrections to apply | מה מוצג מהתיקונים |
| `Ctrl+J` | The correction queue | תור התיקונים |
| `Alt+N` | Nikud | ניקוד |
| `Ctrl+=` | Larger | הגדל |
| `Ctrl+-` | Smaller | הקטן |
| `Ctrl+P` | Print this section | הדפס את הסימן |
| `Ctrl+,` | Settings | הגדרות |

## Not on this card

**Escape** closes whatever is open — the search, the shelf, a drawer, the
correction box. It is not in the table because it is not rebindable: a reader
who bound Escape to something else would have no way out of a panel.

**Ctrl+C** deliberately does not stop the webview's own copy. The words go to
the clipboard the way they always would, and the citation goes with them
(spec.md §10.2 — *the user does nothing different*). If the citation half
fails, you still have the text.

**Clicking a line** opens the mefarshim you have ticked, on that line. Only
when you have ticked at least one — otherwise a click is just a click.
