# fix-approvals-decide-double-submit-idempotency

**Owner**: claude or codex — **P2 RESILIENCE (soak finding).**
**Depends on**: none.
**Authored against**: origin/main `6524852d4` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-approvals-decide-double-submit-idempotency

Source: docs/skycot/audit/2026-06-13-e2e-soak-15x.md FINAL SCORECARD.

## Problem
Under rapid DOUBLE-decide of an approvals row (abuse: clicking approve/refuse
twice fast against a multi-run hold backlog), the soak saw a `decide`-POST
500-class response race (no server-side 500 was logged, so likely a
client/transport race or a second decide on an already-decided row). Other
double-submit paths (hire, run, waitlist) are defended with disable-on-submit
+ idempotent backends; the approvals `decide` path should match.

## Fix
Make the approvals decide path double-submit-safe: disable the
approve/refuse control on first click until the response resolves, AND make
the decide endpoint idempotent — a second decide on an already-resolved
approval returns a clean 200/409 with a clear body, never a 500-class race.
Grep routes_approvals_json::decide / finalize_decision.

## Verify
Test: two rapid decides on one approval → one state change, second is a clean
no-op/conflict (not 500); UI disables the control on first click. Route test +
vitest. Branch + PR.
