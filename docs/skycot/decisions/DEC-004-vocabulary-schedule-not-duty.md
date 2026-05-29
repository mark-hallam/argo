# DEC-004 — User-facing vocabulary: "Schedule" not "Duty"

- **Date**: 2026-05-23
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
In any SPA / hire wizard / marketing / docs / settings copy, use "Schedule"
not "Duty". Code, SQL, audit kinds, and brief filenames keep "duty" (no churn).

## Rationale
"Duty" reads wrong to operators; "Schedule" is the operator-facing mental
model. Renaming internal identifiers would be needless churn + migration risk.

## How to apply
Visible copy = "Schedule" / "Schedules" / "Add schedule". Internal
identifiers (`WorkerDuty`, `/duties` routes, `duty/` dirs, testids) stay "duty".
