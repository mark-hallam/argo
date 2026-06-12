# launch-wire-grant-card-to-capability-requests

**Owner**: claude or codex — web-only (DEC-006), LOCKED BRAND (G2-Elevated).
**Depends on**: none (tg1 #2847 + tg3 #2834 both MERGED — backend fully live).
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-wire-grant-card-to-capability-requests

## Problem (launch audit 2026-06-12)

The locked G2-Elevated grant card reads `result.capability_request` off the
run envelope (`WorkerOverviewContent.tsx:181`) but `routes_worker_run.rs`
never returns that field — and the REAL backend now exists:
`GET /api/v1/workers/<id>/capability-requests` (tg1 #2847, pending rows,
tenant-scoped) + durable `POST /workers/<id>/capability-grants` (tg3
#2834). The connect-on-demand payoff is dark while its backend is live.

## Fix

Fetch pending capability-requests on worker overview mount (and after each
run) via the tg1 endpoint; render the locked card for the newest pending
request; Connect → for a TOOL request call the tg3 durable-grant POST
mapping the card's read/write toggles to `access: read|write` (tg3's
resolve_tool handles file/filesystem); for a CONNECTOR request keep the
existing tg2 authorize fetcher. Honest empty state; mock-gated
(hasRealSession); design stays the locked mockup (no redesign).

## Acceptance
1. vitest: a mocked pending request renders the card; Connect(read) posts
   the durable grant; no request → no card (would FAIL/false-hide today).
2. Full vitest suite stays 0-failed; tsc clean; files <500 LOC.

~80-150 LOC. **🟠 P2 — turns on the locked connect-on-demand moment.**
