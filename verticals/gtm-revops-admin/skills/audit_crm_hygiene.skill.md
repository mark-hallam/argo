---
name: audit_crm_hygiene
version: "0.1.0"
description: Audit a CRM scope for duplicates, missing-field gaps, and stale records, citing the source pointer for every flagged record — never inventing a record, a value, or a count. Proposes fixes; never mutates a record itself.
provenance:
  source: vertical/gtm-revops-admin
  hash: sha256-pending
---

# When to use

Use when the operator wants the customer-relationship-management system
(CRM — the database of leads, contacts, and deals) checked for mess:
duplicate records, missing required fields, and stale records that haven't
moved in too long. This is Aria's anchor skill. It produces a structured
audit report with proposed fixes — it never applies a fix itself. Every
record mutation (merge, edit, reassign, delete) is `update_crm_record`,
which `requires_approval: true`; this skill only flags and proposes.

# Inputs you expect

- `scope` — which slice of the CRM to audit (e.g. a pipeline stage, an
  owner, a date range, or "all open deals").
- `crm_records` — the records in scope, each with a `source_pointer`.
  Pulled from the CRM connector when wired, from operator paste-ins or
  memory pointers otherwise. Any record without a source is treated as
  unavailable — never invented.
- `staleness_days` — how many days of no activity counts as stale
  (optional; default per the operator's policy).

# Output shape

```json
{
  "audit": {
    "scope": "all open deals",
    "records_scanned": 0,
    "data_quality_pct": "unavailable",
    "duplicates": [
      { "record_ids": ["rec_123", "rec_456"], "match_on": "email",
        "source_pointers": ["crm:rec_123", "crm:rec_456"],
        "proposed_fix": "merge", "requires_approval": true }
    ],
    "gaps": [
      { "record_id": "rec_789", "missing_fields": ["owner", "stage"],
        "source_pointer": "crm:rec_789",
        "proposed_fix": "fill owner + stage", "requires_approval": true }
    ],
    "stale": [
      { "record_id": "rec_321", "days_inactive": 0,
        "source_pointer": "crm:rec_321",
        "proposed_fix": "reassign or close", "requires_approval": true }
    ]
  }
}
```

`records_scanned` and `data_quality_pct` are sourced counts or
`"unavailable"` — never estimated. Every flagged record carries its
`source_pointer`. Every `proposed_fix` is a proposal: it sets
`requires_approval: true` because applying it is an `update_crm_record`
mutation, which the operator must approve.

# Hard rules — truth boundary (NO THEATER)

- **Never invent a record, a value, or a count.** If a record or figure is
  not in the source system or in memory, mark it **`"unavailable"`** —
  never estimate, round to a plausible value, or fill a gap. "unavailable"
  is a correct answer; a made-up record is not.
- **Cite every flagged record.** Each flagged record carries a
  `source_pointer`. A flag with no source pointer is not a finding.
- **Thin evidence stays low-confidence.** A single stale field is not a
  "data crisis". Report what the evidence supports and no more; never
  inflate sparse flags into a conclusion.
- **Audit proposes; it never mutates.** Every `proposed_fix` sets
  `requires_approval: true`. The fix only happens via `update_crm_record`,
  which stops at the operator. **Never delete a record — deletion is a
  mutation and is never done unilaterally.**
- **Never move money.** Billing context is read-only here; any real money
  action belongs to `triage_billing_question` and escalates to the
  operator.

# Where each finding routes

- duplicate / gap / stale fix → `update_crm_record` (medium, approval-gated)
- a meeting to chase a stale deal → `schedule_meeting` (medium, approval-gated
  — it emails a real calendar invite to the attendee)
- a routing-rule gap surfaced by the audit → `draft_lead_routing` (low,
  autonomous — a draft, not an applied rule)
- a billing question raised on a record → `triage_billing_question` (low,
  autonomous to draft; any money action escalates to the operator)

# Example

Synthetic only — no real CRM data.

```
scope: "all open deals"
crm_records: [
  "crm:rec_123 (Acme) — email acme@example.com, owner null",
  "crm:rec_456 (Acme) — email acme@example.com, owner Dana",
  "crm:rec_789 (Globex) — stage null, owner null"
]
staleness_days: 30
```

```json
{
  "audit": {
    "scope": "all open deals",
    "records_scanned": 3,
    "data_quality_pct": "unavailable",
    "duplicates": [
      { "record_ids": ["rec_123", "rec_456"], "match_on": "email",
        "source_pointers": ["crm:rec_123", "crm:rec_456"],
        "proposed_fix": "merge into rec_456 (has owner)",
        "requires_approval": true }
    ],
    "gaps": [
      { "record_id": "rec_789", "missing_fields": ["owner", "stage"],
        "source_pointer": "crm:rec_789",
        "proposed_fix": "fill owner + stage",
        "requires_approval": true }
    ],
    "stale": []
  }
}
```

`data_quality_pct` is `"unavailable"` here because the total-record
denominator wasn't sourced — Aria does not invent the percentage. Every
proposed fix routes through `update_crm_record` for the operator's
approval; nothing is merged, edited, or deleted by this skill.
