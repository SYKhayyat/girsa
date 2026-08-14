# The Ksav loop

*← [The shelf, and searching it](the-shelf-and-the-search.md) · [The record](../the-record.md) · [Corrections](corrections.md) →*

---

*Moving a source into a document should feel like AirDrop between two of your
own devices* (spec.md §10). No export dialog, no file, no format decision, no
cleanup — and **the user does nothing different**: Ctrl+C is Ctrl+C.

### One Ctrl+C, three flavours

What changes is what lands on the clipboard beside the text:

| flavour | who takes it | what it has to survive |
|---|---|---|
| `text/plain` | WhatsApp, a terminal, anything | being read with no formatting at all |
| `text/html` | Word, an email, a browser | keeping its shape **and its direction** |
| `application/x-girsa-source+json` | Ksav | carrying the **ref**, so the citation stays alive |

```
$ cargo run -p girsa-app --example send -- corpus "שולחן ערוך, אורח חיים סימן א' סעיף ג'"
── the ref the document stores ──────────────────────────────
girsa:shulchan-arukh/orach-chayim/1:3

── text/plain — WhatsApp, a terminal, anything ──────────────
ראוי לכל ירא שמים שיהא מיצר ודואג על חורבן בית המקדש:
(שולחן ערוך, אורח חיים סימן א' סעיף ג')

── application/x-girsa-source+json — Ksav ───────────────────
{"schema":1,"ref":"girsa:shulchan-arukh/orach-chayim/1:3","display":"שולחן ערוך,
 אורח חיים סימן א' סעיף ג'","text":"ראוי לכל ירא שמים…","nikud":false,"lang":"he",
 "version":{"edition":"Maginei Eretz: Shulchan Aruch Orach Chaim, Lemberg, 1893",
 "provenance":"https://www.sefaria.org/Shulchan_Arukh,_Orach_Chayim"}}
```

The third flavour is **written natively, not from the webview**, and that is not
a detail. `navigator.clipboard.write` will take a custom type, but Chromium puts
it down as a *web custom format* — a private encoding another browser tab can
read and a native application cannot. Written from the window, Ksav would see
the plain text and nothing else, and the pairing would look like it worked.

That the packet is real is checked **in Ksav, against a packet Girsa really
sent**: `ksav/engine/tests/from_girsa.rs` reads the literal output of the command
above and asserts the words of the se'if and the mekor are *on the laid-out
page*, not merely that the document compiled.

### Only the highlighted part goes

`girsa_app::sending` is handed segment ids and **character offsets into the text
the window drew** — markup already turned into runs, nikud already applied. So
both ends slice the same string and neither has to describe a selection to the
other. Highlight four words of a se'if and four words travel; highlight nothing
and the line you are standing on travels, which is what Ctrl+C does everywhere
else.

A selection across three se'ifim keeps the head of the first and the tail of the
last, and its ref is a **span** — `girsa:…/1:1-1:3` — because a quote is a range
(§4.2). Dragged upwards, it is put back into reading order before anything else
looks at it.

### The citation is not the string

What the document stores is `girsa:shulchan-arukh/orach-chayim/1:3`. The printed
form is `girsa-cite`, the formatter **both applications compile**, and it can be
asked for another one at any time:

| style | |
|---|---|
| `HebrewFull` | `שולחן ערוך, אורח חיים סימן א' סעיף א'` |
| `HebrewShort` | `שולחן ערוך, אורח חיים א', א'` |
| `English` | `Shulchan Arukh, Orach Chayim 1:1` |

`סימן` and `סעיף` are not words this app chose. They are the schema's
`heSectionNames`, carried onto every work by `girsa-import --metadata-only`, and
where a schema does not say — 1,101 branch schemas, and all 978 Otzaria-only
works — a sefer is cited by number, which is an ordinary way to write a mekor.
Nothing is invented: **no abbreviation of a title is guessed at**, because
nothing in the data says which of a work's 44 title variants a citation should
use.

The rule the formatter is held to is that Girsa can read back what Girsa
printed. Writing that test found two real defects, both fixed in `sefer-crates`
0.3.0 rather than worked around here: the resolver knew nine of the corpus's 42
section words, so `ברכות דף ב. שורה א'` resolved to `2a:שורה:1` without
complaint; and a whole sefer could not be written down as a ref at all, because
`girsa:bavli/berakhot` means the work `bavli` at a section called `berakhot`.

### When both are running, there is no clipboard at all

