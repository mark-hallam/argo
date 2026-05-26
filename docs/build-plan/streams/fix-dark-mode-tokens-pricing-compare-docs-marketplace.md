# fix-dark-mode-tokens-pricing-compare-docs-marketplace

**Owner**: codex.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-dark-mode-tokens-pricing-compare-docs-marketplace`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` Playwright failure cluster: Dark-mode white-on-white (4 specs).

🟠 **P1 — Four pages render unreadable (white text on white background) in dark mode: `/pricing`, `/compare`, `/docs`, `/for/marketplace`. All 4 marketing/policy surfaces. The Playwright suite catches this with axe-style contrast assertions, all failing.**

## Root cause

These 4 pages were built with hardcoded `text-white` / `bg-white` Tailwind utilities or hex-literal colors instead of consuming the CSS custom-property tokens (`var(--ink)`, `var(--bg)`, `var(--bg-elevated)`, etc.) that the theme provider toggles on `[data-theme="dark"]`. So in dark mode the background flips dark but the text colors don't, OR the text flips dark but the background doesn't, producing white-on-white or near-zero contrast.

Per memory `project_multi_tenant_theming` (operator-locked):
> "All visual tokens as CSS custom properties." Hardcoded Tailwind color utilities are the bug pattern.

The 4 affected pages predate the D3 brand-consistency pass (memory `project_brand_consistency_pass` 2026-05-19) that swept the in-app surfaces. The marketing surfaces were last touched in different waves.

## Findings this brief closes

- chrome-live-walk-and-playwright Playwright cluster: Dark-mode white-on-white (4 specs)
  - `dark-mode: /pricing`
  - `dark-mode: /compare`
  - `dark-mode: /docs`
  - `dark-mode: /for/marketplace`

## Fix recipe

1. **Locate the offending utilities per page**:
   - `web/app/pricing/page.tsx` + any subcomponents in `web/app/pricing/`
   - `web/app/compare/page.tsx` + subcomponents
   - `web/app/docs/page.tsx` + `web/app/docs/**/*.tsx` (might be MDX)
   - `web/app/for/marketplace/page.tsx` + the shared `VerticalLandingShell` component

2. **For each page**, grep for the bug patterns:
   ```bash
   grep -nE 'bg-white|text-white|text-black|bg-black|#fff|#ffffff|#000|#000000' <page-file>
   ```
   Replace each match with a token-driven equivalent:
   - `bg-white` → `bg-[var(--bg)]` or `bg-surface`
   - `text-white` → `text-[var(--ink-inverse)]` or remove (let theme inherit)
   - `text-black` → `text-[var(--ink)]` or `text-ink`
   - hex literals → corresponding token var

3. **Use the existing token catalog.** Check `web/app/globals.css` or the theme provider for the full token vocabulary. Don't invent new tokens; use what's already declared.

4. **Verify in both themes** by opening each page with `?theme=dark` query param (assuming the theme provider supports it) and visually inspecting + running the contrast assertions.

5. **Update the failing Playwright specs** if their selectors need tweaking after the token swap. The specs likely already exercise both themes via a setup hook.

6. **Add a single comprehensive contrast spec** at `web/playwright/marketing/dark-mode-contrast-comprehensive.spec.ts` that asserts WCAG-AA contrast on all marketing pages in both themes. Replaces the 4 fragmented specs.

## Acceptance criteria

- [ ] `pnpm playwright test "dark-mode"` passes 0 failed
- [ ] `/pricing`, `/compare`, `/docs`, `/for/marketplace` all render with WCAG-AA contrast in BOTH light and dark themes
- [ ] Zero hardcoded `bg-white` / `text-white` / `bg-black` / `text-black` / hex literals on the 4 pages (lint rule or grep check)
- [ ] Visual screenshot regression: dark-mode screenshots for the 4 pages match an updated baseline

## May-touch

- `web/app/pricing/page.tsx` + subcomponents
- `web/app/compare/page.tsx` + subcomponents
- `web/app/docs/**/*.tsx` (or `.mdx`)
- `web/app/for/marketplace/page.tsx`
- `web/components/marketing/VerticalLandingShell.tsx` (if shared by `/for/*`)
- `web/playwright/marketing/dark-mode-contrast-comprehensive.spec.ts` (NEW)

## Out of scope

- The other `/for/*` pages if they pass (only `/for/marketplace` listed as failing); add to scope only if grep shows the same pattern there
- New token additions (use existing vocabulary)
- The theme picker itself (already shipped via PR #2368)

## Effort

~50-200 LOC of string replacements across 4 pages + 1 consolidated spec. ~3-5 hours. P1 because dark-mode unusable on 4 marketing pages reads as broken software to any prospect.
