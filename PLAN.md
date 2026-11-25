# PLAN — girsa (work top to bottom, one issue per worker session)

Worker loop: top unchecked item only, fix + resolving test, commit, check off, stop.
Done (closed): #1–#11 (except #9 DUP), #13, #23, #24, #25, #28, #39, #41, #42, #61, #43, #55, #56
(the Girsa half; the Source-Packet half is #64 and needs a sefer-crates semver break).

## SKIP
- #9 DUP of #10 (same button() bug, garbled body). Work #10.

## Phase 1 — Foundations first
- [x] #43 anchor ref+range binding surviving re-segmentation (unblocks #44–#48). (High)
  Landed 2026-10-05: `girsa_corpus::span::Anchored { at, span, was }` is one rule
  for "which segment, which words", and every stored range in the tree goes
  through it. Three silent misplacements fixed — the mined anchor's offset read
  against the text you have corrected, a pin with no words to be found again by,
  and a pin's offsets placed on a piece of a cut parent that does not hold them.
  `span_on` asks `Standing` and takes the printed text as well as the drawn one.
- [x] #28 Rust↔TS command boundary generator/cross-check (unblocks safe renames). (Medium)
  Landed 2026-11-08: `tools/commands.mjs` reads the four lists of the same surface
  — the `#[tauri::command]` declarations in `lib.rs`, the `generate_handler![…]`
  registration, the `call<>(…)` sites in `app/src/api.ts`, and the browser shim's
  87 `case "…"` labels — and fails on any disagreement, in either direction, plus
  a duplicate in any of them. They agree today (138 / 138 / 138); nothing checked
  that they did. `--self-test` breaks each list in memory and demands the check
  notice (4/4), so the gate step cannot pass vacuously. `--where <name>` prints
  every site a rename has to touch — 9 for `recent`, including the dev-fixture
  `case` that a shell-only rename would leave behind and that no compiler sees.
  In the gate (`tools/verify.mjs`, two steps) and in the rust job's CI steps.
  Three prose counts had already drifted past the file they described — the
  module doc said "the twelve commands", `ci.yml` said "5,018 lines and all 100
  tauri commands", against 138 in a 6,914-line file — so the numbers are out of
  the prose and the tool prints them.
- [x] #33 resolving test for spec §N / W-needs citations (unblocks #35-class rot). (Low)
  Landed 2026-11-12 in `265cc97`, with #35: `a_citation_names_something_that_is_there.rs`
  reads every comment in the tree for `spec.md §N`, `BUILDER.md §N`, `W<n>` and
  `B<n>` and demands each number exist in the file it names. It found five
  things on the way — `W29`, `W44b`, a `§19` that was in no document, ~90 `B<n>`
  citations whose source file had been deleted, and one `§1.2` of nothing.
- [ ] #64 the same disease one layer up: a Source Packet carries `Range` as bare
  character offsets and no words, so `/quote` regenerates **different** words
  under a correct-looking citation after a correction. STOP AND ASK (BUILDER.md
  §0.1) — the words have to ride on the packet, and the packet is `girsa-source`'s.
  Filed 2026-10-05 with the field shape and both acceptance halves; needs an
  owner decision before any code.

## Phase 2 — Durability/Integrity Criticals+Highs
- [x] #55 poisoned fetch queue reports success. (High → **Low**, and the reason
  is in the commit: `run` cannot poison its own queue, so the defect was the
  *type*. `next_target` returned `Option`, which cannot say "broken", so the
  invariant lived in a comment — and the comment named the exact failure the
  line below it caused. Now `Result<Option<Target>, usize>`, propagated as
  `FetchError::QueuePoisoned { skipped }`. The resolving test needs no network
  and no corpus, which needed a seam: the worker loop is `drive()` now and `run`
  builds the queue.)
- [x] #56 one bad line disables whole landing index. (High → **Low**, and the
  issue's fix is rejected: skipping a row drops the byte range every edge landing
  on one place lives in, so the index answers with *fewer* links than the gate
  and breaks the one property licensing a second read path —
  `the_index_and_the_gate_answer_alike_for_every_place`. `mutation.mjs` breaks
  the guard *into the shape the issue proposed* so the thing asked for is the
  thing proved wrong. The defect was four facts behind one `Option`; they are
  now four, and the damaged case reaches the reader as *slow, not short*.)
