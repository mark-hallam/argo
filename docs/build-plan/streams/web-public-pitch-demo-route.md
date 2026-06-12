# web-public-pitch-demo-route

**Owner**: claude or codex — **P2 MARKETING FUNNEL (shakedown F6).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-public-pitch-demo-route

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem
Landing differentiator card CTA "Watch Portia refuse to overclaim" points at
/pitch which bounces logged-out prospects to signin — the truth-boundary demo
is unreachable for exactly the audience it exists to convince.

## Fix
Either route the CTA into the demo-workspace journey landing on /pitch, or add
a public read-only pitch demo lane (the anonymous public pitch route already
exists server-side: `POST /api/v1/pitch/ask/public` with per-IP limiting —
build the thin public page over it). Prefer the public page: zero-friction
proof of the headline differentiator.

## Verify
Logged-out click-through reaches a WORKING Portia ask (truth-bounded answer
renders); rate-limit intact; no auth bypass into the rest of the app.
