---
name: review_w2
version: "0.1.0"
description: Extract structured line items from an operator-attached W-2 image or PDF. Outputs JSON with per-field extraction confidence. Autonomous per charter; low-confidence fields route to operator review.
provenance:
  source: vertical/tax-prep
  hash: sha256-pending
---

# When to use

Use when the operator attaches a W-2 (image or PDF) for a client's
return. Pre-`draft_return` step. The extracted line items feed the
`w2_data` memory entry keyed by `(client_id, return_year,
employer_ein)`. Autonomous per the charter — the data sits on the
operator's machine; nothing leaves the workspace.

# Inputs you expect

- `client_id` — opaque ID for the audit trail.
- `return_year` — four-digit tax year (e.g. `2025`).
- `w2_source` — path to the attached W-2 image or PDF on disk.
- `extraction_engine_output` — operator-provided OCR/parse output
  (this skill does not pick an OCR engine; it consumes whatever the
  operator wired up).

# Output shape

```json
{
  "client_id": "synthetic-001",
  "return_year": 2025,
  "employer_ein": "12-3456789",
  "fields": {
    "wages_box1":            { "value": 82500.00, "extraction_confidence": 0.97 },
    "fed_income_tax_box2":   { "value":  9210.00, "extraction_confidence": 0.95 },
    "ss_wages_box3":         { "value": 82500.00, "extraction_confidence": 0.96 },
    "ss_tax_box4":           { "value":  5115.00, "extraction_confidence": 0.94 },
    "medicare_wages_box5":   { "value": 82500.00, "extraction_confidence": 0.96 },
    "medicare_tax_box6":     { "value":  1196.25, "extraction_confidence": 0.94 },
    "state_wages_box16":     { "value": 82500.00, "extraction_confidence": 0.91 },
    "state_tax_box17":       { "value":  4125.00, "extraction_confidence": 0.88 },
    "employee_ssn_last4":    { "value":     6789, "extraction_confidence": 0.99 }
  },
  "box12_codes": [{ "code": "D", "amount": 6000.00, "extraction_confidence": 0.93 }],
  "unexplained_box_diff": false,
  "flags": [],
  "next_action": {
    "name": "append_to_w2_data",
    "requires_approval": false
  }
}
```

# Approval posture

**Autonomous.** No client- or IRS-facing effect; the action only
writes to the operator's local `w2_data` memory entry. Low-confidence
fields (`extraction_confidence < 0.80`) are flagged for operator
review before downstream `draft_return` consumption; the skill does
not silently file a low-confidence wage figure.

# Hard rules

- **SSN: last-4 only.** Never extract or output a full SSN. Only the
  trailing four digits enter the JSON. Full SSN stays in the source
  file; downstream skills do not need it.
- **Confidence on every field.** Every numeric box carries
  `extraction_confidence` in `[0.0, 1.0]`. Any field below `0.80`
  appends to `flags: ["low_confidence_<field>"]`.
- **Box-1 vs Box-3 sanity.** If Box 1 (wages) and Box 3 (SS wages)
  differ by more than $5 and no Box-12 code (`D`/`401k`,
  `W`/`HSA`) explains the gap, set `unexplained_box_diff: true`.
- **No invention.** Missing employer EIN, missing SSN, or partial
  W-2 → set `flags: ["partial_w2"]` and stop. Do not fabricate.
- **Synthetic data only in examples.** Use the test SSN
  `123-45-6789` (last-4 `6789`) and a synthetic EIN `12-3456789`.

# Examples

```
w2_source: "<workspace>/.skycot/tax-prep/intake/synthetic-001-2025-w2.pdf"
extraction_engine_output: { box1: "82500.00", box2: "9210.00", ... }
```

Output is the JSON above; `append_to_w2_data` writes the row to the
`w2_data` memory entry and the workflow continues to
`flag_deduction_eligibility`.
