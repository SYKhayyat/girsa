# The third sitting

*Twenty-four findings from somebody using the shipped window, and what each one
turned out to be underneath.*

The twenty-four as they were written are in
[`the-third-sitting.txt`](the-third-sitting.txt), untouched, the way
[`user_feedback.txt`](user_feedback.txt) holds the first eighteen. Quoting a
reader against a fix is only worth anything if the reader's own words are still
somewhere to check.

The first reader used Girsa for five minutes, wrote down eighteen things and said
it looked like a toy; [`the-five-minute-report.md`](the-five-minute-report.md) is
the answer to those. The second sat with it for an hour and found the paint
coming off wherever he touched it;
[`the-second-sitting.md`](the-second-sitting.md) is the answer to those. This is
the third, and the shape of it has changed again.

**Nothing here is about the corpus, the resolver, the addressing or the speed.**
Twenty-four findings and not one of them says *this is the wrong text* or *this
took too long to load*. Three of them say the window **stopped answering**, which
is a different complaint and a worse one, and both had the same cause. The rest
are about a window that knows the answer and will not hand it over.

**Two of the twenty-four were one bug each in a place nobody would look, and they
account for six of the findings between them.** A `/*` comment opened inside a
CSS selector list, which silently ate the four rules after it. And an `AND` where
the code meant `OR`, in the one function that decides which seforim a search runs
over.

---

## The grade

| | |
|---|---|
| **Overall** | **A−** — every one of the twenty-four is closed at its root and re-measured in the shipped window against the reader's own 7,189-sefer shelf. What is left is not defects; it is the two decisions [`not-yet.md`](not-yet.md) already names |
| **Search** | ~~**F**~~ **A** — *"objectively terrible"* was three separate defects wearing one coat: an AND over the scope, a blocking IPC thread, and a panel that could add a folder and not a sefer. `חייב` over the whole shelf: **182,621 hits in 257 ms** |
| **Reading** | ~~**C**~~ **A** — the heading is its own line, the size buttons move the text, and the mefarshim have a size of their own |
| **Arranging** | ~~**C**~~ **A−** — splits turn and swap, tabs drag, a sefer opens twice, and a small window keeps every control |
| **Saying things** | ~~**C+**~~ **A−** — the Latin is out of the links and the chain, the mareh-makom box says it is one, and the sentence about what you cannot see no longer names a cache |

---

## What the two root causes were

### A comment inside a selector list — findings 2, and four rules nobody could see

`styles.css` had this:

```css
.line,
/* … a paragraph about what the next rule is for … */
.line-text {
```

which is legal CSS and means something entirely different from what it looks
like. The comment does not end the selector list; the parser reads
`.line, .line-text` as one selector **and then keeps going**, so the block that
followed took its properties from a rule four declarations further down. The
reading text drew at 12px with a hairline border and `cursor: pointer` — and
`--reading-size`, the variable both size buttons and the settings slider write
to, was in one of the swallowed rules.

So *"the font size buttons do not work; neither does setting it in settings"* was
true, and had been true since the sheet was written, and no test in this
repository could see it: `styles.test.mjs` reads the file as text and every
property it looks for was present.

Measured in the shipped window afterwards: `--reading-size` 90 → 100 moves the
line from **17.1px to 19px**.

### An AND where the search meant OR — findings 19, 20, 21, 22

`girsa_search::scope::clauses` turned the reader's ticked rows into query
clauses. Each row became its own `Must` clause, so ticking two masechtos asked
for *documents in Berakhos **and** in Shabbos* — which is nothing, always, by
construction. One masechta worked. Two did not. Thirty-seven did not.

> *"It could not find חייב anywhere in shas… It does find when only some of shas
> is checked. Whatever it is, it is not acceptable."*

Exactly right, and the *only some of it works* was the tell: one clause is an
`AND` of one.

Rows are grouped by the **question** they answer now — `Which`, `When`, `Who`,
`Tagged` — and a group is an `OR` inside itself while the groups are `AND`ed
together, which is what a facet rail has always meant everywhere else.

Measured in the shipped window: one shelf ticked → **640 seforim**; two shelves →
**1,634**. `חייב` over the two: **14,551 hits in 125 ms**.

### And one that was neither: the window had one thread doing two jobs

