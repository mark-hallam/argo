---
name: generate_pl_summary
version: "0.1.0"
description: "Produces a P&L summary for a date range from the categorized ledger: revenue, COGS, opex, EBITDA, margin% with MoM and YoY deltas when data is available. Deterministic arithmetic; operator-verifiable totals."
provenance:
  source: vertical/bookkeeping-assistant
  hash: sha256-pending
---

# When to use

Call when the operator requests a P&L for a specific period
(month, quarter, or custom date range). The output is a
read-only summary the operator reviews alongside the
categorized ledger; pair with `monthly_close` for full close
prep. Output is internal — not approval-gated and never
posts.

# Inputs you expect

- date range (`YYYY-MM-DD..YYYY-MM-DD`)
- categorized journal entries spanning at least the range
  plus the trailing 13 months (for YoY deltas)
- chart of accounts (so revenue / COGS / opex grouping is
  unambiguous)

# Output shape

A Markdown report plus a JSON metadata block.

```json
{
  "period": "2026-04-01..2026-04-30",
  "revenue_usd": 84200.00,
  "cogs_usd": 21100.00,
  "gross_profit_usd": 63100.00,
  "gross_margin_pct": 74.94,
  "opex_usd": 38400.00,
  "ebitda_usd": 24700.00,
  "ebitda_margin_pct": 29.34,
  "mom_delta": {"revenue_pct": 8.10, "ebitda_pct": 12.40},
  "yoy_delta": {"revenue_pct": 22.50, "ebitda_pct": 18.10},
  "comparison_available": {"mom": true, "yoy": true}
}
```

The Markdown report is a side-by-side period comparison
(current vs prior month vs same month prior year) — operators
care about trends, not point-in-time totals.

# Examples

- April 2026 with March 2026 and April 2025 data available →
  full three-column side-by-side report; both deltas
  populated.
- A newly-onboarded tenant with only 3 months of history →
  populate `mom_delta`, set `yoy_delta: null` and
  `comparison_available.yoy: false`; the report omits the YoY
  column rather than fabricating it.
- Operator asks for a custom range (e.g. quarter) → roll up
  the same buckets over the range; MoM/YoY deltas compare
  against the equivalent prior period.
- A required account is `unmapped` → exclude from totals,
  list the unmapped rows in a "review-before-finalising"
  footnote, and emit a `comparison_available.partial: true`
  flag.

# Approval posture

Internal-output, not approval-gated. Deterministic
arithmetic — totals are summed from the categorized ledger
with no rounding before the final cell; the operator can
spot-check any bucket against the underlying entries.
Accrual-only categories (`unearned_revenue`,
`prepaid_expense`) are out of v1 scope per the
US-GAAP-cash-basis charter — if any entry uses one, the
skill refuses and routes to the operator for advice rather
than guessing the treatment.
