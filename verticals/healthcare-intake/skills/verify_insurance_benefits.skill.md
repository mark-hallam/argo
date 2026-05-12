---
name: verify_insurance_benefits
version: "0.1.0"
description: "Queries the mocked payer API for in-network status, coverage, deductible-remaining, copay, and referral requirement. Output is approval-gated — operator confirms before benefits land in patient comms or memory."
provenance:
  source: vertical/healthcare-intake
  hash: sha256-pending
---

# When to use

Call after `schedule_appointment` proposes candidates and BEFORE
`send_pre_visit_prep` drafts patient-facing language about cost.
The verified benefits row gates any copay or coverage statement —
without one, patient comms must not quote prices (a hard guardrail
in `persona.toml`).

# Inputs you expect

- `patient_id` (pointer; never raw DOB/SSN)
- `payer` (insurer name as carried on the patient's card)
- `member_id` (as-printed on card; treated as PHI)
- `visit_type` and `provider` (drives the network-check)
- `service_date` (the proposed appointment date)

# Output shape

```json
{
  "patient_id": "<id>",
  "payer": "<insurer>",
  "in_network": true,
  "coverage_status": "active|inactive|pending",
  "deductible_remaining_usd": 450,
  "copay_usd": 25,
  "referral_required": false,
  "verified_at": "<RFC3339>",
  "mock_data": true,
  "approval_required": true,
  "approval_status": "pending"
}
```

# Examples

Active PPO patient, in-network primary visit, $25 copay,
deductible partly met →

```json
{
  "patient_id": "p_0421",
  "payer": "BlueShield PPO (synthetic)",
  "in_network": true,
  "coverage_status": "active",
  "deductible_remaining_usd": 450,
  "copay_usd": 25,
  "referral_required": false,
  "verified_at": "2026-05-12T14:02:00-04:00",
  "mock_data": true,
  "approval_required": true,
  "approval_status": "pending"
}
```

# Rules

- **`mock_data: true` is mandatory** until a real payer connector
  lands. The flag is the seam — output shape stays stable when
  the real connector arrives, only the flag flips.
- **No quoting prices in patient comms** until `approval_status`
  flips to `approved` by the operator. Drafts that reference
  copay or deductible before approval are guardrail violations.
- **Referral logic is informational.** Surface
  `referral_required: true` to the operator; never tell the
  patient directly — rules vary by sub-plan and the mocked
  payer may be wrong.
- **Persist via `insurance_status[]` on approval only.** This
  skill never writes autonomously.

# PHI safety

- `member_id` is PHI — emit to the operator only, never into
  patient comms or calendar titles (titles carry `patient_id`
  only per `system_prompt.md`). Coverage details stay at DEBUG or
  below in logs; vertical_smoke audit-grep enforces this.
- Synthetic payers only in eval / demo. Real payer + real
  member-ID combos halt the skill and emit `phi_detected: true`
  upstream to `update_patient_record`.
- Approval-gated end-to-end: never commits or quotes to a
  patient autonomously.
