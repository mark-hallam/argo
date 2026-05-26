# fix-spa-root-redirect-unauth-to-landing

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-root-redirect-unauth-to-landing`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md` F02 [P1].

🟠 **P1 — Visiting `http://localhost:3000/` (or any deploy root) lands on the in-app Approvals page with no auth gate and no redirect to /landing. A fresh visitor or investor pasting the bare URL sees in-app chrome with "Your AI teammates need a yes or no. · Loading approvals…" — not the marketing hero.**

## Root cause

`web/app/(app)/page.tsx` is the implicit owner of route `/` because the `(app)` route group has no path segment. There is no top-level `web/app/page.tsx` and no middleware rule routing unauthenticated `/` traffic to `/landing`.

`web/middleware.ts:39-55` handles `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` and `NEXT_PUBLIC_SKYCOT_DEMO_MODE` branches but does not guard the bare root. The marketing nav explicitly points investors at `/landing`, but the deployment URL (e.g. `skycot.com` or `localhost:3000`) lands on in-app chrome instead.

Curl evidence:
```
curl -s http://localhost:62927/ | grep '<title>' → "Approvals · SKYCOT"
curl -s http://localhost:62927/ | grep '<h1'      → "Your AI teammates need a yes or no."
```

## Findings this brief closes

- whole-site-mvp-claims-coverage F02 [P1] — Root URL `/` lands on Approvals with no auth gate

## Fix recipe

1. **Add a middleware rule** in `web/middleware.ts` that redirects unauthenticated `/` traffic to `/landing`:
   ```ts
   if (request.nextUrl.pathname === "/") {
     const session = readSessionCookie(request);
     if (!session) {
       return NextResponse.redirect(new URL("/landing", request.url));
     }
   }
   ```
   Reuse the existing session-cookie reader pattern from the auth-bypass branch.

2. **Authenticated users keep landing on Approvals** — that's the canonical in-app home. The redirect ONLY fires when no session cookie is present.

3. **Demo-mode behavior**: when `NEXT_PUBLIC_SKYCOT_DEMO_MODE === "1"` AND the operator has minted a demo session via `/auth/signup` → the cookie IS present → no redirect, Approvals renders. When demo mode is on but no mint has happened, redirect to `/landing` (so the demo URL is still useful for prospects pasting the link).

4. **Auth-bypass exception**: when `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"` (dev/CI), suppress the redirect and let Approvals render — preserves the existing dev workflow.

5. **Edge case**: a user signed out via `/auth/signout` should land on `/landing`, not stay on `/`. Sign-out flow already routes there (verify); this brief just ensures someone who navigates back to `/` after sign-out is bounced.

6. **Tests**: `web/tests/middleware-root-redirect.test.ts`:
   - No cookie, no demo, no bypass → `/` → 307 to `/landing`.
   - Cookie present → `/` → Approvals renders.
   - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → `/` → Approvals (no redirect).
   - Demo-mode without mint → `/` → `/landing`.

## Acceptance criteria

- [ ] `curl -s -o /dev/null -w "%{http_code} %{redirect_url}" http://localhost:3000/` returns `307 .../landing` when no session cookie.
- [ ] Authenticated session at `/` still renders Approvals (no regression).
- [ ] `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` keeps the dev experience unchanged.
- [ ] Investor pasting `skycot.com` lands on /landing hero, not "Loading approvals…".
- [ ] Middleware test covers all 4 branches.

## May-touch

- `web/middleware.ts` (add root-path branch)
- `web/tests/middleware-root-redirect.test.ts` (NEW)

## Out of scope

- Marketing nav copy changes (separate)
- Landing page hero design (locked)
- Renaming `/` to something more semantic (would break TanStack Query cache keys, deferred)

## Effort

~30-60 LOC in middleware + 1 test file. ~1-2 hours.
