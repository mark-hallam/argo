# fix-cohorts-cold-paint-renderer-wedge-three-plus-minutes

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-cohorts-cold-paint-renderer-wedge-three-plus-minutes`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F03)

🟠 **P1 — `/cohorts` cold-paint hangs the Chrome renderer for 3+ minutes on first visit. After preview restart, subsequent visits load in ~6s and the DOM is responsive. The hang did not reproduce after restart, suggesting a one-shot hydration cliff (cohort lists + detail-drawer + 5 magazine grids + sticky tab bar mounting simultaneously) or a cohorts data-fetch race with hydration.**

## Root cause (suspected)

`web/app/(app)/cohorts/page.tsx` mounts the locked D3 magazine layout: 9 dept tabs + per-tab lead-tile / stacked / compact magazine grids + sticky tab bar + cohort detail drawer. The audit observed the renderer becoming unresponsive to `preview_eval` / `preview_snapshot` / `preview_screenshot` for the full 3-minute window before restart recovered it.

Plausible candidate causes (one of these, not all — needs profiling):

1. **`useCohorts()` hook waterfall**: the hook may chain N sequential gateway calls (one per dept tab) instead of batching, blocking first paint until all complete.
2. **`CohortMagazineGrid` hydration cost**: 5 magazine grids × 7 cards each × heavy animated banners = 35+ animated components mounting on hydration.
3. **`useCohort()` per-card hook racing with the parent's `useCohorts()`** — both fire on initial paint, causing redundant fetches.
4. **Suspense boundary missing** around the cohort drawer, so the drawer's mounting cost is in the critical path even when closed.
5. **A specific cohort's render cost** (the project-coordination cohort that falls into "Other" per whole-site F02) may carry oversized SVG art or eager image preload.

Sibling `fix-cohorts-empty-cohort-drawer-focus-trap-leak-renders-unresponsive` (IN_REVIEW #2550) addresses an unresponsiveness pattern in the cohort drawer — overlap possible; verify scope.

## Findings this brief closes

- whole-site F03 [P1] — `/cohorts` cold-paint browser wedge for 3+ minutes on first visit

## Fix recipe

This is a diagnose-then-fix brief. The fix depends on what the profile reveals — author the diagnostic step explicitly.

1. **Profile first paint** using Chrome DevTools Performance tab against a cold preview (`pnpm --dir web build && pnpm --dir web start`, no warm cache, no warm cargo target). Capture a 30-second trace covering navigation to `/cohorts`. Save the trace to `docs/skycot/perf/cohorts-cold-paint-2026-05-28.json` (do not commit the trace if it contains tenant data; sanitise first).
2. **Identify the long task(s)** > 500ms in the trace. Common culprits:
   - Long compile-time tasks in the React commit phase (component depth > 12).
   - Sequential network waterfall (visible as a staircase in the Network panel).
   - Layout thrash from animated banners (large `reflow` calls).
3. **Apply the fix** depending on what the trace says:
   - **If waterfall**: in `web/lib/hooks/use-cohorts.ts`, switch from sequential per-dept fetches to a single batched call (or use React Query's `useQueries` for parallel fetches).
   - **If hydration**: wrap `CohortMagazineGrid` in a `<Suspense>` with a skeleton fallback and lazy-load each dept's grid via `next/dynamic` with `{ ssr: false }`. The dept tab strip can render server-side; the grids hydrate per-tab on click.
   - **If animation cost**: replace `framer-motion` entry animations with CSS-only fade-ins for the initial paint; framer-motion gets enabled for interaction-time animations only.
   - **If image preload**: defer non-visible banner images via `loading="lazy"` and an `IntersectionObserver`.
4. **Add a perf regression test** at `web/playwright/cohorts-cold-paint.spec.ts`:
   - Launch a fresh browser context (`browser.newContext({})`).
   - Navigate to `/cohorts`.
   - Assert `domContentLoaded` < 10000ms and `largest-contentful-paint` < 5000ms.
   - This is a soft regression guard — flake-prone but catches another 3-minute wedge if it returns.
5. **Document the diagnosis + fix** in the PR body so the next engineer who hits this has the trace + the candidate list.

## Acceptance criteria

- [ ] Cold-paint `/cohorts` renders the dept tabs + first tab's lead tile within 10 seconds on a fresh Chrome session.
- [ ] LCP measured under 5 seconds.
- [ ] Playwright perf-regression test passes cold.
- [ ] PR body includes a link to the Chrome Performance trace and identifies the root-cause long task.
- [ ] No regression on existing cohort tests.
- [ ] PR body cites whole-site F03.

## May-touch

- `web/app/(app)/cohorts/page.tsx` (Suspense / dynamic-import wrapping)
- `web/components/skycot/cohorts/CohortMagazineGrid.tsx` (animation / preload)
- `web/lib/hooks/use-cohorts.ts` (batched fetch)
- `web/playwright/cohorts-cold-paint.spec.ts` (NEW)
- `docs/skycot/perf/cohorts-cold-paint-2026-05-28.json` (trace, optional, sanitised)

## Out of scope

- Cohort registry vertical-mapping (covered by `fix-cohorts-dept-registry-add-project-coordination-vertical-alias`)
- Cohort drawer focus-trap leak (covered by `fix-cohorts-empty-cohort-drawer-focus-trap-leak-renders-unresponsive` IN_REVIEW #2550)
- Missing cohorts in seed (covered by separate brief on whole-site F06)

## Effort

M — ~1-2 hours diagnosing + ~3-4 hours fixing depending on root cause + ~1 hour playwright. ~5-7 hours total. P1 because the demo MP4 doesn't visit /cohorts so it isn't recording-blocking, but it's a real-customer cliff.
