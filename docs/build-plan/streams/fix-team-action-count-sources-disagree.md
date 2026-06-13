# fix-team-action-count-sources-disagree

**Owner**: claude or codex — **P2 UX-CORRECTNESS (re-test N1 / F12 remainder).**
**Depends on**: none.
**Authored against**: origin/main `2319a7a69` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-team-action-count-sources-disagree

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md ("Re-test 2026-06-13" section — operator-contracted shakedown re-test).

## Problem
/team header rollup says "7 actions this week" while the teammate card says
"19 actions taken today" — 19 today cannot fit inside 7 this week. Two
different counters (likely audit-events vs billable-actions, or two time
windows). The working-count was fixed (F12); the action-count math is still
wrong.

## Fix
Grep the /team header rollup source + the per-worker card count source
(team-headline rollup + worker activity count). Reconcile to ONE definition of
"action" and consistent windows (week >= today by construction). If header is
"billable actions this week" and card is "audit events today", relabel both
precisely so a 19-today / 7-week pair is impossible OR label them as different
metrics explicitly. Prefer one canonical count.

## Verify
Test: a worker with N actions today shows header-week >= card-today; vitest +
the team-headline rollup test.
