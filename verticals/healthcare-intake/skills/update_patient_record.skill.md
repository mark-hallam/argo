---
name: update_patient_record
version: "0.1.0"
description: "Logs intake-flow actions (appointment booked, prep sent, insurance verified, flags raised) to memory with an audit trail. Halts and emits phi_detected=true if real-PHI patterns appear in inputs."
provenance:
  source: vertical/healthcare-intake
  hash: sha256-pending
---

# When to use

Call at the end of each intake action (after operator approval) to
persist the action to memory. Also called when a downstream skill
detects PHI patterns and needs to halt — in that case the skill
records the halt and emits `phi_detected: true` rather than
persisting the underlying action.

# Inputs you expect

- `patient_id` (pointer; never a name/DOB combo)
- `action_type` — `appointment_booked` | `prep_sent` |
  `insurance_verified` | `flag_raised` | `record_corrected`
- `payload` (pointers to artifact rows — appointment_id,
  message_id, flag id — never raw clinical data)
- `approved_by` (operator handle from the approval record)

# Output shape

```json
{
  "patient_id": "<id>",
  "action_type": "<type>",
  "memory_target": "appointment_history|prep_sends|insurance_status|risk_flags|record_updates",
  "row": { "<field>": "<pointer or non-clinical value>" },
  "applied_at": "<RFC3339>",
  "approved_by": "<operator handle>",
  "phi_detected": false
}
```

# Examples

Appointment booked → row appended to `appointment_history[]`:

```json
{
  "patient_id": "p_0421",
  "action_type": "appointment_booked",
  "memory_target": "appointment_history",
  "row": {"appointment_id": "appt_8821", "provider": "Dr. Patel", "slot_iso": "2026-05-19T09:00:00-04:00", "duration_minutes": 30, "visit_type": "annual_physical", "status": "booked", "source_ref": "schedule_skill_run_3f1"},
  "applied_at": "2026-05-12T14:05:00-04:00",
  "approved_by": "operator:mhall",
  "phi_detected": false
}
```

PHI sniff trips on a real-looking SSN in inputs → halt, persist
nothing:

```json
{"patient_id": "unknown", "action_type": "record_corrected", "row": {"halt_reason": "PHI pattern detected in inputs"}, "applied_at": "2026-05-12T14:07:00-04:00", "approved_by": "n/a", "phi_detected": true}
```

# PHI safety — PHI-fence (load-bearing)

Before persisting, scan inputs against this regex set. Any match
halts the skill, persists nothing, and emits `phi_detected: true`:

- SSN: `\b\d{3}-\d{2}-\d{4}\b`
- DOB-paired-with-name: ISO/US date within 32 chars of a
  `Firstname Lastname` token pair
- Medical record numbers outside the synthetic `MRN-\d{6}` shape
- Phone numbers outside the synthetic `555-01\d\d` reserved range
- Emails outside `*@example.com` / `*@example.org`

If `argo-redact` later exposes a public PHI scanner, switch to
that helper; until then, the regex set above is authoritative.

Other PHI rules:

- **Pointers only.** Rows carry artifact ids — never free-text
  clinical content, never raw PHI (per `memory_schema.toml`).
- **Audit trail required.** Every row records `applied_at` and
  `approved_by`; missing-either fails validation.
- **No clinical edits.** Allergy removals, medication changes,
  diagnosis edits refuse and route to `flag_risk_factors`. Only
  non-clinical corrections (contact, name, insurance card).
- **Approval-gated end-to-end.** Autonomous writes are forbidden;
  `phi_detected: true` is a structured halt operators triage,
  never auto-acked.
