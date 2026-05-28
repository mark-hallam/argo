# rebase-fix-duties-card-vocab-responsibilities-to-schedules

Re-implement the original brief
`streams/fix-duties-card-vocab-responsibilities-to-schedules.md` against
current `origin/main` HEAD.

## Original PR

PR #2438 (closed for re-author) — `fix: DutiesCard + AddResponsibilityDialog
visible copy → 'Schedule'`, created 2026-05-27 against an older `main`.
Conflicted on:

- `web/tests/e2e/no-duty-copy.spec.ts` — main has been consolidated by
  #2602 (`13153e657`) which moved the positive complement to a
  worker-profile assertion and pruned the 6 failing 2026-05-26 cluster
  cases. The PR's extension of that file is incompatible with the new
  consolidated shape.
- `web/components/skycot/worker/Overview.test.tsx` + multiple
  duty/* step files — main's shape evolved with the
  `WorkerSchedulesContent` adjacent surface that ships with its own
  Schedule vocabulary; the rename must be coordinated across both.

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-duties-card-vocab-responsibilities-to-schedules

## Bug is still load-bearing

Confirmed on `a5df90c41` — the worker-overview surface still ships
operator-facing "Responsibilities" copy. From
`web/components/skycot/worker/DutiesCard.tsx`:

- Line 101: `<CardTitle>Responsibilities</CardTitle>`
- Line 108: `No responsibilities yet.`
- Line 133: `Add responsibility`
- Line 88: `toastSuccess("Added the new responsibility.", { … })`

This violates the operator vocab lock
(`project_vocabulary_duty_vs_schedule`, 2026-05-23): UI = "Schedule",
code = "Duty". The Schedule lockup is already partly applied (the toggle
aria-label says "Enable schedule") but the card title + CTA + empty
state + success toast still leak the old vocabulary onto the worker
profile during demo recording.

## Acceptance (same as original brief)

- `DutiesCard` CardTitle: `Responsibilities` → `Schedules`
- `DutiesCard` CTA: `+ Add responsibility` → `+ Add schedule`
- `DutiesCard` empty-state copy + success-toast title
- `AddResponsibilityDialog` (file + export) → `AddScheduleDialog`
- All step children (`duty/Step1Capture.tsx` through
  `duty/Step4Conflict.tsx`) carry the rename through every visible
  string
- `WorkerOverviewContent` stub-toast announcement (`workerId=undefined`
  fallback branch)
- `WorkerPersonalityCard` default `greatAt` line
- `duties.ts` mock fallback `descriptionFor(recurring)` copy
- Code-side identifiers UNCHANGED: type names `WorkerDuty`, `DutyId`,
  `DutyProposal`, etc.; endpoint paths `/api/v1/workers/<id>/duties/...`;
  hook names `useDutyAdd`; React props `onAddDuty`, `onEditDuty`,
  `onToggle(dutyId, …)`; `data-testid` values prefixed `duty-*` /
  `duties-*`; SQL columns; audit kinds.

## Shape of main on touched files

- `web/tests/e2e/no-duty-copy.spec.ts` (current) is the
  post-consolidation shape: an 8-page forbidden-word sweep over
  `/settings, /workflows, /cohorts, /workers, /audit, /approvals,
  /skills, /hire` with a separate worker-profile positive assertion.
  The original PR extended the file with a `/workers/<sample>/overview`
  case + positive-`Schedules`/Add-schedule assertions; the rebase should
  add equivalents to the post-#2602 layout rather than re-introducing
  the pre-#2602 structure.
- `web/components/skycot/worker/Overview.test.tsx` (current) at lines
  178-245 has 3 assertions tied to `'+ Add responsibility'` /
  `Add responsibility is clicked` / "opens the AddResponsibilityDialog";
  these flip to `+ Add schedule` / `Add schedule is clicked` /
  "opens the AddScheduleDialog".

## Notes for the worker

- Rename `AddResponsibilityDialog.tsx` (+ `.test.tsx`) to
  `AddScheduleDialog.tsx` (+ `.test.tsx`) **with a git rename** so the
  history reads as a move, not a delete+add.
- Keep `data-testid="duty-*"` / `duties-*` selectors stable.
- The 14237-byte `AddResponsibilityDialog.test.tsx` has 23 existing
  tests; the test names update for "schedule" but the test scaffolding
  stays.
- Skip `workspace-clippy-fast-fail.sh` — no Rust touched.
