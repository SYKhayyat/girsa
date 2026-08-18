# Girsa performance audit — 18 August 2026

**Scope.** Two questions, asked of the whole tree: *where does Girsa do more
work than it needs to*, and *where does Girsa block when it should not*.

**Method, and its limits.** This audit is a **source read**. Nothing was
compiled, run, profiled or benchmarked for it. Every timing quoted below is
Girsa's own published measurement, cited to the document it came from. Where a
claim is a reading of the code rather than a measurement, it says so. An agent
acting on this document should **measure before and after** each change rather
than trusting the estimate.

**Baseline.** `main` at `c60e1ef`, `node tools/verify.mjs` green 9 of 9, working
tree clean.

**The gate.** Every change below must leave `node tools/verify.mjs` green, run
from the repository root. That is nine steps and it is the only definition of
done. Do not run the four root `cargo` steps by hand and call it verified — they
skip `app/src-tauri`, which owns all the interop and is where most of this
document points.

---

## Contents

| # | Finding | Kind | Size |
|---|---|---|---|
| 1 | One global `Mutex` serialises all 138 commands | blocking | **large** |
| 2 | Three commands hold that lock across unbounded I/O | blocking | **large** |
| 3 | The picker searches on every keystroke with no debounce | blocking + waste | medium |
| 4 | `draw()` opens panes one at a time | blocking | medium |
| 5 | Four startup listeners registered nose to tail | blocking | small |
| 6 | `goToNextPlace` makes one IPC call per highlight | waste | medium |
| 7 | Citation highlighting clones a `String` per character | waste | small |
| 8 | 21 tests sleep 1.1 s each for a filesystem timestamp | waste | medium |
| 9 | The gate runs nine steps serially; three share nothing with cargo | waste | medium |
| 10 | `topLine()` materialises every line on every scroll event | waste | small |

Findings 1–5 are one story told five ways. **Finding 1 is the root cause and
should be done first**, because 3, 4 and 5 buy little or nothing while it
stands.

---

## 1 · One global `Mutex` serialises all 138 commands

**Where.** `app/src-tauri/src/lib.rs:468`

```rust
pub(crate) type Shared = Mutex<State>;
```

134 lock sites in that file. 98 take `let mut state`, 32 take `let state`.

**What the code already says.** The module header at `lib.rs:21`–`:44` documents
a real fix that was already made — every command became
`#[tauri::command(async)]` so that the IPC handler no longer runs inline on the
thread owning the webview and the message loop. That work is correct and is not
in question. The header then closes with:

> *"What this does **not** change is that the state is one `Mutex`, so two
> commands still take their turn. It changes which thread waits."*

**Why that undersells it.** It changes which thread waits *for the search*. It
does not stop every other command waiting for *the lock*. The two commands that
matter:

* `find` — `lib.rs:732` — takes `let mut state` at the top and holds it across
  `bar.ask(&query, &chips, paging, &Context::default())`. Girsa's own figure for
  four real queries is 8, 63, 73 and 90 ms
  (`docs/the-second-sitting.md:796` table). A regex query over ~5 M segments is
  unbounded.
* `sefer_lines` — `lib.rs:1812` — is the scroll path. `pane.ts:353` calls it from
  `load()`, which `extend()` calls from `scrolled()` on every scroll event.

So while a search runs, scrolling the daf beside it cannot be served. This is not
a rare interleaving: the docked search column is an advertised feature — *"the
search panel docks to a column instead of closing, so when the first result turns
out to be the wrong one, the other ten are still on screen"* (`README.md`). The
panel designed to stay open while you read is the one that stalls the reading.

**Why it is fixable.** Nothing about searching needs exclusive access:

* `Bar::ask` is `&self` — `crates/girsa-search/src/bar.rs:272`.
* `Bar` exposes exactly one `&mut` method in the whole file,
  `catalogue_mut` at `bar.rs:216`, which `find` does not call.

The `Bar` is inside the `Mutex` because *everything* is inside the `Mutex`, not
because it needs to be.

