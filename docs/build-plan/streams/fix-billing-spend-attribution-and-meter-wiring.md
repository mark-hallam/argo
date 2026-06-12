# fix-billing-spend-attribution-and-meter-wiring

**Owner**: claude or codex — **P2 BILLING SURFACE (shakedown F11).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-billing-spend-attribution-and-meter-wiring

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem
(a) /billing attributes a fresh Genesis hire's Run-now spend to "Unknown
teammate" — the DEC-038 per-teammate attribution misses this path (check the
demo-session Run-now flow against fix-runnow-process-job-worker-attribution,
which fixed the gateway run path previously — this may be the demo-session
variant or a name-join miss for genesis workers). (b) The usage panel is a
perpetual "Loading usage..." — the meter API exists (GET /api/v1/billing/meter,
#2871) but the SPA never wires it.

## Fix
(a) Thread/stamp worker_id on the demo Run-now spend path; join names for
genesis workers. (b) Wire the meter panel to the real API (included vs used vs
overage + cap) with an honest empty/error state — no infinite spinner.

## Verify
Test: run-now spend row attributes to the named worker; meter renders real
counts (route test + vitest); spinner has a bounded timeout state.
