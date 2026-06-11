# fix-next-config-csp-headers

**Owner**: claude or codex — WS5 perimeter-adjacent → conductor review before merge (web/config only).
**Depends on**: none.
**Authored against**: origin/main `1735b11c3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-next-config-csp-headers

## Problem (verified by conductor's own full vitest run, 2026-06-12)

`web/tests/next-config-csp.test.ts` (4 tests) FAILS on main: the SPA's
`next.config` `headers()` returns **no Content-Security-Policy header** —
"CSP header must be present: expected undefined to be defined". The four
pre-written pins (the verifier already exists — this brief just makes it
true) require: connect-src whitelists the http(s) gateway origin, the
ws(s) gateway origin (live approvals WebSocket handshake), pins the
production `wss` origin, and never re-introduces the legacy 8787 gateway
port. Surfaced by the #2841 harness root-fix (failure pre-existed,
provider-class noise was hiding it).

## Fix

Add a CSP header in `web/next.config.ts` `headers()` satisfying exactly
the 4 existing pins — derive the gateway origins from the same
`NEXT_PUBLIC_GATEWAY_URL` config the SPA fetchers use (no hardcoded
duplicate origin string; grep the existing env helper first). Do NOT
weaken the tests to pass. Keep the policy minimal-but-real: this is the
public-deploy perimeter (DEC-032 deployment readiness), not a checkbox —
if a directive can't be honest yet (e.g. unsafe-inline needed by styled-jsx
in dev), scope it per-environment and say so in the PR body.

## May touch

- `web/next.config.ts` (+ any existing env/origin helper it imports)
- `web/tests/next-config-csp.test.ts` ONLY to extend, never to weaken

## Acceptance / verifier

1. `npx vitest run tests/next-config-csp.test.ts` — 4/4 green.
2. SPA still works against the live gateway in dev (manual or existing
   smoke: signin page loads, one authenticated fetch succeeds — no
   CSP-blocked console errors).
3. `tsc --noEmit` clean.

~20–60 LOC. **🟠 P2 — WS5 perimeter hardening with a pre-existing
4-test verifier.**
