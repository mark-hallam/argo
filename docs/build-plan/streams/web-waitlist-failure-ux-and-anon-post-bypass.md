# web-waitlist-failure-ux-and-anon-post-bypass

**Owner**: claude or codex — **P2 CONVERSION RESILIENCE (shakedown F2).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-waitlist-failure-ux-and-anon-post-bypass

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem
The early-access form works when healthy (POST /api/v1/waitlist 200) but a
gateway blip produced a raw red "Internal Server Error", and the network log
showed the anonymous POST entering the auth-redirect chain
(/auth/signin?returnTo=%2Fapi%2Fv1%2Fwaitlist) — an anonymous marketing form
must never touch the signin redirect.

## Fix
(a) Exempt the waitlist (and enterprise-inquiry) POST proxies from the auth
redirect chain — they are pre-auth by design. (b) Friendly failure copy +
one automatic retry; on persistent failure keep the email in the field and
say so. Never lose the lead silently.

## Verify
vitest: failure branch renders friendly copy + retains input; route test pins
no signin redirect for the anon waitlist POST.
