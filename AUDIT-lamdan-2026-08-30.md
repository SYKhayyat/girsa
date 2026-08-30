# Girsa — Lamdan audit, 2026-08-30

Scope: fresh design-critique sweep of the whole repository, region by region (the `girsa-link`/`girsa-corpus`/`girsa-search` core, the Tauri shell in `app/`, the `girsa-mcp` leaf, the tooling and prose). Written as **fix-and-refine**: nothing here is a plain "delete" — every item is a consolidation, distillation, gating, or local fix whose intent survives. Prior audits in `AUDIT.md`, `AUDIT-2026-08-23.md`, `AUDIT-CORRECTNESS.md` and `lamdan/` were used for the recurrence check (a large fraction of their findings are genuinely fixed; those are credited at the end, not re-litigated).

Each item below is shaped so it can be lifted directly into one GitHub issue.

---

## 1. The chain walk ("which seforim relate to this one") is pure ordinal-prefix descent and never consults `Standing` — thread it in at reader sites

- **Lens:** 2 · **Verdict:** rewrite → **fix and refine**
- **The problem:** the panel path absorbs re-segmentation correctly — `Anchor::names` + `Standing` are used by the panel (`store.rs:308` `Landing::naming`, `inbound.rs:169` `ranges_for`, `repair.rs:576` `drawn_touching`) — but `chain::trace` walks the transmission graph through `Graph::beside` → `far_end`, which decides "is the other end of this edge near me?" by `overlaps` alone, an ordinal-prefix descent (`chain.rs:695–760`; `far_end` at :712). The file is explicit about the choice: *"a chain hops anchor to anchor and nobody is standing on a segment for a `Standing` to be about"* (`chain.rs:683–684`). So on a re-segmented corpus, a merge or insertion upstream is silently mis-followed by the very feature built to survive it.
- **Refine:** thread a `Standing`-resolving closure into `Graph` for the reader-standing site (`chain::trace`), so the first hop — where the reader actually is, a live segment with a shelf — is resolved by `Standing` exactly as the panel already resolves it. Keep pure `overlaps` prefix-descent only for true anchor-to-anchor overlap (the acyclic "edge endpoints coincide" case), not for the walking graph.
- **Cost:** incremental, no wire/format change; the `a_link_survives_a_resegmentation` and `the_chain_is_rows_a_panel_can_draw` tests become the net. Add one regression: a re-segmentation between the reader's segment and its known relation must not drop the hop.
- **Why keep the *panel* path when both could share:** the panel is where the feature already works; the fix is to teach `chain` the panel's method, not to change the panel.

## 2. The re-import address matcher is byte-exact on unnormalized Hebrew — a nikud edit orphans every citation to `Why::Gone`

- **Lens:** 2 · **Verdict:** rewrite → **fix and refine**
- **The problem:** the ordinal scheme is sound *within* one run (`segment.rs`), but durability across a **second import** lives in `import::continuity`, and every matcher there keys on raw text: `same_opening` takes the first whitespace token unnormalized (`continuity.rs:306–311`), `once_by_text` buckets on exact `&str` equality (`:314`), `went_to` uses raw `contains` both ways (`:525`). Upstream correcting or pointing a passage (adding nikud, fixing a ktiv spelling) changes the token, the anchor is discarded, `went_to` cannot see the containment, and the citation resolves to nothing — the exact event this feature exists to survive. The sibling search crate, by contrast, normalizes **every** token through `girsa_hebrew::normalize` on the way in.
- **Refine:** it is a one-repo inconsistency: the matcher uses the un-normalized path while search uses the normalized path. Compare after `normalize` in `same_opening`/`once_by_text`, and give `went_to` a **normalized-containment fallback** that labels the outcome `Resegmented` rather than `Gone`. One normalize pass riding the existing import walk; no new pass.
- **Cost:** incremental; the `a_reimport_keeps_every_name` and `an_anchor_survives_a_split_at_import` tests extend to cover "nikud added between imports."

## 3. Root-inflected search is refused with "no morphological analyser" — but the cheap, conservative half already ships (never use the refusal to justify not building it)

