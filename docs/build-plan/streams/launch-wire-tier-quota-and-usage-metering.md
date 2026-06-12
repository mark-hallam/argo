# launch-wire-tier-quota-and-usage-metering

**Owner**: claude or codex — billing foundation → conductor review before merge.
**Depends on**: none (model-agnostic: needed under EVERY candidate pricing model).
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-wire-tier-quota-and-usage-metering

## Problem (launch-readiness audit 2026-06-12)

The tier-quota middleware is **dead code**: `check_tier_quota_or_reject`
has zero callers (its own header says dispatch wiring is "reserved for a
follow-up"), and the ONLY writer of `tenant_usage` is
`increment_tenant_usage` inside that same unwired middleware
(`quota_middleware.rs:120,135`). Consequences: the Free tier's 100-action
cap is fiction (unmetered free usage = losing money), and the /billing
usage meter + `GET /api/v1/tenant/usage` always show 0.

## Fix

Wire `check_tier_quota_or_reject` + the usage increment into the gateway
dispatch path at the action/run seam (the same place per-teammate spend
attribution hooks — one seam, no parallel counter). Quota-exceeded →
structured 402-class error naming the tier + upgrade path + audit row.
Usage increments for every metered action regardless of tier (the meter
must be true on Pro too — it feeds whichever pricing model the operator
picks). Free=100/mo, Pro=5000/mo from the existing tier config.

## Acceptance
1. Test: Free tenant's 101st action this month is refused with the
   structured upgrade error (would FAIL today); Pro increments visibly.
2. `GET /api/v1/tenant/usage` returns real non-zero counts after actions.
3. Existing billing tests green; clippy clean.

~80-150 LOC. **🔴 P1 — makes Free real and the meter honest; foundation
for the pricing decision.**
