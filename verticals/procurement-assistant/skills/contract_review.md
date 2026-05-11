---
name: contract_review
description: Given a supplier contract and the buyer's bid-comparison position, summarise key terms, redline against firm_terms, and flag clauses that diverge or are missing.
inputs:
  - contract_id
  - contract_text
  - bid_comparison_position
  - vendor_id
outputs:
  - key_terms
  - redlines
  - escalation_flags
  - approval_required
---

# Skill: Contract Review

Read the supplier's paper, compare against the buyer's `firm_terms`,
and produce a redlined position the operator can take into
negotiation. Drafts only — signing is gated through
`[escalation].human_approval_required`.

1. **Key terms summary.** Five-row table: term length, fees &
   escalators, liability cap, indemnity scope, termination & renewal
   terms. Lead with the 3-year TCO implied by the contract — it must
   reconcile to the bid-comparison number; if it does not, flag the
   delta.
2. **Renewal lookahead.** If the current term ends within
   `policy.contract_renewal_lookahead_days` (default 90 / 60 / 30),
   emit `contract_auto_renew_imminent` with days remaining and the
   auto-renewal terms.
3. **Redline pass.** For each row in `firm_terms`: match against
   `required_text` (`compliant`) or `fallback_text`
   (`acceptable_with_note`); divergence or missing topic → propose
   redline / insertion of `required_text`; if
   `escalate_if_missing = true`, add `firm_terms_divergence`.
4. **Sanctions check.** Re-run the vendor name against
   `sanctions_watchlist`; a hit flags `sanctions_hit` and refuses the
   review (signing with a sanctioned entity is the highest-risk
   action this assistant can be near).
5. **Negotiating position.** If the vendor was the recommended winner
   with material trade-offs (lock-in, ship-time), surface the
   leverage ("36-month term locks us in — push back on liability cap").
6. **Approval block.** Always emit `approval_required = "sign_agreement"`.

Output shape:

```json
{
  "key_terms": {
    "term_months": 36, "tco_3yr": 71000, "liability_cap": "12mo fees",
    "indemnity": "IP only", "termination": "30d for convenience",
    "auto_renew": "12mo unless cancelled 60d prior"
  },
  "redlines": [
    {"clause_id": "ft_dpa", "status": "missing",
     "proposed_text": "DPA executed; sub-processors disclosed; SCCs for EU data transfers."}
  ],
  "escalation_flags": ["firm_terms_divergence"],
  "approval_required": "sign_agreement"
}
```
