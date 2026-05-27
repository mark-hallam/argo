# fix-settings-sub-pages-h1-renders-settings-not-subsection-name

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-settings-sub-pages-h1-renders-settings-not-subsection-name`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F14)

🟢 **P3 — `/settings/providers`, `/settings/connectors`, `/settings/members` etc all render h1 = "Settings". The sub-section name (e.g. "Model provider policy", "Team & roles") only appears as h2 inside the content. Accessibility audit might flag the lack of distinguishing h1 across same-h1 sibling pages.**

## Root cause

`web/app/(app)/settings/layout.tsx` likely renders a shared "Settings" h1 above the tab-content area. Each sub-page renders its specific content with an internal h2. Per the locked D top-tabs design (memory `project_settings_page_design_locked`), the visual register is correct (top-level h1 for the section + h2 for the sub-section), but the semantic heading hierarchy assumes screen-reader users can navigate by h1, and every settings sub-page has the same h1.

Accessibility heuristic: each navigable URL deserves a distinguishing h1 OR the same h1 paired with a clearly-marked `aria-current` sub-nav. The current implementation has neither — `<h1>Settings</h1>` appears on all 11 sub-tab routes.

## Findings this brief closes

- whole-site F14 [P3] — Settings sub-pages all render h1="Settings" not the sub-section name

## Fix recipe

Two paths; ship Path B as it preserves the locked visual register.

**Path A (semantic-only) — promote the sub-section h2 to h1, demote the section heading to a sub-text or breadcrumb.** Breaks the locked D top-tabs design — not preferred.

**Path B (additive aria) — keep the visible h1="Settings", add an `<h1 className="sr-only">` per sub-page that screen readers announce.**

1. **In each settings sub-page file** (`web/app/(app)/settings/providers/page.tsx`, `…/connectors/page.tsx`, etc), add:
   ```tsx
   <h1 className="sr-only">Settings: Model Providers</h1>
   ```
   at the top of the page-specific content area. The visible "Settings" h1 stays in the shared layout but becomes `<h2>` or `<p className="settings-section-label">` semantically.
2. **Alternative**: keep visible h1="Settings" as a non-heading visual element (`<p>` styled like an h1) and make the per-page sr-only `<h1>` the only h1 on the page. Cleaner semantics.
3. **Add `aria-current="page"` to the active tab** in the tab strip so screen readers announce which sub-section the user is viewing.
4. **Vitest** at `web/app/(app)/settings/<each>/page.test.tsx`:
   - Assert exactly one `<h1>` exists on the page.
   - Assert its text includes the sub-section name.
5. **Audit with axe-core** to confirm no other a11y regression introduced.

## Acceptance criteria

- [ ] Each /settings sub-route has exactly one `<h1>` whose text includes the sub-section name (e.g. "Model Providers", "Connectors", "Members").
- [ ] Visible D top-tabs design unchanged (no layout shift).
- [ ] `aria-current="page"` set on the active tab.
- [ ] axe-core audit no new issues.
- [ ] vitest covers each sub-page.
- [ ] PR body cites whole-site F14.

## May-touch

- `web/app/(app)/settings/layout.tsx` (demote shared "Settings" h1)
- `web/app/(app)/settings/providers/page.tsx`, `…/connectors/page.tsx`, `…/members/page.tsx`, … (~11 files)
- `web/components/skycot/settings/SettingsTabStrip.tsx` (add `aria-current`)
- `web/app/(app)/settings/<sub>/page.test.tsx` per sub-page

## Out of scope

- The locked D top-tabs visual register (preserve as-is per memory `project_settings_page_design_locked`)
- Settings sub-pages title suffix (covered by `fix-settings-sub-pages-title-suffix-consistency`)
- Other a11y findings (separate audit pass)

## Effort

S — ~5 LOC per sub-page × 11 + ~30 LOC tests per page (or a single parametrised test). ~2-3 hours.
