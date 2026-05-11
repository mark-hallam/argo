---
name: compare_catering_bids
description: Given a set of catering bids and the cited budget line, produce a per-bid dietary-coverage matrix, total-cost-per-head comparison, and a single recommendation with trade-offs surfaced.
inputs:
  - event_id
  - budget_line_id
  - headcount
  - bids
outputs:
  - spend_summary
  - bid_table
  - dietary_matrix
  - recommendation
  - escalation_flags
---

# Skill: Compare Catering Bids

Bids in, ranked recommendation out, dietary gaps surfaced as
escalation. Placing the order is a `place_catering_order` action.

1. **Spend summary.** Lead with the lowest projected total cost
   and the cited budget line in the bracketed format. If the
   lowest TCO > `remaining_usd`, flag `over_budget` and stop.
2. **3-bid check.** If projected spend ≥ $5k and `bids.length < 3`,
   flag `single_bid_over_threshold` and stop.
3. **Vendor qualification.** Drop any bid from a vendor not in
   `approved_vendors` (category `catering`); flag `unapproved_vendor`.
4. **Dietary-coverage matrix.** For each surviving bid build the
   six-column matrix (gluten-free / dairy-free / nut-free / halal /
   kosher / vegan). Cell values: `yes` / `no` / `surcharge`. Any
   `no` is a `dietary_gap` naming the missing categories; that bid
   drops from the recommendation pool.
5. **TCO table.** For each surviving bid: `cost_per_head`,
   `service_fee`, `gratuity`, `total_cost`.
6. **Rank and recommend.** Lowest total cost among bids with full
   dietary coverage. Exactly one named recommendation with the
   trade-off named. Emit `APPROVAL_REQUIRED`.

Output shape:

```json
{
  "spend_summary": "[budget: bl_offsite_q3_fy26 | period: Q3-FY26 | ceiling: $80000 | used: $0 | remaining: $80000]",
  "bid_table": [
    {"vendor": "Acme Catering", "cost_per_head": 65,
     "service_fee": 800, "gratuity": 1200, "total_cost": 9800}
  ],
  "dietary_matrix": [
    {"vendor": "Acme Catering", "gluten_free": "yes", "dairy_free": "yes",
     "nut_free": "yes", "halal": "yes", "kosher": "surcharge", "vegan": "yes"}
  ],
  "recommendation": {
    "vendor": "Acme Catering",
    "rationale": "Lowest TCO with full dietary coverage; trade-off: kosher is surcharge.",
    "approval_required": "place_catering_order"
  },
  "escalation_flags": []
}
```
