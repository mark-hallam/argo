# fix-billing-meter-zero-while-spend-accrues

**Owner**: claude or codex — **P2 BILLING SURFACE (re-test N2).**
**Depends on**: none.
**Authored against**: origin/main `2319a7a69` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-billing-meter-zero-while-spend-accrues

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md ("Re-test 2026-06-13" section — operator-contracted shakedown re-test).

## Problem
/billing showed "Actions this month 0 / 100" while Rex ran twice, took "19
actions today" per /team, and real spend ($0.0369) accrued. 0 actions next to
nonzero spend reads as broken. ALSO two "Actions this month" rows render (plan
0/100 + metered 0/Unlimited) — confusing duplication.

## Fix
(a) Reconcile the meter: if it counts only approved/billable actions
(defensible) it must not sit at 0 while spend is nonzero — either count the
runs that produced spend, or label the meter so 0-with-spend is self-
explanatory. Cross-check against the #2871 tenant_usage metering seam and the
demo-mode run path (does a demo/probation run increment tenant_usage?).
(b) De-duplicate the two "Actions this month" rows into one clear line (plan
allowance vs metered overage should not both render as bare "Actions this
month 0/...").

## Verify
Test: a tenant with spend > 0 and >=1 completed run shows a nonzero or clearly-
labeled meter; only one "Actions this month" row renders. Route test + vitest.
