# fix-duties-card-vocab-responsibilities-to-schedules

**Owner**: claude.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-duties-card-vocab-responsibilities-to-schedules`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/scene-1-and-2-claim-1-and-2.md` F6 [P1].

🟠 **P1 — `DutiesCard` (the schedules-card on every worker overview, camera-visible during Scene 2) still says "Responsibilities" as its CardTitle and "Add responsibility" as its CTA button. The aria-label on each row's toggle correctly says "Enable schedule" / "Disable schedule" — so the same component contradicts itself between visible label and a11y. The `AddResponsibilityDialog` (mounted by the same card) also keeps the wrong terminology in its copy. Operator-locked memory `project_vocabulary_duty_vs_schedule` (2026-05-23) says UI copy = "Schedule", code = "Duty". This brief is the production-UI relabel for the DutiesCard component, distinct from the prior playwright-spec sweep brief.**

## Root cause

`web/components/skycot/worker/DutiesCard.tsx`:

- Line 89: `<CardTitle>Responsibilities</CardTitle>` — visible header on the worker-overview Schedules card.
- Line 121: button label `"Add responsibility"` — visible CTA on the same card.
- Line 215: `aria-label={enabled ? "Disable schedule" : "Enable schedule"}` — correct.

`web/components/skycot/worker/AddResponsibilityDialog.tsx` — the dialog mounted via line 125-133 of DutiesCard. Title, body copy, button labels all use "Responsibility" / "Responsibilities" instead of "Schedule" / "Schedules".

Memory `project_vocabulary_duty_vs_schedule` (operator-locked 2026-05-23):
> "in any SPA / hire wizard / marketing / docs / settings copy, use 'Schedule' not 'Duty'; code/SQL/audit/brief filenames keep 'duty' (no churn)."

Adjacent briefs partially cover the area:
- `fix-playwright-duty-to-schedule-vocab-completion` — addresses `/settings`, `/workflows`, `/cohorts` page copy. Does NOT touch DutiesCard.tsx.
- `fix-runbook-scene-2-personality-card-vs-tab-vocab` — addresses runbook narration alignment, not the component copy.
- `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak` — addresses the toggle behavior + toast brief-ID leak, not the CardTitle.

This brief is the missing piece: the DutiesCard component itself plus the AddResponsibilityDialog rename. The component is mounted on every worker overview page — the most camera-visible "Responsibilities" string in the product.

Note: "Responsibilities" is also semantically not-quite-wrong — schedules can include both responsibilities (ongoing) and one-off duties — but operator's vocab lock prioritises the single canonical word "Schedule" for v1.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F6 [P1] — DutiesCard title violates operator vocab lock (says "Responsibilities" / "Add responsibility", not "Schedules")

## Fix recipe

1. **DutiesCard.tsx — rename visible strings**:
   - Line 89: `<CardTitle>Responsibilities</CardTitle>` → `<CardTitle>Schedules</CardTitle>`
   - Line 121: button label `"Add responsibility"` → `"Add schedule"`
   - Audit other strings in the file for "responsibility/responsibilities" — replace all visible occurrences with "schedule/schedules".
2. **AddResponsibilityDialog.tsx — rename visible strings**:
   - Dialog title: `"Add a responsibility"` → `"Add a schedule"`
   - Field labels referencing "responsibility" → "schedule"
   - CTA button label → `"Add schedule"`
   - Rename the COMPONENT file + export to `AddScheduleDialog.tsx` for code-side consistency. Update the import in DutiesCard.tsx.
3. **Sweep other DutiesCard-adjacent components**:
   - `grep -rn 'responsibilit' web/components/skycot/worker/` → audit each match.
   - Any "Responsibility" in user-visible JSX → rename to "Schedule".
   - Any "responsibility" in user-visible JSX → rename to "schedule".
4. **DO NOT touch**:
   - File names containing `Duty` / `duty` in code paths
   - Type names like `Duty`, `DutyId`
   - API endpoint paths like `/api/v1/workers/<id>/duties/`
   - SQL columns / migration names
   - Test fixtures using "duty" in code-level identifiers
5. **Vitest**: snapshot the DutiesCard render — assert no visible string contains "Responsibility" or "responsibility".
6. **Playwright**: extend the `no-duty-copy` spec set to assert `/workers/<id>/overview` does not contain "Responsibility" / "Responsibilities" in visible DOM.

## Acceptance criteria

- [ ] `/workers/<id>/overview` shows "Schedules" as the card title, not "Responsibilities".
- [ ] The "Add" CTA reads "Add schedule".
- [ ] The Add dialog header + body + CTA all use "Schedule" terminology.
- [ ] Vitest snapshot pin asserts no "Responsibility" string in DutiesCard render output.
- [ ] Playwright spec covers the worker-overview Schedules card vocabulary.
- [ ] Code-side identifiers (`Duty`, `DutyId`, `duties` table, etc.) are unchanged.

## May-touch

- `web/components/skycot/worker/DutiesCard.tsx:89, 121` (and any other visible-string sites)
- `web/components/skycot/worker/AddResponsibilityDialog.tsx` → rename to `AddScheduleDialog.tsx`; rewrite visible strings
- Any other consumer of `AddResponsibilityDialog` (import paths)
- Vitest fixture under `web/components/skycot/worker/`
- Playwright spec under `web/playwright/worker-overview/`

## Out of scope

- Settings / workflows / cohorts page vocab (covered by `fix-playwright-duty-to-schedule-vocab-completion`).
- Toggle PATCH wiring + brief-ID leak (covered by `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak`).
- Schedules tab promotion to top-level worker tab (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end`).
- Runbook narration alignment (covered by `fix-runbook-scene-2-personality-card-vs-tab-vocab`).
- API endpoint path renames (code-side stays "duty" per operator vocab lock).

## Effort

~30-80 LOC across two component files + 1 vitest + 1 playwright spec. ~1 hour. P1 because DutiesCard is on every worker overview = camera-visible during Scene 2; vocab mismatch is on-screen text the operator does not control.
