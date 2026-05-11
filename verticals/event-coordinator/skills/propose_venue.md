---
name: propose_venue
description: Given a target date, headcount, budget line, and region, propose ranked venue options with pros/cons, total-cost-of-ownership, and ADA-compliance status.
inputs:
  - target_date
  - headcount
  - budget_line_id
  - region
outputs:
  - spend_summary
  - venue_options
  - recommendation
  - escalation_flags
---

# Skill: Propose Venue

Date + size + budget + region in; ranked venue options with named
trade-offs and ADA status out. Booking the venue routes through
`[escalation].human_approval_required`.

1. **Spend summary.** Lead with the lowest projected total cost and
   the cited budget line in the bracketed format. If the lowest
   TCO > `remaining_usd`, flag `over_budget` and stop.
2. **3-bid check.** For spend ≥ $5k, surface at least three venue
   options with written quotes. Fewer is `single_bid_over_threshold`.
3. **Vendor qualification.** Every venue must appear in
   `approved_vendors` (category `venue`). Drop unapproved venues
   and flag `unapproved_vendor`.
4. **ADA-accessibility check.** For each option surface ADA status:
   wheelchair access, accessible restrooms, hearing-loop / captioning,
   accessible parking. If memory has no status, mark
   `ada_status = "unknown — verify with venue"` and add
   `ada_unverified`. Do NOT silently assume compliance.
5. **International-travel check.** If the region implies any attendee
   crossing a country border, flag `international_travel_component`
   and surface visa / customs as out-of-scope.
6. **Rank and recommend.** Lowest TCO meeting headcount and ADA
   baseline first, then "best fit", then fallback. Exactly one named
   recommendation with the trade-off named explicitly. Emit
   `APPROVAL_REQUIRED` (next step is a `book_venue` action).

Output shape:

```json
{
  "spend_summary": "[budget: bl_offsite_q3_fy26 | period: Q3-FY26 | ceiling: $80000 | used: $0 | remaining: $80000]",
  "venue_options": [
    {"venue": "Lakeside Conference Center", "headcount_capacity": 120,
     "tco_usd": 42000, "ada_status": "full",
     "pros": ["full ADA", "on-site AV"], "cons": ["30 min from airport"]}
  ],
  "recommendation": {
    "venue": "Lakeside Conference Center",
    "rationale": "Lowest TCO and full ADA; trade-off: 30 min further from airport than Skyline.",
    "approval_required": "book_venue"
  },
  "escalation_flags": []
}
```
