# fix-browser-hire-csrf-after-gateway-restart

**Owner**: claude or codex — perimeter/CSRF; conductor review before merge.
**Depends on**: none (middleware_csrf + routes_auth_csrf already shipped).
**Authored against**: origin/main `5e73d548b` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-browser-hire-csrf-after-gateway-restart

## Problem (found live in the 2026-06-11 E2E stranger walkthrough)

`POST /api/v1/workers` (the Genesis hire commit — a browser cookie-auth request
gated by the CSRF double-submit middleware) returns **403** after the gateway is
rebuilt mid-session, while `run-now` (same-origin Next proxy, bearer-auth,
CSRF-exempt) keeps working. The FIRST hire of the session succeeded; it degrades
only after a gateway bounce. Two contributing factors observed:
1. The SPA's in-memory CSRF token can fall out of sync with the `__Host-csrf`
   cookie after a gateway restart (the token is fetched once; nothing
   re-syncs it), so the double-submit compare fails → 403.
2. The `__Host-csrf` cookie's `__Host-` prefix requires `Secure`+HTTPS and does
   not round-trip on `http://localhost` in some browsers, so on a local-dev HTTP
   origin the cookie may be absent for the compare.

A real stranger on an HTTPS deployment does not restart the gateway mid-session,
so this is not a production show-stopper, but the re-sync gap is worth
hardening so a transient 403 self-heals instead of dead-ending the hire.
Evidence: `docs/skycot/audit/2026-06-11-e2e-stranger-walkthrough.md`
("Environmental blockers" + the day-1 magic-link P0 sibling).

## Goal

A CSRF 403 on a mutating request self-heals: the SPA re-fetches a fresh token
from `GET /api/v1/auth/csrf` and retries once, transparently. And the local-dev
HTTP path has a working CSRF cookie (or a documented dev fallback) so hire works
on `http://localhost`.

## Scope / approach

1. **SPA auto-recover.** In the shared API client (the CSRF double-submit
   helper in `web/lib/api/client.ts` — it already warns when the token is
   missing), on a `403 csrf_missing|csrf_mismatch` response, re-`GET
   /api/v1/auth/csrf`, update the cached token, and retry the original request
   ONCE. Do not loop. Covers the post-restart desync for hire and every other
   mutating call.
2. **Local-dev cookie.** Confirm whether `__Host-csrf` is set on
   `http://localhost` (it is not, in Chrome). If not, in non-production /
   dev-origin mode emit the CSRF cookie WITHOUT the `__Host-` prefix (e.g.
   `csrf` with `SameSite=Lax`) so the double-submit compare has a cookie to
   read locally; keep `__Host-csrf` + Secure on HTTPS. Gate strictly on
   dev/non-production so the strict prod cookie is unchanged.
3. Honest error if the retry still fails (don't silently swallow).

## Acceptance

- A mutating request that 403s with `csrf_missing`/`csrf_mismatch` retries once
  after a fresh token fetch and succeeds (vitest on the client helper, mocking
  the 403-then-200).
- On `http://localhost` (dev), a hire POST carries a readable CSRF cookie and
  succeeds; on HTTPS/prod the `__Host-csrf` + Secure cookie is unchanged
  (gateway test pins the prod cookie attributes).
- No weakening of the prod CSRF posture (the strict `__Host-` + Secure cookie
  and the double-submit compare stay intact in production mode).

## Parallel-surface check

`rg 'csrf|CSRF' crates/argo-cli/src/handlers/gateway/middleware_csrf.rs
routes_auth_csrf.rs web/lib/api/client.ts` — extend the existing helper +
middleware; do not add a parallel CSRF path.
