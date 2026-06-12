# fix-demo-mode-cta-posture-and-error-copy

**Owner**: claude or codex — **P1 CONVERSION (shakedown F9).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-demo-mode-cta-posture-and-error-copy

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem
Signup's "Or try a demo workspace · no email required" dead-ends when the
gateway lacks ARGO_DEMO_MODE=1, showing a prospect literal env-var
instructions. The restart script + runbook never set the flag, so this is the
DEFAULT state.

## Fix
(a) Posture: gated-deploy + dev default = demo mode ON — set it in
scripts/restart-live-gateway.sh and document in PRODUCTION-RUNBOOK.md (public
flip can revisit). (b) Error copy: replace with human copy ("The instant demo
is napping — leave your email and we'll wake it" or similar brand voice) +
keep the email path usable. (c) Hide-or-fallback: if the demo-mode probe
fails, the button can route to the waitlist form instead of erroring.

## Verify
With demo mode off: human copy + working fallback (vitest on the error
branch). With on: existing journey unchanged. Restart script sets the env
(assertion in script tests if present, else grep check in PR body).
