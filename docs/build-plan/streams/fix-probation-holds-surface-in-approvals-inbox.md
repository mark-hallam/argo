# fix-probation-holds-surface-in-approvals-inbox

**Owner**: claude or codex — **P1 PRODUCT-PROMISE (shakedown F10).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-probation-holds-surface-in-approvals-inbox

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem
A fresh Genesis hire's first run is held by the probation window (worker-page
toast: "Approve the held tools below") but `/approvals` shows 0 pending. The
inbox header promises "Every decision your AI teammates queue for a human
shows up here" — broken on the FIRST journey every new user takes.

## Fix
Surface probation/held-tool grants as approvals-inbox rows (P2-low severity
band). Reuse the existing approvals row plumbing — grep
`routes_approvals_json.rs` + the probation hold writer (worker run path) and
emit/queryable-join the held grants into the inbox list + pending count.
Approving from the inbox must resolve the hold exactly like the worker-page
control. NO parallel approvals store.

## Verify
Test: fresh worker in probation + run -> held grant appears in inbox list +
PENDING count; approve via inbox -> hold cleared, next run proceeds; existing
approvals tests stay green. Web: inbox renders the row (vitest).
