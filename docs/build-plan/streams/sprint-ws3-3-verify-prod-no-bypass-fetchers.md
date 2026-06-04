# sprint-ws3-3-verify-prod-no-bypass-fetchers

**Owner**: claude or codex. **Frontend de-mock — ensures a public user sees REAL data, never fixtures.**
**Depends on**: none.
**Authored against**: origin/main `b32525458` on `2026-06-04` (verified against the gap-audit WS3: `web/lib/skycot/demo-mode.ts` gate semantics + the flagged thin-gate modules).
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws3-3-verify-prod-no-bypass-fetchers`.

## Why

The gap-audit (WS3) confirmed the mock fallbacks are gated on
`isGatewayAuthBypassed()` = `NODE_ENV !== "production" && NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`,
so a production build *should* ignore the bypass and call real endpoints. But it flagged
several fetchers with thin/absent live gates + high sample-ref counts — `orchestration.ts`,
`workers.ts`, `worker-trust.ts`, `workflow-runs.ts`, `workplace-knowledge.ts` — as the
highest risk of silently serving fixture data to a real signed-in user. This brief
AUDIT-CONFIRMS (and fixes where needed) that every one of those calls the live endpoint
under `NODE_ENV=production` and never renders `web/lib/sample-*.ts` data for a real user.

## What

For each flagged module (`web/lib/api/orchestration.ts`, `workers.ts`, `worker-trust.ts`,
`workflow-runs.ts`, `workplace-knowledge.ts` — re-confirm the set at claim time via
`rg "sample-" web/lib/api`):
1. Trace the fetch path: under `NODE_ENV=production` (bypass off), does it call the real
   `apiGet`/`apiPost`, and does the sample-`*` fallback only fire when `isGatewayAuthBypassed()`
   is true (dev)? 
2. If any path can serve sample data to a production/real-session user (a missing
   `isGatewayAuthBypassed()` guard around a fixture fallback, or a fixture default), FIX it so
   the live endpoint is authoritative and a missing/errored response surfaces an honest
   error/empty state — never bundled fixtures.
3. Add a vitest per fixed module asserting: under the prod/non-bypass path the live fetcher is
   called and the sample fallback does NOT fire (mock the gate + assert).

NO THEATER: a real user must never see `sample-*` data. This is verification-first — if a
module is already correct, the deliverable is the pinning test (prove it), not a needless edit.

## May-touch
- the flagged `web/lib/api/*.ts` modules (only where a real fix is needed)
- vitest specs pinning the prod-path behavior
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

Do NOT alter the dev-bypass behavior (fixtures are fine in dev); only ensure prod/real-session never sees them.

## Acceptance criteria
- [ ] Each flagged module is confirmed (with a pinning vitest) to call the live endpoint under `NODE_ENV=production` and to NOT render sample data for a real session.
- [ ] Any module that COULD leak fixtures to a real user is fixed (honest error/empty instead).
- [ ] PR body lists, per module, "already correct (test added)" vs "fixed + test".
- [ ] `pnpm typecheck` + `pnpm lint` clean; `pnpm vitest run` GREEN (report counts).

## Out of scope
- The no-live-endpoint modules (retrain/retros/duties → WS3-1) and workflow save/publish (WS3-2); the sample-fixture tree-shake (WS3-4).

## Parallel-surface check
- `rg 'isGatewayAuthBypassed|isDemoLive|sample-' web/lib/api` — work within the established gate; do not add a parallel gating helper.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS3, "endpoint exists, gate just needs flipping / empty-state polish").