- **Lens:** 1 · **Verdict:** rewrite → **fix (add the low-rung, call-site-free)**
- **The problem:** the want explicitly promises root-inflected Hebrew, and the ladder's only answer is `Standing::Deferred` — *"there is no rabbinic-Hebrew-and-Aramaic morphological analyser to build it on (spec.md §9.4)"* (`ladder.rs:112–115`). Meanwhile the engine already has the conservative **consonant-skeleton** match — `Match::Letters`, "these letters in this order with others between them" (`torat_emet.rs:47–52`) with its `pattern_for` machinery — gated behind a per-word chip and **excluded from the ladder** entirely. So the recall win that needs no morphology is left off the relaxation path the spec itself ordered.
- **Refine:** add a `Rung::Forms(ConsonantSkeleton)` reusing `Letters`/`pattern_for`, priced through the existing `count_widened_in` offer path so a zero stays a zero ("offered, not applied"). Rungs are data, so this is call-site-free and cannot drift from the chip surface.
- **Cost:** incremental; the `the_ladder_never_widens_into_meaning` and `offered_not_applied` tests pin it. This is the single biggest recall win available without touching the deferred morphology ruling.
- **Note:** leave the morphology ruling (spec §9.4) intact; the consonant-skeleton is *not* morphology, it is a containment match the crate already owns.

## 4. `girsa-mcp` is a heavy leaf with no in-tree caller — keep the read spine, gate the write surface

- **Lens:** 2 · **Verdict:** wrong-but-keep → **fix (gate the growth, keep the read spine)**
- **The problem:** `girsa-mcp` (~2,350 lines: `lib.rs` 314 + `tools.rs` 1,839 + `protocol.rs` 194) is a leaf that nothing in either repo calls; its only dependents are its own tests and the fixture/dev surface. It pulls the whole shelf stack — including `girsa-lane`'s embedded lane model and, through the `index` feature, tantivy via `girsa-search` — into every `cargo test` on the workspace. Its write capability (`write_note`, `correct`, and siblings) sits behind `--writable`, which no in-repo consumer turns on.
- **Refine:** re-frame the crate honestly as the third product surface (read spine first): keep the read-only paths that give a program the *same* engine and refusals a person gets, and either gate the write tools behind a feature CI never enables or keep them only if there is a stakeholder (see Interop/Open Questions). Do not delete the read spine — it is the only in-tree way to prove "a program gets the same engine."
- **Cost:** incremental; zero in-repo callers to migrate.
- **Also:** the dev `girsa-fixture` with `features=["index"]` drags the heavy index into everyone's test build; scope that feature off for the crates that do not exercise a programmatic read, so a leaf's tests do not tax the whole workspace.

## 5. The desktop shell's self-description and its real surface have diverged — "the twelve commands behind it" is a 140-command, 6,861-line adapter monolith

- **Lens:** 2 · **Verdict:** rewrite → **fix (re-declare honestly, then split)**
- **The problem:** `app/src-tauri/src/lib.rs:1` still opens *"The window, and the twelve commands behind it,"* and the module header still says *"Nothing is decided in this crate."* But the file now carries **140** `#[tauri::command(async)]` commands in one 6,861-line file — the crate mutated from "adapter, nothing decided here" into a 140-command product surface while keeping the old self-description. Recurrence of the "self-description is prose that lags the code" pattern.
- **Refine:**
  - Rewrite the module header to say plainly what it is (a third product surface — the Tauri shell — on top of the `girsa_app` engine), and correct the "twelve commands" sentence.
  - Split the 6,861-line monolith by area (shelf, reading/lens, links/chain, search/ladder, notes/personal, settings), each thin command module forwarding to `girsa_app`. Cheapest immediate win; behaviour unchanged.
- **Cost:** the split is a large mechanical move (the commands share module state and helpers), so do it in 2–3 consecutive PRs with the existing `app/test/*` harness as the net. The honest header is a one-line fix that can land first.

## 6. The Rust↔TypeScript command boundary is hand-twinned by bare strings — no generator, no cross-check