- [x] #54 corrupt session.json resets then overwrites. (High)
  Landed 2026-11-12 in `9d11125`: a file that will not read is kept, not
  replaced, and the window says where it went.
- [x] #53 MCP read re-parses whole sefer per call. (High)
  Landed 2026-11-18 in `57cd36f`: `Server::sefer` holds the parsed sefer in
  `girsa_app::held::Held`, the cache the window already had, and `fix`/`unfix`
  forget it in the turn they write. `a_sefer_is_parsed_once.rs` asserts the
  sharing with `Arc::ptr_eq` and the invalidation by reading after a correction;
  both were watched failing.
- [x] #24 byte-exact re-import matcher — **verified held**: `296e4ad` compares
  through `normalize`, so a nikud edit costs no name.

## Phase 3 — Search quality Highs/Mediums
- [ ] #17 morphology+fuzzy Smart rung, #16 re-ingest Otzaria (stale shelf), #19 embedding hybrid lane.
- [x] #2 search recall/pagination/identity bundle, #3 stale races/blocking, #7
  validation consistency — all three closed 2026-08-27; the audit's successors
  live in this plan (`#59`–`#62`).
- [x] #59 merge drops unserializable, #58 negative-cache poison, #57 anchor
  substring-match — all three landed: `ffe186a` (a record that will not write is
  refused rather than written as an empty line, so a merge can no longer count
  two unreadable records as one it already had), `d523bf1` (three caches stopped
  remembering absence — the shelf and the citation bar both held *I could not
  find it* as *there is none*, and the shelf grows mid-session), `10e449e`
  (`data-order` no longer matches inside `xdata-order`).
- [x] #60 SegmentStore O(n) — `526a1e3`. The nth segment was `live.keys().nth(n)`,
  so a sweep of a sefer was quadratic: twice the sefer cost 4.1× the sweep. The
  order is kept beside the map now, and `resolve_into` carries a visited set so a
  redirect diamond is walked once per id rather than once per path.
- [x] #62 unbounded regex bound — `378c781`. `Query::parse` checked what a pattern
  said and never how much of it there was; `MOST_PATTERNS` (16) and
  `MOST_PATTERN_CHARS` (200) are refused rather than trimmed, and the automaton
  size limit that is tantivy's is named in the module docs so nobody hunts for it.
- [ ] #61 (closed 2026-09-09 — verify), #53 (landed, above).
- [x] #77 correcting a typo on a corrected sefer cost 5x what it costs on an
  empty one, and half of that was work done twice. Landed 2026-11-18 in
  `1b59fbd`: `replay` asked every line whether it was a tombstone, parsing it
  whole to find out (a third of the layer open), and `Layer::apply` built a
  `Vec` per segment for 18,120 segments. And the guardrail itself: at sixteen
  thousand corrections the empty layer is not the other end of a ratio, so the
  top measurement is a doubling (8k against 16k, allowance 3, measured 1.44–1.56).
- [x] #73 the ≥95% resolution promise rested on a number nobody ran. Measured: **99.98%
  exact, 0.01% ambiguous, 0 unresolved** over 20,000 real Sefaria citations.
  `tools/resolver.mjs` holds it as a gate step (`--record` to move it) and says out loud
  when `corpus/` is absent; the corpus-free half that keeps a sefer's name out of an
  address is `girsa_app::nav`'s `is_a_place`. (Medium)

