# Girsa correctness, interop and concurrency audit — 18 August 2026

**Scope.** Four questions asked of the whole tree, with the Girsa↔Ksav seam
named as a first-class subject: *what is wrong*, *what costs more than it
should*, *what races*, and *what is built halfway*.

**Baseline.** `main` at `15cebb3`, `sefer-crates` at `779225d` (0.5.5), working
tree clean at the start of the audit. Line numbers below are against those two
revisions.

**Method, and its limits.** This is primarily a **source read** — every crate in
the workspace, the whole of `app/src-tauri/src`, the thirty-five window modules,
and the four `sefer-crates` crates that cross the seam (`girsa-post`,
`girsa-ksav`, `girsa-source`, and the parts of `girsa-ref` they use). Three
findings were then **executed**: finding 1 was reproduced with a standalone
binary against the real `girsa-ksav`, and the window was driven over CDP against
the reader's own 7,189-sefer shelf for the live checks in
[§ What was checked in the running window](#what-was-checked-in-the-running-window).
Every claim below says which kind it is. Nothing here is an estimate presented
as a measurement.

**Relationship to [`AUDIT.md`](AUDIT.md).** That document is the performance
audit of the same day, written against `c60e1ef`. Its findings 1 and 2 have
since been implemented in `15cebb3` — the shelf moved behind its own `RwLock`,
`find` drops the state guard before searching, and `add_mine`, `export_sefer`
and `buffer_save` stopped holding it across I/O. **This audit does not repeat
any of that.** Where it touches the same ground it is because the fix was
applied to the three commands that audit named and not to the nine it did not;
see [§4](#4--responsiveness-the-doctrine-and-the-nine-commands-it-was-not-applied-to).

**A note on the codebase.** The standard here is high and the audit should say
so plainly. Corrections are counted in characters and not bytes with a test
named after the bug; the personal layer is an append-only log with
write-beside-and-rename compaction; the LRU that holds open seforim has a module
note explaining why a queue was the wrong shape; `docs/not-yet.md` is an honest
public list of what does not work. Nothing in this report was easy to find, and
none of it is a beginner's defect. The findings cluster in exactly the two
places a codebase of this quality leaves them: **the seams between two
processes**, and **the paths a guard was written for but does not reach.**

---

## In one paragraph

Thirteen headings and about thirty distinct defects, of which six matter this
week. **One is a reproduced
crash**: a 26 KB `.ksav` file aborts the process through unbounded recursion in
`girsa-ksav`, which both applications compile, and which Girsa reaches by
dropping a file on the window. **One is a silent wrong quote**: `/refresh` drops
citations it cannot parse from a list the pen zips by position, so one bad ref
in a document re-quotes everything after it from the wrong place. **Three are
durability**: the document you are typing, your shelf arrangement and your saved
copy are all written with a truncating `fs::write`, in a layer whose own log
does write-beside-and-rename and says why. **One is measured**: a chain walk
holds the global state lock for 2.4 seconds, during which a 3 ms scroll takes
2,423 ms — the same defect `15cebb3` fixed for `find`, in nine commands it did
not reach. And one is a single click: `00:00` in the Settings hour list puts the
luach a day ahead for ever. Everything else is smaller, and the codebase is
better than any of these findings suggest; they sit almost entirely on the seams
between two processes and on the paths a guard was written for but does not
reach.

---

## Contents

| | | Severity |
|---|---|---|
| 1 | [A `.ksav` file crashes the process — reproduced](#1--a-ksav-file-crashes-the-process) | **Critical** |
| 2 | [A document Ksav refreshes can be re-quoted with the wrong sources](#2--a-document-ksav-refreshes-can-be-re-quoted-with-the-wrong-sources) | **High** |
| 3 | [Five ways the reader's own writing is not durable](#3--five-ways-the-readers-own-writing-is-not-durable) | **High** |
| 4 | [The responsiveness doctrine, and the nine commands it was not applied to](#4--responsiveness-the-doctrine-and-the-nine-commands-it-was-not-applied-to) | **High** |
| 5 | [Two embedding jobs, one store](#5--two-embedding-jobs-one-store) | **High** |
| 6 | [`00:00` puts the luach a day ahead, permanently](#6--0000-puts-the-luach-a-day-ahead-permanently) | **High** |
| 7 | [Ctrl+C fails silently on the path a reader actually uses](#7--ctrlc-fails-silently-on-the-path-a-reader-actually-uses) | **Medium** |
| 8 | [Six defects in the loopback and the packet](#8--six-defects-in-the-loopback-and-the-packet) | **Medium** |
| 9 | [The find bar is the one panel without the stale-answer guard](#9--the-find-bar-is-the-one-panel-without-the-stale-answer-guard) | **Medium** |
| 10 | [Two guards that do not reach what they were written for](#10--two-guards-that-do-not-reach-what-they-were-written-for) | **Medium** |
| 11 | [Four caches that outlive their question](#11--four-caches-that-outlive-their-question) | **Medium** |
| 12 | [`..` is not on the list of things a downloaded name may not contain](#12----is-not-on-the-list-of-things-a-downloaded-name-may-not-contain) | **Medium** |
| 13 | [Smaller things, with their evidence](#13--smaller-things-with-their-evidence) | Low |
| | [What was checked and found sound](#what-was-checked-and-found-sound) | |
| | [What was checked in the running window](#what-was-checked-in-the-running-window) | |
| | [Suggested order of work](#suggested-order-of-work) | |

---

## 1 · A `.ksav` file crashes the process

**Severity: Critical. Reproduced, not inferred.**

**Where.** `sefer-crates/crates/girsa-ksav/src/read.rs:294` (`read`), `:348`
(`run`), `:395` (`command`), `:512` (`content`), `:540` (`sub`).

**What.** The document reader is mutually recursive — `run` → `command` →
`content` → `sub` → `run` — and **nothing bounds the recursion**. The `depth`
parameter threaded through all four is a *list-nesting label* that ends up on
`Block::Item`; it is `saturating_add`ed and never compared against anything.

A document that opens a content command without closing it costs one stack frame
per level, and the levels are free to the writer.

**Reproduction.** A standalone binary against `girsa-ksav` at `779225d`, release
profile, on the 8 MB main thread:

```
depth   400  →  survived: 1 blocks
depth   700  →  survived: 1 blocks
depth  1000  →  thread 'main' has overflowed its stack
depth  2000  →  thread 'main' has overflowed its stack     (26 KB of input)
depth 40000  →  thread 'main' has overflowed its stack
```

The input is `"#ציטוט["` repeated *n* times, four Hebrew letters, and `]`
repeated *n* times. **A 26 KB file is enough.** A Rust stack overflow is an
immediate abort: no unwinding, no `Result`, no catch — the process is gone
between one instruction and the next.

**Why it matters here.** Three things make this worse than a parser bug:

1. **It is reachable by a gesture the window advertises.** Dropping a file on
   the window is `add_mine` (`lib.rs:1510`) → `girsa_app::shelf::read_mine` →
   `girsa_corpus::import::mine.rs:214`, which is `girsa_ksav::read(&markup)`.
   Reading somebody else's document is a thing this application exists to do.
2. **The thread has less stack than the reproduction did.** Every command is
   `#[tauri::command(async)]`, which puts it on the runtime's blocking pool.
   Those threads do not get the main thread's 8 MB, so the real threshold is
   some hundreds of levels, not a thousand.
3. **It takes the writing drawer with it.** `writing.ts` autosaves on a 900 ms
   timer (`SAVE_AFTER_MS`). An abort loses up to 900 ms of typing with no
   chance to flush — and see [finding 3](#3--five-ways-the-readers-own-writing-is-not-durable),
   which is why the file on disk may be worse than 900 ms stale.

**And it is in the shared crate**, so it is Ksav's bug too, on its own primary
input: the documents its users open.

**Fix.** Two lines and a test.

* Carry a real recursion counter (separate from the list `depth`), cap it —
  64 is generous for a document; Typst's own nesting limit is in that
  neighbourhood — and return the words as literal text past the cap rather than
  recursing. A truncated reading of a pathological document is the right answer;
  an aborted process never is.
* While there: `sub()` re-collects `markup.char_indices()` into a fresh `Vec`
  for every nested command (`read.rs:540`). Nesting *n* deep copies the tail
  *n* times, so the same input is quadratic in time and memory before it is
  fatal in stack.
* The test belongs in `girsa-ksav`'s own `prohibitions.rs`, which is where this
  crate keeps the rules it will not break.

---

## 2 · A document Ksav refreshes can be re-quoted with the wrong sources

**Severity: High. Source read; contract stated in the code's own doc comments.**

**Where.** `crates/girsa-desk/src/refreshing.rs:69` (`wanted`) and `:175`
(`refreshed_reporting`), served by `app/src-tauri/src/post.rs:258` (`/refresh`).

**The contract, in the module's own words** (`refreshing.rs:34`):

> *"One row per citation, **in the order they appear in the document**, from the
> same `girsa_ksav::cited_in` both applications compile. The pen re-runs that
> scanner on its own buffer and **zips by position**: one scanner, one order,
> and no ref matched by string."*

**What the code does** (`refreshing.rs:69`):

```rust
pub fn wanted(markup: &str) -> Vec<Wanted> {
    girsa_ksav::cited_in(markup)
        .into_iter()
        .filter_map(|cited| Some(Wanted {
            reference: cited.reference.parse().ok()?,   // ← the row disappears
            range: cited.range,
        }))
        .collect()
}
```

`cited_in` yields every `מקור:` whose value starts with `girsa:`. `wanted` then
**silently drops** the ones this build's `Ref` parser rejects — its own doc says
so: *"A `מקור:` whose value does not parse as a ref is dropped rather than
reported."*

**So the two scanners do not agree on the count.** Ksav scans its buffer and
gets *n* citations; Girsa returns *n − k* rows; the pen zips by position. Every
citation after the first dropped one is paired with **the words of a different
place** — and the failure is invisible, because each row is individually
well-formed and carries a plausible citation.

This is precisely the failure the design exists to prevent. `sending.rs:98`:

> *"A quote taken from the se'if next door is exactly the silent wrongness this
> system exists to make impossible."*

**How a ref fails to parse in practice.** Not only malice: a document written by
a newer Girsa whose ref syntax this build does not know; a ref hand-edited in
the `.typ`; a `girsa:` string a copy-paste truncated. The pen has no way to tell
— `/refresh` also returns `"total": rows.len()`, which is the *post-drop* count,
so even a caller that wanted to check gets the number that agrees with itself.

**Fix.** Make the row list total. Give `Refreshed` a variant for *this ref did
not parse* — the type already carries `trouble: Option<String>` for exactly this
purpose, and `Refreshed::lost` already exists. Drop nothing:

```rust
// in wanted(): keep the row, mark it
match cited.reference.parse() {
    Ok(reference) => Wanted::Parsed { reference, range: cited.range },
    Err(_)        => Wanted::Unreadable { text: cited.reference },
}
```

Then `refreshed_reporting` emits one row per citation always, and the zip is
sound by construction rather than by luck. A test that puts one malformed
`מקור:` in the middle of three good ones and asserts four rows back is the
fence.

---

## 3 · Five ways the reader's own writing is not durable

**Severity: High. Source read.**

The personal layer's design is good: `girsa_personal::Log` is append-only, one
`write_all` per line on a handle opened for append, and the one whole-file write
it does — compaction — goes beside-and-renames (`log.rs:393`). `Session::save`
was given the same treatment with the argument written down beside it
(`session.rs:598`):

> *"What this call still owes is the atomicity, which is why it is a rename and
> not a write."*

**Three files never got that argument applied to them.**

### 3a · The document you are typing — `crates/girsa-desk/src/buffer.rs:126`

```rust
std::fs::write(&path, &self.text)
```

`fs::write` opens with `O_TRUNC`. The file is empty from the moment of the open
until the write returns. This is the **most frequently written file in the
application** — `writing.ts:172` schedules a save 900 ms after every keystroke —
and it holds the one thing in the layer that cannot be re-derived from anything
else. A crash or a power loss inside that window leaves a truncated or empty
document where the reader's writing was.

The same call is the tail of `buffer_to_ksav` and `buffer_save`.

### 3b · Your shelf arrangement — `crates/girsa-app/src/arrangement.rs:215`

Same `fs::write`, same exposure, for the file that holds every shelf the reader
made, every sefer they moved and every rename. The failure mode is already
written out at `arrangement.rs:196` — *"your shelf arrangement would not read
… and the shipped shelf is being shown"* — which is what a reader would see
after a torn write: their whole arrangement replaced by the shipped one.

**Fix for both:** the four lines from `session.rs:614`, verbatim.

### 3c · A copy written into the reader's own folder — `lib.rs:3676`

`buffer_write_to` does `create_dir_all` then `fs::write` on a path the reader
chose, which may be a network share or a removable disk. Same truncate-first
exposure, on the one copy the reader deliberately made to keep.

### 3d · Compaction can lose a record when a second process is running

`Log::rewrite` (`log.rs:393`) reads the live records, writes them beside, and
renames over. That is atomic against a crash and **not** against a second
writer. Girsa ships two other processes that open the same personal layer: the
MCP server (`girsa-mcp`, whose whole point is that a program can write into your
own layer) and `girsa-suspects`. An append made by one between another's read
and its rename is gone, with no error on either side.

Not urgent — it needs both processes live and a compaction in the same
millisecond window — but it is the one place where an append-only log stops
being append-only, and it is worth a lock file rather than a comment.

### 3e · Renaming a document silently overwrites another one

`writing.ts:161` — `rename()` sets `this.name` and saves. `Buffer::save`
truncates whatever is at the new name. Renaming *ראש השנה* to a name you are
already using destroys the document you had there, with no prompt and no
mention. The method's own comment is careful about the *old* file (*"a rename
that quietly deleted the thing you had been writing is not a rename"*) and does
not consider the new one.

---

## 4 · Responsiveness: the doctrine, and the nine commands it was not applied to

**Severity: High (cumulative). Source read.**

`15cebb3` implemented [`AUDIT.md`](AUDIT.md) findings 1 and 2 and did it well.
`find` now reads the chips, **drops the guard**, searches, and re-locks only to
name the rows — with the reasoning written at `lib.rs:735`:

> *"for every one of those milliseconds the scroll handler beside it could not
> be served, in a panel whose whole design is to stay open while you read."*

That reasoning is general and the fix was applied to `find`, `find_rung`,
`add_mine`, `export_sefer` and `buffer_save`. **Nine other commands hold the
global `Mutex<State>` across work of the same kind or worse**, and four of them
are reachable from Ksav rather than from the window, which means the reader did
not even ask for the pause they are about to get.

| Command | Where | What it holds the lock across |
|---|---|---|
| `lane_ask` | `lib.rs:4822` | Embeds the query with a neural model and searches every vector store in the lane. The heaviest single operation in the application. |
| `chain_walk` | `lib.rs:3300` | A depth-12 graph walk that reads `edges.jsonl` shards off disk. Berakhot's alone is 3.4 MB / 21,065 rows. **Measured at 2,443 ms, during which a 3 ms scroll took 2,423 ms.** |
| `chain_forks` | `lib.rs:3344` | The same walk. |
| `links` | `lib.rs:2926` | `girsa_app::touching` over the repair layer, plus `Lenses::load`, which is a file read. Asked on every click on a line. **Measured at 128 ms.** |
| `who_cites` | `lib.rs:3766` | `State::documents` → `Documents::open` (parse the registry) **and `refreshed()`, which reads the full text of every stale `.ksav` the reader has registered.** No cap on file size or count. Asked on a click. Measured at 10 ms on a machine whose registry is empty — the cost at scale is a reading of the code, not a measurement. |
| `buffer_write_to` | `lib.rs:3676` | `create_dir_all` + `fs::write` to a folder that may be a network share. |
| `export_layer` | `lib.rs:6154` | Writes the entire personal layer to disk. |
| `send_to_ksav` | `lib.rs:3854` | **A blocking loopback round trip to another process** — see below. |
| `/refresh`, `/document`, `/where-from`, `/linkify` | `post.rs:258`, `:454`, `:336`, `:421` | Ksav's errands, all four taking the state lock in the desk's serving thread. `/refresh` regenerates every citation in a document, opening seforim as it goes; `/document` re-reads document files; `/where-from` runs a full index search. |

Three of these deserve to be named on their own.

**`send_to_ksav` can freeze the window for five seconds, and can wedge both
applications.** It takes the lock, builds the packet, and calls
`girsa_post::send` **with the guard still alive**. `girsa-post` allows 400 ms
per read and a 5 s whole-exchange deadline (`girsa-post/src/lib.rs:24`, `:31`).
So a Ksav that is slow, paging in, or waiting on a modal dialog holds Girsa's
entire state lock for up to five seconds — the scroll handler included.

Worse, it is a **cycle**: Girsa's desk serves Ksav's errands on a single thread
(`desk.rs:88` — `for request in server.incoming_requests()`), and every handler
takes the same lock. If Ksav, while handling `/insert`, asks Girsa anything at
all — a `/cite` to print the citation it was just handed is the obvious one —
that request blocks on the lock `send_to_ksav` is holding, and `send_to_ksav`
blocks on the reply. Nothing deadlocks permanently, because the 5 s deadline
breaks it; but the reader gets a five-second freeze and an error for an
operation that worked.

**`/health` cannot be answered while a long errand runs**, for the same
single-threaded reason. Ksav's presence chip therefore reports Girsa as
`Stale` — *"the answer was not girsa's"*, the state that exists for a killed
process leaving its endpoint file behind — while Girsa is merely busy answering
Ksav's own previous question. Presence is designed as three states precisely so
that this distinction survives, and this path collapses it.

**`marks_in` renders the whole sefer to place a handful of marks**
(`lib.rs:5887`). Before it asks the layer anything, it builds a
`HashMap<String, String>` of **every segment in the sefer**, each one run
through `shemos::written` and `display::Shown` — tens of megabytes of string
work for Berakhot — and only then iterates `shelf.marks().in_work(&slug)`, which
is usually empty. It is called on every pane open (`main.ts:674`) and again
after every change to the layer (`repaintMarks`). **Measured on a shelf with no
highlights in either sefer: 15 ms for Berakhot and 70 ms for Shulchan Arukh
Orach Chayim, to return an empty list both times.** Asking for the marks first
and rendering only the segments that carry one is a five-line change.

**Two smaller ones on the same theme.** `mefarshim_of` (`lib.rs:1876`) clones
the whole `Marks` table on every call — the table the cache one field up exists
to avoid re-reading. And the OCR path sends a rasterised page across the IPC
boundary as `png: Vec<u8>` ⇄ `number[]` (`lib.rs:2378`, `scanview.ts:682`): a
300-dpi page is megabytes, JSON-encoded as decimal integers at roughly 4× that,
once per page, for a job that runs over a whole sefer.

**A refinement, not a contradiction, on `copy` and `sefer_sheet`.**
[`AUDIT.md`](AUDIT.md) records that these two are deliberately *not*
`(async)` — *"the clipboard and the print sheet talk to the platform rather than
to the shelf, they are fast"* — and tells a future agent not to re-fix it. The
reasoning is right about what those commands *do* and wrong about what they can
*wait for*: both call `State::sefer` / `State::reading`, which on a cache miss
reads a whole work off disk (11 ms in the published table, more on a cold cache
or a large sefer) **on the thread that owns the message loop**. Ctrl+C on a
sefer that has just been evicted from the twelve-entry cache is a synchronous
disk read in the paint path. The cheap fix is not to make them `(async)` but to
call `hold(&shared, slug)` first, exactly as `open_sefer` does.

---

## 5 · Two embedding jobs, one store

**Severity: High. Source read.**

**Where.** `lib.rs:5053` (`lane_embed`), `app/src/laneview.ts:508`–`:529`.

`lane_embed` copies what it needs, drops the guard, spawns a thread and returns
`Ok(())` immediately — the right shape, and `AUDIT.md` holds it up as the model.
**Nothing stops a second one from starting.** The only guard is
`go.disabled = this.working` at `laneview.ts:514`, which is evaluated *when the
row is drawn*; the click handler sets `this.working = true` but never disables
the button it is on, and the row is not redrawn until the first progress event
arrives. Two clicks inside that gap start two threads.

What the second thread does:

* opens its **own** `Vectors` store for the same slug (`girsa-lane/src/lane.rs:680`
  → `Vectors::open`) and appends to the same files as the first;
* re-embeds every segment the first is already embedding — double the CPU or GPU
  for a job measured in hours;
* **defeats Stop.** `lib.rs:5090` — every run begins `stop.store(false)` on the
  one shared `AtomicBool`. A run that starts after the reader pressed Stop
  clears the flag for the run that was stopping.

And `told()` (`laneview.ts:277`) sets `working = false` on the **first** `done`
event, so the button comes back while the other job is still running.

`lane_bring` has the same shape (`lib.rs:4930`, `laneview.ts:428`): two clicks,
two concurrent downloads of a multi-hundred-megabyte model into one directory.

**Fix.** The flag belongs in Rust, where the job is: refuse a second
`lane_embed` while one is in flight (`AtomicBool` swap, or hold the job handle
on `State`), and return a named refusal the window can say. Disabling the button
in the click handler is worth doing too and is not sufficient on its own — the
loopback and a second window are not bound by the DOM.

---

## 6 · `00:00` puts the luach a day ahead, permanently

**Severity: High. Source read; one click to reproduce.**

**Where.** `crates/girsa-app/src/luach.rs:646`, `lib.rs:4520`
(`set_day_turns_at`), `app/src/settingsview.ts:298`.

```rust
pub fn at(date: Civil, hour: u8, turns_at: u8) -> Luach {
    let rd = fixed_from_civil(date);
    of_fixed(if hour >= turns_at { rd + 1 } else { rd })
}
```

The setting is offered in Settings as a plain list of all twenty-four hours:

```ts
Array.from({ length: 24 }, (_, hour) => ({ value: String(hour), label: `${...}:00` }))
```

Choose `00:00` and `hour >= 0` is true at every hour of every day — reproduced
against the real session, [below](#0000--finding-6-reproduced-against-the-readers-own-session). The luach
shows **tomorrow's** daf, all day, for ever — with today's date in the header
above it, because `of_fixed` recomputes the civil date from the advanced day.
`set_day_turns_at` validates the upper bound (`hour > 23` is refused, with a
sentence) and not the lower one, because `0` is a valid hour; it is not a valid
*turnover*.

The reader who reaches for this setting is, by the comment at
`settingsview.ts:292`, *"a reader who has noticed the daf is a day behind at
night"* — someone already unsure which day the window is on. Handing them a
one-click way to be permanently a day ahead is the worst available outcome.

**Fix.** Either drop `00:00` from the list and refuse `0` in `set_day_turns_at`
with a named refusal, or — better — define `0` as *never turn over* and write
`if turns_at > 0 && hour >= turns_at`, which gives the reader the midnight
behaviour the option looks like it is offering.

---

## 7 · Ctrl+C fails silently on the path a reader actually uses

**Severity: Medium. Source read.**

**Where.** `app/src/main.ts:2250` (`copySource`), `app/src/pane.ts:988`
(`selection`).

Two distinct defects on the shortest, most-used path in the application.

**7a · No `catch` on the sefer branch.** `copySource` wraps its *scan* branch in
`try/catch` and hands failures to `trouble()` — the module written for exactly
this. The **sefer** branch, twenty lines below, has none:

```ts
copied = await api.copy(chosen.from, chosen.to, chosen.fromChar, chosen.toChar);
```

and the caller is `void copySource()` (`main.ts:2219`), so a rejection becomes an
unhandled promise rejection and **nothing at all appears**. Every refusal
`girsa_app::send` can make dies there — including `SendError::Empty`, whose own
doc explains that a quote is refused rather than sent empty because *"a quote
block with no words in it arrives in the document looking like a failure of the
paste"*. The refusal is made, correctly, and then thrown away. Highlighting
punctuation or trailing whitespace and pressing Ctrl+C does nothing and says
nothing. Note also that the default is deliberately not prevented, so the
webview's own plain-text copy still happens — the reader gets *some* clipboard
content and no sign that the source packet never went.

`sourceForBuffer` (`main.ts:2340`) has the same shape.

**7b · A highlight in the other pane is read as no highlight.**
`PaneView.selection()` returns `null` in three different situations: nothing is
selected; the selection is not inside a line; **and the selection is not inside
*this* pane** (`pane.ts:993` — `if (!this.body.contains(from) …) return null`).
The caller cannot tell them apart and treats `null` as *nothing selected*, which
means it copies **the whole line the reader is standing on in the focused
pane** — a different passage from the one highlighted, with a correct-looking
citation on it and no error.

That is the same class as the bug `sending.rs` is proudest of refusing. Return
a discriminated result (`"none" | "elsewhere" | Selection`), and let the caller
say *the highlight is in the other pane* rather than quietly quoting something
else.

---

## 8 · Six defects in the loopback and the packet

**Severity: Medium (8a is arguably High). Source read.**

### 8a · A document Ksav says it deleted keeps answering "where did I use this"

`post.rs:454` — `/document`, the `forget` branch:

```rust
if saved.forget {
    return match documents.forget(&path) { … };     // ← returns here
}
…
state.documents = None;                              // ← never reached
```

The registry on disk is updated; **the copy Girsa is holding is not**.
`State::documents` (`lib.rs:565`) reads once and holds — its own doc says *"The
desk's `/document` clears it, because that is where a row is added"* — and the
clear is on the add path only. Until the window restarts, `who_cites` keeps
naming a document Ksav has told Girsa is gone. One line, in the branch that
returns early.

### 8b · The `pid` field is written and never read

`girsa-post/src/lib.rs:136`:

> *"`pid`: So a stale file can be told from a live one **before anything is
> sent**."*

`std::process::id()` fills it at `desk.rs:115`. Nothing anywhere reads it —
`send()` (`lib.rs:322`) goes straight to `TcpStream::connect_timeout`. The
staleness check the field documents does not exist, which is why every stale
endpoint costs a connect timeout instead of a `kill(pid, 0)`. Either implement
it or delete the field and the sentence; a documented mechanism that is not
there is worse than an absent one, because the next reader will trust it.

### 8c · The escaper covers the characters, not the line starts

`girsa-ksav/src/lib.rs:386`:

```rust
pub const MARKUP: &[char] = &['#', '[', ']', '\\', '$', '*', '_', '<', '>', '@'];
```

Ten characters, and the note beside them is right that five were once missing.
But Typst also reads constructs at the **start of a line** — `=` (heading),
`-` and `+` (list item), `/` (term list), `1.` (enum) — and
`quote_block(text)` (`lib.rs:104`) embeds multi-line corpus text into `#ציטוט[…]`
verbatim. A quoted line beginning with any of those changes the structure of the
reader's document rather than its content, which is the failure mode the escaper
exists to prevent. Escape at line starts as well, or normalise the leading
character.

### 8d · `cited_in` will find a citation inside a quotation

`girsa-ksav/src/lib.rs:176` scans for the literal `מקור:` anywhere in the
markup, with no regard for whether it is inside a `[…]` body. A quoted passage
containing that string followed by a `"girsa:…"` — which is exactly what a
sefer *about* Girsa citations, or a document quoting another document, would
contain — is counted as a citation of the enclosing document. `retargeted`
(`:219`) then rewrites it, against its own promise that *"a `girsa:` ref sitting
in the prose, in a comment, or in somebody else's markup … is not this
function's to rewrite."*

### 8e · `/document` means two different things in the two directions

Ksav → Girsa (`post.rs:454`) it carries `{path, name, forget}` — *a document is
saved here*. Girsa → Ksav (`lib.rs:3800`, `buffer_to_ksav`) it carries
`{name, text}` — *take this document*. Two errands, one name, one shared crate,
and nothing on either side that says so. Rename one.

### 8f · `clipboard::put` reports success it did not verify

`clipboard.rs:88` sets `plain: true, html: true, packet: true` on a single
`Ok(())` from `context.set`. The struct's own doc says the point is that what
reached the clipboard is *"reported rather than assumed. A copy that put down
two flavours out of three is a paste into Ksav that arrives as plain text."*
`clipboard-rs` sets the list in one open, so the all-or-nothing reading is
defensible — but then the three booleans are one boolean wearing a costume, and
the sentence promising otherwise should go.

---

## 9 · The find bar is the one panel without the stale-answer guard

**Severity: Medium. Source read.**

`latest.ts` exists so that no panel has to grow its own stale-answer rule, and
seven do use it: `picker`, `search`, `shelf`, `scopeview`, `tocview`,
`linksview`, `chainview`, `desksview`. **`findhere.ts` does not.**

`look()` (`findhere.ts:217`) guards only with `if (query === this.asked) return`,
and it sets `this.asked` *before* the await. Two consequences:

* **Out-of-order answers.** `drawChips` (`:245`) resets `this.asked = null` and
  calls `look()` again when a chip is clicked; a chip changed while a search is
  in flight leaves two asks running, and `this.places` / `this.total` belong to
  whichever returns last, not to what the reader last asked.
* **A closed bar still moves the page.** `close()` (`:192`) hides the element
  and clears the debounce timer but does not cancel the ask, does not null
  `this.pane`, and does not invalidate anything. An answer that lands after the
  close runs `this.show()` → `pane.goToWords(...)`, which scrolls the reader
  somewhere they did not ask to be — directly against the method's own comment
  that *"The reader keeps the place the find put them on. Nothing is scrolled
  back."*

There is a wrinkle worth noting for whoever fixes it: [`AUDIT.md`](AUDIT.md)
finding 3 recommends copying `findhere.ts`'s debounce into the picker, and the
picker's own comment (`picker.ts:108`) says *"A debounce and a stale-answer
guard solve two different problems, so both."* The picker has both. The file
being held up as the model has one.

**Fix.** A `Latest` on `FindHere`, a ticket taken in `look()` and checked before
`this.places` is assigned and again before `show()`, and a ticket burned in
`close()`.

---

## 10 · Two guards that do not reach what they were written for

**Severity: Medium. Source read; counted.**

### 10a · The shell writes two Hebrew sentences the reader sees

`crates/girsa-app/tests/the_rules_this_repository_wrote_down.rs:832` —
`the_shell_does_not_write_sentences_for_the_reader` — is the fence for finding
20, and its comment names the exact defect:

> *"Against exactly one sentence in that crate written in Hebrew — which is the
> same defect pointing the other way, because an English window would have shown
> that one in Hebrew."*

There are two now, and both are on success paths, which the guard does not look
at:

* `lib.rs:2866` — `unfix` returns `said: "הוחזר כפי שנדפס".to_string()`.
* `lib.rs:3260` — `export_sefer` composes
  `"בלי תיקונים" / "תיקון אחד" / format!("{n} תיקונים")`.

The second is worse than a language bug: it is a **hand-rolled Hebrew number
agreement** in the shell, three weeks after `93f4979` established that the
window says a count as *label: number* precisely because agreement is
`girsa_plain::said`'s job and nobody else's. It is the `1 arrangements` finding,
in Rust, on the export toast.

### 10b · The guard matches four shapes; fifty-three sentences use others

The same test looks for exactly these:

```rust
trimmed.contains("Err(format!(\"")
  || trimmed.contains("trouble: Some(format!(")
  || trimmed.contains("refused: Some(format!(")
  || (trimmed.starts_with("return Err(\"") && …)
```

A sweep of `lib.rs` and `post.rs` for the shapes it does **not** see —
`ok_or("…")`, `ok_or_else(|| format!("…"))`, `ok_or_else(|| "…".to_string())` —
finds **53 more**, every one of them a sentence the shell composed for the
reader. A sample:

```
lib.rs:2266  ok_or_else(|| format!("{slug} is not a scan"))
lib.rs:2441  ok_or_else(|| format!("nobody has read page {page} of {slug}"))
lib.rs:2500  ok_or("those words are not on this page")
lib.rs:3081  ok_or("which type?")
lib.rs:3114  ok_or("that is not an edge")
lib.rs:3441  ok_or("there is no such candidate")
```

**To be fair to the design, this is not the original bug.** `trouble.ts:254`
falls back to `troubleUnknown` for anything it cannot name, so what a Hebrew
reader sees is a Hebrew sentence, with the English on the hover. The cost is
different and still real: *"nobody has read page 12 of this scan"* — an
actionable sentence — reaches the reader as *"something went wrong"*. The guard
was written to make refusals nameable; fifty-three of them are still anonymous.

**Fix.** Widen the guard to `ok_or`/`ok_or_else`/`map_err` producing a string
literal or a `format!`, and let the resulting red build name them. Most deserve
an existing `Code`; a handful want new ones, which is what
`every_refusal_this_codebase_names_has_a_sentence_in_the_window` already
enforces.

---

## 11 · Four caches that outlive their question

**Severity: Medium. Source read.**

### 11a · The chain cache has no ceiling

`crates/girsa-link/src/chain.rs:314` — `Cache { works: HashMap<String, Held> }`.
Every work a chain walk touches is read in and **kept for the life of the
process**; it is cleared only on a repair or a corpus change. A depth-12 walk
crosses many works, Shulchan Arukh Orach Chayim alone is 156,076 edges, and
Berakhot's shard is 3.4 MB. Nothing evicts.

Beside it, `girsa_app::held::Held` caps open seforim at twelve and has a module
note arguing why — *"a work is tens of megabytes of text and a reader has a
handful open, not a library"*. The argument applies to edges. The cap does not.

### 11b · `choose_corpus` clears eight things and leaves the scope

`lib.rs:763` clears the shelf, the chain cache, the search bar, the lexicon, the
lane, the open seforim, the marks, the joins, the words, the documents and the
queue — and its own comment is emphatic: *"Nothing about the old corpus
survives… a stale answer about a sefer that still exists in the new corpus is
worse than no answer: it looks right."*

`state.chips` and `state.here_chips` survive. The scope inside them holds work
slugs and shelf keys **from the corpus that was just replaced**, so the first
search after pointing the window at a new folder runs against a filter naming
seforim that are not there. Two lines.

### 11c · A document saved twice in one second is never re-read

`documents.rs:103` — `is_stale()` is `modified(path) > self.read_at`, both
truncated to whole seconds (`:126`). The sequence that loses a save is the
normal one: Ksav writes the file at second *T* and immediately posts
`/document`; Girsa reads it and records `read_at = T`; Ksav writes again inside
the same second. `T > T` is false, so the second save is invisible to
`who_cites` until some later write bumps the clock. Compare `!=` rather than
`>`, or keep sub-second precision, or record the size as well.

### 11d · `chain_walk` drops the cache on any early return

`lib.rs:3311` takes the cache with `std::mem::take` and puts it back only after
the walk returns. Any `?` in between — no shelf, no timeline, a poisoned lock —
returns from the function with `state.chains` left as `default()`, silently
discarding everything read so far. Harmless in effect, but it is a `mem::take`
without a guard, which is the shape that becomes a real bug the first time
somebody adds a fallible line to the block.

---

## 12 · `..` is not on the list of things a downloaded name may not contain

**Severity: Medium (hardening; requires a hostile or compromised upstream).
Source read.**

`crates/girsa-corpus/src/fetch.rs:213` — `disk_path` is a real sanitiser: it
percent-encodes the eight characters Windows forbids and the control range,
handles trailing dots and spaces, and prefixes the twenty-two reserved device
names. It is clearly written to defend against a hostile name.

It does not touch `..`, and the name comes off the wire —
`target_from(o: Object)` at `:166` takes `o.name` straight out of the bucket
listing JSON. `is_wanted_text` (`:162`) requires the name to contain `/Hebrew/`
and end in `/merged.json`, which
`anything/Hebrew/../../../../../evil/merged.json` satisfies. `root.join(...)` on
that escapes the corpus root and `fetch_one` (`:414`) writes there.

The exposure needs a compromised bucket or a successful MITM against HTTPS, so
it is not urgent. It is four lines: reject any component that is `..`, `.`, or
empty, and reject an absolute path, before joining.

---

## 13 · Smaller things, with their evidence

* **`desk_open` discards an unnamed layout.** `lib.rs:4206` saves the current
  workspace into `desks` only `if let Some(here) = state.session.desk`. A reader
  who has never named their arrangement, then opens a saved desk, loses every
  tab and pane they had, with no prompt. Either refuse until it is named, or
  keep an implicit `""` desk.
* **`desk_keep` overwrites a desk of the same name** without a word
  (`lib.rs:4185`).
* **`quote()`'s reverse-selection branch widens the quote.** `sending.rs:398` —
  when `first > last` the ends are swapped and `from_char`/`to_char` are
  **replaced with `0` / `None`**, silently promoting a partial selection to whole
  segments. Unreachable from the window (a DOM `Range` is always in document
  order), reachable from the MCP server and the loopback, which both take the
  offsets from a caller.
* **`open_sefer` scans the whole sefer for nikud.** `lib.rs:2050` —
  `segments.iter().any(display::has_marks)` short-circuits on a pointed sefer
  and reads all 7.7 MB of an unpointed one, on every open. The answer is a
  property of the work and belongs on the catalogue row, computed at import.
* **`docs/not-yet.md` has rotted on one row.** Its first entry is
  *"The window says `1 arrangements`"*, closed by `93f4979`. The page's own
  closing rule says a copy nothing regenerates is a copy that rots; this is the
  first line of the rot, and the second is finding 10a above, which is that same
  defect reopened in Rust.

---

## What was checked and found sound

Named so that nobody re-derives them. Each was read for this audit and is
correct as it stands.

* **Correction application is character-counted and overlap-safe.**
  `girsa-fix/src/lib.rs:617` — `applying` collects `Vec<char>`, sorts resolved
  spans, refuses a second patch that starts before the last one ended, and keeps
  `wrote` and `at` in step across both the applied and the noted branch. The
  arithmetic is right in the case that is easy to get wrong (a patch that is
  *noted* rather than applied does not advance the cursor, and the following
  `head` copy covers it).
* **The append-only log.** `girsa-personal/src/log.rs` — one `write_all` per
  append on a handle opened for append; compaction beside-and-rename; a
  tombstone that cannot be mistaken for a record. Single-process behaviour is
  right; see 3d for the multi-process caveat.
* **The LRU.** `girsa-app/src/held.rs` — `get` touches, `put` evicts the true
  oldest, and the module note correctly identifies the queue-vs-cache bug it
  replaced. `peek` deliberately does not touch, and every hot path that uses it
  has already touched through `sefer()` first.
* **Repeated headings across a scroll boundary.** `only_when_it_changes`
  (`view.rs:311`) restarts per call, so each 600-line chunk re-sends `above` on
  its first line — and `pane.ts:395` compensates by walking back through the
  lines it already holds. Checked specifically; not a bug.
* **Pane identity across a desk switch.** `draw()` (`main.ts:551`) keeps a
  `PaneView` whose id is still open without comparing its slug, and pane ids are
  minted per workspace so two desks can use the same ones. The hazard is real
  and is closed at the one place it arises: `desksview.onChanged`
  (`main.ts:171`) clears `views` and `scans` before reloading. Worth a test; not
  worth a fix.
* **The token check on the desk.** `desk.rs:129` — checked before the path,
  including for `/health`, with a real constant-time compare; the endpoint file
  is created private rather than chmodded into privacy, and the directory is
  restricted too.
* **The MCP server is stdio only.** No socket, no port. The multi-process
  concern in 3d is about the shared personal layer, not about the transport.
* **`Session::save` is atomic and throttled correctly** — write-beside-and-
  rename, with scroll positions throttled against `SAVE_SCROLL_EVERY` and every
  decision saving at once.
* **The `hold` / `hold_marks` pre-warm pattern** (`lib.rs:498`, `:530`) is
  correct, including the double-check after re-locking, and is the model the
  nine commands in finding 4 should follow.

---

## What was checked in the running window

`girsa-shell.exe` built with `--features tauri/custom-protocol` and launched
with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`,
driven over CDP against the reader's own **7,189-sefer** shelf. `invoke` is
reached by importing the same core chunk the page already loaded, so the calls
below are the window's own bridge and not a second one.

### The lock, measured — and the fix that works, measured beside it

Two commands, each raced against a scroll (`sefer_lines`, 40 lines of Berakhot)
issued 20 ms after it. The scroll costs **3–5 ms** with nothing else running.

| While this runs… | It takes | …a scroll issued 20 ms later takes |
|---|---|---|
| `find` — regex `/א.מר/`, 313,048 hits over the whole shelf | 116 ms | **5 ms** |
| `chain_walk` — Berakhot 2a, back, depth 12 | 2,443 ms | **2,423 ms** |

That is finding 4 in one table. The command `15cebb3` fixed is genuinely fixed:
a regex over the whole shelf no longer stops the reading. The command beside it,
which does more I/O and takes twenty times longer, stalls the reader for the
whole of it — and `chain_walk` is reached by clicking *The chain* in the panel
next to the daf you are reading.

For scale, the same session: `find "אמר"` — 682,286 hits — is 278 ms, and does
not block. `chain_walk` blocks for 2.4 seconds.

### `marks_in` costs 70 ms to return nothing

Measured on the same shelf, which has no highlights in either sefer:

| Call | Marks returned | Time |
|---|---|---|
| `marks_in bavli/berakhot` | 0 | 15 ms, 13 ms |
| `marks_in shulchan-arukh/orach-chayim` | 0 | **70 ms** |
| `links` on one line of Berakhot | — | 128 ms |

Every one of those milliseconds is spent under the global lock, and `marks_in`
runs twice per pane on open (`main.ts:674`) and again on every change to the
layer. The 70 ms is spent rendering 25,265 segments in order to place zero
marks.

### `00:00` — finding 6, reproduced against the reader's own session

The session's `day_turns_at` was 19 and today is 18 Elul. Setting the hour to
`0` — the first entry in the Settings list — and asking for the luach at
**09:00 in the morning**:

```
day_turns_at = 19  →  09:00  ⇒  18 ה' אלול תשפ"ו     (correct)
day_turns_at =  0  →  09:00  ⇒  19 ו' אלול תשפ"ו     (a day ahead)
day_turns_at =  0  →  00:00  ⇒  19 ו' אלול תשפ"ו     (a day ahead)
```

The setting was restored to 19 immediately afterwards and confirmed.

### The refusal that never reaches the reader

`copy` with a highlight that covers no words is refused by Rust, correctly:

```
invoke('copy', { from: 'girsa:bavli/berakhot/2a:1#1', to: …, fromChar: 3, toChar: 3 })
  ⇒  REFUSED: nothing is selected
```

That string is what `copySource` (`main.ts:2281`) throws away — there is no
`catch` on that branch and the caller is `void copySource()`. Note also that the
refusal arrives **uncoded**: it is `SendError::Empty`'s `Display` through
`map_err(|e| e.to_string())`, so even after finding 7a is fixed, `trouble.ts`
would render it as *"something went wrong"* until finding 10b is too. The two
findings are one sentence apart on the same path.

*(An attempt to drive the whole gesture — set a DOM selection over a single
space, dispatch the bound Ctrl+C — produced no toast and no captured rejection,
but the synthesized `keydown` could not be shown to have reached the binding, so
that run is recorded as inconclusive rather than as evidence.)*

### Two things the audit expected to find and did not

Recorded because a negative result is worth as much as a positive one and costs
the next reader the same hour.

* **The tab strip prints Hebrew titles, not slugs.** An earlier run showed
  `shulchan-arukh/orach-chayim` as a tab label. That was the stale binary below,
  whose IPC router has no `titles` command at all. Against a current build,
  `titles` and `open_set` both answer `שולחן ערוך, אורח חיים`.
* **Headings do not repeat at a scroll boundary.** `only_when_it_changes`
  restarts per chunk, but `pane.ts:395` walks back through the lines it already
  holds and suppresses the repeat. See
  [What was checked and found sound](#what-was-checked-and-found-sound).

### A note on the binary that was almost audited

The `app/src-tauri/target/debug/girsa-shell.exe` is dated **6 August**
and `cargo build` in that directory reports `Finished` without relinking it —
even after `cargo clean -p girsa-shell`, and even after touching `src/lib.rs`.
The binary genuinely predates the current tree: `luach`, `open_set`, `titles`
and `sefer_indices_of` do not appear in its strings and its IPC router answers
`Command not found` for all four. Building into a fresh `CARGO_TARGET_DIR`
produces a current binary from the same source, so this is a stale-fingerprint
condition in that target directory rather than anything in the source. It is
worth knowing about because **it is exactly the condition under which somebody
verifies a fix against a binary that does not contain it.**

---

## Coverage, and what this audit did not reach

An audit that does not say where it was thin is claiming a uniformity it does
not have.

**Read closely, line by line:** the whole of `app/src-tauri/src` (`lib.rs`,
`post.rs`, `clipboard.rs` — 6,790 lines); `girsa-desk` entire; `girsa-fix`'s
application path; `girsa-personal::log`; `girsa-app`'s `sending`, `held`,
`workspace`, `luach::at`, `view::only_when_it_changes`, `arrangement::save`,
`session::save`; `girsa-post` entire; `girsa-ksav`'s writer, scanner and reader;
`girsa-corpus::fetch`'s path handling; and the window modules `main.ts`,
`pane.ts`, `api.ts`, `findhere.ts`, `writing.ts`, `laneview.ts`, `scanview.ts`,
`trouble.ts`, `latest.ts`, `desksview.ts`, `settingsview.ts`.

**Read for shape only, and a second pass should go here first:**

* **`girsa-corpus`'s import** — `import/mod.rs`, `continuity.rs`, `sections.rs`,
  `taxonomy.rs`, `work.rs`, ~6,000 lines. The re-import invariants (an anchor
  surviving a split, a redirect row that keeps an old ref resolving) are the
  highest-consequence logic in the tree and were taken on the strength of their
  tests rather than re-derived.
* **`girsa-search`'s index** — `index.rs`, `citation.rs`, `ladder.rs`,
  `facets.rs`, ~4,000 lines. Read for lock behaviour and regex handling only.
  Scoring, facet arithmetic and the widening ladder's counts are unaudited.
* **`girsa-lane` / `girsa-nearby`** beyond the job lifecycle in finding 5.
  Nothing was checked about the vectors themselves.
* **`girsa-scan`** — the OCR grouping, paging schemes and anchor arithmetic.
* **`girsa-note`, `girsa-link`'s repair layer, `girsa-export`, `girsa-mcp`'s
  tool surface** — sampled, not swept.
* **The test suites themselves.** 24 window test files and the Rust tests were
  read where they bore on a finding. Nobody asked *which of these tests would
  still pass if the thing they name were broken* — which, given
  `docs/…/tests pin the bug they are named after`, is the question this
  repository would most want asked, and it is a day of work on its own.

**Two limits worth stating plainly:**

1. **Only one side of the seam is here.** Ksav's repository was not available to
   this audit. Every interop finding is derived from Girsa's side and from the
   shared crates both compile; the claims about *what Ksav does with a reply*
   (finding 2's zip-by-position, finding 8e's errand naming) are read off the
   contracts written in `girsa-desk` and `girsa-ksav`, and should be confirmed
   against Ksav before the fixes are shaped. If those contracts are wrong about
   Ksav, finding 2 changes severity in either direction.
2. **The live pass was one session, one machine, one shelf.** Windows 11,
   WebView2, the reader's own 7,189-sefer corpus, the semantic lane off and the
   document registry empty. Nothing here was checked on WebKitGTK or on macOS
   WebKit, and `docs/not-yet.md` is right that those remain unseen.

---

## Suggested order of work

1. **Finding 1** — the crash. Two lines and a test, in `sefer-crates`, and it
   fixes Ksav at the same time. Nothing else here can take the process down.
2. **Finding 3a/3b/3c** — atomic writes. The pattern is already in the tree at
   `session.rs:614`; this is copying it three times.
3. **Finding 2** — the refresh zip. Silent wrong quotes are the failure this
   whole design exists to prevent, and the fix is a shape change to one function.
4. **Finding 6** — `00:00`. One comparison, one refusal.
5. **Finding 8a** — the `forget` branch. One line.
6. **Finding 5** — the embedding guard, in Rust rather than in the DOM.
7. **Finding 7** — the `catch` and the three-way selection result.
8. **Finding 4** — the nine commands, in the order they hurt: `who_cites`,
   `links`, `send_to_ksav`, `/refresh`, `marks_in`, then the rest.
9. **Findings 9, 10, 11, 12, 13** — each is small and independent.

Every change must leave `node tools/verify.mjs` green, run from the repository
root. That is nine steps and it is the only definition of done; the four root
`cargo` steps skip `app/src-tauri`, which owns all the interop and is where most
of this document points. And see the note above about the stale binary: verify
against something you watched link.
