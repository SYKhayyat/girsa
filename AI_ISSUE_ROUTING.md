# AI ISSUE ROUTING — read this before opening ANY issue in these repos

You are an AI with write access to issues. Follow this exactly.

## 1. Read PLAN.md first
Every repo has a PLAN.md with phases ordered **foundations → safety → correctness → quality → features**. Open it before filing.

## 2. Place, don't append
- Decide which phase your issue belongs in:
  - Data model, wire/boundary contract, test infrastructure, secret/safety shape → FIRST phases.
  - Crash, data loss, silent corruption, security bypass, unbootable/destructive config → safety phase (near top).
  - Wrong answer, failed operation, perf blowup → correctness phase.
  - Duplication, hygiene, docs drift → quality phase.
  - New capability, proposal, UX wish → LAST phase.
- Insert the issue into that phase's list in PLAN.md in the same edit (or immediately after) as opening the GitHub issue. An issue with no PLAN.md line is incomplete work.
- Never place a feature above an unfixed safety/correctness item. Never append to the end by default.

## 3. Duplicates and false positives
- Search open issues for the same root cause first. Same root = fold into the existing issue/PLAN line, do not file.
- Mirror batches (e.g. one script filing N near-identical issues) go in one SKIP note with evidence, not N lines.
- If verification shows the claim is wrong (file/symbol absent, behavior contradicted by code with line refs), label it `false-positive` with the evidence and put it under the repo's SKIP section. Do NOT close it — the owner closes.

## 4. Issue body contract (so the next AI can verify)
- Claim + file:symbol/line evidence + repro or test gap + proposed resolving test.
- No bare prose claims ("X is slow", "counts agree") without the measurement location.

## 5. Worker sessions (fixing)
- One issue per session. Fix + its resolving test, commit, check the PLAN.md box, stop.
- Daily-driver repos (emacs-config, nixOS_config): correctness before perf, rebuild/dry-run check before commit.