**Proposed change.**

1. Move the read-mostly members out of `State` and behind their own handles:
   * `bar: Option<Arc<Bar>>` — immutable after `open_bar_for`.
   * `shelf: Option<Arc<RwLock<Shelf>>>` — readers dominate; the mutators are
     `add_mine`, `shelf_put_work`, `shelf_put_shelf`, `shelf_rename`,
     `shelf_pin`, `shelf_make`, `shelf_reset`, and `choose_corpus`, all of which
     are reader-initiated one-off acts, not per-frame traffic.
2. Rewrite `find` (and `find_rung`, `find_narrow`, `search`) as: lock → clone the
   cheap session scalars and the chips → clone the two `Arc`s → **drop the
   guard** → `ask` → build the rows.
3. `sefer_lines`, `state`, `recent` then contend only with genuine writers.

**Complication to plan for.** `state.names()` (`lib.rs:396`) returns
`girsa_app::Names<'_>`, which borrows the shelf, and `hit_row` needs it while
building result rows. Under an `RwLock` the read guard must outlive the row
build; that is fine, but it means `find` holds a *read* guard, not the write
guard, and readers no longer exclude each other. Confirm `Shelf: Sync` before
starting.

**How to verify.** Add a bench or a manual check that issues a slow `find` on one
thread and times `sefer_lines` on another. Today the second waits for the first;
after the change it should not.

---

## 2 · Three commands hold the global lock across unbounded I/O

**The pattern is already in this file and is already right in three places.**

* `scan_ocr_page` (`lib.rs`, search for `fn scan_ocr_page`) — locks, copies the
  personal path out, **drops the guard**, runs Tesseract, re-locks to record.
* `lane_bring` (`lib.rs:4520`) and the embed job (`lib.rs:4668`) — lock, copy
  what the job needs, drop, `std::thread::spawn`, emit progress events to the
  window. The window's side is `whenLaneWorks` in `api.ts`, and `spec.md` §9.9
  is explicit that embedding must never block reading.

**Three places where it is missing:**

### 2a · `add_mine` — `lib.rs:1510`

```rust
let mut state = shared.lock()...;
let shelf = state.shelf.as_mut().ok_or(trouble)?;
for path in paths {
    match shelf.add_mine(&file, None) { ... }   // parse, copy, write catalogue
}
for slug in fresh { state.searchable(&slug); }
```

The entire import loop runs under the lock, for however many files were dropped.
No yield, no progress event, no bound on `paths.len()`. This is the
drag-and-drop path, so by definition the reader is at the window when it runs.

**Fix.** Model it on `lane_bring`: copy the shelf handle out, spawn, emit a
progress event per file, re-lock per file (or in batches) to record. The
frontend already knows how to consume progress events.

### 2b · `export_sefer` — `lib.rs:2827`

`girsa_export::export(sefer, fixes, format, pointing, shemos, &to)` runs under
the lock. It writes a whole sefer — Mishnah Berurah is 17,418 segments — to
`session.export_into`, a folder the reader chose, which may be a network share
or removable media. Arbitrary-duration I/O under a global lock.

**Fix.** The export needs `sefer`, `fixes` and four scalars. Gather them under
the lock, drop the guard, then write. If `Open` cannot be cloned cheaply, this
is the case for spawning as in 2a.

**Also here (minor).** The function calls `state.sefer(&slug)?` and then
`state.open.peek(&slug)` for the same sefer — the first to ensure it is loaded,
the second to borrow it. Two lookups where one loaded handle would do.

### 2c · `buffer_save` — `lib.rs:3255`

```rust
let state = shared.lock()...;
let shelf = state.shelf.as_ref()...;
let mut buffer = girsa_desk::Buffer::new(name);
buffer.text = text;
Ok(buffer.save(shelf.personal())...)   // synchronous file write, still locked
```

