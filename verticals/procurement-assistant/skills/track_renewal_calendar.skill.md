---
name: track_renewal_calendar
description: Given the operator's active vendor list and contract end dates, produce a 90-day-lookahead renewal report with current spend per contract and a recommended action per row. Internal report only — renewal decisions route through human approval.
inputs:
  - contracts
  - as_of_date
  - lookahead_days
outputs:
  - renewal_window
  - renewal_rows
  - markdown_summary
  - escalation_flags
---

# Skill: Track Renewal Calendar

Surface every contract whose end date falls inside the next
`lookahead_days` (default 90) so the operator can act before
auto-renewal fires. Draft only — renewing, letting lapse, or
consolidating routes through `[escalation].human_approval_required`.

1. **Window.** Compute `as_of_date` (default today) +
   `lookahead_days` (default 90). Anything ending strictly later than
   the window is out of scope; do not speculate beyond 90 days.
2. **Vendor lookup.** For each contract, cross-reference
   `vendor_id` against `approved_vendors`. Vendors absent from the
   roster get `unapproved_vendor` on the row; do not drop the row —
   the operator may want to let-lapse precisely because the vendor is
   unapproved.
3. **Sanctions check.** Compare each vendor name against
   `sanctions_watchlist`. A hit flags `sanctions_hit` on the row and
   forces `recommended_action = "let_lapse"`; downstream renewal of a
   sanctioned entity is forbidden.
4. **Spend per contract.** Cite `current_annual_spend_usd` from the
   contract record (do not infer); if the contract record lacks it,
   set `current_annual_spend_usd = null` and add
   `spend_unknown_cite_contract` to the row's notes. Never fabricate.
5. **Recommendation.** Exactly one value from the 4-action vocabulary
   per row:
   - `renew_as_is` — pricing & terms stable, no consolidation
     candidate, vendor in good standing.
   - `renegotiate` — pricing drift, expiring discount, or firm-terms
     divergence cited from `contract_review` history.
   - `let_lapse` — `sanctions_hit`, vendor exited the category, or
     `current_annual_spend_usd` < 12-month utilisation floor cited
     from inventory.
   - `consolidate_with_<vendor_id>` — another contract in the same
     `category` exists with overlapping scope; name the target
     vendor_id explicitly. Do not propose consolidation without a
     concrete target.
6. **Approval block.** Always emit `approval_required =
   "renew_contract"` at the report level. Each row's recommendation
   is a draft.
7. **Markdown summary.** One paragraph header (`N contracts ending in
   the next D days, $X total annual spend at risk`) plus a
   contract_id-keyed bullet list. The JSON is the audit artefact; the
   markdown is the operator-facing brief.

Output shape:

```json
{
  "renewal_window": {"as_of_date": "2026-05-12", "lookahead_days": 90,
                     "window_end": "2026-08-10"},
  "renewal_rows": [
    {"contract_id": "c_acme_saas_2024", "vendor_id": "v_acme_saas",
     "vendor_name": "Acme Cloud Inc.", "category": "saas",
     "end_date": "2026-07-04", "days_remaining": 53,
     "current_annual_spend_usd": 18000,
     "recommended_action": "renegotiate",
     "rationale": "Tier-1 wire reports vendor raised list prices 12% in Q1 2026; renegotiate before auto-renew.",
     "evidence_clause": "section 7.2 (annual pricing review)",
     "notes": []}
  ],
  "markdown_summary": "3 contracts ending in the next 90 days, $54000 total annual spend at risk.\n\n- c_acme_saas_2024 — renegotiate (53d, $18000)\n- ...",
  "escalation_flags": [],
  "approval_required": "renew_contract"
}
```
