# launch-deploy-smoke-first-green-run

**Owner**: claude or codex — deploy verification (needs local Docker).
**Depends on**: launch-production-runbook-and-dockerfile-parity (image parity) — soft; can run against compose images as-is first.
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-deploy-smoke-first-green-run

## Problem (launch audit 2026-06-12 — DEC-029 in its purest form)

The deploy stack has NEVER been brought up: images build green (verified
2026-06-09) but `deploy/smoke/run.sh` has no recorded PASS on any machine,
the smoke compose is explicitly "runtime UNVERIFIED", and the deploy-smoke
GHA workflow is disabled (billing). "Authored + builds" is not "works".

## Fix

Run `deploy/smoke/run.sh` on this machine (Docker required — check
`docker info` first; if the daemon is unavailable, STOP and mark the row
BLOCKED-operator with the reason rather than faking it). Fix forward
whatever breaks (compose wiring, env, healthchecks) in MINIMAL diffs; do
NOT weaken the smoke to pass. Record the PASS: commit the smoke output
tail as `deploy/smoke/LAST-PASS.md` (date, image digests, steps green) and
make the smoke script append/refresh it on every green run.

## Acceptance
1. `deploy/smoke/run.sh` exits 0 on this machine; LAST-PASS.md committed
   with the evidence tail. 2. Any fixes are deploy-config only (no product
   logic). 3. The runbooks reference the smoke as the pre-deploy gate.

Effort: hours-to-day (discovery-heavy). **🔴 P1 — the first proof the
stack runs anywhere.**
