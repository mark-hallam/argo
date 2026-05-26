# fix-changelog-pagination-for-96-entries

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-changelog-pagination-for-96-entries`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F21 (P3).

🟢 **P3 — `/changelog` renders 96 timeline entries inline on a single page. The scroll is fairly long; performance is OK on desktop but mobile becomes sluggish, and search-engine indexing benefits from per-page URLs. Not a blocker; long-tail SEO + perf polish.**

## Root cause

`web/app/changelog/page.tsx` renders all entries from the feeder JSON / MDX inline. No pagination, no "older entries" affordance. The page weight scales linearly with the number of entries; at 96 the page is heavy but functional. As the changelog grows past 200 entries, the perf hit will become user-visible.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F21 [P3] — `/changelog` has 96 entries but no pagination / "older entries" affordance

## Fix recipe

1. Decide the page size (recommend 20 — fits in one viewport on desktop, ~2 viewports on mobile).
2. Add `?page=N` query-string support to `/changelog/page.tsx`:
   - Read `searchParams.page` (default 1).
   - Slice the entries: `entries.slice((page - 1) * 20, page * 20)`.
   - Render entries + a "Newer / Older" pair at the bottom.
3. Generate static paths for SEO: emit a sitemap entry per page so crawlers index each page separately.
4. Optional: add a "Jump to year" affordance in the rail (clicking 2026 / 2025 sets `?page=...` to the page containing the first entry of that year).

## Acceptance criteria

- [ ] `/changelog` and `/changelog?page=1` render the 20 most-recent entries.
- [ ] `/changelog?page=5` (or last page) renders the oldest 20.
- [ ] "Newer / Older" navigation works in both directions.
- [ ] Sitemap includes one entry per changelog page.
- [ ] SEO crawl simulation (`curl /changelog?page=2`) returns a valid page with `<link rel="prev/next">` head tags.

## May-touch

- `web/app/changelog/page.tsx` (~50 LOC pagination logic + nav)
- `web/app/sitemap.ts` (~10 LOC dynamic per-page entries)
- `web/components/skycot/changelog/Pagination.tsx` (NEW or extracted — ~30 LOC)

## Out of scope

- The "this week" date rail (covered by `fix-changelog-this-week-date-rail-auto-refresh`).
- Mobile breakpoints (covered by `fix-marketing-pages-compare-changelog-mobile-breakpoints`).
- The changelog content itself.

## Effort

S — ~100 LOC across the page, sitemap, and Pagination component. ~1.5 hours. P3 because perf is still tolerable at 96 entries; ship before the page grows past 200.