> *"When xing out things to search in, the UI started hanging. In general, it is
> a very unresponsive UI — almost like openoffice."*

Every `#[tauri::command]` in the shell was the blocking kind. That is not a
synonym for *fast*: a blocking Tauri command runs **inline on the thread that
carries IPC**, so a 2.7-second chain walk did not merely take 2.7 seconds — it
held every other message the window sent for 2.7 seconds, including the ones that
draw. All 136 commands are `#[tauri::command(async)]` now, but for `copy` and
`sefer_sheet`, which touch the clipboard and must stay on the thread that owns
it.

---

## The twenty-four

| | The finding | What it was | Where |
|---|---|---|---|
| 1 | *"It seems to start at a totally random spot in the middle"* | The pane and the shell disagreed about what *where I was* meant. `Text` carries an `at` now, so a pane can land at the head or at the remembered place and nowhere else. Measured: a sefer with nothing remembered opens at line 1; one with a remembered place opens there | `view.rs`, `lib.rs` |
| 2 | *"The font size buttons do not work"* | The comment in the selector list, above | `styles.css` |
| 2b | *"a seperate control for mefarshim and top level"* | `--mefarshim-size`, its own setting, its own row in Settings | `session.rs`, `styles.css` |
| 3 | *"Setting the language closes settings immediately"* | The panel rebuilt itself on a language change and the rebuild started shut | `settingsview.ts` |
| 4 | *"the header is right in front of the actual text"* | Headings were run into the line they belonged to. A heading is its own element now, and `only_when_it_changes` says each one once rather than on every mishnah | `view.rs`, `pane.ts` |
| 5 | *"a way to open a new window in a tab"* | `＋` went to whichever tab already held the sefer. It makes a tab now — that is its name | `workspace.rs`, `main.ts` |
| 6 | *"Checking off a box… brings you to the top"* | The list was rebuilt from scratch on a tick. Measured after: the list stayed at scrollTop **2488** of a 7,465px list | `picker.ts` |
| 7 | *"it should be a discrete symbol, not words"* | `◇` — the ticked-mefarshim marker with nothing filled in — and the sentence moved to its hover | `mefarshim.ts` |
| 8 | *"A small window truncates its controls and is uncloseable"* | `.pane-tools` was `flex: 0 0 auto` with no wrap, so eight controls measuring 478px hung off the inline edge of a 310px pane — at x = −168 in a right-to-left window. What leaves first is the furthest one, and the furthest one is **Close** | `styles.css` |
| 9 | *"Tabs should be splittable in any way and movable"* | The tree has held both axes since it was written and every caller passed `Vertical`. A divider turns and swaps now; tabs drag along the strip | `workspace.rs`, `layout.ts` |
| 10 | *"Even without own scroll on, the two are not linked"* | Following was set but the follower did not move. Linked is the default and the label says which state clicking gets you | `beside.rs`, `pane.ts` |
| 11 | *"The latin text in links makes it hard to read"* | `hop.edge_type` — a wire key like `comments-on` — printed straight into a right-to-left column. One naming door for the links panel and the chain both | `say.ts`, `chainview.ts` |
| 12 | *"Links does not seem to filter based on the dropdown"* | It never did: the dropdown says what kind a link **you draw** should be. It is at the foot now, behind its own name, and the lens row that does filter says so | `linksview.ts` |
| 13 | *"Links is based on where you were when you opened it"* | It follows the pane now, and says whether it is following or pinned | `linksview.ts` |
| 14 | *"Chain is left-aligned in english"* | The chain's list is corpus text — a column of Hebrew sefer names — and inherited `dir="ltr"` from the document. The **list** is pinned right-to-left; the panel around it still reads in the interface language | `styles.css` |
| 15 | *"The chain seems to hang"* | Every walk re-read every inbound shard: 24 works, 2.7 s, from cold, every time. The graph keeps its cache between walks now. Measured: **4,368 ms** the first time, **46 ms** the second | `chain.rs`, `lib.rs` |
| 16 | *"does not match שלחן ערוך to שולחן ערוך"* | The shelf search compared spellings letter for letter. It compares skeletons too — interior vav and yud dropped, and only interior. Measured: **40 rows** for either spelling | `shelf.rs` |
| 17 | *"no way to open one sefer in two tabs"* | `Workspace::open_again`. The plain route still goes to the open one | `workspace.rs` |
| 18 | *"ctrl-enter is supposed to open in the same tab. It does not"* | Three landings and the row advertised none of them. Its hover reads *Open — Ctrl beside what I am reading, Shift in a tab of its own* | `search.ts`, `say.ts` |
| 19 | *"it should be more clear what is and is not included"* | A tri-state tick tree and one counted line. Measured in the window: *Searching: 7,189 of 7,189 seforim* over 15 boxes | `scopeview.ts` |
| 20 | *"you can only add or subtract a folder, not a single sefer"* | Shelves twist open to the seforim standing on them, and there is a clear-everything | `scopeview.ts` |
| 21 | *"could not find חייב anywhere in shas"* | The AND, above. **182,621 hits in 257 ms** | `scope.rs`, `facets.rs` |
| 22 | *"the UI started hanging… almost like openoffice"* | One thread doing two jobs, above | `lib.rs` |
| 23 | *"The guide talks about a search box where you can write טור סימן א"* | There is no such box and there was never going to be one — the shelf search reads a mareh makom as a place. It says so in its own placeholder now. All four of the guide's examples land, `טור או"ח סימן א` included | `say.ts`, `start-here.md` |
| 24 | *"the sentence of what I cannot see is hard to decipher"* | *no inbound cache — links into this line are not shown. Run girsa-link-types.* Three implementation nouns and a command name, at a person who came to read. It says which half of the list is missing now, in the words the list itself is drawn in, and the command is on the hover | `say.ts`, `linksview.ts` |

