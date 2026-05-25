# fix-replay-page-fabricated-forensic-report-for-unknown-action-ids

**Owner**: claude.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-replay-page-fabricated-forensic-report-for-unknown-action-ids`.
**Source**: e2e-ux-audit Agent 4 finding F1 (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`). Independent re-verification of prior-audit F67.

🔴 **P0 DEMO-GATE — Scene 6 customer-trust risk. Theatre the camera sees.**

`/audit/action/{id}/replay` **fabricates a forensic mock-report for any well-formed 32-char-hex action ID** the gateway returns 404 on. The fabrication renders:

> **Policy regression detected.** Replay output would have failed the current policy. Operator notified.

— plus a fake "ORIGINAL output" / "REPLAYED output" diff whose strings are concatenated from the URL parameter. A buyer who clicks any audit row that 404s on the detail endpoint (see also `fix-audit-list-detail-endpoint-id-mismatch` for the broader issue) lands on a synthetic forensic report. The page CLAIMS to do cryptographic replay; the output is generated from the URL. Function-side theatre per the function-vs-dressing rule (memory `feedback_no_theater_in_demo`).

## Root cause

`web/lib/api/replay.ts:69-88` catches gateway 404 → falls through to `mockOutcomeForActionId()` at `web/lib/sample-replay.ts:142-201`. Unlike `audit.ts` and `approvals.ts` (both gated on `isDemoMode()`), the replay client's mock fallback is **unconditional in any non-strict-prod build** (`NODE_ENV !== production && NEXT_PUBLIC_ENV !== production`).

`mockVerdict()` hashes the last char-code of the action ID mod 3 → one of `reproducible` / `drifted-aligned` / `drifted-misaligned`. Verdict 2 renders the red "Policy regression detected" banner at `web/components/skycot/audit/ReplayHeader.tsx:23-38`.

Agent-4 curl verification: an unknown 32-char-hex ID `01234567890abcdef0123456789abcde` returns gateway 404 → client matches `isMissingOrUnreachable` → returns synthetic report. The earlier prior-audit-mentioned `act_test_obviously_fake_999` test case failed format check (400) and didn't trigger the mock, which is why the prior audit's repro was narrower but the underlying defect is real.

## Fix (recommended recipe)

Gate the mock fallback at `web/lib/api/replay.ts:83` on:
1. `isDemoMode()` from `web/lib/api/demo-mode.ts` (consistent with `audit.ts` + `approvals.ts`)
2. AND `findSampleReplay(actionId)` returning a non-null entry from the canonical sample-replay TABLE

For any action ID outside the canonical 5-persona sample table, render `<FailedView>` with a real error message (e.g. "Action not found — replay sandbox covers the seed personas only"). Do NOT synthesise content.

## Acceptance criteria

- [ ] `web/lib/api/replay.ts:69-88` gates mock fallback on `isDemoMode() && findSampleReplay(actionId)`.
- [ ] curl-test: visiting `/audit/action/01234567890abcdef0123456789abcde/replay` in dev-bypass mode renders `<FailedView>` with a clear error, NOT the synthetic report.
- [ ] curl-test: visiting `/audit/action/{real-sample-replay-id}/replay` in `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` still renders the canonical sandbox content.
- [ ] No regression on the demo-mode happy path for the 5 seed personas.
- [ ] PR body includes before/after curl output for both cases + a screenshot of `<FailedView>` post-fix.

## May-touch

- `web/lib/api/replay.ts` (gate the mock fallback)
- `web/lib/sample-replay.ts` (only if `findSampleReplay` needs export tweaks)
- `web/components/skycot/audit/ReplayHeader.tsx` (verify no other code paths render the red banner spuriously)
- `web/app/(app)/audit/action/[actionId]/replay/page.tsx` (verify `<FailedView>` is wired)

## Out of scope

- Renaming the replay route
- Migrating away from styled-jsx (separate brief if needed)
- Touching the gateway's audit endpoints (separate brief: `fix-audit-list-detail-endpoint-id-mismatch`)

## Effort

~30-50 LOC + 1-2 regression tests (curl-based smoke). ~30-60 min.