- **Lens:** 3 · **Verdict:** wrong-but-keep → **fix (a cheap honest consistency test now; the expensive generator is not worth it yet)**
- **The problem:** every command crosses the boundary as a bare string: the Rust side declares `#[tauri::command(async)] async fn sefer_index_of(...)` and the TS side has `call<...>("sefer_index_of", ...)` (`app/src/api.ts:1320` wrapper, `:1368–1371` for `sefer_indices_of`, ~139 `call<` sites total). Nothing ties a Rust command name to a TS call site. A rename on one side is a runtime "command not found" shipping to the window at the exact moment a brand-new listener sits down.
- **Refine:** ship the small cheap move — a test that walks the TS `call<T>("name")` sites and greps the Rust `#[tauri::command(...)]` names, failing on a bare string with no twin (and the reverse for orphaned commands). Build a generator **only** if the boundary grows again; the discipline today is that the two lists cannot silently diverge.
- **Cost:** one test file; incremental. (Ksav's audit Interop-B groups this with Ksav's English-vocabulary seam: emit the pairing from the authoritative code once, then cross-check.)

## 7. The shelf-step order and the "which seforim relate" contract are restated in several prose venues — one has already rotted

- **Lens:** 3 · **Verdict:** wrong-but-keep → **fix (make the prose a reader of the compiler, not a second author)**
- **The problem:** the order of the shelf build steps, and the meaning of the relation/companions list, are described in the code, in `docs/the-libraries.md`, in `docs/troubleshooting.md`, and in `spec.md` — four venues for one fact. `docs/troubleshooting.md:180` still instructs running `cargo run --release -p girsa-app --bin girsa-companions` as a manual step, and `docs/the-libraries.md:219–228` frames the same cache as something you may "skip" (making the list "offered but short"). When these disagree with the compiler, the reader follows the prose and the panel is silently short.
- **Refine:** store the shelf-step ordering once (the crate already owns it in `Shelf`/`girsa-app`); make the docs reference that one thing rather than restate it, and mark the companions cache in a single place the `docs/the-libraries.md` page links to. This is the "emit once, cross-check" rule (Ksav audit Interop-B) applied to prose venues instead of code boundaries.
- **Cost:** incremental; a doc read-through plus the one test that walks the shelf-step order and asserts the prose lists its consequences.

## 8. The document-gate machine is applied to two documents; the rest of the narrative is unmarked and reproduces the rot it was built to end

- **Lens:** 1 · **Verdict:** rewrite → **fix (extend the gate convention, don't stop explaining)**
- **The problem:** the repo built a genuine machine for keeping prose true to code — `tools/verify.mjs` as one command, `tools/readme-numbers.sh` (with the `the_numbers_in_the_readme_are_measurements` test) keeping README's counts honest, `docs/coverage.md` + `tools/check-coverage.mjs` + `tools/mutation.mjs` keeping the coverage manifest honest. But that machine is wired to exactly **two** documents (README's numbers and the coverage manifest); the dozens of other narrative files (`docs/*`, `docs/record/*`, `HANDOFF.md`, `lamdan/`, `AUDIT-*`) carry no markers and drift — and already have (item 7 is one such drift; see item 9). The prose footprint is ~888 KB of Markdown across ~39 files against ~5.6 MB of source.
- **Refine:** adopt the repo's own convention outward — author an invariant once (code or `coverage.md`), reference it from doc comments, and let `git log` / a one-line dated log carry the history (this is the exact distill pattern in Ksav audit item 10). Concretely: give the shelf-step order (item 7) and the reading/lens invariants a home in `coverage.md` so the existing mutation machine can pin them, instead of leaving them as prose claims any edit can break.
- **Cost:** near-zero reader-visible change; the machine and its tests already exist, they just need more rows.

## 9. Concrete victim of the unmarked prose: the מפרשים/companions list can go silently short because its build step lives only in narrative docs

