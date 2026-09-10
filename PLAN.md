# PLAN — girsa (work top to bottom, one issue per worker session)

Worker loop: top unchecked item only, fix + resolving test, commit, check off, stop.
Done (closed): #1–#11 (except #9 DUP), #13, #23, #24, #25, #39, #41, #42, #61.

## SKIP
- #9 DUP of #10 (same button() bug, garbled body). Work #10.

## Phase 1 — Foundations first
- [ ] #43 anchor ref+range binding surviving re-segmentation (unblocks #44–#48). (High)
- [ ] #28 Rust↔TS command boundary generator/cross-check (unblocks safe renames). (Medium)
- [ ] #33 resolving test for spec §N / W-needs citations (unblocks #35-class rot). (Low)

## Phase 2 — Durability/Integrity Criticals+Highs
- [ ] #55 poisoned fetch queue reports success. (High — silent skip)
- [ ] #56 one bad line disables whole landing index. (High)
- [ ] #54 corrupt session.json resets then overwrites. (High)
- [ ] #53 MCP read re-parses whole sefer per call. (High)
- [ ] #24 byte-exact re-import matcher — VERIFY fix 09-03 held, else redo. (High)

## Phase 3 — Search quality Highs/Mediums
- [ ] #17 morphology+fuzzy Smart rung, #16 re-ingest Otzaria (stale shelf), #19 embedding hybrid lane.
- [ ] #2 search recall/pagination/identity bundle (split per sub-item), #3 stale races/blocking, #7 validation consistency.
- [ ] #59 merge drops unserializable, #58 negative-cache poison, #57 anchor substring-match.
- [ ] #62 unbounded regex bound, #60 SegmentStore O(n), #61 (closed — verify), #53 (above).

## Phase 4 — Docs/process + features (Low/Info)
- [ ] #31 companions silently short, #30 doc-gate coverage, #29 shelf-step prose rot, #27 header divergence, #26 mcp leaf trim, #34 spec numbers guard, #32 audit volumes distill, #35 false §12 cite, #36/#37/#38 interop.
- [ ] Features: #44 Gedolah desk, #45 live quoted refs, #46 commentators register, #47 parallels desk, #48 concordance, #49 mitzvot registry, #50 lanes, #51 whole-word+scope, #52 biographies, #22 Otzaria grab-bag, #18 plugins, #20 magiah, #21 hash-verified updates.
- [ ] Perf: #53 (above), #15 pane walks, #14 batched linkWords, #12 degraded-ocr dup, #11 (closed — verify).

## Routing rule for new issues
Any AI opening an issue here MUST insert it into the phase above it belongs in. Data-model/boundary items (#43/#28-class) go in Phase 1 even if filed later; never append features above integrity fixes. See AI_ISSUE_ROUTING.md.