The lock is taken **only to read a `PathBuf`**, then held across the write. This
runs every 900 ms while the reader types — `app/src/writing.ts:22`,
`SAVE_AFTER_MS = 900`.

**Fix.** Three lines:

```rust
let personal = { shared.lock()...?.shelf.as_ref().ok_or(...)?.personal().to_path_buf() };
// guard dropped here
buffer.save(&personal)
```

This is the cheapest item in the document and should probably go in first as a
warm-up.

---

## 3 · The picker searches on every keystroke with no debounce

**Where.** `app/src/picker.ts:323`, and its own comment states the problem:

> *"Behind `Latest`: one `api.search` goes out per keystroke and they do not come
> back in order, so a slow answer to `ברכ` could land after — and on top of — the
> answer to `ברכות`."*

`Latest` (`app/src/latest.ts:46`) issues tickets and drops stale answers. That
fixes the **ordering**, which was a real correctness bug. It does nothing about
the **cost**: every one of those calls still runs to completion in Rust, each one
taking the global `Mutex` from finding 1, and every answer but the last is thrown
away. Typing `ברכות` is five fuzzy searches over 7,189 works, four of them
discarded.

`search` at `lib.rs:578` holds the lock across
`shelf.search(&query, girsa_app::enough::NAMES_OFFERED)`.

**The fix already exists in this codebase.** `app/src/findhere.ts:32` declares
`const SETTLED = 160` and `findhere.ts:214` uses a `setTimeout` to wait for the
typing to settle before asking. Apply the same to the picker. Keep `Latest` — a
debounce and a stale-answer guard solve two different problems and you want
both.

**Note on the numbers.** `docs/the-second-sitting.md:1194` reports the picker
offering *"eight sensible works in under a second"*. Under a second **per
keystroke**, five keystrokes deep, is the shape of the problem.

---

## 4 · `draw()` opens panes one at a time

**Where.** `app/src/main.ts:537`, the `for (const pane of open.panes)` loop:

```ts
text = await api.openSefer(pane.slug);      // ~11 ms per the published table
...
view.setMarks(await api.marksIn(pane.slug)); // ~0.07 s for Berakhot per the
                                             // comment at main.ts:590
await drawMefarshim(view);
```

Three awaits per pane, serial, and the loop itself is serial across panes. The
headline picture in `README.md` is *three columns side by side with a Gemara and
two mefarshim*. Each pane's chain is independent of the others'.

`repaintMarks` (`main.ts:606`) has the same shape — one `marksIn` per view, in
sequence.

**Important caveat, and the reason this is ranked below finding 1.**
Parallelising this buys **nothing** while the global `Mutex` stands, because the
three chains would immediately queue on the lock. Findings 1 and 4 are one
problem seen from the two ends of the IPC boundary. Do 1 first. Then this becomes
a real win; before that it is motion without movement.

---

## 5 · Four startup listeners registered nose to tail

**Where.** the tail of `main()` in `app/src/main.ts`:

```ts
await whenFilesDropped(whenDropped);
await whenAskedToOpen(whenAskedFor);
await whenAskedToSearch((phrase) => void find.showPhrase(openFound, phrase));
await watchForKsav();
await reload();
```

Each of the first four is a separate dynamic `import()` **plus** an IPC round
trip — see `api.ts:1797` (`whenAskedToOpen`), `api.ts:1805` (`whenAskedToSearch`),
`api.ts:1834` (`whenFilesDropped`), each of which does
`await import("@tauri-apps/api/event")` then `await listen(...)`. None depends on
any other. All four sit on the cold-start path ahead of `reload()`, which is what
puts anything on screen.

**Fix.** `await Promise.all([...])` over the four. One line.

**Do not over-claim the win.** `docs/the-second-sitting.md:796` measures *window
on screen from a cold start* at 0.2–1.0 s, and `§18` calls the cold start *"the
best screen in the application"*. This is a few tens of milliseconds off an
already-good number, not a rescue.

---

## 6 · `goToNextPlace` makes one IPC call per highlight

