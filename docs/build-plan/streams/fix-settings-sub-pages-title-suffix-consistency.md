# fix-settings-sub-pages-title-suffix-consistency

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-sub-pages-title-suffix-consistency`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/whole-site-mvp-claims-coverage.md` F14 [P2].

🟡 **P2 — Four settings sub-pages emit page-only titles, breaking the global `· SKYCOT` suffix:
- `/settings/members` → "Members"
- `/settings/api-keys` → "API keys"
- `/settings/security` → "Security"
- `/settings/data-export` → "Data export"

The other 7 settings sub-pages emit "Settings · SKYCOT" (parent metadata inherited correctly).**

## Root cause

The 4 listed sub-pages set `metadata.title` as a plain string (e.g. `title: "Members"`) which overrides the parent layout's template rather than slotting into it. The other 7 sub-pages either don't set their own title (inheriting "Settings · SKYCOT") or use proper templating.

## Findings this brief closes

- whole-site-mvp-claims-coverage F14 [P2] — Four settings sub-pages emit page-only titles

## Fix recipe

For each of the 4 affected pages, EITHER:

**Option A: drop the page-level title override** so the layout's inherited title applies:
- Delete the `export const metadata = { title: "Members" }` line
- Each page becomes "Settings · SKYCOT" — consistent but less descriptive

**Option B (recommended): change to slot into the template properly**:
```tsx
// instead of:
export const metadata = { title: "Members" };
// use:
export const metadata = { title: "Members · Settings" };
// or:
export const metadata = { title: { absolute: "Members · Settings · SKYCOT" } };
```

The global template at root is `title.template = "%s · SKYCOT"`. If a page sets `title: "X"` it becomes `"X · SKYCOT"` (the template applies). So actually setting `title: "Members"` should produce `"Members · SKYCOT"` — investigate why the audit reports otherwise. Two possibilities:
1. The (app) layout (`web/app/(app)/layout.tsx:18-23`) sets `title.template = "%s · SKYCOT"` AND `title.default = "Approvals"`. If a page overrides with `title: { absolute: "Members" }`, the template is bypassed.
2. The page is using `title: { absolute: "Members" }` somewhere — find and fix.

So the fix is:
- Locate the actual metadata export in each of the 4 files
- If using `absolute`, switch to a plain string (which the template wraps)
- If already plain string, the template should apply — check the layout chain isn't overriding

## Acceptance criteria

- [ ] `curl /settings/members | grep title` → `Members · Settings · SKYCOT` (or `Members · SKYCOT`)
- [ ] Same shape for /settings/api-keys, /settings/security, /settings/data-export
- [ ] All 11 settings sub-pages have consistent title structure
- [ ] Playwright spec covers consistency across the 11 sub-tabs

## May-touch

- `web/app/(app)/settings/members/page.tsx`
- `web/app/(app)/settings/api-keys/page.tsx`
- `web/app/(app)/settings/security/page.tsx`
- `web/app/(app)/settings/data-export/page.tsx`
- Possibly `web/app/(app)/settings/layout.tsx` (verify template)
- `web/playwright/settings/sub-pages-titles-consistent.spec.ts` (NEW)

## Out of scope

- Other tab-title bugs (covered by sibling briefs)
- Settings page content (locked)
- The 7 settings pages that already work correctly

## Effort

~10-30 LOC + 1 spec. ~1 hour. P2 polish.