---

## What was found while fixing them

Two defects nobody reported, both adjacent to the work and both silent.

**A divider was addressed by a pane beside it.** `Workspace::set_ratio` took a
`PaneId` and matched the split one of whose children *is* that leaf, while
`layout.ts` handed it `firstPaneOf(layout.first)` — the leftmost leaf of the
first child, which for a nested first child is a grandchild rather than a child.
So dragging the outer divider of `Split { Split { a | b } | c }` resized the
**inner** one. The pointer moved one line and a different line moved, quietly,
because there is always some split that matches and the wrong one is still a
legal answer. Dividers are named by which divider they are now — pre-order, the
order `layout.ts` draws them.

**The README's command count had gone stale in the measurement rather than in the
README.** `the_numbers_in_the_readme_are_measurements` counted the literal
`#[tauri::command]`, and the day every command became
`#[tauri::command(async)]` the count fell from 132 to the 3 that stayed blocking.
The test was working — it said the claim and the tree disagreed — but the fix
belonged in the counter, not in the sentence. It counts the attribute at the head
of a line now, which also stops it counting the two in the module's own doc
comment: the shell would otherwise gain two commands by explaining itself.

---

## How this was checked

Three ways, and the third is the one that matters.

**The gate**, nine of nine: build, test, clippy, fmt, the shell's clippy and fmt,
`tsc`, 537 window assertions across 24 files, and `npm run eyes`.

**The eye** — `app/tools/eyes.mjs`, the only check in this repository that has
ever seen a pixel — went from 47 assertions to **60**. The new ones are finding 8
(no control has left its pane, at three widths, with the full eight-control row
rather than the five it had) and finding 9 (a divider's controls are invisible
until it is touched, visible once it is, and do not widen the 5px line they hang
off). Both were written after the fix and both fail when it is reverted; the
finding-8 assertion names `סגור` in its failure message at both widths.

**The shipped window, over CDP.** `girsa-shell.exe` built with
`--features tauri/custom-protocol`, launched with
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`, driven
against the reader's own **7,189-sefer** shelf. Every number in the table above
marked *measured* was read off that window. `withGlobalTauri` is off, so the
bridge is an `import()` of the same `core-*.js` chunk the page already loaded —
the module cache hands back the live `invoke` rather than a second copy.

Two of the sweep's own failures were the sweep's fault and worth recording,
because both are the shape a careless test takes: it looked for `א+` and
`הגדרות` in a window whose interface was set to **English**, and it demanded the
first line from a sefer that had a remembered place — which is the behaviour
finding 1 asked for, stated in the finding's own parenthesis.
