# fix-worker-tab-strip-add-schedules-tab-vocab-consistency

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-tab-strip-add-schedules-tab-vocab-consistency`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-1-and-2-claim-1-and-2.md` F8 [P1].

🟠 **P1 — Worker profile tab strip has no "Schedules" tab. `WORKER_TAB_IDS` is `overview · activity · memory · skills · trust · escalation · settings` — 7 tabs, none labeled "Schedules". Schedules live inside the Overview tab under a `DutiesCard` titled "Responsibilities" with cron-shaped right-aligned chips ("daily 06:30"). The runbook Scene 2 narrates "click Schedules" but there is no Schedules tab. The card heading uses responsibility-vocab while the chips use schedule-vocab. Per operator-locked vocab memory `project_vocabulary_duty_vs_schedule`: UI says "Schedule", code says "duty" — but the card title is "Responsibilities" (a third word).**

## Root cause

Three competing vocab choices on a single surface:

1. **Tab strip** (`web/components/skycot/WorkerTabs.tsx:19-27`): no Schedules tab; schedules live nested inside Overview.
2. **Card title** (`web/components/skycot/worker/DutiesCard.tsx:101`): `CardTitle` = "Responsibilities".
3. **Cadence chips inside the card**: "daily 06:30", "as items arrive" — schedule-vocab.
4. **Code/SQL/file names**: `duty`, `DutiesCard`, `duty-toggle-*` testids — duty-vocab.

Per operator memory `project_vocabulary_duty_vs_schedule` (locked 2026-05-23): in any SPA / hire wizard / marketing / docs / settings copy, use **"Schedule"** not "Duty". Code/SQL/audit/brief filenames keep "duty" (no churn). So the UI vocab should be "Schedule" across the board; the card title "Responsibilities" is the outlier.

For Scene 2 to be recordable as-scripted ("click Schedules"), one of two paths:

- **(a)** Rewrite the runbook to say "click the Responsibilities card on Overview" — cheap, but contradicts the locked vocab memory.
- **(b)** Hoist the `DutiesCard` into its own Schedules tab in the tab strip + rename the card heading to "Schedules" + keep duty-vocab in code.

Recommend (b) — aligns with locked vocab.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F8 [P1] — Worker profile tab strip has no "Schedules" tab (vocab + structure mismatch with runbook)

## Fix recipe

1. **Rename `DutiesCard` title** at `web/components/skycot/worker/DutiesCard.tsx:101` from "Responsibilities" → "Schedules". One-word swap.
2. **Add a Schedules tab to the worker profile.** In `web/components/skycot/WorkerTabs.tsx:19-27`, add `"schedules"` to `WORKER_TAB_IDS` between `overview` and `activity`. The tab renders the existing `DutiesCard` (or a richer schedules view if scope allows). Overview tab retains a summarized schedules preview (collapsible 3-row teaser with "view all" link to the new tab).
3. **Coordinate with existing brief** `fix-duties-card-vocab-responsibilities-to-schedules` — verify scope overlap; this brief covers the tab-strip addition + card rename; the other brief may cover the vocab-only portion. If overlap is high, mark this brief as superseding or vice versa.
4. **Update routing.** `/workers/{id}/schedules` route either renders the dedicated tab or aliases `/workers/{id}/overview#schedules`. Pick whichever matches the existing route shape per `web/app/(app)/workers/[workerId]/[tab]/`.
5. **Update the runbook narration target.** `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 2 — verify "click Schedules" lands on a real tab.
6. **Vitest pin** asserts the tab strip renders "Schedules" between Overview and Activity.

## Acceptance criteria

- [ ] `WorkerTabs.tsx` exports `WORKER_TAB_IDS` containing "schedules" between "overview" and "activity".
- [ ] `DutiesCard` heading renders "Schedules" not "Responsibilities".
- [ ] `/workers/{id}/schedules` route exists and renders the schedules surface.
- [ ] Scene 2 narration "click Schedules" has a working target.
- [ ] Code/SQL/file/brief names keep `duty`/`duties` (per operator vocab memory).
- [ ] Vitest pin asserts tab strip composition.

## May-touch

- `web/components/skycot/WorkerTabs.tsx:19-39` (tab strip)
- `web/components/skycot/worker/DutiesCard.tsx:101` (card title)
- `web/app/(app)/workers/[workerId]/[tab]/page.tsx` (route handling for the new tab)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Scene 2 narration target verification)
- `web/components/skycot/__tests__/WorkerTabs.test.tsx` (new or extend)

## Out of scope

- Renaming `DutiesCard` filename + code symbols (operator vocab memory says code stays "duty")
- Making cadence-text chips interactive (covered by `fix-worker-dynamic-tab-placeholder-strip-brief-id-leak` or similar)
- The `fake-mutation` duty-toggle (already resolved per audit's "Resolved since" section)

## Effort

S-M — ~80-150 LOC across tab strip + route + tests + runbook update. ~2-3 hours. P1 because Scene 2 of the v1.0 demo MP4 narrates "click Schedules" and today the user has no Schedules tab to click — the runbook is misaligned with the product.
