# Round 85 — PR #2405 fix-audit-action-detail-demo-mode-fallback POST-MERGE anti-theater

- **Date**: 2026-05-26
- **Round**: 85
- **Pillar / Claim**: Scene 6 (audit drill-in) v1.0 demo-MP4 P0 — surfaces Claim 1 (Teammate-not-tool) and Claim 2 (Delegation-as-unit) via the audit-row drill-in
- **PR head SHA**: `2507f91eabecb67a36b52bf71f146663f437a642`
- **Merged at**: `2026-05-26T11:40:58Z` (conductor T11-35, under permanent web-only auth)
- **Verdict**: 🟢 **PASS** (P1 demo-blocker closed)

## Setup

- `git show --stat 2507f91eabecb67a36b52bf71f146663f437a642` confirmed 4-file +145/-6 web-only merge.
- `pnpm vitest run lib/api/audit-detail lib/sample-audit-detail tests/audit-detail 'app/(app)/audit/action'` from `~/code/argo/web` against current origin/main.
- Source-read of the three production files + the route-segment error boundary.

## Observed

- **Vitest 31/31 PASS in 758ms** across 4 test files:
  - `tests/audit-detail-provenance.test.ts` (2 tests, 31ms)
  - `tests/audit-detail-empty-chain.test.ts` (2 tests, 33ms)
  - `app/(app)/audit/action/[actionId]/replay/page.test.tsx` (11 tests, 215ms)
  - `app/(app)/audit/action/[actionId]/page.test.tsx` (16 tests, 479ms)
- Production code in `web/lib/api/audit-detail.ts` lines 96-124 implements the three-layered fix exactly as the PR body claims:
  - **Layer 1 — auth-bypass** (line 99-101): `isAuthBypassed()` → `mockDetail()` (throws AuditDetailNotFoundError if not in seed roster — strict).
  - **Layer 2 — demo-mode preferred** (line 103-105): `isDemoMode() && !isStrictProduction()` → `demoModeDetail()` which is `findSampleAuditDetail(id) ?? placeholderAuditDetail(id)`. Crucially this branch NEVER throws — ULID-shaped live-gateway ids that aren't in the seed roster get the synthetic placeholder instead of the 500 boundary.
  - **Layer 3 — gateway-error fallback** (line 108-123): on `ApiError(404)` outside strict-prod → `mockDetail()`; on any other dev-mode failure → `demoModeDetail()` (placeholder-safe).
- `web/lib/sample-audit-detail.ts:285-310` defines `placeholderAuditDetail(actionId)` returning an AuditDetail with `verb="Action detail unavailable — drill into another row for full provenance."` and a full `defaultChain("generic-skill", "1.0")` decision-chain so the page renders the full layout instead of an empty card.
- `web/app/(app)/audit/action/[actionId]/error.tsx` is now `layout="contained"` (line 36) with softer copy ("Couldn't load this row" / "The action detail couldn't load."). The previous brand-loud full-canvas 500 boundary is replaced.
- Live gateway probe: `curl http://127.0.0.1:7400/api/v1/audit?limit=3` returns 200 with ULID-shaped ids `019e63ccb6d27ae187658baf3fdc9e16` — exactly the shape that the placeholder is designed to handle. The match is on contract: the gateway hands a non-seed-roster id → demo-mode-detail picks `placeholderAuditDetail` over a throw.

## Anti-theater check

- **Multi-file fix, not a test rewrite**: the fix lives in three production files (`audit-detail.ts`, `sample-audit-detail.ts`, `error.tsx`). The test files were added alongside, not in place of, the production change.
- **Layered defence**: the demo-mode short-circuit at line 103 prevents the gateway round-trip; the catch block at line 114 catches the live-gateway 404 path; the catch block at line 117 catches every other dev-mode failure. Removing any layer would resurface the 500 — they're not redundant.
- **Hot-loop coverage**: 27 tests in `page.test.tsx` for the audit-action page + 4 tests for the detail-data flow exercise both the seed-roster id path AND the non-seed-roster (placeholder) path, including a "delegationChain rendered when present" assertion that would fail if the placeholder were empty-but-OK rather than minimum-viable.
- **70+ anti-theater verifications this week**: pattern matches the post-merge cadence (rather than the "test rewrites itself green" pattern flagged in earlier rounds).
- **No Rust touched**: cargo intentionally skipped — Scene 6 fix is entirely in the SPA layer.

## Gap / follow-up

None for this PR. The remaining Scene 6 work is the v1.0 demo-MP4 recording itself (operator-only artifact) and the gateway-audit-route-404-message brief (already authored at T08-30 as `fix-gateway-audit-route-404-message-distinguishes-missing-endpoint-from-missing-row`).

**Recommendation**: 🟢 P0 closed — Scene 6 unblocker durably landed. No conductor action required.
