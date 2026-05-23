# ops-fix-by-tracker-add-p0-cu-macos-blocker

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `a3a881fa3` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-fix-by-tracker-add-p0-cu-macos-blocker`.

🟡 **P1 OPS.** Add a P0-blocker matcher to `scripts/check-findings-only-fix-by.sh` (or sibling gate) so any P0 row unfixed >24h escalates regardless of whether it's a FINDINGS-ONLY row with a `fix_by` field.

## Why

Evaluator round-30 (2026-05-23T22:30Z, SHARED.md) flagged that the cu-macos workspace-build P0 blocker has been open 14h+ without escalation. The protocol gate that *would have* escalated it (`scripts/check-findings-only-fix-by.sh`) tracks only the 5 named FINDINGS-ONLY rows; it has no general "any P0 unfixed >24h" matcher. The brief that should fix the cu-macos error is `fix-cu-macos-permission-revoke-test-add-tenant-worker-fields` (currently CLAIMED) — not on the fix-by tracker.

## Scope

- Choose ONE: (a) add a `p0_fix_by` field to ledger rows for non-FINDINGS-ONLY P0s, OR (b) write a sibling gate that greps for `🔴 P0` rows that are not MERGED and have been TODO/CLAIMED/CHANGES-REQUESTED >24h. Recommend (b) — less ledger churn, additive.
- New file: `scripts/check-p0-aging.sh` — exit 1 if any unfixed P0 >24h.
- Wire into conductor tick: PROTOCOL_LOOP_PROMPT step 4.8 already calls `check-findings-only-fix-by.sh`; add a parallel call to the new gate.
- ≤80 LOC bash.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `scripts/check-p0-aging.sh` (new)
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` (one-line addition)
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Production Rust code.
- Other scripts.

## Acceptance criteria

- [ ] New gate detects current cu-macos workspace-build blocker as overdue (>14h at authoring time).
- [ ] Bash linting clean (`shellcheck` if available).
- [ ] PROTOCOL_LOOP_PROMPT references the new gate.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
