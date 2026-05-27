# fix-settings-tabs-prefix-startswith-double-highlights-workspace

**Owner:** claude
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-settings-tabs-prefix-startswith-double-highlights-workspace`
**Source:** ux-audit-2026-05-27-2351

🟡 **P1 — Two tabs render with `aria-current="page"` simultaneously on every settings sub-page (the actual active tab + Workspace). Visible visual glitch + a11y breakage.**

## Root cause

`web/components/skycot/settings/SettingsTabs.tsx:110-112`:

```ts
function isActive(pathname: string, tab: SettingsTab): boolean {
  return tab.match.some((prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`));
}
```

The Workspace tab's `match` array is `["/settings", "/settings/workspace"]`. The `pathname.startsWith("/settings/")` clause matches **every** settings sub-route — `/settings/billing`, `/settings/security`, `/settings/data-export`, etc. all return `true` for Workspace. Result: TWO tabs render with `aria-current="page"` simultaneously on every settings sub-page (the actual active tab + Workspace).

> Visible in the screenshot — both "Workspace" and "Billing" highlighted at the same time on `/settings/billing`. Verified on `/settings/data-export` too.
> — `whole-site-mvp-claims-coverage.md` F17

This is a screen-reader and visual breakage at once: a11y rules require exactly one `aria-current="page"` per navigation; visually the locked D-top-tabs design (per `project_settings_page_design_locked`) intends one active tab indicator.

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F17 [P1] — settings tab "Workspace" is permanently highlighted because the active-tab matcher uses prefix-startsWith

## Fix recipe

1. **Change the Workspace tab's match** in `web/components/skycot/settings/SettingsTabs.tsx`:
   - Replace `["/settings", "/settings/workspace"]` with just `["/settings/workspace"]`.
   - Update root `/settings` redirect (`web/app/(app)/settings/page.tsx`) to 307 → `/settings/workspace` so the Workspace tab is the canonical landing.

2. **Or — special-case the root match** to require exact equality only:
   ```ts
   function isActive(pathname: string, tab: SettingsTab): boolean {
     return tab.match.some((prefix) =>
       pathname === prefix ||
       (prefix.endsWith("/") ? pathname.startsWith(prefix) : pathname.startsWith(`${prefix}/`))
     );
   }
   ```
   And drop the `/settings` entry from Workspace's match list.

3. **Component test** at `web/components/skycot/settings/SettingsTabs.test.tsx`:
   - Render with pathname `/settings/billing` → exactly one tab has `aria-current="page"` and it's the Billing tab.
   - Render with pathname `/settings/workspace` → exactly one tab has `aria-current="page"` and it's Workspace.
   - Render with pathname `/settings/data-export` → exactly one tab (Data export).
   - Render with pathname `/settings` → either redirected away or exactly one tab (Workspace).

4. **Playwright spec** at `web/playwright/settings/tabs-single-active.spec.ts`:
   - For each settings sub-page, assert exactly one `[aria-current="page"]` is present in the tabs list.

## Acceptance criteria

- [ ] Visiting any `/settings/*` sub-page highlights exactly one tab.
- [ ] Screen-reader announces a single current page.
- [ ] Component + Playwright tests pass cold.
- [ ] No regression on the locked D top-tabs visual design.
- [ ] PR body cites finding F17.

## May-touch

- `web/components/skycot/settings/SettingsTabs.tsx`
- `web/app/(app)/settings/page.tsx` (if root redirect needed)
- `web/components/skycot/settings/SettingsTabs.test.tsx` (NEW or extend)
- `web/playwright/settings/tabs-single-active.spec.ts` (NEW)

## Out of scope

- Settings shell double-h1 (covered by `fix-settings-shell-double-h1-and-danger-zone-redirect`)
- Settings sub-page metadata (covered by `fix-settings-sub-pages-add-missing-metadata` + `fix-settings-sub-pages-title-suffix-consistency`)
- Tab bar desktop overflow (covered by separate brief `fix-settings-tab-bar-desktop-overflow-no-scroll-affordance`)
- Knowledge tab dead route (covered by separate brief `fix-settings-knowledge-route-advertised-but-404`)

## Effort

XS — ~10-20 LOC + 2 tests. ~30-60 minutes.
