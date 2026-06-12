# web-route-gating-posture-logged-out

**Owner**: claude or codex — **P2 PERIMETER UX (shakedown F4+F5).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-route-gating-posture-logged-out

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem
Logged-out: `/` renders the app shell as Guest (empty Approvals) while
/skills, /billing etc. redirect to signin, and /team, /audit, /genesis render
as Guest. Inconsistent; a stranger at the root sees an empty dashboard instead
of marketing.

## Fix
One rule: logged-out + no demo session -> app routes redirect to /auth/signin;
`/` serves the marketing landing (or redirects to /landing) for logged-out
visitors; demo session opens the full app surface uniformly. Respect the
production-auth mode seam (#2874) — dev auth-bypass may keep the open lane,
but the PRODUCTION posture must be the consistent one and tested.

## Verify
Route tests: logged-out / -> marketing; logged-out app routes -> signin
(production mode); demo session -> app accessible. No regression to the demo
journey.