spec.md §10.6. Girsa opens a **desk** on loopback — `127.0.0.1`, a port the
system picks, a token minted per run and published in a file only you can read
— and so does Ksav. Each asks the other whether it is there:

| | |
|---|---|
| `Live` | answering, and it says which version it is |
| `NotRunning` | there is no endpoint file — it has not been started |
| `Stale` | there is a file and nothing behind it, **with the reason** |

The window shows which of the three it is, and the send button only exists for
the first. That is the whole of *presence* (§10.6): an affordance is never
offered when it would fail, and a crashed Ksav is told apart from one that was
never started, because those are different things to a reader.

Ctrl+Shift+C sends the selection straight into the open document. What comes
back the other way is Ksav asking the library questions only the library can
answer:

| | |
|---|---|
| `POST /open` | *show me this place* — the window opens the sefer and lands on the segment |
| `POST /cite` | *print this ref in that style* |
| `POST /quote` | *the words again*, read out of the corpus as it stands now |
| `POST /refresh` | *this whole document again* — every citation in it, re-read |
| `POST /where-from` | *where is this phrase from?* — cite-on-selection |
| `POST /search` | *nothing fitted* — put the phrase in the search and open it |
| `POST /linkify` | *which of these are citations?* — only the certain ones |
| `POST /document` | *I have saved a document here* — so *where did I use this* is true |

`/cite` and `/quote` are what make a citation alive. Because a Ksav document
stores the **ref** and not the printed string, a whole sefer can be switched
from abbreviated to full-form citations, and every quote regenerated against a
corrected edition (§7) — but only if something knows the title, the words the
schema uses for a level, and the text. All three live in the library, so Ksav
asks rather than keeping a copy that nobody would remember to update.

`/refresh` is the one that makes the port worth having. Everything Girsa
*hands* Ksav, the operating system could carry: a source on the clipboard is
push, one direction, no reply, and Ctrl+V is the whole protocol. What a
clipboard cannot be is a **question**. §10.2's promise is stated about a
document — forty citations at once, some of which name a sefer this shelf does
not have — and one call comes back with a row for each, in the order the
document has them, a reason in the ones that failed and the other
thirty-nine still refreshed. The decision *one missing sefer is not a failed
document* is made once, in the library.

What comes back is rows, not a rewritten file. A correction somebody else made
silently changing the words in the sefer you are writing is the surprise §7.1
is built to avoid, so the writer sees what moved and says yes.

**Localhost is not private**, and the token is not decoration: every process on
the machine can reach a loopback port, and so can a web page. So it is required
on every path including `/health`, it travels in a header rather than a URL, and
the desk answers no preflight and sends no CORS header — a tab that guessed the
port and the token still cannot read a word of the reply.

### A citation is a link, and it was already one

`girsa://open?ref=…` opens a place. So does a bare `girsa:bavli/berakhot/2a:1` —
because **a ref is already a URI**. Nothing had to be generated: the string the
document has been storing all along is the link, which is why the citation in
the HTML clipboard flavour is `<a href="girsa:…">`. Paste a quote into Word,
print it to PDF, and the mekor in the PDF opens the page it names.

Anything that is not one of the two errands is refused rather than approximated.
A URL handler is an entry point every page on the machine can reach.

### A place to write, in the same window

spec.md §10.3. You are learning, you have a thought, and switching applications
to record one line is how the line does not get recorded. **Ctrl+E** opens a
drawer along the foot of the window — not a pane, because the sefer you are
writing about has to stay on the screen.

What it writes is **real Ksav markup from the first keystroke**:

```
#כותרת1[השכמת הבוקר]
#ציטוט[ראוי לכל ירא שמים שיהא מיצר ודואג על חורבן בית המקדש:]#מראה_מקום[שולחן ערוך, אורח חיים סימן א' סעיף ג']

וצריך עיון.
```

That is a `.ksav` file in your own layer, and the acceptance is checked **from
the other side**: `ksav/engine/tests/from_girsa.rs` takes a buffer this window
wrote, compiles it with the real Typst engine, and reads the words off the laid
out page — including that the mekor lands *below* the quote, where a footnote
belongs.

The markup is not written here. `#ציטוט[…]` comes from `girsa-ksav`, the crate
Ksav itself compiles, because *lightweight means the UI, not the format*: a
second writer in TypeScript would be two applications producing documents that
differ depending on which end wrote them. The window decides where the caret is
and nothing else.

