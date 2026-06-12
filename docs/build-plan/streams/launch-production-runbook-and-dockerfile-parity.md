# launch-production-runbook-and-dockerfile-parity

**Owner**: claude or codex — deploy correctness; conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-production-runbook-and-dockerfile-parity

## Problem (launch audit 2026-06-12)

(1) The production/ECS image (root `Dockerfile:30-32`) copies only
`manifests/ + verticals/` — NO `policies/`, NO `skill-library/`, no pitch
env in the ECS task def — the exact "policy-less gateway" failure class
WS5 #2763 deleted for the compose path (truth-boundary reads fall back to
container paths that don't exist). (2) DEPLOYMENT.md is explicitly the
GATED-DEMO runbook (loopback gateway + basic-auth + demo-bypass); the
paying-customer shape (production-auth + bearer + magic-link/SES + Stripe
+ real domain) is a 10-line "scale-later" note, not a runbook.

## Fix

(1) Bring root Dockerfile to parity with `deploy/Dockerfile.gateway`
(COPY policies/ + skill-library/ + set the pitch/policy env), and add the
same env to `infra/terraform/modules/ecs/main.tf`; one container boot
smoke proving the gateway starts + serves a policy-dependent route in the
distroless image. (2) Author
`docs/skycot/operator/PRODUCTION-RUNBOOK.md`: the paying-customer
deployment shape end-to-end (non-loopback bind → production-auth mode,
bearer + JWT secrets, SES email, SKYCOT_PUBLIC_BASE_URL, Stripe env,
backup cron from the sibling brief, monitoring), each step labelled
operator-runs vs agent-verified, NO THEATER disclaimers where unverified.

## Acceptance
1. Distroless image boot smoke green incl. one policy-dependent route.
2. PRODUCTION-RUNBOOK.md exists, complete, cross-linked from
   DEPLOYMENT.md; env templates consistent across compose + ECS.

~80-150 LOC + doc. **🟠 P2 — removes the two-divergent-images trap.**
