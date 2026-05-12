---
name: flag_risk_factors
version: "0.1.0"
description: "Surfaces patterns in intake answers that need clinician pre-review — medication interactions, recent ER visits, abnormal vitals. Routes for clinician review only; never makes clinical judgements."
provenance:
  source: vertical/healthcare-intake
  hash: sha256-pending
---

# When to use

Call when intake answers contain any pattern that warrants a
clinician's eyes before the appointment. Pair with
`triage_rubric.skill.md`: that skill handles red-flag *symptoms*
needing immediate routing; this skill handles slower-burn risk
factors that should be reviewed before the visit but don't trigger
911. When both apply, run `triage_rubric` first.

# Inputs you expect

- `patient_id` (pointer)
- `intake_answers` (structured form responses — vitals, medications,
  recent care events, allergies)
- `patient_history[]` and `risk_flags[]` (prior flags for de-dup)

# Output shape

```json
{
  "patient_id": "<id>",
  "flags": [
    {
      "flag_type": "medication_interaction|recent_er|high_risk_condition|abnormal_vital|other",
      "severity": "info|advisory|urgent",
      "summary": "<one-line, non-clinical phrasing>",
      "source": "intake_form|phone|portal_message",
      "routing": "clinician_review_queue"
    }
  ],
  "clinical_judgement_made": false,
  "approval_required": true,
  "approval_status": "pending"
}
```

# Examples

Patient reports a new SSRI + an OTC supplement (St. John's Wort)
on the intake form →

```json
{
  "patient_id": "p_0421",
  "flags": [
    {"flag_type": "medication_interaction", "severity": "advisory", "summary": "new prescription + OTC supplement reported on intake; clinician review recommended before visit", "source": "intake_form", "routing": "clinician_review_queue"}
  ],
  "clinical_judgement_made": false,
  "approval_required": true,
  "approval_status": "pending"
}
```

# Rules

- **Never make a clinical claim.** "Patient has uncontrolled
  hypertension" is wrong; "blood pressure values trend high;
  clinician review recommended" is right. `summary` is workflow-
  routing language, not medical opinion.
- **3-level severity is routing, not medical urgency.** `info`
  no-SLA awareness; `advisory` review before the visit; `urgent`
  same-day review, intake holds. Medical-danger judgement is the
  clinician's.
- **De-dup against `risk_flags[]`.** Identical flags within 30
  days collapse to one row with a count unless severity rose.
- **No EMR write, no patient comms.** Routes to the clinician-
  review queue only.

# PHI safety

- `summary` is short and non-clinical — no diagnoses, no lab
  values, no medications beyond what the patient volunteered.
  Flag rows persist as pointers + summary; PHI stays at DEBUG
  in logs.
- Synthetic data only. Real PHI halts the skill and emits
  `phi_detected: true` upstream to `update_patient_record`.
- `clinical_judgement_made: false` is asserted in every output —
  a guardrail invariant. Avery is non-clinical by charter.
