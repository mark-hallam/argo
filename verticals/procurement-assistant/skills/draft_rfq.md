---
name: draft_rfq
description: Given a requirement, projected budget, and shortlisted vendors, produce an RFQ document with eval criteria, scoring weights, and a deadline.
inputs:
  - requirement
  - projected_spend_usd
  - budget_line_id
  - shortlisted_vendors
  - response_window_days
outputs:
  - spend_summary
  - rfq
  - escalation_flags
---

# Skill: Draft RFQ

Assemble an RFQ that suppliers can respond to in a single round, the
buyer can score deterministically, and procurement can audit later.

1. **Spend summary.** Lead with the projected spend and the budget
   line in the system-prompt-enforced bracketed format. If projected
   spend > `remaining_usd` for the cited line, set
   `escalation_flags = ["over_budget"]` and stop.
2. **Vendor shortlist.** Verify every named vendor is in
   `approved_vendors`. Drop any unapproved vendor and flag
   `unapproved_vendor` if the input named one. Compare every vendor
   name against `sanctions_watchlist`; a hit drops the vendor and
   flags `sanctions_hit`.
3. **3-quote check.** If projected spend ≥ `three_quote_threshold_usd`
   and the shortlist has < 3 vendors, flag
   `single_bid_over_threshold` and stop. Sole-source justifications
   route through escalation.
4. **Requirement.** Restate the requirement in 3–6 bullets, separating
   must-haves from nice-to-haves.
5. **Eval criteria.** 4–6 criteria with explicit weights summing to
   100. Always include: total-cost-of-ownership, delivery / SLA,
   security & compliance, references, switching cost.
6. **Response window.** Set the deadline = today + `response_window_days`.
   Flag if the requested window is < 5 business days for spend
   ≥ `three_quote_threshold_usd`.

Output shape:

```json
{
  "spend_summary": "[budget: bl_eng_saas_fy26 | period: FY26 | remaining: $51600 | spend_window: saas]",
  "rfq": {
    "requirement_must_have": ["..."],
    "requirement_nice_to_have": ["..."],
    "eval_criteria": [{"name": "tco_3yr", "weight": 35}, {"name": "sla", "weight": 20}],
    "shortlisted_vendors": [{"vendor_id": "v_acme_saas", "name": "Acme Cloud Inc."}],
    "response_deadline": "2026-05-26",
    "submission_format": "written_quote_pdf"
  },
  "escalation_flags": []
}
```
