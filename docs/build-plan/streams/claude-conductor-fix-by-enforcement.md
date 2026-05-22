# claude-conductor-fix-by-enforcement

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `61b9f1b1c` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-conductor-fix-by-enforcement`.

🟢 **P3 CONDUCTOR PROCESS HARDENING.** Source: evaluator round-3 (`docs/skycot/evals/scheduled/LATEST.md`) process recommendation:

> "every `MERGED-FINDINGS-ONLY` commit should carry a `fix-by <date>` line; conductor's tick should fail-loud if a FINDINGS-ONLY merge passes its fix-by without a fix-PR."

Context: PR #2046 was merged as `MERGED-FINDINGS-ONLY` (shipped 3 `#[ignore]`'d regression tests documenting a real cross-tenant export bug but no actual fix). The conductor noticed and authored the fix-brief manually within 30 min — but in a less-attentive run this gap could persist. The evaluator's recommendation is a soft contract: findings-only commits get a deadline; if the deadline lapses, the conductor's tick fails loudly to force attention.

## Scope

Three changes:

1. **Define the wire format**: extend the `MERGED-FINDINGS-ONLY` ledger-note convention with a required `fix-by YYYY-MM-DD` token. Document in `docs/build-plan/PROTOCOL.md` under a new "MERGED-FINDINGS-ONLY contract" section.

2. **Sweep script**: `NEW: scripts/check-findings-only-fix-by.sh` — scans `BUILD_STATUS.md` for `MERGED-FINDINGS-ONLY` rows, parses `fix-by` dates, and reports:
   - 🟢 within budget (deadline in the future) — for awareness only
   - 🟡 due today
   - 🔴 overdue (no fix-PR merged after the date) — fails the script with exit 1

3. **Conductor tick integration**: add a step to `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` (between step 4.6 heartbeat and step 5 PR review) that runs the sweep and surfaces overdue findings-only rows in the tick summary. Don't make the conductor's tick FAIL — just print the warning prominently so the human/agent driving the loop notices.

The wire format change is backward-compatible: existing `MERGED-FINDINGS-ONLY` rows without `fix-by` are treated as "no deadline" (status 🟢 informational, never fail).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `docs/build-plan/PROTOCOL.md` — narrowly scoped: add a "MERGED-FINDINGS-ONLY contract" section near the existing ledger-status taxonomy. ~30 LOC.
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — narrowly scoped: insert a step 4.7 between heartbeat and PR review. ~10 LOC.
- `NEW: scripts/check-findings-only-fix-by.sh` — the sweep script. ~80-100 LOC bash.
- `NEW: scripts/check-findings-only-fix-by-tests.sh` (or inline assertions) — smoke test against a temp BUILD_STATUS.md fixture covering all 4 cases (no fix-by / future / today / overdue).
- `docs/build-plan/BUILD_STATUS.md` — narrowly scoped:
  - Update the existing `mission-cross-tenant-test-audit-export` row (line ~164) to add `fix-by 2026-05-25` to its `MERGED-FINDINGS-ONLY` note (giving the fix-brief authored in tick-64 a 2-day deadline).
  - Standard row flip on this brief.

## Files you must NOT touch

- The PROTOCOL_BRIEF_TEMPLATE.md — this is a process hardening, not a brief-template change.
- Any source code (Rust / TypeScript). Pure ops/docs/scripts.
- The auditor or evaluator scheduled-task configs.
- BUILD_STATUS.md beyond the 2 narrowly-scoped edits above.

## Acceptance criteria

- [ ] `docs/build-plan/PROTOCOL.md` has a "MERGED-FINDINGS-ONLY contract" section explaining the wire format + the `fix-by YYYY-MM-DD` requirement.
- [ ] `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` step 4.7 added between heartbeat and PR review, invoking the sweep script + surfacing overdue rows in the tick summary.
- [ ] `scripts/check-findings-only-fix-by.sh` exists, is executable, and exits 1 only when ≥1 overdue row is found (warnings on due-today + informational on within-budget exit 0).
- [ ] Smoke test passes: fixture with 4 rows (no fix-by / future / today / overdue) → expected output + exit code.
- [ ] `mission-cross-tenant-test-audit-export` row in BUILD_STATUS.md is updated with `fix-by 2026-05-25` in its MERGED-FINDINGS-ONLY note (2-day budget from when the fix-brief was authored in tick-64).
- [ ] Running the sweep against current main returns 🟢 on `mission-cross-tenant-test-audit-export` (deadline 2026-05-25 is in the future).
- [ ] No false-positive: rows without `fix-by` token return 🟢 informational (backward-compat).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-conductor-fix-by-enforcement`
- PR title: `conductor: fix-by enforcement on MERGED-FINDINGS-ONLY ledger rows (claude-conductor-fix-by-enforcement)`

## Notes for the implementer

- ~100-180 LOC expected. Bash sweep script ~80, docs updates ~50, tests ~30.
- **The sweep script is the load-bearing piece.** Make sure the date parser handles both `YYYY-MM-DD` (canonical) AND a few common variants the operator might type by accident (`YYYY/MM/DD`, `D Mon YYYY`). Failure mode for unparseable dates: log a warning, treat as "no fix-by", continue.
- **Don't fail the conductor tick on overdue findings.** The recommendation explicitly says "fail-loud" not "fail-hard" — the goal is operator/agent attention, not blocking work. Print the warning prominently (use color codes if possible) so the next human/agent reading the tick output sees it. Use it as a `WARN` not an `ERROR`.
- **The `fix-by` budget for the example update**: 2 days from the original FINDINGS-ONLY merge is reasonable for a P0-shaped findings-only. The pattern: if the original brief was P0 → 2-day budget; P1 → 5-day; P2 → 14-day. Document this as a soft guideline in PROTOCOL.md, not enforced by the script.
- **No source code changes** — this is a process/tooling brief.
- **Pre-flight**: confirm the sweep script doesn't accidentally pattern-match `MERGED-FINDINGS-ONLY` in other contexts (e.g. inside historical note text within a row that's now MERGED). Use a strict regex anchored at the start of the status column.
