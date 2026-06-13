# fix-probation-rehold-after-approval-or-fix-copy

**Owner**: claude or codex — **P1 PRODUCT-PROMISE (re-test N3).**
**Depends on**: none.
**Authored against**: origin/main `2319a7a69` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-probation-rehold-after-approval-or-fix-copy

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md ("Re-test 2026-06-13" section — operator-contracted shakedown re-test).

## Problem
After a probation-held tool is approved from the /approvals inbox, the NEXT
"Run now" re-holds the SAME tools (memory/todo) with the same toast: "Rex ran,
but their tools are held by the probation window — your OK rides on the next
run." Either the approval is meant to clear those tools for subsequent runs
(and the clear isn't applied), or the 24h probation by design holds every
action (and the toast copy overpromises — "your OK rides on the next run" is
false). Pick one and make truth and behavior agree.

## Decide + fix (first-principles)
Grep the probation-hold writer + the demo_override_granted approval path
(worker run path + routes_approvals). Determine the intended semantics:
- If probation = "hold every action for 24h regardless of prior approvals":
  fix the TOAST copy so it does NOT imply a one-time grant ("Held while Rex is
  on probation — approvals apply per run for the first 24h" or similar).
- If approval is meant to grant the tool for the probation window: persist the
  grant so the same (worker, tool) does not re-hold on the next run; the toast
  then becomes accurate.
The truth-bounded path (NO THEATER) is the load-bearing constraint — the UI
must not claim a grant the backend doesn't honour.

## Verify
Test: hire → run (held) → approve from inbox → run again → assert the chosen
semantics (either same tool NOT re-held, OR toast copy carries no
"next run" promise). cargo test for the touched crate + web vitest on the
toast copy if changed.
