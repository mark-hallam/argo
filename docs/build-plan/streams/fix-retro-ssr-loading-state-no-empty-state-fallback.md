# fix-retro-ssr-loading-state-no-empty-state-fallback

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-retro-ssr-loading-state-no-empty-state-fallback`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/whole-site-mvp-claims-coverage.md` F19.

🟢 **P2 — `web/app/(app)/retro/page.tsx:142` renders the literal text "Loading retros…" with no skeleton, no empty-state fallback. Out-of-runbook path (Scene 6 doesn't visit /retro), but visible from the in-product nav and reads as half-finished. Compounds the broader cluster of SSR-empty-flash issues (cohorts, settings) with the same root pattern: pages that render loading copy as their default state instead of using a real empty-state or skeleton component.**

## Root cause

`web/app/(app)/retro/page.tsx:142` renders:
```tsx
<div>Loading retros…</div>
```

as the page's primary rendered content. Two defects in one render:
- (a) The loading copy is a placeholder string, not a skeleton component. Even during SSR or first hydration, the user reads "Loading…" instead of seeing a sized-correctly skeleton matching the eventual rendered shape.
- (b) When the data resolves and there are no retros, the page should show an empty-state ("No retros yet — kick off your first retrospective.") with a CTA, not "Loading retros…" forever.

Compounds with the cohorts SSR empty-flash pattern (`fix-cohorts-ssr-empty-preview-flash-before-hydrate.md`) — both are pages where the loading + empty + populated states aren't separated cleanly.

## Findings this brief closes

- whole-site-mvp-claims-coverage F19 [P2] — `/retro` SSR renders "Loading retros…" with no empty-state fallback

## Fix recipe

1. **Replace the "Loading retros…" string with a skeleton.** In `web/app/(app)/retro/page.tsx:142`, render the existing Skycot `Skeleton` component (likely `web/components/ui/Skeleton.tsx` or `web/components/skycot/Skeleton.tsx` — verify path) sized to match the populated retro list.
2. **Add a real empty-state branch.** When `retros.length === 0 && !isLoading`, render:
   ```
   No retros yet
   Capture what worked + what didn't last sprint. Start a retro →
   ```
   with a CTA button routing to a "new retro" action (if it exists; otherwise just the prompt).
3. **Match the rendering ternary pattern** used by other Skycot pages:
   ```tsx
   {retrosQuery.isPending ? <RetroListSkeleton /> :
    retrosQuery.isSuccess && retros.length === 0 ? <RetroEmptyState /> :
    <RetroList retros={retros} />}
   ```

## Acceptance criteria

- [ ] `/retro` SSR renders a skeleton (NOT the literal "Loading retros…" string).
- [ ] When the query resolves with 0 retros, the page renders the empty-state with a CTA.
- [ ] When the query resolves with >0 retros, the page renders the list (no regression).
- [ ] Playwright pin: walk `/retro` in mock-offline mode, assert no "Loading retros…" string is in the DOM after hydration.

## May-touch

- `web/app/(app)/retro/page.tsx` (rendering branches)
- `web/components/skycot/retro/RetroListSkeleton.tsx` (NEW — or existing skeleton component reused)
- `web/components/skycot/retro/RetroEmptyState.tsx` (NEW)
- `web/app/(app)/retro/__tests__/page.test.tsx` (pin)

## Out of scope

- The retros backend data source (out-of-v1 scope per audit)
- The cohorts SSR empty-flash (covered by `fix-cohorts-ssr-empty-preview-flash-before-hydrate.md` + `fix-cohorts-empty-state-render-locked-d3-magazine-preview.md`)
- The /retro metadata gap (covered by `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages.md`)

## Effort

S — ~50-80 LOC across page + 2 components + pin. ~1-2 hours. P2 because the page is reachable but not in the demo recording path.
