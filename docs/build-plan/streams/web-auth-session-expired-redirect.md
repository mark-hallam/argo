# web-auth-session-expired-redirect

**Owner**: claude.
**Depends on**: `web-auth-logout-button-wire` MERGED (✅ via #2015).
**Authored against**: origin/main `b1f23a73e` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-auth-session-expired-redirect`.

🟢 **P3 AUTH UX POLISH.** When a SKYCOT session JWT expires mid-use, the user currently sees a generic 401 error toast and is stranded on the current page. They have to manually navigate to `/signin`, log in, and then re-navigate to where they were. Surface a session-expired redirect that takes them to `/signin?returnTo=<their current path>`, then bounces them back after re-auth.

## Scope

The SPA already has a gateway API client wrapper (likely `web/lib/api/client.ts` or similar — locate before claiming) that handles auth headers + 401 responses. Extend the 401 handler so when the response indicates expired-session (vs. genuinely-unauthorized), the SPA:

1. Captures the current `window.location.pathname + search` as `returnTo`.
2. Redirects to `/signin?returnTo=<encoded>`.
3. After successful re-auth (magic-link verify or whatever auth path exists), if `returnTo` is present and points to an `/app/*` route inside the SPA (NOT external — security check), redirect there.
4. Falls back to `/` if `returnTo` is missing, invalid, or external.

Add a small flash banner on the destination after re-auth: "Welcome back — picking up where you left off." Disappears after 4s.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/lib/api/client.ts` (or equivalent — gateway API wrapper) — extend 401 handler to redirect with `returnTo`.
- `web/app/(auth)/signin/page.tsx` (or wherever the signin UI lives) — narrowly scoped: ONLY if needed to capture/pass `returnTo` through the magic-link verify flow.
- `web/app/(auth)/verify/page.tsx` (or equivalent) — narrowly scoped: post-verify redirect honors `returnTo`.
- `NEW: web/lib/auth/return-to.ts` — small helper: `parseReturnTo(searchParams)` + `safeReturnToOrHome(returnTo)`. Returns `'/'` if external, missing, or doesn't start with `/app/` or whitelisted route.
- `NEW: web/__tests__/auth/return-to.test.ts` — unit tests for `safeReturnToOrHome` (whitelist enforcement is load-bearing for security).
- `web/components/ui/welcome-back-banner.tsx` — narrowly scoped: NEW small banner component if no equivalent exists.

## Files you must NOT touch

- `web/auth.config.ts` — locked from `web-auth-config-server-only-split` (also CLAIMED currently; coordinate via branch rebases if both land same week).
- `web/auth.ts` — same lock as above.
- Any gateway crate.
- The magic-link signup flow — out of scope.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] On a 401-expired response from the gateway, SPA captures the current pathname+search as `returnTo` and redirects to `/signin?returnTo=<encoded>`.
- [ ] After a successful magic-link verify, if `returnTo` is set AND is internal (`/app/*` or whitelist), redirect there.
- [ ] If `returnTo` is external (e.g. `https://evil.com/`), or doesn't start with `/`, redirect to `/` instead. Test covers this.
- [ ] `safeReturnToOrHome` rejects: protocol-relative URLs (`//evil.com`), absolute URLs (`https://...`), `javascript:`, `data:`, and any path NOT in the whitelist.
- [ ] "Welcome back — picking up where you left off." banner appears on destination, auto-dismisses after 4s.
- [ ] No regression on first-time signin: fresh `/signin` without `returnTo` still redirects to `/` post-verify.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm lint` clean.
- [ ] `pnpm test --run` passes (incl. new return-to unit tests).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-auth-session-expired-redirect`
- PR title: `web(auth): session-expired returnTo redirect (web-auth-session-expired-redirect)`

## Notes for the implementer

- ~200-300 LOC expected. The `safeReturnToOrHome` helper is the load-bearing safety boundary; if you skimp on tests there, an attacker can craft a malicious `returnTo` that bounces the user to a phishing page after re-auth.
- The whitelist for "internal" should be: `/app/`, `/workflows/`, `/audit/`, `/cohorts/`, `/approvals/`, `/skills/`, `/settings/`, `/onboarding/`. Anything outside the whitelist → `'/'`.
- The 401 detection should be precise — only redirect on session-EXPIRED. A genuine RBAC denial (operator lacks a capability) should NOT redirect to signin; it should surface the existing permission-denied banner.
- The "Welcome back" banner can use the existing toast / banner component if one exists; don't invent a new one if there's a sibling.
- If the magic-link verify flow is currently a single page that auto-redirects to `/`, you'll need to thread `returnTo` through: `/signin?returnTo=X` → magic link with `returnTo=X` → verify page reads `returnTo` → redirect after success.
