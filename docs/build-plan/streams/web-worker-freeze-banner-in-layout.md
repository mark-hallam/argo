# web-worker-freeze-banner-in-layout — restore freeze banner after redirect-to-overview fix

**Owner**: claude. **Sizing**: ~100-150 LOC.
**Authored against**: origin/main `3121e32e` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-worker-freeze-banner-in-layout`.
**Priority**: P2 — feature parity restoration. Demo audit Pass-16 (`9e191c08`) made `/workers/[id]` auto-redirect to `/overview` so the page didn't render the placeholder "Open overview →" link. That redirect dropped the freeze-banner rendering (it lived on the now-skipped index page).

## Scope

Move the `FreezeBanner` + `FreezeButton` from the deleted index render to `web/app/(app)/workers/[workerId]/layout.tsx`, alongside the existing `<WorkerHero>`.

The pattern:
- Fetch freeze state in the layout (server-side) via the existing `fetchFreezeState` helper
- Render `<FreezeBanner>` above `<WorkerHero>` when state is non-null
- Render `<FreezeButton>` either inside the hero overflow menu OR top-right of the hero (designer call)

After this brief, freezing a worker shows the red banner on EVERY tab of the worker profile, not just the redirect-target index.

## Acceptance criteria

- [ ] Freezing a worker shows the red FreezeBanner on `/workers/[id]/overview`, `/activity`, `/memory`, `/skills`, `/trust`, `/settings` tabs.
- [ ] Unfreezing dismisses the banner from all tabs.
- [ ] FreezeButton is reachable from the hero (overflow menu acceptable).
- [ ] Existing FreezeButton tests still pass.

## Out of scope

- Adding freeze-history view (separate brief).
- Real-time freeze-state updates via SSE (separate brief).
