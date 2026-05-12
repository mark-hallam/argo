---
name: draft_return
version: "0.1.0"
description: Draft a 1040 (plus required schedules) from gathered docs and approved deductions. Outputs a structured form-line JSON plus a Markdown working paper for the licensed preparer. Approval-gated; never an IRS-ready return.
provenance:
  source: vertical/tax-prep
  hash: sha256-pending
---

# When to use

Use after `review_w2` and `flag_deduction_eligibility` have run AND
every proposed deduction is in state `approved` or `rejected`. The
output is a preparer working paper, not a filing. Approval-gated
because tax calculation is licensed-judgment territory.

# Inputs you expect

- `client_id`, `return_year`, `filing_status`
  (`single | mfj | mfs | hoh | qss`).
- `w2_entries` — list of rows from `w2_data` memory.
- `form_1099_summary` — list of `{kind, payer_ein, amount}` (NEC,
  INT, DIV, R, etc.).
- `approved_deductions` — only those marked `approved` upstream.
- `prior_year_return` — optional pointer for the diff section.

# Output shape

```json
{
  "client_id": "synthetic-001",
  "return_year": 2025,
  "filing_status": "single",
  "form_lines": {
    "1040_line_1a_wages":         82500.00,
    "1040_line_2b_taxable_int":     142.00,
    "1040_line_8_other_income":       0.00,
    "1040_line_10_adjustments":    3000.00,
    "1040_line_11_agi":           79642.00,
    "1040_line_12_deduction":     14600.00,
    "1040_line_15_taxable":       65042.00,
    "1040_line_16_tax":           10024.00,
    "1040_line_22_total_tax":      9824.00,
    "1040_line_25a_withholding":   9210.00,
    "1040_line_34_refund":           0.00,
    "1040_line_37_owe":             614.00
  },
  "schedules_used": ["Schedule 1", "Schedule B"],
  "draft_markdown_path": "<workspace>/.skycot/tax-prep/returns/synthetic-001-2025-draft.md",
  "what_changed_since_last_year": [
    "Filing status: mfj → single (divorce 2025-03).",
    "Added Schedule B (interest > $1,500 threshold)."
  ],
  "what_we_couldnt_determine": [
    "1099-NEC $4,200 lacks expense records — Schedule C deferred."
  ],
  "next_action": {
    "name": "send_draft_for_preparer_review",
    "requires_approval": true,
    "auto_file_allowed": false
  }
}
```

# Approval posture

**Approval-gated.** `requires_approval: true` is load-bearing — the
draft never reaches the client or IRS without the preparer signing
the working paper. `auto_file_allowed: false` is also load-bearing;
`file_return` re-checks the gate independently.

# Hard rules

- **SSN: initials + last-4 only.** Markdown body never contains a
  full SSN; the form-line JSON omits SSN entirely.
- **Approved deductions only.** Skip `proposed` / `rejected`; if a
  required category is still `proposed`, add it to
  `what_we_couldnt_determine` and continue.
- **No state returns.** Federal 1040 only in v1.
- **No business returns beyond Schedule C.** 1120 / 1065 escalate.
- **Never marks `status = filed`.** Only `file_return` does that.
- **Footer disclaimer mandatory.** Markdown ends with: `DRAFT — for
  preparer review. Not an IRS-ready return.`

# Examples

Synthetic — `synthetic-001`, single filer, one W-2 ($82.5k wages),
$142 interest, $3,000 traditional-IRA deduction approved. Draft
shows $614 owed. Preparer signs the working paper; `file_return`
then gates on the 8879 authorization.
