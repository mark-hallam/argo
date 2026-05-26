# fix-cohorts-ssr-empty-preview-flash-before-hydrate

**Owner**: claude.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cohorts-ssr-empty-preview-flash-before-hydrate`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/whole-site-mvp-claims-coverage.md` F13 [P2] + F14 [P2].

🟡 **P2 — `/cohorts` SSR renders `CohortsEmptyPreview` even when the real API has 1 cohort, because `cohortsQuery.isPending` is `false` during SSR (no fetch attempted yet — server-render returns the empty array). After hydration the magazine grid appears. Demo recording would catch ~200-500ms of empty-preview flash before the real grid renders. Also: the hero subtitle pluralises "0 cohorts" SSR → "1 cohort" post-hydrate — visible flicker on first paint.**

## Root cause

`web/app/(app)/cohorts/page.tsx:103-110` ternary:

```tsx
{cohortsQuery.isPending ? (
  <CohortsSkeleton />
) : cohorts.length === 0 ? (
  <CohortsEmptyPreview />
) : (
  <MagazineGrid cohorts={cohorts} />
)}
```

During SSR, React Query's `useQuery` returns `isPending: false` because no fetch has been initiated on the server — the query is "idle, no data". So `cohorts.length === 0` evaluates true and `CohortsEmptyPreview` renders. On hydration, the client-side fetch fires, `isPending` flips to `true` briefly, then resolves with the real data → magazine grid appears.

User-visible: ~200-500ms of empty preview on first paint, then the real grid swaps in.

Same root cause affects the hero subtitle at lines 80-86:

```tsx
{total} {total === 1 ? "cohort" : "cohorts"} across the fleet
```

SSR has `total === 0` → "0 cohorts across the fleet". Post-hydrate total flips to 1 → "1 cohort across the fleet". Visible flicker on the SAME render frame as the body swap.

This is the classic React Query SSR flash pattern. Two fixes work:

- **(a) Use `isSuccess` instead of `isPending` for the empty-state condition.** `isSuccess` is only true after the fetch resolves; during SSR or pending, the ternary falls into the skeleton branch.
- **(b) Server-pre-fetch the query** with `prefetchQuery` + Hydrate — the cohort data is available on the server render before paint.

(a) is simpler. (b) is more correct (no fetch round-trip on the client). Both work.

## Findings this brief closes

- whole-site-mvp-claims-coverage F13 [P2] — `/cohorts` SSR shows CohortsEmptyPreview even with 1 real cohort
- whole-site-mvp-claims-coverage F14 [P2] — Cohorts hero subtitle pluralises "0 cohorts" SSR → "1 cohort" post-hydrate

## Fix recipe

### Path A — gate on `isSuccess` (cheaper)

1. Change the ternary at `web/app/(app)/cohorts/page.tsx:103-110`:
   ```tsx
   {cohortsQuery.isSuccess && cohorts.length === 0 ? (
     <CohortsEmptyPreview />
   ) : cohortsQuery.isSuccess ? (
     <MagazineGrid cohorts={cohorts} />
   ) : (
     <CohortsSkeleton />
   )}
   ```
2. Change the hero subtitle at lines 80-86 to also gate on `isSuccess`:
   ```tsx
   {cohortsQuery.isSuccess ? (
     <p>{total} {total === 1 ? "cohort" : "cohorts"} across the fleet</p>
   ) : (
     <p className="opacity-0 select-none">placeholder</p> // preserve layout height
   )}
   ```
   Or use a `<SkeletonText>` component for the subtitle line during loading.
3. Verify there is no flicker on first paint with the dev server.

### Path B — server prefetch (preferred long-term)

4. In `web/app/(app)/cohorts/page.tsx`, convert to a server component or add `getServerSideProps`-equivalent that pre-fetches the cohorts query and passes hydrated state down.
5. Use `<HydrationBoundary state={dehydratedState}>` to inject the prefetch result into the React Query cache before client hydration.

Either path works; Path A is the 30-minute fix, Path B is the 2-hour proper fix. Recommend Path A for the immediate polish; flag Path B as a follow-up if SSR perf is later prioritized.

### Test

6. Vitest + jsdom: simulate SSR render → assert NO `CohortsEmptyPreview` in markup when query hasn't resolved.
7. Playwright: navigate to `/cohorts` → assert no empty-state flash within first 1s (visually-stable).

## Acceptance criteria

- [ ] SSR render does NOT include `CohortsEmptyPreview` when the cohorts query hasn't completed yet.
- [ ] First paint shows a skeleton (or stable placeholder), not empty-state text.
- [ ] Hero subtitle does not flash from "0 cohorts" to "1 cohort".
- [ ] Vitest pin asserts the empty-state path requires `isSuccess === true && data.length === 0`.
- [ ] No regression on the post-hydration magazine grid render.

## May-touch

- `web/app/(app)/cohorts/page.tsx:80-86` (hero subtitle gating)
- `web/app/(app)/cohorts/page.tsx:103-110` (body ternary)
- Optional: `web/app/(app)/cohorts/page.tsx` server-component conversion + HydrationBoundary
- Vitest + Playwright specs under `web/playwright/cohorts/`

## Out of scope

- `CohortsEmptyPreview` component itself (covered by `fix-cohorts-empty-state-render-locked-d3-magazine-preview`).
- D3 magazine grid layout (covered by the locked design memory).
- Department-tab registry drift (covered by `fix-cohorts-dept-registry-design-lock-drift-6-vs-8`).
- Cohorts memory drift (covered by `fix-cohorts-memory-drift-project-ops-tab-position`).

## Effort

Path A: ~30 LOC + tests. ~30 min. Path B: ~80-120 LOC. ~2 hours. P2 because the flash is visible but transient; not load-bearing on demo narration, but a polish miss visible on every cold load.
