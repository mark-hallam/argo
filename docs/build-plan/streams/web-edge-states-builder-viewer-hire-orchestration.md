# web-edge-states-builder-viewer-hire-orchestration — add empty/loading/error states to 4 pages

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-edge-states-builder-viewer-hire-orchestration`.
**Priority**: P1 — wave-1 edge-states audit found 21 ❌ MISSING cells / 90 audited (3 states × 30 pages). Worst offenders: builder, hire-wizard, viewer, orchestration.
**Sizing**: ~300-500 LOC (skeleton + error + empty components per page + tests).

## Why this brief exists

From the wave-1 edge-states audit (`docs/skycot/audit/edge-states/2026-05-20-wave-1-edge-states.md`):

> Top 5 pages with missing edge states:
> 1. `/workflows/[v]/[name]/build` (builder) — class-leading visual
>    canvas with zero data-fetch states; a gateway 500 ejects users
>    mid-edit.
> 2. `/hire` + `/hire/step/[step]` + `/hire/done/[workerId]` —
>    Suspense fallback is `null`, draft hydration failure = white screen.
> 3. `/workflows/[v]/[name]` (viewer) — only "not found" handling;
>    no skeleton, no error path.
> 4. `/orchestration` — has error + loading but no empty state, so
>    brand-new tenants spin in LoadingState forever.

This brief fixes those 4 pages (the worst offenders). /team's error
state already shipped wave-1; this brief covers the next tier.

## Goals

Per page:

**/workflows/[v]/[name]/build (builder):**
- LoadingSkeleton showing the canvas grid + side panel skeleton while workflow loads
- ErrorState (uses ErrorShell) when fetch fails
- Empty state: "This workflow has no steps yet. Add one to get started." with primary CTA

**/hire wizard (3 pages):**
- Replace `Suspense fallback={null}` with a WizardLoadingSkeleton (mirror the WizardEditorialShell shape)
- ErrorState for draft-hydration failure (graceful "Couldn't restore your draft — start over?" with reset action)
- /hire/done worker-fetch error state

**/workflows/[v]/[name] (viewer):**
- LoadingSkeleton (timeline skeleton with 5 step placeholders)
- ErrorState for fetch failure
- Existing "not found" stays

**/orchestration:**
- EmptyState for brand-new tenants ("No orchestration yet. Hire your first teammate to start.")

## Steps

1. Read existing `<ErrorShell>` at `web/components/error/ErrorShell.tsx` (reuse, don't duplicate).
2. Build `<WorkflowBuilderSkeleton>`, `<WorkflowViewerSkeleton>`, `<WizardSkeleton>` skeletons matching the page shapes.
3. Wire `isPending`/`isError`/`isEmpty` branches in each page's component tree.
4. Add tests for each new state.

## may-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (viewer)
- `web/app/(hire)/hire/page.tsx`
- `web/app/(hire)/hire/step/[step]/page.tsx`
- `web/app/(hire)/hire/done/[workerId]/page.tsx`
- `web/app/(app)/orchestration/page.tsx`
- New skeletons under `web/components/skycot/workflows/` and `web/components/auth/`

## must-NOT-touch

- `<ErrorShell>` (reuse, don't refactor)
- `web/app/(app)/team/page.tsx` (already has error state from wave-1)
- `web/app/(app)/page.tsx` (already has all 3 states)

## Acceptance criteria

- [ ] All 4 pages above render dedicated loading / error / empty states (matrix from the audit, no ❌ cells).
- [ ] Builder gateway-500 simulation lands on ErrorState with reset action (NOT white screen).
- [ ] /hire fallback shows skeleton (NOT null).
- [ ] /orchestration brand-new-tenant empty state renders ("hire your first teammate").
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Edge-states re-audit shows the 4 pages all ✅ across the 3 states.

## References

- Audit: `docs/skycot/audit/edge-states/2026-05-20-wave-1-edge-states.md`
- Pattern: `/team` ErrorState (commit 09b5f1cc) — reuse the shape.
