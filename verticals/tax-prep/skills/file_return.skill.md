---
name: file_return
version: "0.1.0"
description: Package an approved 1040 draft for IRS e-file submission. ALWAYS approval-gated — maps to the Form 8879 e-file authorization signature. Refuses if any line is low-confidence or any deduction is un-approved.
provenance:
  source: vertical/tax-prep
  hash: sha256-pending
---

# When to use

Use only after `draft_return` produced a working paper, the preparer
signed it, and the client signed **Form 8879** (the IRS e-file
authorization). Packages the return into a filing envelope; the
real IRS submission is a separate Tier-A connector in production.
**8879 is the load-bearing safety mapping** — no code path from
this skill bypasses it.

# Inputs you expect

- `client_id`, `return_year`.
- `draft_path` — markdown working paper from `draft_return`.
- `form_lines` — JSON form-line payload from `draft_return`.
- `form_8879_signed` (bool, must be `true`),
  `form_8879_signed_at` (ISO-8601 client signature timestamp).
- `preparer_signed` (bool, must be `true`).
- `bank_routing` / `bank_account` — optional; refund direct-deposit
  only, must derive from a written client record.
- `deduction_candidates` — full intake set (un-approved refusal).
- `field_confidences` — per-field confidences from upstream skills.

# Output shape

```json
{
  "client_id": "synthetic-001",
  "return_year": 2025,
  "filing_envelope": {
    "form_1040_lines": { "...": "..." },
    "schedules":       ["Schedule 1", "Schedule B"],
    "transmitter_receipt_placeholder": "<pending real e-file connector>",
    "submission_id_mock":               "mock-1234567890",
    "submitted_at":                     "2026-04-12T14:32:11Z"
  },
  "authorizations": {
    "form_8879_signed":    true,
    "form_8879_signed_at": "2026-04-11T19:05:00Z",
    "preparer_signed":     true
  },
  "next_action": { "name": "submit_to_irs_efile", "requires_approval": true },
  "refusal_reason": null,
  "flags": []
}
```

# Approval posture

**Always approval-gated.** `requires_approval: true` is
non-negotiable. The 8879 signature is the IRS-mandated client
authorization for e-file; without it, the return cannot lawfully be
transmitted. No memory entry, env var, or flag flips this.

# Hard rules — refusal cases

The skill refuses (sets `next_action.name: "refuse_filing"` and
populates `refusal_reason`) when **any** of these hold:

- `form_8879_signed` is not `true` — cite 8879 in `refusal_reason`.
- `preparer_signed` is not `true`.
- Any `field_confidences[<line>] < 0.80` (low-confidence line).
- Any `deduction_candidates[*].status` is `proposed`, or any
  `requires_approval: true` deduction is un-resolved.
- `bank_routing` / `bank_account` present without `written_record:
  true`.

Refusal is a clean state, not an error — never call the IRS API
speculatively. The operator resolves the gap and re-submits.

- **No real IRS call in v1.** Mock submission id + placeholder
  receipt; production swaps in the real e-file MCP connector.
- **Federal 1040 only** — no state filings.
- **Synthetic data only in examples.** Use SSN `123-45-6789`.

# Examples

Synthetic — `synthetic-001`, federal 1040, 8879 signed
2026-04-11, preparer signed, no low-confidence lines. Output is the
envelope above with a mock submission id. If 8879 were `false`:
`{"next_action": {"name": "refuse_filing"}, "refusal_reason":
"Form 8879 not on file — IRS e-file authorization required."}`.