**Where.** `app/src/pane.ts:764`

```ts
const places: { at: number; id: string }[] = [];
for (const mark of this.marks) {
  const known = this.byId.get(mark.at) ?? (await api.seferIndexOf(this.slug, mark.at));
  if (known !== null && known !== undefined) places.push({ at: known, id: mark.at });
}
```

Sequential, uncached, and repeated in full on **every** press of the key. Each
call is `sefer_index_of` at `lib.rs`, which takes `let mut state` — so each one
also acquires the global lock from finding 1.

The marks that miss `byId` are exactly the ones outside the window of lines the
pane is currently holding. On a long sefer that is **most of them**. A reader
with 200 highlights in Mishnah Berurah pays up to 200 serialised IPC round trips
to answer "which is the next one".

**The fix has precedent one file over.** `app/src/linksview.ts:372` already
batches the identical shape:

```ts
said = await api.linkWords(group.work, group.links.map((link) => link.at));
```

Add `sefer_indices_of(slug, ats: Vec<String>) -> Vec<Option<usize>>` beside
`sefer_index_of` and call it once. Optionally memoise the resolved indices on the
pane and invalidate when `setMarks` runs.

**Two more callers of the same command** worth checking while you are there:
`pane.ts:858` (`goToId`) and `pane.ts:1080` (`goToWords`). Both are single
lookups on a user action, so they are fine as they stand — listed only so nobody
"fixes" them by mistake.

---

## 7 · Citation highlighting clones a `String` per character

**Where.** `crates/girsa-app/src/display.rs:410`

```rust
fn cite_chars(cites: &[crate::Linked]) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    for cite in cites {
        for n in cite.from..cite.to {
            out.insert(n, cite.reference.clone());   // one clone per character
        }
    }
```

and then again at `display.rs:379` in `runs_citing`:

```rust
let cite = (at..at + len.max(1)).find_map(|n| cited.get(&n)).cloned();
```

A reference such as `girsa:bavli/shabbat/12b:3#242` is ~30 bytes. A citation
spanning 20 characters allocates it 20 times in the map, then once more per
`Bit::Letter` in the run builder, and the run-merge test at `display.rs:381`
compares those `String`s for equality per character.

So the cost is **O(characters)** allocations and comparisons where the data is
**O(citations)**.

**Contrast with the sibling function, which is already right.** `hit_chars` at
`display.rs:421` carries the comment *"One walk of the text, so a segment with
forty marks costs one pass"*, and returns a `BTreeSet<usize>` — no per-character
allocation. The search-highlighting path was thought about; the citation path was
not.

**Fix.** Store `BTreeMap<usize, u32>` mapping character to an index into `cites`,
or keep a sorted `Vec<(Range<usize>, &str)>` and binary-search it. Make `Run.cite`
hold an `Rc<str>`/`Arc<str>` or an index so the merge test is a pointer or
integer comparison rather than a string comparison.

**Honest sizing.** This is small in absolute terms and it is on the drawing path
for the writing drawer (`linkify` output), not on the daf. Worth doing when
`display.rs` is open for another reason; not worth a dedicated sitting.

---

## 8 · 21 tests sleep 1.1 seconds each for a filesystem timestamp

**Where.**

| File | Count |
|---|---|
| `crates/girsa-note/src/since.rs` | 15 |
| `crates/girsa-app/tests/never_a_silent_gap.rs` | 3 |
| `crates/girsa-note/src/note.rs` | 1 |
| `crates/girsa-desk/src/documents.rs` | 1 |
| `crates/girsa-corpus/src/fetch.rs` | 1 (a retry backoff, **not** a test — leave it alone) |

All twenty of the test sleeps are `std::thread::sleep(Duration::from_millis(1100))`.

**Why 1100.** `crates/girsa-note/src/since.rs:104` documents the comparison as
*"Seconds of the source file's mtime, since the epoch"*, and `since.rs:143` and
`:343` read `metadata().modified()`. The tests sleep past a one-second boundary
so that "the note is newer than the index" is distinguishable. See
`since.rs:571` and `:588` for the two canonical shapes.

