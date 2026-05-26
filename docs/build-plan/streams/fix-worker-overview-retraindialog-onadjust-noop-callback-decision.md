# fix-worker-overview-retraindialog-onadjust-noop-callback-decision

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-overview-retraindialog-onadjust-noop-callback-decision`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-1-and-2-claim-1-and-2.md` F12.

🟢 **P3 — `WorkerOverviewContent.tsx:71-73` defines `onAdjust = useCallback(() => { /* no-op — dialog mount lives in WhatSheDoesCard */ }, [])`. The inline comment claims the callback is "kept as an analytics seam for callers who want to know an adjust was launched" — but the callback fires no analytics event. Either wire it or delete it; the comment misrepresents what the code does.**

## Root cause

`web/components/skycot/worker/WorkerOverviewContent.tsx:71-73`:

```ts
const onAdjust = useCallback(() => {
  // no-op — dialog mount lives in WhatSheDoesCard; this callback is kept as an
  // analytics seam for callers who want to know an adjust was launched.
}, []);
```

The dialog (`RetrainDialog`) mounts inside `WhatSheDoesCard.tsx`; the parent's `onAdjust` callback is plumbed as a prop but never invoked with any signal — and even if invoked, the body is empty. The comment is aspirational. The code reads as "we plan to wire analytics here" but no `track('worker.adjust.opened')` or equivalent is in place.

Two clean resolutions:
- **(a) Delete the no-op + the comment.** If no analytics need exists today, the callback's dead-code. The dialog mounts and works without parental awareness.
- **(b) Actually wire the analytics seam.** Look for the project's analytics pattern (likely `web/lib/analytics.ts` or similar — verify via `rg 'track\(|analytics\.' web/lib/`), and call it with a meaningful event.

Recommend (a) for v1.0 demo — there's no analytics dependency in the demo path, and the dead-code is small but reads as half-finished.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F12 [P3] — RetrainDialog mounted but `onAdjust` is a comment-only no-op (no analytics fire)

## Fix recipe

**Recommended path (a): delete the no-op + the comment + the prop plumbing.**

1. In `web/components/skycot/worker/WorkerOverviewContent.tsx:71-73`, delete the `onAdjust` `useCallback`.
2. Walk the prop wire — wherever `onAdjust` was passed to `WhatSheDoesCard` or downstream, drop the prop. `WhatSheDoesCard.tsx` should no longer accept `onAdjust` either if the dialog mounts internally.
3. Verify the dialog still opens and closes correctly from the worker-overview page (Playwright pin).

**Alternative path (b): wire to a real analytics call.**

4. Locate the project's analytics shim (`web/lib/analytics.ts` if present). Add a call like `track('worker.adjust.opened', { workerId })`.
5. Update the comment to match the wired behaviour.

## Acceptance criteria

- [ ] Either: `onAdjust` is removed entirely AND `WhatSheDoesCard` props are cleaned up, OR `onAdjust` invokes a real analytics call.
- [ ] No `// no-op — ... analytics seam` comments remain in `WorkerOverviewContent.tsx`.
- [ ] Vitest pin: `WorkerOverviewContent` renders without `onAdjust` prop (if path a) OR fires the analytics call on dialog open (if path b).
- [ ] Playwright pin: the Adjust dialog still opens and closes from the worker overview page.

## May-touch

- `web/components/skycot/worker/WorkerOverviewContent.tsx` (delete callback OR wire analytics)
- `web/components/skycot/worker/WhatSheDoesCard.tsx` (drop prop if path a)
- `web/lib/analytics.ts` (if it exists and path b is chosen)
- `web/components/skycot/worker/__tests__/WorkerOverviewContent.test.tsx` (pin)

## Out of scope

- Wider analytics-event taxonomy decisions (operator-level)
- The duty-toggle stub (covered by `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.md`)

## Effort

XS — ~10-30 LOC depending on path. ~15-30 minutes. P3 because the dialog works regardless; this is dead-code cleanup.
