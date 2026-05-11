---
name: compare_bids
description: Given a set of supplier bids and the cited budget line, produce a 3-year total-cost-of-ownership comparison, risk-adjusted ranking, and a single recommendation with named trade-offs.
inputs:
  - rfq_id
  - budget_line_id
  - bids
  - tco_horizon_years
outputs:
  - spend_summary
  - bid_table
  - recommendation
  - escalation_flags
---

# Skill: Compare Bids

Bids in, ranked recommendation out. The recommendation is a draft —
placing the PO is gated through `[escalation].human_approval_required`.

1. **Spend summary.** Lead with the lowest projected total-cost-of-
   ownership across the bids and the cited budget line in the
   bracketed format. If the lowest TCO > `remaining_usd`, flag
   `over_budget` and stop. If the lowest TCO > `per_po_ceiling_usd`,
   flag `over_po_ceiling` and stop.
2. **3-quote check.** If projected spend ≥ `three_quote_threshold_usd`
   and `bids.length < 3`, flag `single_bid_over_threshold` and stop.
   Sole-source still routes through escalation.
3. **Vendor qualification.** Drop any bid from a vendor not in
   `approved_vendors` (flag `unapproved_vendor`) or matching
   `sanctions_watchlist` (flag `sanctions_hit`).
4. **TCO table.** For each surviving bid: `unit_price`,
   `recurring_annual` × `tco_horizon_years` (default 3),
   `implementation_cost`, `switching_cost` (migration off at horizon),
   `risk_adjustment` (named uplift or discount — references, financial
   signals, SLA history), `tco_3yr` = sum.
5. **Rank and recommend.** Lowest risk-adjusted TCO first. Exactly
   one named recommendation with the trade-off named explicitly
   ("Recommend B — lowest 3-year TCO; lose 2-week ship advantage to
   A but save $42k"). Always emit `APPROVAL_REQUIRED` because acting
   on the recommendation is a `place_purchase_order` action.

Output shape:

```json
{
  "spend_summary": "[budget: bl_eng_saas_fy26 | period: FY26 | remaining: $51600 | spend_window: saas]",
  "bid_table": [
    {"vendor": "Acme Cloud Inc.", "unit_price": 0, "recurring_annual": 18000,
     "implementation_cost": 5000, "switching_cost": 12000,
     "risk_adjustment": 0, "tco_3yr": 71000}
  ],
  "recommendation": {
    "vendor": "Acme Cloud Inc.",
    "rationale": "Lowest risk-adjusted 3-year TCO; trade-off: 36-month term locks us in.",
    "approval_required": "place_purchase_order"
  },
  "escalation_flags": []
}
```
