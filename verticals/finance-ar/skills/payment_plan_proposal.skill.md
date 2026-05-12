---
name: payment_plan_proposal
version: "0.1.0"
description: "Proposes a 3-tier payment plan (minimal / balanced / aggressive) for an at-risk account. Each tier shows the per-installment math, due dates, and default consequence. JSON output; downstream approval required before any plan is offered to the customer."
provenance:
  source: vertical/finance-ar
  hash: sha256-pending
---

# When to use

Operator flags an account as at-risk — typically after a 31-60 or
61-90 bucket reminder went unanswered, or as the recommended action
from `escalate_past_due`. This skill builds the menu the operator
picks from; it does NOT pick a tier or send the offer.

# Inputs you expect

- `invoice_id` — the past-due invoice (or array, if bundling)
- `amount_outstanding_usd` — total to be financed
- `today` — used to compute installment due dates
- optional `interest_pct_annual` — finance charges (default 0.0)

# Output shape

A single fenced JSON block. Three tiers, fixed cadence, with the
math shown so the operator can audit.

```json
{
  "invoice_id": "<id or array>",
  "amount_outstanding_usd": 0.00,
  "interest_pct_annual": 0.0,
  "tiers": [
    {"name": "minimal",    "schedule_days": [15, 30, 45], "per_installment_usd": 0.00, "total_usd": 0.00, "due_dates": ["<YYYY-MM-DD>", "<YYYY-MM-DD>", "<YYYY-MM-DD>"], "formula": "total_outstanding / 3, due [+15d, +30d, +45d]", "default_consequence": "Reverts to escalate_past_due flow and a collections referral."},
    {"name": "balanced",   "schedule_days": [10, 20, 30], "per_installment_usd": 0.00, "total_usd": 0.00, "due_dates": ["<YYYY-MM-DD>", "<YYYY-MM-DD>", "<YYYY-MM-DD>"], "formula": "total_outstanding / 3, due [+10d, +20d, +30d]", "default_consequence": "Reverts to escalate_past_due flow."},
    {"name": "aggressive", "schedule_days": [5, 15, 25],  "per_installment_usd": 0.00, "total_usd": 0.00, "due_dates": ["<YYYY-MM-DD>", "<YYYY-MM-DD>", "<YYYY-MM-DD>"], "formula": "total_outstanding / 3, due [+5d, +15d, +25d]", "default_consequence": "Single missed installment triggers immediate escalation."}
  ],
  "requires_approval": true
}
```

# Rules

- **Show the formula, not just the result.** Every tier carries its
  `formula` string. The operator must be able to recompute the
  numbers by hand; the skill is the source of truth for arithmetic.
- **`per_installment_usd = round(total / 3, 2)`** with the final
  installment absorbing the rounding remainder, so the three sum to
  `total_usd` exactly.
- **Interest is opt-in.** When `interest_pct_annual > 0`, compute
  simple interest over each tier's longest schedule day, add to
  `total_usd`, and show the interest amount in `formula`. When 0,
  write `"no interest"` and keep `total_usd == amount_outstanding_usd`.
- **Due dates are absolute.** `due_dates[i] = today +
  schedule_days[i]`, formatted YYYY-MM-DD. Never emit relative days
  alone — the customer artifact needs concrete dates.
- **Three tiers, no more, no less.** The operator's mental model is
  fixed at minimal/balanced/aggressive.
- **Never auto-offer.** Output is a proposal menu; the
  customer-facing offer is a separate approval-gated action.

# Examples

Input: `invoice_id: INV-2026-0119`, `amount_outstanding_usd: 4200.00`,
`today: 2026-05-12`, no interest. →

```json
{
  "invoice_id": "INV-2026-0119",
  "amount_outstanding_usd": 4200.00,
  "interest_pct_annual": 0.0,
  "tiers": [
    {"name": "minimal",    "schedule_days": [15, 30, 45], "per_installment_usd": 1400.00, "total_usd": 4200.00, "due_dates": ["2026-05-27", "2026-06-11", "2026-06-26"], "formula": "4200.00 / 3 = 1400.00 per installment, no interest", "default_consequence": "Reverts to escalate_past_due flow and a collections referral."},
    {"name": "balanced",   "schedule_days": [10, 20, 30], "per_installment_usd": 1400.00, "total_usd": 4200.00, "due_dates": ["2026-05-22", "2026-06-01", "2026-06-11"], "formula": "4200.00 / 3 = 1400.00 per installment, no interest", "default_consequence": "Reverts to escalate_past_due flow."},
    {"name": "aggressive", "schedule_days": [5, 15, 25],  "per_installment_usd": 1400.00, "total_usd": 4200.00, "due_dates": ["2026-05-17", "2026-05-27", "2026-06-06"], "formula": "4200.00 / 3 = 1400.00 per installment, no interest", "default_consequence": "Single missed installment triggers immediate escalation."}
  ],
  "requires_approval": true
}
```
