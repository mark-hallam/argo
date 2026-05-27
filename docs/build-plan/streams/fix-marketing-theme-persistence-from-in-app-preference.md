# fix-marketing-theme-persistence-from-in-app-preference

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-marketing-theme-persistence-from-in-app-preference`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F07)

🟡 **P2 — A user who toggles to dark in-app, then navigates to `/landing` / `/pricing` / `/customers`, sees the marketing pages render in light regardless. Coming back through onboarding the app flips back to dark unannounced. The experience is jarring for a returning user, and the marketing/in-app theme mismatch is the v1 footgun ahead of the larger per-tenant white-label work.**

## Root cause

`web/components/marketing/MarketingNav.tsx` does not import `ThemeToggle` and the marketing layout's root `<html>` doesn't read from the theme cookie that `AppShell` persists. The marketing pages have a deliberate brand-light register so this is not "broken", but the lack of bridge between in-app and marketing means:

1. A user who chose dark in-app sees marketing light.
2. Returning to in-app, dark snaps back without warning.

Memory `project_multi_tenant_theming.md` calls for per-tenant white-label themes — that's a separate larger scope. This brief is the v1 footgun fix: respect the user's existing in-app theme preference when rendering marketing pages.

## Findings this brief closes

- whole-site F07 [P2] — Marketing pages don't carry over user's in-app theme preference

## Fix recipe

Two paths; ship Path A as the minimum, consider Path B for the next iteration.

**Path A (minimum) — respect the in-app theme cookie when rendering marketing layouts.**

1. **Read the theme cookie in the marketing layout's server component**. The cookie is `__Host-skycot-theme` or similar (verify by reading `AppShell` for the cookie name). In `web/app/layout.tsx` (or marketing-layout-specific equivalent), set the `<html class={theme}>` based on the cookie.
2. **Keep the marketing default = light** when no cookie is set. New visitors hit the brand-light register.
3. **Persist the preference cross-section** — toggle in-app, navigate to /landing, see the same theme. Navigate /landing → /onboarding, the in-app picks up the same theme.

**Path B (next iteration) — add a small theme toggle to MarketingNav.**

4. **Add a 24×24 sun/moon toggle** at the right edge of `MarketingNav.tsx` so the user can switch theme on the marketing pages directly. Persists the same cookie.
5. **A11y**: `aria-label="Toggle theme"`, keyboard-focusable, announces state.

**Tests:**

6. **vitest** at `web/components/marketing/MarketingNav.test.tsx` (NEW or extend):
   - Render with cookie=`dark`; assert html class includes `dark`.
   - Render with cookie unset; assert html class is `light`.
7. **Playwright** at `web/playwright/marketing/theme-cross-section.spec.ts`:
   - Sign in, toggle dark, navigate to `/landing`, assert dark theme rendered.

## Acceptance criteria

- [ ] Marketing pages respect the in-app theme cookie when set.
- [ ] New visitors (no cookie) get the marketing-default light theme.
- [ ] Cross-section navigation does not cause a theme flash (no light→dark→light flicker).
- [ ] vitest + playwright cover the cookie-respect path.
- [ ] (Path B only): MarketingNav renders a theme toggle with proper a11y.
- [ ] PR body cites whole-site F07 and notes the link to `project_multi_tenant_theming.md` (out of scope).

## May-touch

- `web/app/layout.tsx` (or marketing-specific root layout)
- `web/components/marketing/MarketingNav.tsx` (Path B)
- `web/components/skycot/ThemeToggle.tsx` (reuse, may need export)
- `web/lib/theme.ts` (cookie name helper if not centralised)
- `web/components/marketing/MarketingNav.test.tsx` (NEW or extend)
- `web/playwright/marketing/theme-cross-section.spec.ts` (NEW)

## Out of scope

- Per-tenant white-label themes (covered by `project_multi_tenant_theming.md`, separate larger scope)
- In-app theme picker quirks (covered by `fix-spa-theme-picker-only-on-approvals-not-settings`)
- Brand-divergence (gradient on light) acceptable per memory `project_audit_page_design_locked`

## Effort

S — ~30 LOC layout read + ~50 LOC tests + (Path B) ~30 LOC toggle + a11y. ~2-3 hours including playwright.
