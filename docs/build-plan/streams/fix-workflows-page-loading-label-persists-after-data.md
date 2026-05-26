# fix-workflows-page-loading-label-persists-after-data

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-page-loading-label-persists-after-data`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` F6 [P2].

🟡 **P2 — On `/workflows`, 7 workflow cards render correctly with proper chips and metadata. Count pill correctly says "7 workflows". BUT the text "Loading workflows..." remains visible directly above the card grid — a stale loading state that should have been replaced when the cards rendered.**

## Root cause

The loading label is gated on a different condition than the card grid render. Likely:
- Cards render when `data?.workflows` is truthy
- Loading label renders when `isLoading || !data` — but `isLoading` flag fails to flip after the first paint, or the label and grid are derived from different state hooks that have drifted

Locate `web/app/(app)/workflows/page.tsx` around the workflows-list render. The loading label and cards likely live in adjacent JSX with subtly different conditions.

## Findings this brief closes

- chrome-live-walk-and-playwright F6 [P2] — Workflows page "Loading workflows..." label persists after data loads

## Fix recipe

1. **Unify the loading + content gates** in `web/app/(app)/workflows/page.tsx`:
   ```tsx
   if (isLoading) return <WorkflowsLoadingState />;
   if (workflows.length === 0) return <WorkflowsEmptyState />;
   return <WorkflowsGrid items={workflows} />;
   ```
   No "loading" label outside the loading state.

2. **Or, if the page is intentionally streaming** (skeleton-render while data fetches): replace the static text with a skeleton row that gets replaced when the data lands. Don't show stale text.

3. **Add a Playwright spec** at `web/playwright/workflows/no-stale-loading-label-after-data.spec.ts`:
   - Load `/workflows`
   - Wait for first workflow card to render
   - Assert no element contains text "Loading workflows..."

## Acceptance criteria

- [ ] "Loading workflows..." text NOT visible when ≥1 workflow card has rendered
- [ ] If isLoading is true, only loading-state UI shows (no card grid)
- [ ] If empty, empty-state UI shows (no loading label)
- [ ] New Playwright spec passes

## May-touch

- `web/app/(app)/workflows/page.tsx`
- Possibly `web/lib/api/workflows.ts` if the `isLoading` flag is the bug source
- `web/playwright/workflows/no-stale-loading-label-after-data.spec.ts` (NEW)

## Out of scope

- The workflow seed (covered by separate brief)
- The Run-now button disabled state (covered by separate brief)
- Card design (locked)

## Effort

~20-50 LOC + 1 spec. ~1 hour. P2 polish; small but visible.
