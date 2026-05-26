# fix-playwright-auth-logout-and-signup-centering-cluster

**Owner**: codex.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-playwright-auth-logout-and-signup-centering-cluster`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` Playwright failure cluster: Auth (4 specs).

🟠 **P1 — 4 auth-related Playwright specs failing: `auth-logout` × 3 + `auth-signup-centering` × 1. The logout flow is broken in 3 ways; the signup-page hero centering has drifted from the locked design.**

## Root cause

Two distinct sub-clusters under the auth umbrella:

### Sub-cluster A: `auth-logout` × 3

Likely covers: (a) the logout button is missing or not clickable on some surface, (b) the post-logout redirect goes to the wrong destination (was `/landing` per memory `fix-spa-root-redirect-unauth-to-landing`, may have regressed), (c) session cookie is not properly cleared. Need to read the actual run.log to confirm.

### Sub-cluster B: `auth-signup-centering`

Per memory `project_auth_pages_design_locked` (2026-05-19): "Auth pages design locked — A editorial-center across signup / verify / signin (matches Onboarding F1)". The signup page hero may have drifted from the centered layout, either because of a recent grid/flex change or a missing CSS token. Existing brief `fix-auth-signup-desktop-hero-off-center` was authored for an earlier wave; this regression may be that same brief recurring or a separate sibling.

## Findings this brief closes

- chrome-live-walk-and-playwright Playwright cluster: Auth (4 specs)
  - `auth-logout` × 3
  - `auth-signup-centering` × 1

## Fix recipe

1. **Read the playwright run.log** at `docs/skycot/ux-audits/2026-05-26-2007/playwright/run.log` and identify each failure's actual error.

2. **For `auth-logout` × 3**:
   - Locate the logout handler at `web/lib/auth/use-logout.ts` (or equivalent)
   - Locate the logout button(s): likely in `web/components/skycot/AppShell` user-menu + possibly `/settings/account`
   - Verify the logout flow: click → session cookie cleared → redirect to `/landing`
   - Fix whichever step is broken; ensure all 3 specs (different surfaces or different assertions) pass

3. **For `auth-signup-centering`**:
   - Compare current `/auth/signup` rendering against the locked A-editorial-center mockup at `docs/skycot/design-explorations/2026-05-19/auth/option-A.html` (if it exists)
   - Identify the layout drift: probably `max-width` / `margin: auto` / grid placement
   - Fix in `web/app/auth/signup/page.tsx` or the shared `AuthShell` component

4. **Re-run the cluster**: `pnpm playwright test "auth-"` should be 0 failed.

5. **Add a snapshot test** for the auth pages' hero positioning so a future grid regression catches itself.

## Acceptance criteria

- [ ] `pnpm playwright test "auth-logout"` passes 0 failed (all 3)
- [ ] `pnpm playwright test "auth-signup-centering"` passes
- [ ] Logout from any in-app surface redirects to `/landing` and clears the session cookie
- [ ] `/auth/signup` hero is horizontally centered at all viewports ≥ 1024px (matches locked design)

## May-touch

- `web/lib/auth/use-logout.ts` (or `web/app/auth/logout/route.ts` if it's a route handler)
- `web/components/skycot/AppShell/UserMenu.tsx` (or equivalent)
- `web/app/auth/signup/page.tsx` + `web/components/skycot/auth/AuthShell.tsx`
- `web/playwright/auth/auth-logout-*.spec.ts`
- `web/playwright/auth/auth-signup-centering.spec.ts`

## Out of scope

- Magic-link signup flow (already shipped per memory)
- Other auth surfaces (signin / verify) unless the logout regression touches them
- Multi-tenant theming on auth pages (separate concern)

## Effort

~50-150 LOC across handlers + components + specs. ~3-5 hours. P1 because broken logout is a credibility-killer on any extended demo session.
