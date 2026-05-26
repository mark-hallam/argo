# fix-marketing-for-vertical-pages-double-suffix-title

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-for-vertical-pages-double-suffix-title`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/whole-site-mvp-claims-coverage.md` F6 [P1].

🟠 **P1 — All 6 `/for/*` vertical-landing pages emit titles like `"AI sales teammates that never sleep — SKYCOT · SKYCOT"`. Page sets its own "— SKYCOT" suffix AND inherits the global "· SKYCOT" suffix from root template. Browser tab reads doubled brand. Marketing nav now links to `/for/*` pages from `/landing`, so a demo viewer Cmd-clicking any vertical card opens a tab with the double-SKYCOT title.**

## Root cause

`web/app/for/_components/VerticalLandingShell.tsx` (or per-page `metadata.title`) sets the title as `"X — SKYCOT"`. `web/app/layout.tsx` template appends ` · SKYCOT` via `title.template = "%s · SKYCOT"`. Result: `"X — SKYCOT · SKYCOT"`.

## Findings this brief closes

- whole-site-mvp-claims-coverage F6 [P1] — `/for/*` pages double-suffix titles

## Fix recipe

1. **Locate the metadata source** for each `/for/{sales,recruiting,marketing,finance,ops,support}` page. Likely either:
   - Per-page `export const metadata = { title: "X — SKYCOT" }` — drop the ` — SKYCOT` suffix
   - OR shared `VerticalLandingShell` setting via dynamic metadata helper — drop the suffix in the helper

2. **For each page**, change `title: "AI sales teammates that never sleep — SKYCOT"` → `title: "AI sales teammates that never sleep"`. The global `· SKYCOT` template will handle brand suffix.

3. **Verify with curl**:
   ```bash
   for v in sales recruiting marketing finance ops support; do
     curl -s http://localhost:3000/for/$v | perl -nle 'print "$v: $1" if /<title[^>]*>([^<]+)/'
   done
   ```
   Expected: each line ends in ` · SKYCOT` exactly once.

4. **Add a Playwright spec** at `web/playwright/marketing/no-double-suffix-titles.spec.ts`:
   - For each of the 6 vertical paths, load + assert `document.title` matches `/^[^·]+ · SKYCOT$/` (single suffix only)

## Acceptance criteria

- [ ] All 6 `/for/*` pages have single ` · SKYCOT` suffix in `<title>`
- [ ] No page has ` — SKYCOT · SKYCOT` or any double-suffix pattern
- [ ] New Playwright spec passes
- [ ] Browser tab on each /for/* page reads e.g. "AI sales teammates that never sleep · SKYCOT"

## May-touch

- `web/app/for/sales/page.tsx`
- `web/app/for/recruiting/page.tsx`
- `web/app/for/marketing/page.tsx`
- `web/app/for/finance/page.tsx`
- `web/app/for/ops/page.tsx`
- `web/app/for/support/page.tsx`
- Possibly `web/app/for/_components/VerticalLandingShell.tsx` if metadata is centralised
- `web/playwright/marketing/no-double-suffix-titles.spec.ts` (NEW)

## Out of scope

- `/hire` double-suffix (covered by `fix-hire-page-title-double-suffix`)
- `/workflows/templates` missing suffix (covered by `fix-workflows-templates-page-title-suffix`)
- Settings sub-page titles (covered by `fix-settings-sub-pages-title-suffix-consistency`)

## Effort

~15-40 LOC + 1 spec. ~1 hour. P1 because viral marketing relies on shareable tabs with clean titles.
