# fix-spa-theme-toggle-broken-storage-key-mismatch

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-theme-toggle-broken-storage-key-mismatch`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F1 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🔴 **P0 — SPA-wide theme toggle is broken. The whole app is permanently dark regardless of the operator's preference. Customer-visible regression, not just a demo-blocker.**

## Root cause

Clicking the sidebar `[data-testid=appshell-theme-toggle]` button writes `theme=dark` (the *current* value, not the next) to `localStorage`, never flips the `<html>` class, and never changes the visible theme. Verified across 3 consecutive clicks on `/approvals` at 1440 viewport — class never flips, `aria-label` stays "Switch to light theme".

`web/components/skycot/ThemeToggle.tsx:29` calls `setTheme("light")` correctly when in dark mode. The breakage is downstream — either:
- (a) The `next-themes` `ThemeProvider` is configured with `attribute="class"` + `value={{light:"skycot-light", dark:"skycot-dark"}}` but the FOUC-prevention (Flash Of Unstyled Content — script in `<head>` that sets the theme class before React mounts) inline `<script>` in the root layout writes back `"dark"` before React commits, OR
- (b) The `storageKey` the FOUC script reads is different from what `next-themes` writes to (e.g. one reads `theme`, the other writes `skycot-theme`).

The "writes wrong value" symptom is the giveaway — `setTheme("light")` going to `localStorage.theme === "dark"` means the local store is being overwritten by a different code path on the same render tick.

## Findings this brief closes

- whole-site-mvp-claims-coverage F1 [P0] — Theme toggle never switches modes; persists wrong value across SPA

## Fix recipe

1. **Audit `web/app/layout.tsx`** for the FOUC inline `<script>` block. Confirm its `storageKey` and the attribute it writes to `<html>` match `next-themes`'s `ThemeProvider` props. The two MUST be aligned:
   - `storageKey: "theme"` (or whatever — pick one)
   - `attribute: "class"` (or `data-theme` — pick one)
   - `value: { light: "skycot-light", dark: "skycot-dark" }` if using custom class values, MUST match the FOUC script's branching

2. **Refactor the FOUC script** to read from the same key + use the same value mapping as the ThemeProvider. If keeping a custom inline FOUC, the script should:
   - Read `localStorage.getItem("theme")` (or whatever key)
   - Map `"light"` → add class `skycot-light`, remove `skycot-dark`
   - Map `"dark"` → add `skycot-dark`, remove `skycot-light`
   - Default to system preference if no stored value

3. **Add a Playwright/Vitest test** at `web/tests/theme-toggle.spec.ts`:
   - Mount app shell, observe initial `<html>` class
   - Click theme toggle
   - Assert `<html>` class flipped + `localStorage.theme` matches the new theme
   - Click again, assert flip-back

4. **Manual smoke**: open `/approvals`, click moon icon → page flips to light theme + `<html class>` removes `dark`, `localStorage.theme === "light"`.

## Acceptance criteria

- [ ] Single click of the theme toggle changes the visible theme (light ↔ dark).
- [ ] `localStorage.theme` after the click equals the NEW theme (not the previous one).
- [ ] `<html>` class attribute reflects the new theme.
- [ ] `aria-label` updates to "Switch to <other> theme".
- [ ] Reload preserves the toggled theme (no FOUC flicker back to default).
- [ ] New `theme-toggle.spec.ts` passes locally.

## May-touch

- `web/app/layout.tsx` (FOUC script + ThemeProvider props)
- `web/components/skycot/ThemeToggle.tsx` (only if `setTheme` call is wrong, unlikely)
- `web/components/skycot/ThemeProvider.tsx` (if it exists as a wrapper)
- `web/tests/theme-toggle.spec.ts` (NEW)

## Out of scope

- Per-tenant white-label theming (separate roadmap item per `project_multi_tenant_theming` memory)
- Marketing page theme drift (separate brief `fix-marketing-pages-theme-drift-landing-vs-compare`)
- Hydration warnings broader than theme provider (separate brief `fix-spa-hydration-warnings-theme-and-motion-providers`)

## Effort

~40-100 LOC (mostly debugging the FOUC + ThemeProvider mismatch) + 1 spec test. ~1-2 hours. Demo-recording-blocker AND customer-visible regression — ship soon.
