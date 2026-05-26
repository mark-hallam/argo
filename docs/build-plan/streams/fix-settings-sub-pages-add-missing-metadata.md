# fix-settings-sub-pages-add-missing-metadata

**Owner**: claude or codex.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-sub-pages-add-missing-metadata`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/whole-site-mvp-claims-coverage.md` F11 [P1].

🟠 **P1 — 7 of 11 `/settings/*` sub-pages have no `export const metadata` in their `layout.tsx`. Each missing-metadata page falls through to the parent settings shell's default title and renders "Settings · SKYCOT" in the browser tab regardless of which sub-page is open. The 4 that do export metadata (`api-keys`, `data-export`, `members`, `security`) read properly. This is a mechanical 7×4-LOC fix, but it's been TODO since at least the prior synthesis pass — the demo camera reads tab titles on every shot.**

## Root cause

`grep -l "export const metadata" web/app/(app)/settings/*/layout.tsx`:

Present (4):
- `/settings/api-keys`
- `/settings/data-export`
- `/settings/members`
- `/settings/security`

Missing (7):
- `/settings/billing`
- `/settings/connectors`
- `/settings/knowledge`
- `/settings/notifications`
- `/settings/notify-channels`
- `/settings/providers`
- `/settings/workspace`

The parent `(app)/settings/layout.tsx` sets `metadata.title.default = "Settings"` and inherits the `template: "%s · SKYCOT"` from `(app)/layout.tsx`. Sub-pages without their own metadata inherit the parent's default → tab reads `"Settings · SKYCOT"` instead of `"Connectors · SKYCOT"`, `"Workspace · SKYCOT"`, etc.

This is the complement of `fix-settings-sub-pages-title-suffix-consistency` (which addressed the 4 EXISTING titles using a wrong format) — this brief is about ADDING metadata to the 7 that don't have any.

Camera-visibility: the operator may navigate through `/settings/*` during a runbook side-trip (settings demonstration is in some recording flows). Wrong tab title is a small but persistent polish miss.

## Findings this brief closes

- whole-site-mvp-claims-coverage F11 [P1] — 7 of 11 `/settings/*` sub-pages have no metadata export

## Fix recipe

1. **Add `export const metadata` to each of the 7 missing layout.tsx files**. Pattern:

   `web/app/(app)/settings/connectors/layout.tsx`:
   ```tsx
   import type { Metadata } from "next";
   import type { ReactNode } from "react";

   export const metadata: Metadata = { title: "Connectors" };

   export default function ConnectorsLayout({ children }: { children: ReactNode }) {
     return <>{children}</>;
   }
   ```

   Titles (one per page):
   - `/settings/billing` → `title: "Billing"`
   - `/settings/connectors` → `title: "Connectors"`
   - `/settings/knowledge` → `title: "Knowledge"`
   - `/settings/notifications` → `title: "Notifications"`
   - `/settings/notify-channels` → `title: "Notify channels"`
   - `/settings/providers` → `title: "Providers"`
   - `/settings/workspace` → `title: "Workspace"`
2. **Verify the `(app)/layout.tsx` template `"%s · SKYCOT"` slots correctly** — passing `title: "Connectors"` should yield browser tab `"Connectors · SKYCOT"`.
3. **Playwright/vitest pin**: navigate to each of the 7 sub-pages, assert `document.title` matches `"<expected> · SKYCOT"`.
4. **Coordinate with `fix-settings-sub-pages-title-suffix-consistency`** — that brief addresses the 4 existing-metadata pages that emit page-only titles. After both land, all 11 sub-pages render `"<page> · SKYCOT"` uniformly.

## Acceptance criteria

- [ ] All 11 `/settings/*` sub-pages have a `metadata` export (4 existing + 7 newly added).
- [ ] Each sub-page's browser tab reads `"<Page name> · SKYCOT"` (e.g. `"Connectors · SKYCOT"`).
- [ ] Playwright spec walks all 11 sub-pages and asserts the tab title.
- [ ] No regression on existing `/settings/api-keys`, `/settings/data-export`, `/settings/members`, `/settings/security` titles.

## May-touch

- 7 × `web/app/(app)/settings/<page>/layout.tsx` files (NEW metadata export each)
- Playwright spec under `web/playwright/settings/` (NEW or extend)

## Out of scope

- Settings sub-page H1 / heading hierarchy (covered by `fix-settings-shell-double-h1-and-danger-zone-redirect`).
- Settings sub-page title suffix consistency on the 4 existing (covered by `fix-settings-sub-pages-title-suffix-consistency`).
- Default-title leak to other in-app pages (covered by `fix-spa-page-titles-metadata-leak-default-approvals`).
- Sub-page body content / D top-tabs design (covered by `fix-settings-page-implement-d-top-tabs-design`).

## Effort

~7 × 8 LOC = ~56 LOC new layout files + ~30 LOC playwright spec. ~30 min. P1 because every settings sub-page is a discoverable surface from sidebar; wrong tab title persists on every visit.