**Why this is fixable without touching the production code.** The tests do not
need real time to pass; they need two files with different mtimes.
`std::fs::File::set_modified` has been stable since Rust 1.75 and this workspace
declares `rust-version = "1.85"` (`Cargo.toml:66`), so **no new dependency is
needed**.

**Fix.** One helper in the test module:

```rust
/// Stamp a file as though it were written `secs` seconds ago.
fn stamp(path: &std::path::Path, secs: u64) {
    let at = std::time::SystemTime::now() - std::time::Duration::from_secs(secs);
    std::fs::OpenOptions::new().write(true).open(path).unwrap().set_modified(at).unwrap();
}
```

Then replace each `sleep(1100)` with an explicit stamp on whichever of the note
or the index stamp should be the older one. This makes the tests *say what they
mean* — "the index was built before the note was written" — instead of implying
it through elapsed wall-clock, which is also why they are currently fragile on a
loaded machine.

**Sizing, honestly.** 20 × 1.1 s = 22 s of sleeping, but `cargo test` runs a
crate's tests on a thread pool, so the wall-clock cost is a few seconds, not
twenty-two. The stronger argument is not speed, it is that a test which sleeps
for a second is a test that can flake, and 15 of the 20 are in one file.

---

## 9 · The gate runs nine steps serially; three share nothing with cargo

**Where.** `tools/verify.mjs:59`–`:81` defines `GATE`; `tools/verify.mjs:93`
iterates it with a plain `for` and a blocking `spawnSync`.

The ordering rationale in the comment at `verify.mjs:45` is sound and should be
kept for the cargo steps: compilation first, cheap lint before slow browser, the
shell's two after the workspace's four. Steps 1–6 all invoke `cargo` against one
workspace, one lockfile and one `target/` — they share the cargo lock and
**must** stay sequential.

**Steps 7, 8 and 9 share nothing with them:**

| Step | Command | Toolchain |
|---|---|---|
| 7 | `npx tsc --noEmit` in `app` | TypeScript only |
| 8 | `node test/run.mjs` in `app` | esbuild (`test/run.mjs:9`, `:56`) |
| 9 | `node tools/eyes.mjs` in `app` | esbuild + headless browser |

None reads `target/`, none takes the cargo lock, none needs `app/dist`.

**Fix.** Run the gate as two lanes — the cargo lane (1–6, in the existing order)
and the window lane (7–9, in the existing order) — and join at the end. Report
both lanes' failures rather than short-circuiting on the first, since they are
now independent. `spawnSync` will have to become `spawn` with promises.

**Keep.** `--from <n>` must keep working. Note the existing trap recorded in the
project memory: `--from N` skips the README measurement that `cargo fmt` later
invalidates, so a resume is not a run. Whatever the two-lane version looks like,
do not make that worse.

**Sizing.** On a warm cache the cargo lane dominates, so the window lane becomes
free. On a cold cache the win is smaller because the cargo lane is much longer
than the window lane either way.

---

## 10 · `topLine()` materialises every line on every scroll event

**Where.** `app/src/pane.ts:735`

```ts
const lines = [...this.body.querySelectorAll<HTMLElement>(":scope > .line")];
```

`topLine()` is called from `scrolled()` (`pane.ts:715`) on every scroll event.
The pane holds up to ~400 lines, so this is a 400-element `querySelectorAll` plus
a spread into a fresh array, per scroll event, purely to have something
indexable for the binary search below it.

**Credit where due.** The binary search itself (`pane.ts:739`–`:746`) is already
an optimisation, and the doc comment says so: *"This read up to 400
`getBoundingClientRect()`s per scroll event to find the one that flips; it reads
about nine."* That work is good. The array materialisation is the residue.