**פתח ב־כְּתָב** hands the whole document to the real Ksav over the loopback —
offered only when presence says it is there. There is no conversion step, which
is the point: Ksav is opening a document it can already read.

### Where is this from, and who quotes it

spec.md §10.4 says these are one feature asked from two directions, and they
are **one function**: the only difference is whether the sefer you are standing
in is left out of the answer.

```
$ girsa-index where-from index corpus "משעה שהכהנים נכנסים לאכול בתרומתן"
משעה שהכהנים נכנסים לאכול בתרומתן  —  ב־61 מקומות
  ברכות             מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בָּעֲרָבִין? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים…
  רש"י על ברכות     מאימתי קורין את שמע בערבין. משעה שהכהנים נכנסים לאכול בתרומתן…
  הלכות גדולות      מאימתי קורין את שמע בערבין משעה שהכהנים נכנסים לאכול בתרומתן…

$ girsa-index where-from index corpus --except bavli/berakhot "משעה שהכהנים…"
משעה שהכהנים נכנסים לאכול בתרומתן  —  ב־59 מקומות
```

61 places, and 59 of them are not the Gemara — which is the answer to *who
quotes this*. In Ksav it is Ctrl+Shift+M on a highlighted phrase: the first
mekor appears, Tab cycles the rest, Enter inserts it as a `#מראה_מקום`, and if
none fits, **the last row opens Girsa's search with the phrase already in it**.
A citation nobody could settle is not a citation to guess at.

What the engine is careful about is not finding — a phrase search always finds
something. It is not *lying*:

```
$ girsa-index where-from index corpus "אמר רבי יוחנן"
אמר רבי יוחנן  —  ב־12347 מקומות — ביטוי, לא ציטוט
(not offered as a source: 12347 places)
```

12,347 places has no source; it has a language. The list is still shown — the
reader may recognise one — but **nothing is preselected and nothing is called
the mekor**. And a quotation that is not letter for letter says so: the literal
search runs first, the ladder is climbed only on a zero, and what comes back
carries the rung that was used, so a near match is never shown as an exact one.

### Closing the loop

Three things fall out of one fact, and the fact had to be fixed first.

**The ref is in the document now.** For three work orders it was not: the markup
carried `#מראה_מקום[שו"ע או"ח סימן א' סעיף ג']` and the ref went nowhere, which
made §10.2's promise quietly false. It is now

```
#מראה_מקום(מקור: "girsa:shulchan-arukh/orach-chayim/1:3")[שולחן ערוך, אורח חיים סימן א' סעיף ג']
```

— printed exactly as before, and **storing the place**. Everything below is
that one change, seen from three sides:

- **Auto mareh mekomos.** `#מראה_מקומות()` collects every citation that carried
  a ref into a list at the back. Cheap by construction: the refs are already
  there, so it is a sort and a print. Checked by the real Typst engine.
- **Where did I use this.** Standing on a passage, Girsa scans your own layer
  for refs that *cover* it — a citation of `2a:1-2a:4` answers a question about
  `2a:3`, and a citation of siman 1 answers one about se'if 3 of it. A scan,
  not a guess.
- **Your writing is a sefer.** A `.ksav` file goes on the shelf like anything
  else: the words are read out of the markup by the same crate that wrote it,
  so `#כותרת1[` is never indexed and never shown, and the segments carry
  permanent ids like every other sefer.

### Linkify, and how much it refuses

spec.md §10.5 and decision 12: **high-confidence patterns only, anything
ambiguous stays plain text.** Three rules, and each refuses more than it
accepts:

| | |
|---|---|
| the resolver must say **Exact** | `או"ח` is the Shulchan Arukh's volume *and* the Tur's; a citation naming two seforim is left alone |
| there must be an **address** | *the Shulchan Arukh writes at length* is a subject, not a mekor |
| every level of it is a **number or a daf** | else `ברכות ב. ועיין שו"ע` reads as Berakhot at a section called *ועיין שו"ע*, and swallows the next citation whole |

A leading prefix letter is peeled — `וכתב בשו"ע או"ח סימן א' סעיף ג'` is how a
citation is actually written — and that widens *where* one is found, never what
it is found to be.

What comes back is wrapped as `#מקור_חי(מקור: "girsa:…")[…]`: the words print
exactly as they were typed, the ref rides underneath, and in a compiled PDF the
citation is a **link that opens the page it names**.

---

*← [The shelf, and searching it](the-shelf-and-the-search.md) · [The record](../the-record.md) · [Corrections](corrections.md) →*
