---
name: flag_amendment_opportunity
version: "0.1.0"
description: Review a prior-year return for amendment opportunities (missed deductions, life-event timing, retroactive credits). Outputs an amendment envelope gated on the 3-year IRS statute of limitations. Approval-gated; never auto-files Form 1040-X.
provenance:
  source: vertical/tax-prep
  hash: sha256-pending
---

# When to use

Use when the operator hands over a prior-year return (current + 3
prior years in scope) and asks the assistant to scan for amendment
candidates. Outputs an envelope per candidate. Approval-gated
because filing Form 1040-X is client- and IRS-facing. Tax preparers
leave ~20% of legal refund money on the table by not amending; this
is the vertical's highest-value autonomous-discovery surface.

# Inputs you expect

- `client_id`.
- `prior_return` — `{form, return_year, filed_at (ISO-8601),
  filing_status, form_lines, deduction_candidates}` for the year
  being reviewed.
- `new_evidence` — new docs, life events, or rubric matches that
  weren't reflected when the return was filed.
- `current_date` — for the 3-year SOL window calculation.

# Output shape

```json
{
  "client_id": "synthetic-001",
  "prior_form": "1040",
  "prior_return_year": 2023,
  "filed_at": "2024-04-12T14:32:11Z",
  "sol_window": {
    "deadline": "2027-04-15",
    "within_3_year_sol": true,
    "days_remaining": 338
  },
  "amendment_envelope": {
    "change_category": "deduction",
    "rubric_hit": "student_loan_interest",
    "evidence_summary": "Form 1098-E for tax year 2023, $1,840 interest, surfaced 2026-04-30.",
    "estimated_refund_change_usd": 405.00,
    "form_1040x_lines_affected": ["line_10_adjustments", "line_11_agi", "line_22_total_tax"]
  },
  "next_action": {
    "name": "draft_form_1040x",
    "requires_approval": true,
    "auto_file_allowed": false
  },
  "flags": []
}
```

# Approval posture

**Approval-gated.** `requires_approval: true` and
`auto_file_allowed: false` are both load-bearing — filing an
amendment is a client- and IRS-facing effect. The licensed preparer
must sign the 1040-X working paper and the client must re-authorize
via 8879 before any e-file path runs.

# Hard rules

- **3-year SOL gate is mandatory.** IRS amendment deadline is
  **3 years from return's filed date OR 2 years from tax-paid
  date, whichever is later**. If `current_date > filed_at + 3y` AND
  no extended-payment rationale applies, set `within_3_year_sol:
  false`, `flags: ["sol_expired"]`, and refuse to emit an envelope.
- **One envelope per change.** Never bundle multiple changes.
- **Change categories are fixed.** `deduction`, `credit`, `income`,
  `status`. Anything else → `flags: ["unknown_category"]`.
- **Estimate is an estimate.** `estimated_refund_change_usd` is the
  skill's estimate; final 1040-X line math is the preparer's call.
- **No protected-class signals** in the rationale — same rule as
  `flag_deduction_eligibility`.
- **Synthetic data only in examples.** SSN `123-45-6789`.

# Examples

Synthetic — `synthetic-001` filed 2023 return on 2024-04-12; on
2026-04-30 the client surfaces a 1098-E for $1,840 of student-loan
interest that was missed at filing. SOL deadline 2027-04-15 = 338
days remaining → within window. Envelope estimates ~$405 refund
change. Preparer reviews, drafts 1040-X, client re-authorizes 8879.
