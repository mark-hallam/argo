# fix-settings-tab-bar-desktop-overflow-no-scroll-affordance

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-settings-tab-bar-desktop-overflow-no-scroll-affordance`
**Source:** ux-audit-2026-05-27-2351

🟢 **P2 — At 1440px viewport the 12 settings tabs + Danger zone don't fit; the right-side tabs (Billing, Data export, Danger zone) are clipped. No scroll arrow, scroll fade, or overflow indicator.**

## Root cause

> 12 tabs + Danger zone don't fit; the right-side tabs (Billing, Data export, Danger zone) are clipped. There's no scroll arrow, scroll fade, or overflow indicator. On a mobile/tablet viewport the tabs do horizontally scroll, but the same affordance is missing at desktop width.
> — `whole-site-mvp-claims-coverage.md` F18

The locked D-top-tabs design (per `project_settings_page_design_locked`) was authored before the tab count grew past ~8. At 1440px each tab needs ~110-130px including padding; 12 + Danger zone ≈ 1500-1700px total before the brand sidebar's ~230px is deducted.

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F18 [P2] — settings tab bar overflows at 1440 viewport with no horizontal-scroll affordance

## Fix recipe

Three operator-decisional options:

### Option A — Desktop horizontal scroll with edge fade + arrows

1. In `web/components/skycot/settings/SettingsTabs.tsx`, add `overflow-x: auto` + a gradient fade on each side + clickable scroll arrows that appear when the bar overflows.
2. Use `ResizeObserver` to show/hide arrows based on `scrollWidth > clientWidth`.
3. The mobile horizontal-scroll behaviour already works — promote it to all viewports.

### Option B — Collapse into "More" overflow menu

1. Detect overflow with `ResizeObserver`.
2. When overflowing, render the first N tabs that fit, then a `<details>` / `<DropdownMenu>` containing the rest.
3. The "More" menu icon stays visible at narrow viewports.

### Option C — Reduce the tab count

1. Audit which tabs are MVP-essential.
2. Move secondary tabs (e.g. Data export, Integrations) into a "More" sub-page or under Danger zone.
3. Re-locked design with the operator.

Option A is the demo-day fix. Option B is the better mobile-first pattern. Option C is a design call requiring an operator decision.

## Acceptance criteria

- [ ] At 1440px viewport, all settings tabs are reachable (visible or via scroll/arrow/overflow menu).
- [ ] No tab is clipped without an affordance to reach it.
- [ ] Danger zone remains right-aligned per locked design.
- [ ] Mobile horizontal-scroll behaviour preserved.
- [ ] Component test at `web/components/skycot/settings/SettingsTabs.test.tsx` covers the overflow case.
- [ ] PR body cites finding F18.

## May-touch

- `web/components/skycot/settings/SettingsTabs.tsx`
- `web/components/skycot/settings/SettingsTabs.test.tsx` (NEW or extend)
- Storybook fixture (if any)

## Out of scope

- Active-tab matcher double-highlight (covered by `fix-settings-tabs-prefix-startswith-double-highlights-workspace`)
- Adding or removing tabs (separate design decision)
- Knowledge tab dead route (covered by separate brief)

## Effort

S to M — Option A: ~80-150 LOC + 1 test. ~3-4 hours.
