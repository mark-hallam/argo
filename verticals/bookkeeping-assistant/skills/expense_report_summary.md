---
name: expense_report_summary
description: Bucketize a CSV of employee-submitted expenses into reimbursable categories with per-row receipt notes; flag rows missing receipts or exceeding policy caps.
inputs:
  - employee_id
  - csv_rows
  - reporting_period
outputs:
  - buckets
  - reimbursable_total_usd
  - flagged_rows
---

# Skill: Expense Report Summary

Use this skill to turn an employee's expense-report CSV into a clean,
reimbursable bucketization the operator can review and approve.

1. **Read each CSV row.** Expected columns: `date`, `vendor`,
   `description`, `amount_usd`, `receipt_url`, `category_hint?`.
2. **Bucket each row** into one of:
   - `travel_airfare` (`6810`)
   - `travel_lodging` (`6820`)
   - `travel_other` (`6830` — car rental, rideshare, parking)
   - `meals_50` (`6300` — 50% deductible)
   - `mileage` (`6850` — IRS standard rate, current year)
   - `software` (`6200`)
   - `supplies` (`6500`)
   - `other` (`6900` — anything that doesn't fit; will be reviewed)
3. **Flag rows that fail policy.**
   - `amount_usd > 75` AND `receipt_url` missing → flag as
     `missing_receipt`.
   - Single meal > $100 → flag as `meals_policy_cap`.
   - Vendor matches a personal-use heuristic (Netflix, Spotify,
     grocery without travel context) → flag as `personal_use_suspect`.
   - Any single row > $1,000 → flag for `categorize_large_txn`
     approval; do not auto-bucket.
4. **Sum the reimbursable total** across buckets, excluding any flagged
   row, and report it in USD to two decimals.

Output shape:

```json
{
  "employee_id": "emp-0042",
  "reporting_period": "2026-04",
  "buckets": {
    "travel_airfare": 412.50,
    "meals_50": 78.20,
    "software": 19.99
  },
  "reimbursable_total_usd": 510.69,
  "flagged_rows": [
    {"row_idx": 7, "reason": "missing_receipt", "amount_usd": 96.40}
  ]
}
```

Never reimburse from this skill — submitting the bundle for payment is
an approval-gated action outside this vertical's v1 surface.
