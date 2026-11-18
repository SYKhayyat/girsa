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
- [ ] #68 the command layer is one 6,914-line file with `api.ts` as its twin: split
  into `app/src-tauri/src/commands/` domain modules, **one domain per commit**,
  pure movement, `node tools/commands.mjs` green after every one (#28 pins the
  surface, so the split can be verified rather than trusted). (Medium)
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
- [ ] #74 nav asks the wrong questions twice: `סימן א` is "nothing matched" because
  `bare_place` only reads ASCII, and one ambiguous member turns a whole list into
  "nothing matched" — a refusal that is untrue. (Low)
- [ ] #31 companions silently short, #30 doc-gate coverage, #29 shelf-step prose rot, #27 header (closed — landed with #70; the split it also asked for is #68), #26 mcp leaf trim, #34 spec numbers guard, #32 audit volumes distill, #35 false §12 cite, #36/#37/#38 interop.
- [ ] Features: #63 smart navigation (**the reading half has landed** —
  `girsa_app::nav`, 8 tests: splitting, the sheet flag, the bare-place question,
  and the guard that stops a comma-separated pair from becoming one fabricated
  address; what is left is the input element and the source-sheet assembly) #44 Gedolah desk, #45 live quoted refs, #46 commentators register, #47 parallels desk, #48 concordance, #49 mitzvot registry, #50 lanes, #51 whole-word+scope, #52 biographies, #22 Otzaria grab-bag, #18 plugins, #20 magiah, #21 hash-verified updates.
- [ ] Perf: #15 pane walks, #14 batched linkWords, #12 degraded-ocr dup, #11
  (closed 2026-09-03 — verify).

## Routing rule for new issues
Any AI opening an issue here MUST insert it into the phase above it belongs in. Data-model/boundary items (#43/#28-class) go in Phase 1 even if filed later; never append features above integrity fixes. See AI_ISSUE_ROUTING.md.
