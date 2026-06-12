# launch-email-delivery-honesty-and-public-base-url

**Owner**: claude or codex — front-door honesty (NO THEATER on the first user touch).
**Depends on**: none.
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-email-delivery-honesty-and-public-base-url

## Problem (2026-06-11 walkthrough P0 + launch audit 2026-06-12)

(1) `POST /auth/signup` returns `{"status":"sent"}` unconditionally —
`deliver_magic_link` swallows send errors into a warn log, so a stranger
whose email never sends gets an irrecoverable "Check your inbox". (2) The
magic-link base URL defaults to hardcoded `https://app.skycot.com`;
`SKYCOT_PUBLIC_BASE_URL` is not a first-class deploy var. (3) The deploy
env templates (`deploy/secrets/skycot-production.env.example`) omit BOTH
the base URL and the `[email.outbound]`/SES values — a by-the-book deploy
ships log-only email with wrong-host links.

## Fix

(1) Thread the delivery result into the signup response (`status:
"sent" | "delivery_failed" | "not_configured"`) + SPA check-email page
shows the honest failure state with a support path. Do NOT leak
provider error detail to the client (generic message; detail to log+audit).
(2) `SKYCOT_PUBLIC_BASE_URL` env read at the link-mint site, default
falls back to current behavior, validated at boot in production profile
(warn loudly if unset). (3) Add both + SES SMTP vars to
`deploy/secrets/skycot-production.env.example` AND the ECS task env in
`infra/terraform/modules/ecs/` AND AWS_DEPLOY_RUNBOOK step list.

## Acceptance
1. Test: noop provider → signup response carries `not_configured`; SPA
   renders the honest state (vitest). Forced send error → `delivery_failed`.
2. Link host honors SKYCOT_PUBLIC_BASE_URL (test).
3. Both env templates updated; clippy + vitest + tsc clean.

~80-140 LOC. **🔴 P1 — the first thing every stranger touches.**
