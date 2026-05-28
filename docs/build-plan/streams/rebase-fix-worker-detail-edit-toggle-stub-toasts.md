# rebase-fix-worker-detail-edit-toggle-stub-toasts

Re-implement the original brief
`streams/fix-worker-detail-edit-toggle-stub-toasts.md` against current
`origin/main` HEAD.

## Original PR

PR #2592 (closed for re-author) — `fix(worker-detail): wire duty Toggle
+ Edit to real gateway PATCH`, created 2026-05-27 against an older
`main`. At 635 additions / 18 deletions across 6 files including a new
`EditDutyDialog.tsx` component and a new `use-duties.ts` hook, the PR
size is past the manual-rebase threshold. Conflicted on:

- `web/components/skycot/worker/WorkerOverviewContent.tsx` — main has
  evolved since the PR was authored (operator-facing copy + the
  `WorkerSchedulesContent` adjacent surface). The PR's `onToggle` /
  `onEditDuty` wiring needs to land against the current shape.
- `web/components/skycot/worker/Overview.test.tsx` — main retains two
  anti-theater guards ("renders disabled" assertions) from the prior
  `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak`
  PR. The rebase flips those to "wires the real mutation / opens the
  dialog" assertions; needs careful merge with any newer assertions
  added since.
- `web/lib/api/duties.ts` — main hasn't grown `updateDuty` yet, but
  other duty helpers may have evolved.

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-worker-detail-edit-toggle-stub-toasts

## Bug is still load-bearing

Confirmed on `a5df90c41`:

- `EditDutyDialog.tsx` does NOT exist on main.
- `web/lib/hooks/use-duties.ts` does NOT exist on main.
- The gateway PATCH endpoint at
  `crates/argo-cli/src/handlers/gateway/routes_duties.rs::patch_response`
  has been live for some time, accepting `enabled` / `schedule`
  (5-field cron) / `label`.
- The DutiesCard toggle is rendered as visibly disabled per
  `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak`
  (see `DutiesCard.tsx` lines 25-37 `canToggleDuty` prop). The body
  click is a no-op. The operator cannot edit a worker's schedules
  from `/workers/<id>/overview` despite the gateway being ready.

## Acceptance (same as original brief)

- `web/lib/api/duties.ts` exports `updateDuty()` —
  `PATCH /api/v1/workers/<id>/duties/<duty-id>` with the same
  404-falls-back-to-mock contract as `commitDuty`.
- New `web/lib/hooks/use-duties.ts` exports `useUpdateDuty` +
  `useToggleDuty` (TanStack Query mutations) that write the
  gateway-returned duty into the worker-overview cache so the row
  re-renders with persisted state.
- New `web/components/skycot/worker/EditDutyDialog.tsx` — modal editor
  for label / 5-field cron / enabled, validated client-side against
  the same bounds the gateway uses (`MIN_LABEL_LEN=4..=100`, 5-field
  cron).
- `WorkerOverviewContent.tsx`: drops `canToggleDuty={false}`; wires
  `onToggle` + `onEditDuty` to the real mutations; mounts the
  `EditDutyDialog`. Mutation failures surface a destructive toast
  ("Couldn't save schedule. Try again.").
- `Overview.test.tsx`: flip the two anti-theater guards from "renders
  disabled" to "wires the real mutation / opens the dialog"; mock
  `updateDuty` so the test runs without a gateway.
- New `web/tests/e2e/edit-and-toggle-duty.spec.ts` Playwright spec
  that intercepts PATCH and asserts:
  1. Toggle fires PATCH with `{enabled:false}`.
  2. Body click opens the dialog.
  3. Save fires PATCH with the edited fields.
  4. The customer surface never contains an engineering brief-ID like
     `E4-h`.

## Shape of main on touched files

- The gateway PATCH endpoint lives at
  `crates/argo-cli/src/handlers/gateway/routes_duties.rs` (NOT
  `routes_worker_duties.rs` as the original brief named — that file
  was split after the brief was authored). PATCH endpoint accepts
  `enabled` / `schedule` / `label`.
- `web/components/skycot/worker/DutiesCard.tsx` (current) has
  `canToggleDuty` defaulting to `canEdit` for back-compat; the
  `WorkerOverviewContent.tsx` consumer passes `canToggleDuty={false}`
  explicitly. The rebase removes that explicit `false` and replaces
  the `onToggle` / `onEditDuty` props with the real mutation
  callbacks.
- The Playwright spec path is `web/tests/e2e/` (the lane per
  `playwright.config` `testDir`), NOT `web/playwright/worker-detail/`
  as the original brief named.

## Notes for the worker

- Coordinate with brief
  `rebase-fix-duties-card-vocab-responsibilities-to-schedules` — both
  touch `WorkerOverviewContent.tsx` + `Overview.test.tsx`. Land the
  vocab rebase FIRST (it has less surface conflict) so this brief
  inherits the schedule vocab in its toast strings ("Couldn't save
  schedule. Try again.").
- Brief-ID leaks: the test's assertion 4 (customer surface contains no
  `E4-h`) catches a real prior regression — keep it.
- The new `EditDutyDialog.tsx` must stay under 500 LOC.
- `tsc --noEmit` + `next lint` + `vitest run components/skycot/worker/Overview.test.tsx`
  + a representative Playwright run is the verification floor.