- **Lens:** 1 · **Verdict:** rewrite → **fix (assert the consequence the prose promises)**
- **The problem:** `girsa-companions` records which seforim open beside which (a "top-200 per work" cache, per `BUILDER.md:1831–1832`). The feature's value is that a reader sees the full מפרשים list; but "run this once, it seeds the list" lives only in unmarked prose (`docs/the-libraries.md:228`: skip it and "the list is offered but short"). Because the cache is optional-by-prose, there is no check that a corpus built by following the docs ever gets a non-empty list — the exact "silently empty because the step was a caption" shape the audit keeps finding.
- **Refine:** one test (or coverage-manifest row) asserting that a shelf built through the documented path presents a non-empty relation/companions list, and that the documented build step is reachable from the code (a bin that still exists, a cache that a fixture can seed). Converts a prose promise into a predicate `the_shelf_is_in_the_order_it_is_printed_in` already gestures at.
- **Cost:** incremental; the `girsa-app` fixtures already build real shelves.

## 10. The committed audit volumes are a second, unshipped documentation layer — distill, don't re-explain

- **Lens:** 1 · **Verdict:** rewrite → **fix (distill to conclusions, keep every durable rule in CONTRIBUTING, let git log hold the history)**
- **The problem:** `AUDIT.md` (~24 KB), `AUDIT-2026-08-23.md` (~36 KB), `AUDIT-CORRECTNESS.md` (~52 KB) and `lamdan/girsa-2026-08-06.md` (~53 KB) are committed narrative that CI neither runs nor cross-checks — they are history best kept in `git log`, plus a few durable rules that belong in `CONTRIBUTING.md`. Keeping four dated essays word-for-word invites re-reading them as current on the next sweep (this audit's recurrence check had to do exactly that).
- **Refine:** distill the committed audit volumes into one `audit/README.md` (or a section in `CONTRIBUTING.md`) stating the 2–3 conclusions each stands for, keep every durable rule in `CONTRIBUTING.md`, and let git history hold the full text. Nothing loses correctness evidence this way because none of it is asserted; what survives is the *rule*, not the essay.
- **Cost:** file-level; no code change.

## 11. Code-comment citations to `spec.md §N` and `W`-needs are human-read and unchecked — add a resolving test

- **Lens:** 3 · **Verdict:** rewrite → **fix (make the citations checkable, like the rulings it already asserts)**
- **The problem:** comments cite the contract constantly — `spec.md §9.4` (ladder.rs), `W2`, `W23`, `W28`, `W30` (mcp/chain/continuity comments) — and several of those citations are what a future reader trusts to find the rule. Nothing verifies a `spec.md §N` or `W\d+` reference still exists; a section renumbered or a `W`-need renamed silently points at thin air (the `§12` case is item 13).
- **Refine:** one test that scans the source comments for `spec.md §N` and `W\d+` (and `B\d+` builder rulings) and asserts each resolves against the live `spec.md`/`BUILDER.md` — the same "the rules this repo wrote down exist" test the corpus already runs for its own index. Mechanical; catches the next mis-written citation on the first PR that adds one.
- **Cost:** one test; incremental.

## 12. Number claims in `spec.md`'s own tables are gated by nothing, while `README.md`'s are regenerated — close the asymmetry

- **Lens:** 3 · **Verdict:** wrong-but-keep → **fix (extend the measurement discipline one level up)**
- **The problem:** README's counts are honest because `readme-numbers.sh` regenerates them and a test fails on drift. But `spec.md` carries comparable counts inside its tables (e.g. the product/repo rows that cite §12, the milestone `M`-row numbers) with no generator and no drop-below check — so a number in the highest-authority document is the *least* guarded, the inverse of its weight.
- **Refine:** treat spec's checkable rows the README way — either regenerate them from the crate/spec counts the tests already compute, or add a drop-below floor test (cheap, and it is the same tool as `readme-numbers.sh`). Do not hand the numbers to another prose author.
- **Cost:** incremental; the numbers to protect are few.

## 13. `girsa-mcp/src/lib.rs` claims "spec.md §12 … MCP on both ends" — that citation is false

- **Lens:** 3 · **Verdict:** rewrite → **fix (cite the real authority)**
- **The problem:** `crates/girsa-mcp/src/lib.rs:3` opens *"spec.md §12 and BUILDER.md W28: MCP on both ends."* `spec.md §12` is **Architecture** (`spec.md:821`, subsections "Repos and crates" `:847` and "Platforms" `:886`); it never mentions MCP. Finding this is independent of finding the leaf itself (item 4): the crate exists, in part, because its own governing prose *appears* to bless it. The actual place MCP is specified is `BUILDER.md W28 — Chain tracing, semantic lane, MCP` (`BUILDER.md:805`) and milestone `M8` (`spec.md:962`); spec also names the "MCP surface" once in passing at `:690`.
- **Refine:** fix the citation to `BUILDER.md W28` (and `M8`), and — because a *future* reader will trust it — let item 11's resolving test cover this exact string so a reference to a section that never names MCP cannot be introduced silently again. (Ksav audit Interop-B cross-references this item.)
- **Cost:** a sentence in lib.rs:1–3 plus one test assertion.

---

## Interop with Ksav (the shared sefer-crates seam)

### A · The prose-tax finding is the same disease in one shared patient
Both repos carry the bulk of their prose as record/narrative rather than living documentation, and both built machines to keep prose true to code that are themselves (a form of) prose. Girsa's share: items 8, 9, 10, 12 above; Ksav's: its items 10–13. **Do not fix one without the other** — the effort is the same, and each repo's gate convention (Girsa's `readme-numbers`/coverage/mutation, Ksav's `assert_same_page`/documentation-tower) already teaches the fix the other needs.
- **Refine:** adopt one convention across both: author an invariant once (code or a committed, machine-checked contract), reference it from doc comments, and let `git log`/a one-line dated log carry the history.

### B · Cite the compiler, not the prose
Girsa's bare-string Rust↔TS boundary (item 6) and the shelf-step prose duplication (item 7) are the same bug Ksav has at its English-vocabulary seam: a supervising truth (the Rust `#[tauri::command]` / the Typst source / `spec.md`) is read back by a hand-written regex or a bare string, and the pair drifts. Emit each pairing from the authoritative code once, and cross-check rather than re-derive. The false `spec.md §12 … MCP` citation (Girsa item 13) is the same defect in kind: prose asserting what a different prose says.

### C · Product-identity questions only you can answer
- Was the desktop shell worth it (the 140-command surface), or was "a library + the CLI + slim tools" the actual want? (The shell grew beyond the "adapter, nothing decided here" framing — items 5–6.)
- Is the `girsa`↔`ksav` two-repo split right, or is a single artifact (the `sefer-crates` shared contract plus one app) the real shape? The seam exists to be sealed; items 6 and 13 are both places where a shared, emitted contract would have caught drift.
- Does `girsa-mcp` have a stakeholder (item 4)? The read-spine is worth keeping either way; the write tools and the lane-model-to-every-feature payload are the decision.

---

## Credit — what is genuinely healthy at this HEAD (lens 1 holds)
Do not re-litigate; these are materially healthy and were verified at HEAD: the **panel path** absorbing re-segmentation through `Anchor::names` + `Standing` (item 1's healthy half — it already works where it is wired); the index/search stack's **no-auto-widen discipline** and per-gap proximity correctness; the ladder's **"offered, not applied / never a bare zero"** contract (item 3 builds on it rather than loosening it); the **README machine-gating** that took it from ~2,500 to 69 lines with every marked number verified true; the single-command gate `tools/verify.mjs` (the "gate exists in prose instead of a program" lesson already learned once and acted on); the **coverage manifest + mutation gate** (`docs/coverage.md`, `check-coverage.mjs`, `mutation.mjs`) — the mechanism items 8–9 ask to extend, not replace.

## Red flag for a future audit
If a *future* run independently re-derives a "false `spec.md §N` citation" or a "prose promises a non-empty list the machine doesn't guarantee" finding, treat it as a **coverage regression** in this audit's items 13 and 9, not corroboration — both should be closed (made impossible) rather than re-confirmed. The chain-`Standing` gap (item 1) and the raw-text matcher (item 2) are the two known-live Recurrents to re-open first.