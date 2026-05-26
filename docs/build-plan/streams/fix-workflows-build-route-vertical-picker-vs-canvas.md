# fix-workflows-build-route-vertical-picker-vs-canvas

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-build-route-vertical-picker-vs-canvas`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-4-and-5-claim-4-and-5.md` F9 [P2].

🟢 **P2 — `/workflows/build` renders a 5-tile vertical-picker (Sales / Support / GTM / People Ops / Finance), not the react-flow canvas. Clicking a tile routes to `/workflows/<vertical>/untitled/build` which IS the real canvas. If Scene 5 narration says "open the builder", the camera lands on a card grid for one extra beat.**

## Root cause

`web/app/(app)/workflows/build/page.tsx:12` (verify line range) renders a vertical-picker shell:

- 5 vertical tiles (Sales / Support / GTM / People Ops / Finance)
- Each tile links to `/workflows/<vertical>/untitled/build`
- The actual builder canvas lives at `/workflows/<vertical>/<name>/build`

The intermediate picker is reasonable UX for users who don't yet know which vertical their new workflow belongs to. But for the v1.0 demo MP4, Scene 5 narration "open the builder" lands the camera on the picker — the operator must then click a tile to reach the canvas. The picker beat is a lost half-second of camera time on a 6-scene demo.

This is recordable as-is (the narration could just include the tile click), but the operator's reflex on "open the builder" is to expect a canvas, not another picker.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F9 [P2] — Workflow-builder canvas `/workflows/build` is a vertical-picker, not a canvas

## Fix recipe

Three acceptable approaches, operator-decisional:

### Option A — runbook talk-track edit (cheapest, no code change)

Edit `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 5 narration: "We'll start a new sales workflow" + the operator clicks the Sales tile, lands on `/workflows/sales/untitled/build`. The picker beat becomes part of the natural narration ("which vertical? Sales") rather than an unexplained delay.

### Option B — default the picker to the operator's last-used vertical

Persist the last-clicked vertical in `localStorage.skycot.lastWorkflowVertical`. On `/workflows/build` mount, if a vertical is remembered, redirect to `/workflows/<remembered>/untitled/build` directly. The first-time experience still shows the picker; subsequent visits skip it.

### Option C — promote `/workflows/build` to the canvas + default vertical to "sales"

Make `/workflows/build` render the canvas directly with `vertical=sales` as the default. Add a "Switch vertical" dropdown in the canvas chrome. The picker becomes a secondary surface at `/workflows/build/pick-vertical`. Highest-touch; biggest UX commitment.

**Recommend Option A for v1.0** (no code change, runbook ships). Option B/C are post-v1.0 polish.

## Acceptance criteria

- [ ] Scene 5 narration matches what the camera sees on the `/workflows/build` page.
- [ ] If Option B / C ships: a Playwright spec asserts the new flow.
- [ ] No regression on first-time-user experience (the picker still works for users who arrive without context).

## May-touch

- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Option A — Scene 5 narration block)
- `web/app/(app)/workflows/build/page.tsx` (Option B / C — picker → redirect logic)
- `web/lib/storage/last-workflow-vertical.ts` (Option B — NEW, ~20 LOC localStorage helper)
- `web/playwright/workflows/build-route.spec.ts` (Option B / C — extend or NEW)

## Out of scope

- Sales tile redirect to onboarding (covered by `fix-workflows-build-sales-tile-redirect-to-onboarding`)
- Top-level entry point (covered by `fix-workflows-build-top-level-entry-point`)
- Marketplace add-button on picker (covered by `fix-workflows-build-vertical-picker-add-marketplace`)
- Workflow-builder URL token + palette CU split (covered by `fix-workflow-builder-url-token-and-palette-cu-split`)

## Effort

Option A: ~10-20 lines doc edit, ~15 min. Option B: ~40-80 LOC + 1 spec. Option C: ~150-250 LOC + redesign. P2 because the picker is functional; this is talk-track / polish.
