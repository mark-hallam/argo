---
name: schedule_appointment
version: "0.1.0"
description: "Proposes 3 candidate appointment slots from operator-provided clinician availability. Honors duration-by-visit-type and a 10-minute no-back-to-back buffer. Approval-gated — never books autonomously."
provenance:
  source: vertical/healthcare-intake
  hash: sha256-pending
---

# When to use

Call when a patient requests an appointment AND the triage rubric
has cleared the contact (no red-flag pattern from
`triage_rubric.skill.md`). If `triage_rubric` flagged the contact,
do NOT call this skill — route to `flag_risk_factors` instead and
wait for clinician acknowledgement before resuming.

# Inputs you expect

- `patient_id` (required — pointer to `inventory/sample_patients.json`,
  never raw PHI fields like DOB/SSN)
- `visit_type` (e.g. `annual_physical`, `follow_up`, `urgent_care`,
  `lab_only`)
- `preferred_window` (date range + preferred time-of-day)
- `provider_preference` (named clinician or `any`)
- `clinician_availability[]` (operator-provided slot list)

# Output shape

```json
{
  "patient_id": "<id>",
  "visit_type": "<type>",
  "duration_minutes": 20,
  "candidates": [
    {"provider": "<name>", "slot_iso": "<RFC3339>", "rationale": "<why this fits>"}
  ],
  "buffer_minutes": 10,
  "approval_required": true,
  "approval_status": "pending"
}
```

# Examples

A patient requesting a 30-min annual physical with Dr. Patel,
Tuesday–Friday morning preferred →

```json
{
  "patient_id": "p_0421",
  "visit_type": "annual_physical",
  "duration_minutes": 30,
  "candidates": [
    {"provider": "Dr. Patel", "slot_iso": "2026-05-19T09:00:00-04:00", "rationale": "first morning slot in preferred window"},
    {"provider": "Dr. Patel", "slot_iso": "2026-05-21T10:30:00-04:00", "rationale": "second-choice morning slot"},
    {"provider": "Dr. Patel", "slot_iso": "2026-05-22T09:30:00-04:00", "rationale": "Friday morning fallback"}
  ],
  "buffer_minutes": 10,
  "approval_required": true,
  "approval_status": "pending"
}
```

# Rules

- **Exactly 3 candidates.** If fewer than 3 slots fit constraints,
  emit what fits and set `"insufficient_availability": true` with
  the missing-constraint reason.
- **No back-to-back booking.** Enforce the 10-min buffer against
  `clinician_availability[]` — a slot that abuts an existing
  appointment within 10 minutes is ineligible.
- **Duration by visit-type.** Use clinic defaults: `lab_only` 10m,
  `follow_up` 20m, `annual_physical` 30m, `urgent_care` 20m.
  Operator may override per-request.
- **Approval-gated.** `approval_required: true` is always set;
  this skill proposes, the operator confirms before
  `appointment_history[]` records a booking.

# PHI safety

- Inputs are pointers (`patient_id`); refuse to operate on raw
  name + DOB. Rationale text carries only logistical reasoning —
  no diagnoses, medications, or lab values.
- Synthetic data only. Real PHI upstream halts the skill and
  emits `phi_detected: true` to `update_patient_record`.
- Approval-gating is non-negotiable: this skill never auto-books.
