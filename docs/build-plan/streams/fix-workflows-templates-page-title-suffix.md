# fix-workflows-templates-page-title-suffix

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-templates-page-title-suffix`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/whole-site-mvp-claims-coverage.md` F13 [P2].

🟡 **P2 — `/workflows/templates` opts out of the global `· SKYCOT` title template; tab reads just "Workflow templates". Every other page uses the global title template `"X · SKYCOT"`. Inconsistent.**

## Root cause

`web/app/(app)/workflows/templates/page.tsx` (or its layout) sets metadata as `{ title: { absolute: "Workflow templates" } }` — the `absolute` field bypasses the parent template. Other pages don't use absolute.

## Findings this brief closes

- whole-site-mvp-claims-coverage F13 [P2] — `/workflows/templates` missing global `· SKYCOT` suffix

## Fix recipe

1. **Locate the metadata** at `web/app/(app)/workflows/templates/page.tsx` or `web/app/(app)/workflows/templates/layout.tsx`.

2. **Change** `title: { absolute: "Workflow templates" }` → `title: "Workflow templates"` so the parent template applies.

3. **Verify**: `curl /workflows/templates | grep title` → expect `Workflow templates · SKYCOT`.

4. **Add a one-line assertion** to an existing comprehensive title-format Playwright spec (or create one if none exists) so any future absolute-title regression is caught.

## Acceptance criteria

- [ ] `curl /workflows/templates | grep title` returns `Workflow templates · SKYCOT`
- [ ] No `absolute:` in metadata of this page
- [ ] Playwright spec passes

## May-touch

- `web/app/(app)/workflows/templates/page.tsx` (or `layout.tsx`)
- A comprehensive title-format Playwright spec

## Out of scope

- Other tab-title bugs (covered by sibling briefs)
- Page content (preserve)

## Effort

~5-10 LOC. ~15 min. P2 polish.
