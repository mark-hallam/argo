# fix-spa-theme-picker-only-on-approvals-not-settings

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-theme-picker-only-on-approvals-not-settings`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F13 [P1].

🟠 **P1 — `<ThemePicker />` is only rendered on `/` (approvals queue). It is NOT in `/settings/workspace` or any settings sub-tab, and the AppShell sidebar/header doesn't expose mode toggling. Operators wanting to switch theme must navigate to /approvals.**

## Root cause

`grep -rn '<ThemePicker' web/` returns a single match at `web/app/(app)/page.tsx:360`. Per global instructions and memory `project_multi_tenant_theming`, multi-tenant theming and light/dark are first-class. The locked Settings design memory (`project_settings_page_design_locked`) calls workspace the natural home for theme.

## Code evidence

```bash
$ grep -rn '<ThemePicker' ~/code/argo/web/
~/code/argo/web/app/(app)/page.tsx:360:  <ThemePicker />
```
(only one hit)

## Findings this brief closes

- whole-site-mvp-claims-coverage F13 [P1] — Theme picker only on /approvals, not in Settings or AppShell

## Fix recipe

1. **Render `<ThemePicker />` in `/settings/workspace`** (`web/app/(app)/settings/workspace/page.tsx`). Place under a "Appearance" section card alongside any workspace-level visual preferences.
2. **Optionally add a compact theme-toggle to the AppShell header / user-menu drawer** for one-click access. Reuse the same `ThemePicker` component or a smaller `<ThemeToggle />` variant.
3. **Remove the duplicate `<ThemePicker />` on `/`** OR keep it (if the operator prefers theme-toggle visible on the home page). Operator preference defaults to removing the duplicate to keep `/` focused.
4. **Verify persistence**: theme choice should persist across page navigations and SSR hydration. If not already wired, hook into the existing theme-storage key (per memory `feedback_spa-theme-toggle-broken-storage-key-mismatch` and existing `fix-spa-theme-toggle-broken-storage-key-mismatch` brief — this brief depends on that one).
5. **Web test** asserts the theme picker on `/settings/workspace` toggles theme and persists across reload.

## Acceptance criteria

- [ ] `<ThemePicker />` renders in `/settings/workspace`.
- [ ] Theme toggle is reachable from at least one always-visible chrome surface (AppShell header or user-menu).
- [ ] Theme persists across navigation + reload.
- [ ] No regression on the existing toggle on `/` (or it's removed deliberately).
- [ ] Web test covers theme picker on settings.

## May-touch

- `web/app/(app)/settings/workspace/page.tsx` (add picker)
- `web/components/skycot/AppShellHeader.tsx` (small toggle, if desired)
- `web/app/(app)/page.tsx:360` (remove or keep)
- `web/tests/settings-theme-picker.test.ts` (NEW)

## Out of scope

- Multi-tenant white-label theming (separate brief: not yet authored — see `project_multi_tenant_theming` memory)
- Storage-key mismatch (separate brief: `fix-spa-theme-toggle-broken-storage-key-mismatch` — this brief depends on it landing first)
- Brand reference page tokens (separate brief: `fix-brand-reference-page-d3-consistency-pass`)

## Effort

~50-80 LOC + 1 test. ~1-2 hours.
