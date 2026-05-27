# fix-demo-banner-dismiss-button-z-index-freeze-button-overlap

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-banner-dismiss-button-z-index-freeze-button-overlap`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-1-and-2-claim-1-and-2.md` F11 [P2].

🟡 **P2 — `DemoBanner` is `sticky top-0 z-30 … pointer-events-none` with an internal `pointer-events-auto` on the X dismiss button. The worker page's `FreezeButton` renders at a lower z-index. On narrow viewports the freeze button can shift up into the banner's y-band and intercept clicks meant for the banner dismiss button. Not a Scene-1/2 blocker but a real UX paper cut.**

## Root cause

Two z-index / pointer-events interactions:

1. `web/components/skycot/DemoBanner.tsx:42` — outer wrapper has `sticky top-0 z-30 … pointer-events-none`, inner X dismiss button has `pointer-events-auto`.
2. `web/app/(app)/workers/[workerId]/FreezeButton.tsx` — renders with a lower or unspecified z-index.

When the viewport narrows (mobile or small-window), the freeze button (typically anchored top-right of the worker hero) can vertically collide with the banner's y-band. Because the banner wrapper uses `pointer-events-none`, the freeze button intercepts clicks through the banner. But the X dismiss button uses `pointer-events-auto`, so when the user goes to dismiss the banner, the click can land on the freeze button instead — pausing/freezing the worker.

This is the kind of bug that bites once per recording session and survives all CI gates because no test exercises narrow-viewport overlap.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F11 [P2] — Demo banner sticky-top dismiss button can be obscured by worker freeze button

## Fix recipe

1. **Bump the dismiss button's z-index to `z-40` explicitly.** In `DemoBanner.tsx:42`, ensure the inner X button has `z-40` (or higher than `FreezeButton`'s z).
2. **OR** move `FreezeButton` outside the banner's vertical column. Put it inside the worker hero card below the brand-loud header so it never collides with the sticky banner band.
3. **Vertical separation**: add `mt-12` or similar to `FreezeButton`'s wrapper so even when the banner is undismissed the click targets don't overlap on mobile.
4. **Vitest pin**: render `/workers/{id}/overview` at 375px viewport with banner shown; assert the bounding rectangle of `[data-testid=demo-banner-dismiss]` does NOT intersect `[data-testid=worker-freeze-button]`.
5. **(Optional) Playwright/visual regression**: capture a screenshot at 375px wide and compare to baseline. Catches future drift.

## Acceptance criteria

- [ ] On 375px wide viewport, the demo banner dismiss button's bounding box does not intersect the freeze button's bounding box.
- [ ] Clicking the banner's X button always dismisses the banner; never accidentally triggers the freeze action.
- [ ] No visual regression on desktop (≥1024px).
- [ ] Vitest pin asserts the bounding-box non-intersection.

## May-touch

- `web/components/skycot/DemoBanner.tsx:42` (z-index bump or pointer-events scope)
- `web/app/(app)/workers/[workerId]/FreezeButton.tsx` (positioning + z-index)
- Possibly `web/app/(app)/workers/[workerId]/overview/page.tsx` (layout adjustment if FreezeButton moves)
- Vitest fixture (new)

## Out of scope

- Demo banner sidebar overlap on desktop (covered by `fix-demo-banner-sidebar-overlap-desktop`)
- Demo banner copy (covered by `fix-demo-banner-copy-branch-on-data-source-or-suppress`)
- FreezeButton functional behaviour (covered separately if there's an existing brief)

## Effort

XS — ~10-30 LOC. ~20-40 min. P2 because the demo MP4 is recorded on a desktop viewport where the overlap doesn't materialize; this is a customer-trial polish nit.
