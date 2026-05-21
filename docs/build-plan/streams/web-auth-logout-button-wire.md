# web-auth-logout-button-wire

**Authored against**: `origin/main` `1617b41ea` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-auth-logout-button-wire`.

## Summary

#1968 just landed the auth-revocation infrastructure: `revoked_tokens` table + `middleware_session_verify` + `routes_auth_logout` endpoint. The backend works. Now wire the SPA's "Sign out" button to call the new endpoint so a logged-out session is truly revoked (not just cookie-cleared client-side).

Pre-condition: **#1968 MERGED ✅** (logout endpoint exists).

## Acceptance

- [ ] The "Sign out" button (likely in `web/components/skycot/AppShell.tsx` or a user-menu component — verify) calls `POST /api/v1/auth/logout` with the current session token.
- [ ] On success: clear local cookies, redirect to `/signin`, success toast "Signed out".
- [ ] On 401 (token already revoked): treat as success, redirect to `/signin`.
- [ ] On 5xx: red toast "Couldn't sign out — please refresh and try again", do NOT clear cookies (let user retry).
- [ ] Verify the post-logout session: opening another tab + trying any authenticated API call returns 401 (proves revocation worked end-to-end).
- [ ] At least one Playwright E2E: sign in → click sign out → assert /signin redirect + token rejected on subsequent API call.

## May touch

- `web/components/skycot/AppShell.tsx` (or wherever the sign-out button lives)
- `web/lib/api/auth.ts` (extend with `signOut` function)
- Playwright spec for the logout flow

## Must NOT touch

- The middleware verification (#1968 settled).
- The `revoked_tokens` table or migration (#1968 settled).
- Any non-auth surface.

## Notes

- Sizing: 100–200 LOC.
- Pre-flight: `rg -ln 'Sign out|Logout|logout' web/components/` to find the canonical button location.
- This is the visible payoff of #1968's hardening — without this wire-up, the new revocation infra has no end-to-end proof.
