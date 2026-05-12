---
name: send_pre_visit_prep
version: "0.1.0"
description: "Drafts a pre-visit prep email from operator-provided visit-type templates. Never fabricates clinic-specific policy — defers to front-desk when a detail is missing. Approval-gated before any send."
provenance:
  source: vertical/healthcare-intake
  hash: sha256-pending
---

# When to use

Call after `schedule_appointment` is approved AND, where relevant,
`verify_insurance_benefits` has cleared the visit-cost language.
Skip if the appointment was cancelled or if the patient opted out
of email comms (check `patient_history[]`).

# Inputs you expect

- `appointment_id` (pointer to `appointment_history[]`)
- `visit_type` (drives the template branch)
- `prep_template` (operator-provided text — clinic-specific policy
  belongs here, never invented by the skill)
- `patient_id` (pointer; never raw name/DOB in the draft body)

# Output shape

```json
{
  "appointment_id": "<id>",
  "visit_type": "<type>",
  "recipient_token": "<patient_email pointer>",
  "subject": "<short, specific>",
  "body": "<draft>",
  "missing_details": ["<what the template did not cover>"],
  "approval_required": true,
  "approval_status": "pending"
}
```

# Examples

Annual physical, template covers fasting + parking but not the
visitor-policy question the patient asked →

```json
{
  "appointment_id": "appt_8821",
  "visit_type": "annual_physical",
  "recipient_token": "p_0421.email",
  "subject": "Prep for your annual physical on Tue May 19",
  "body": "Hi — we'd love to get you settled in for your annual physical on Tuesday morning. Please fast for 8 hours before the visit (water is fine). Parking is in the south lot; the front desk will validate. If anything changes before your visit, give us a call.",
  "missing_details": ["visitor policy (not in operator template — please advise patient to call the front desk)"],
  "approval_required": true,
  "approval_status": "pending"
}
```

# Rules

- **Never fabricate clinic policy.** Anything not in the
  operator's template lands in `missing_details` and the draft
  defers to the front desk. Inventing policy is a guardrail
  violation.
- **Voice from `persona.toml`.** Open warm, lead with patient
  name + appointment specifics, close with the "If anything
  changes…" phrasing. Avoid `avoid_phrasing` ("Per your request",
  "Final notice").
- **No clinical content.** Diagnoses, lab values, medications,
  allergy lists never appear in email bodies (load-bearing rule
  from `system_prompt.md`). One CTA only.

# PHI safety

- Body carries only logistical prep — never diagnoses,
  medications, allergy lists, or lab values.
- `recipient_token` is a pointer; the operator resolves it to a
  real address at send-time, post-approval.
- Synthetic data only. Real PHI patterns (real name + real email
  combo) halt the skill and emit `phi_detected: true` upstream
  to `update_patient_record`. Approval-gated end-to-end.