## Phase 4 — Docs/process + features (Low/Info)
- [x] #68 the command layer was one 6,914-line file with `api.ts` as its twin.
  **Landed** as twenty-two domain modules under `app/src-tauri/src/commands/` plus
  `state.rs`, one domain per commit, pure movement, `node tools/commands.mjs` green
  at 140/140/140 after every one (#28 pins the surface, so the split was verified
  rather than trusted). `lib.rs` is 591 lines: the handler list, the plugins, `run`,
  and the two commands that open a corpus. The measurement moved with it —
  `shell-lines` walks the tree now (#72's shape, one split later). (Medium)
- [x] #72 both programs that recount the README counted `#[tauri::command]` in
  `lib.rs` alone, so #68's first move put the number one behind — and
  `tools/readme-numbers.sh` wrote the wrong one back. **Landed**: both walk the
  whole `app/src-tauri/src/` tree, and the rule is tested against a temporary
  tree (`a_command_declared_outside_lib_rs_is_counted`) rather than against this
  repository's current arrangement.
- [x] #69 the lexicon holds Sefaria's spellings, not the reader's: `brachos`,
  `rambam hilchos teshuva`, `mishnayos peah`, `shulchan aruch orech chaim` all
  come back unresolved against the shipped `corpus/lexicon.tsv` (measured, #63).
  **Landed** in `girsa_corpus::lexicon::reader_spellings`, applied by
  `build-lexicon` rather than to the shipped file: substitutions to a fixed
  point plus the Rambam prefix, with a negative test that `Bava Basra` makes
  nothing.
- [x] #74 nav asks the wrong questions twice — `7b7ff4d`. `bare_place` reads
  Hebrew numerals through `girsa_ref`'s reader (strict enough that a word that
  merely sums to a number, like `ייחוד`, does not become a "which sefer?"), and
  `Nav::Several` carries `unread` so one ambiguous member no longer sinks the
  whole list: the refs that resolved are answered, the rest is named, three ways. (Low)
- [x] #35 the false `§12` citations — `5763a92`. Item 13's header was fixed by
  hand in November and held by a test; walking the same sentence found the
  address cited **twice more** in the same crate (*may a program write into your
  own layer*, and the test that asserts it), because §12 is Architecture and
  your own layer is §11. The rule that can be mechanised now is: a
  parenthesised citation on a line naming a subject a reader has been misled
  about must resolve to a section that says it. The half that cannot be checked
  by content (§12's storage sentence mentions *personal layer*, and mentioning a
  thing is not what may be written to it) is held by name, and the test says why
  rather than passing quietly.
- [x] #34 spec.md's own tables are guarded like the README's — `dc01a2c`. §2.3's
  union and Otzaria-only cells carry markers and `girsa-import` reads them out
  of the document instead of remembering `7576`/`978` beside it; §9.3's mode
  count is `girsa_search::Mode`'s variants, recounted. `spec.md` is in `PAGES`,
  so any marker in it must have a measurer — this tree's, a program the gate
  runs, or `MEASURED_ON_A_SHELF`, which holds the only thing a checkout can
  prove about a corpus number: that the program measuring it reads the number
  out of the document.
- [x] #15 an edge paints what it added — `6600d50`. `grown` bounded the page and
  nothing bounded the work of adding to it: every `extend()` repainted the whole
  window (1000 rows) and asked `querySelector` about every mark in the sefer.
  `added()` and `byLine()` are functions now, `paint(rows)` is the scoped half,
  and 240 edges cost 72,400 row visits instead of 243,900.
- [x] #78/#79 the mutation tool's debris, and what it put into HEAD — `a ledger,
  a sweep and a gate that refuses`. `tools/mutation.mjs` restores in a `finally`,
  which covers a throw and not a signal, and a killed run left two guards broken
  in the working tree; `git add -A` then published them (`if false =>` in the
  read-only MCP door, and the importer's `lexicon.tsv` write). The checker that
  would have caught it was right all along — `check-coverage.mjs` names the
  missing guard — it had simply been run on a tree that changed a minute later.
  So: a ledger of original bytes under `tools/.mutation-in-flight/` (gitignored)
  written before the first break and swept at the next start, `--restore` to do
  only that, handlers for `SIGINT`/`SIGTERM`, a `--self-test` that proves the
  restore in a scratch directory (a gate step, because this code only runs when
  something has already gone wrong), and `node tools/verify.mjs` now refuses to
  gate a tree holding a ledger. Not fixed: a person can still run the gate and
  then mutate the tree; a pre-commit hook is a tooling decision, not this patch.
- [x] #32 the audit volumes are history, and their conclusions are a page —
  `audit/README.md`. Four documents, ~165 KB of narrative that nothing ran and
  nothing cross-checked, replaced by 2–3 conclusions each: the performance
  audit's *two findings were one story* and its lanes; the correctness audit's
  *the seams between two processes and the paths a guard does not reach*;
  the graded audit's *a diff, not a verdict*; and lamdan's §3, which is worth
  keeping because it was right and it is now fixed. Each section says which of
  its conclusions still stands. `AUDIT-lamdan-2026-08-30.md` stays where it is:
  its items are open issues and four tests cite it by number. The doc-link walk
  now enters `audit/` as well, because a directory the walk does not enter is
  unchecked links.
- [x] #26 the leaf, and the three copies of one list — `WRITES`. The audit asked
  for a re-framing and a feature gate. The re-framing is in the crate header: the
  third surface, the only one nothing in this tree calls, the cost of that said
  out loud. The feature gate is *declined*, recorded rather than omitted — it
  would put ten tools and their tests behind a flag nobody sets, which is the
  vacuous gate the tree names elsewhere. What was actually wrong in the crate was
  smaller and worse: **ten write tools written out three times** (the catalogue's
  second half, the door's alternation, the refusal test's table) and described as
  *three* in two headers. `tools::WRITES` is the list now; the door reads it, the
  catalogue is built from it, and a test compares the two catalogues — the
  difference *is* `WRITES`, in both directions, as a comparison rather than a
  fourth copy. Its weight claim did not survive `cargo tree`: the lane and the
  index belong to default members.
- [x] #51 *in this sefer*, and *these words and not their near forms* —
  `mekoros::Asking`. Whole-word as a *rung* turned out to be here already (the
  matching chip's `the word` is `Match::Word`, and it is the default); what was
  missing was the **phrase search** saying it. `where_from` took four positional
  arguments and had no way to hear either question, so a Sources panel was
  offered near matches it had not asked for and could not turn off. `Asking`
  carries `except`, `scope`, `whole_word` and `limit`; `whole_word` stops the
  ladder and `Found.as_typed` makes *these words are in no sefer* a different
  sentence from *too long to widen*; `scope` narrows through the same `Scope` the
  window's chip builds, with `except` applied on top. Reached from three
  surfaces in this repo — `girsa-index --in/--whole-word`, the MCP tool's `in_`/
  `whole_word`, and `POST /where-from`'s `in`/`wholeWord` — with a slug that is
  not on the shelf refused rather than searched. What is left is Ksav's own UI,
  in the other repository.
- [ ] #31 companions silently short, #30 doc-gate coverage, #29 shelf-step prose rot, #27 header (closed — landed with #70; the split it also asked for is #68), #26 mcp leaf trim, #32 audit volumes distill, #36/#37/#38 interop.
- [x] Features: #63 smart navigation — all four surfaces. `girsa_app::nav` reads
  the address; `girsa_app::meant` answers *which sefer is that name* when the
  lexicon has nothing (folded titles and slugs, the worst word deciding,
  `One`/`Several`/`None` and never a guess); `nav::suggest` is the layer that asks
  the second question **only** where the first was silent, and hands back the
  reader's own line with the name corrected; `Ctrl+G` opens `navbox.ts` over the
  pane, which offers those lines as choices and asks again on a click; and
  `girsa_app::source_sheet` assembles a parenthesised list into one docked column
  (`sheetview.ts`), with each reference that could not be read drawn where the
  passage would have been. 19 tests across the four, and the two unbuildable-here
  shell adapters noted on the issue. What is left is Ksav's half: the combined-block
  insertion that mirrors the sheet.
- [ ] Features: #44 Gedolah desk, #45 live quoted refs, #46 commentators register, #47 parallels desk, #48 concordance, #49 mitzvot registry, #50 lanes, #51 whole-word+scope, #52 biographies, #22 Otzaria grab-bag, #18 plugins, #20 magiah, #21 hash-verified updates.
- [ ] Perf: #15 pane walks, #14 batched linkWords, #12 degraded-ocr dup, #11
  (closed 2026-09-03 — verify).

## Routing rule for new issues
Any AI opening an issue here MUST insert it into the phase above it belongs in. Data-model/boundary items (#43/#28-class) go in Phase 1 even if filed later; never append features above integrity fixes. See AI_ISSUE_ROUTING.md.
