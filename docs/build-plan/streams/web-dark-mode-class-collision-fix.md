# web-dark-mode-class-collision-fix

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `7ce67c46d` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-dark-mode-class-collision-fix`.

🟡 **P2 BRAND/UX — html element has BOTH `light` AND `dark` classes simultaneously, no theme toggle.** Source: evaluator round-6 SHARED.md handoff 2026-05-23T05:30:00Z (Chrome MCP comprehensive E2E campaign).

> "web-dark-mode-class-collision-fix — html has BOTH light AND dark classes simultaneously + no theme toggle"

Context: per operator memory `project_multi_tenant_theming.md` the SPA needs light + dark + per-tenant white-label themes via CSS custom properties. The current state has both classes co-existing on `<html>` (probably the result of an SSR + client hydration race or a leftover from the D3 brand pass). The dark-mode CSS variables silently override the light-mode ones, producing an inconsistent visual state. **No user-facing theme toggle exists at all.**

## Scope

Two coordinated changes:

### 1. Fix the class collision

Find where `<html>` is rendered with classes — most likely `web/app/layout.tsx` or `web/components/skycot/AppShell.tsx`. Audit for hardcoded `className="light dark"` or string-concat that produces both. The fix is either:
- **Default to `light` only on SSR** (matches operator memory: light is the base) + use `next-themes` (already a workspace dep — check `web/package.json`) for client-side toggle.
- **Default to `dark` only on the marketing surface, `light` on the SPA** if that's the intended split.

Run Chrome MCP after the fix and confirm `document.documentElement.classList` returns EITHER `light` OR `dark` — NEVER both.

### 2. Add a theme toggle

Add a small toggle button to `AppShell.tsx` (next to the UserPill) — sun/moon icon + accessible label, persists choice via `next-themes`'s default `localStorage` mechanism. Match the brand-consistent register per memory `project_brand_consistency_pass.md` (D3 design system).

### Regression test

`web/__tests__/theme/no-class-collision.test.tsx`:
- Renders `<RootLayout><div /></RootLayout>` via test seam.
- Asserts the rendered `<html>` has exactly one of `light` / `dark`.
- Asserts theme toggle changes the class on click.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/app/layout.tsx` — narrowly scoped: html className.
- `web/components/skycot/AppShell.tsx` (or wherever toggle lands) — narrowly scoped: add toggle.
- `web/components/skycot/ThemeToggle.tsx` — NEW: ~40 LOC.
- `web/__tests__/theme/no-class-collision.test.tsx` — NEW vitest.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Marketing pages (`marketing/`) — separate app, separate theme handling.
- Global CSS variable definitions — those should already exist per the D3 brand pass.
- Per-tenant theming infrastructure — out of scope for this brief.

## Acceptance criteria

- [ ] `<html>` element has EXACTLY ONE of `light` / `dark` class at any moment.
- [ ] Theme toggle button is keyboard-accessible (Tab + Enter) and respects `prefers-color-scheme` on first mount.
- [ ] Theme choice persists across page navigations.
- [ ] Regression test passes.
- [ ] `pnpm typecheck` + `pnpm lint` + vitest clean.
- [ ] No styled-jsx `<style jsx>` blocks introduced (per memory feedback_styled_jsx_requires_use_client.md — keep "use client" boundaries tight).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `web-dark-mode-class-collision-fix`
- PR title: `fix(web): resolve light+dark class collision on html + add theme toggle (web-dark-mode-class-collision-fix)`

## Notes for the implementer

- ~150-250 LOC expected. Class fix ~10, toggle component ~40, AppShell wire ~10, vitest ~70, light eslint guard optional.
- **Use `next-themes` if it's already a dep**. Don't roll a custom theme provider — there's a battle-tested library. Check `web/package.json` first; if absent, add it.
- **The operator memory says default to light for SPA + dark for "moments"** — match that. Don't flip to dark-by-default; the D3 brand pass intentionally landed light.
- **The toggle must be discoverable** — placement next to UserPill is the canonical spot. Don't hide it behind a settings menu — operators want it visible.
- Cite the evaluator round-6 report verbatim in the PR body.