**Fix.** Index `this.body.children` directly — it is a live `HTMLCollection` with
`.length` and `.item(i)`, so the binary search can run over it without copying.
The `:scope > .line` filter matters because `.line-said` comment blocks sit
between lines (see `dropAbove` at `pane.ts:680`), so the probe must skip
non-`.line` siblings rather than assuming a dense array.

**Sizing.** Small. `docs/the-second-sitting.md:798` reports *"~17 ms frames with
two spikes in twenty screens"*, so this is not the bottleneck. Listed for
completeness, and because it is a five-line change.

---

## What was checked and found sound

An agent picking this up should **not** spend time re-deriving these. Each was
examined for this audit and is correct as it stands:

* **Tauri command execution context.** All 138 commands are
  `#[tauri::command(async)]` except `copy` (`lib.rs:3197`) and `sefer_sheet`
  (`lib.rs:3938`), and the header comment at `lib.rs:21` gives the correct
  reasoning for both the rule and the two exceptions. This was a real and
  severe bug and it is fixed. Do not "fix" it again.
* **Regex compilation.** No `Regex::new` anywhere outside tests. Nothing is
  recompiling a pattern in a loop.
* **Session persistence.** `State::save_scroll` (`lib.rs:421`) throttles writes
  from the scroll handler against `SAVE_SCROLL_EVERY`, while decisions call
  `State::save` directly. Correct as designed.
* **Sequential `await`s in the frontend, generally.** A scan for two independent
  `await api.*` calls in a row across all 35 modules found only the pane loop of
  finding 4. Everything else is either dependent or already batched.
* **`linkWords` and `titles`** are already batch commands taking arrays. The
  N+1-over-IPC pattern is largely absent; finding 6 is the exception, not the
  rule.
* **Memoisation in the panels.** `mefarshimFor` (`main.ts:745`), `ScopeView.twist`
  (`scopeview.ts:310`) and the `views`/`scans`/`named` maps in `main.ts` all
  cache correctly and invalidate on the right events.
* **Search-hit highlighting.** `hit_chars` (`display.rs:421`) is a single pass
  returning a `BTreeSet`. Contrast finding 7, which is the citation path only.
* **Layout thrash.** A scan for layout reads inside loops found two sites, both
  benign: `scrolled()` batches its three reads before any write, and the binary
  search in `topLine()` does not mutate between probes so the layout is cached.
* **Long-running background jobs.** `lane_bring` (`lib.rs:4520`) and the embed
  job (`lib.rs:4668`) already do the right thing — copy state, drop the guard,
  spawn a thread, emit progress. They are the model for finding 2.
* **`scan_ocr_page`** already drops the lock across Tesseract. Also a model for
  finding 2.
* **Cold start.** `open_corpus` runs inside Tauri's `setup` hook
  (`lib.rs:4896`), which is before the window appears — but the measured result
  is 0.2–1.0 s (`docs/the-second-sitting.md:796`) and `§18` of that document
  calls the cold start the best screen in the application. Not a problem. The
  one thing to watch is `girsa_nearby::Adjacency::open` at `lib.rs:1297`, which
  loads a side-loaded model when the semantic lane is switched on; the comment
  there says this is deliberate. If a reader with the lane on reports a slow
  launch, that is the line to look at, and the trade-off was made knowingly.

---

## Suggested order of work

1. **Finding 2c** (`buffer_save`) — three lines, no design decisions, proves the
   pattern.
2. **Finding 1** — the structural change. Everything else scales off it.
3. **Findings 2a and 2b** (`add_mine`, `export_sefer`) — same shape as 2c but
   they need the spawn-and-report treatment, and `add_mine` needs a progress
   event the frontend does not have yet.
4. **Finding 3** (picker debounce) — one file, copies `findhere.ts`.
5. **Finding 6** (batch `sefer_indices_of`) — one new command, one call site.
6. **Findings 4 and 5** — only meaningful after 1 has landed.
7. **Findings 8, 9** — developer-loop quality, independent of everything above,
   can be done at any time by anyone.
8. **Findings 7, 10** — small; do them when the file is open anyway.
