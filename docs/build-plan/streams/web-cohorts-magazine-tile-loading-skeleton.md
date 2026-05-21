# web-cohorts-magazine-tile-loading-skeleton

**Authored against**: `origin/main` `652a1825` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-cohorts-magazine-tile-loading-skeleton`.

## Summary

`/cohorts` now ships the locked D3 magazine-tile layout (#1940). Cold-load currently shows a blank grid before data hydrates — small but visible polish gap. Add Suspense + skeleton placeholders matching the lead/stacked/compact tile shapes, gradient-blob backdrop preserved.

Pre-condition: **#1940 MERGED ✅** (D3 magazine-tile module on main).

## Acceptance

- [ ] On cold load (no cached fetch), `/cohorts` renders shimmer skeletons for: 1 lead tile + 2 stacked tiles + 4 compact tiles (matches the locked layout).
- [ ] Skeletons preserve the asymmetric layout — no layout shift when real data arrives.
- [ ] Reduced-motion: shimmer animation is disabled and a static muted fill is used per `prefers-reduced-motion`.
- [ ] No regression on the locked D3 register (Audiowide kicker, gradient avatars on lead, restrained editorial on stacked/compact).
- [ ] One unit test: skeleton renders 7 placeholders in the correct shapes.

## May touch

- `web/components/skycot/cohorts/CohortMagazineGrid.tsx` (wrap children with Suspense; render `<CohortMagazineSkeleton>` fallback)
- `web/components/skycot/cohorts/CohortMagazineSkeleton.tsx` (new file)
- `web/components/skycot/cohorts/CohortMagazineStyles.tsx` (skeleton-specific styles)
- `web/app/(app)/cohorts/page.tsx` if Suspense boundary needs to live there

## Must NOT touch

- The locked D3 tile components (Lead/Stacked/Compact) — render unchanged.
- `CohortDeptTabs` — tab UX is settled.
- Any other route's loading state.

## Notes

- Sizing: 80–150 LOC.
- Operator memory: `project_cohorts_page_design_locked.md` — D3 layout (1 lead + 2 stacked + 4 compact). Match that shape exactly.
- This is small + demo-visible polish — quick win.
