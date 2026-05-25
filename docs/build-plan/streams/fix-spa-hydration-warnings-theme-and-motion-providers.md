# fix-spa-hydration-warnings-theme-and-motion-providers

**Owner**: claude.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-hydration-warnings-theme-and-motion-providers`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F20 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟡 **P1 — Console error harvest shows React hydration warnings on multiple app-shell pages, not just `/settings/members`. Root cause: client-only providers (next-themes, motion, sonner) emit different markup SSR vs CSR.**

## Root cause

The 66 KB console-error transcript from the audit shows hydration warnings appear on multiple pages — `/settings/members` is just the worst case because it also uses `styled-jsx`. The cross-cutting source is the provider stack in `web/app/layout.tsx`:

- `next-themes` `ThemeProvider` resolves theme client-side from `localStorage`; on the server it doesn't know the user preference, so SSR markup may render with no class (or `system` default) while CSR mounts with `dark` or `light`.
- `framer-motion` / `motion` `<MotionConfig reducedMotion="user">` reads `matchMedia` on the client — different SSR.
- `sonner` `<Toaster />` uses Portal + random keys for toast instances — SSR-CSR drift.

Without `suppressHydrationWarning` on the right elements, React logs warnings on every page load. The visible damage on `/settings/members` is brutal (styled-jsx fails to apply); on other pages it's a console-only issue that creates a "broken app" smell when a buyer opens DevTools.

## Findings this brief closes

- whole-site-mvp-claims-coverage F20 [P1] — Hydration warnings on multiple app-shell pages (not just settings/members)

## Fix recipe

1. **Audit `web/app/layout.tsx`** — walk every provider in the root tree. For each provider that reads client-only state on first render, add `suppressHydrationWarning` to the wrapping element OR rework the provider to not render different markup SSR vs CSR.

2. **Specific changes**:
   - `<html>` and `<body>` elements: add `suppressHydrationWarning` (standard next-themes pattern).
   - `<ThemeProvider>`: ensure `enableSystem={true}` + `defaultTheme="system"` matches what the FOUC script outputs.
   - `<MotionConfig>`: wrap in `'use client'` boundary OR add a `mounted` guard.
   - `<Toaster />`: confirm portal mounts client-only — verify it's behind a mount guard.

3. **Pin with a console-error harvest test**. Author `web/tests/no-hydration-warnings.spec.ts`:
   - For each canonical app-shell page (`/`, `/approvals`, `/audit`, `/team`, `/workers`, `/workflows`, `/settings`), navigate via Playwright/preview, attach a console listener, assert no message matches `/hydrat(ed|ion)/i`.

4. **Cross-check `feedback_styled_jsx_requires_use_client` memory**. The styled-jsx blocks rely on `'use client'` boundaries that the audit may be tripping. Confirm every page using `<style jsx>` has `'use client'` at the top.

## Acceptance criteria

- [ ] DevTools console shows zero hydration warnings on `/approvals`, `/audit`, `/team`, `/workers`, `/workflows`, `/settings`, `/settings/members` on first navigation.
- [ ] `no-hydration-warnings.spec.ts` passes locally.
- [ ] Theme toggle still works (no regression from `fix-spa-theme-toggle-broken-storage-key-mismatch`).
- [ ] FOUC behavior unchanged — no theme flash on reload.

## May-touch

- `web/app/layout.tsx` (provider tree + suppressHydrationWarning)
- `web/components/skycot/ThemeProvider.tsx` (if wrapper exists)
- `web/components/skycot/MotionProvider.tsx` (if wrapper exists)
- `web/tests/no-hydration-warnings.spec.ts` (NEW)

## Out of scope

- The specific `/settings/members` styled-jsx fix (separate brief `fix-settings-members-hydration-mismatch-unstyled`)
- The theme toggle bug (separate brief `fix-spa-theme-toggle-broken-storage-key-mismatch`)
- Refactoring providers to RSC-only

## Effort

~60-150 LOC + 1 Playwright spec. ~2-3 hours. P1 because it's cross-cutting and a buyer opening DevTools sees noise even if pages render.
