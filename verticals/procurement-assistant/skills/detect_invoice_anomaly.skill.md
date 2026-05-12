---
name: detect_invoice_anomaly
description: Given a vendor invoice and the cited contract terms, flag the four documented anomaly types — pricing drift, unbilled discounts, late-fee additions, scope expansion — and emit a recommended action per anomaly. Approval-gated before any disputed-amount communication.
inputs:
  - invoice
  - contract_id
  - contract_terms
  - pricing_drift_threshold_pct
outputs:
  - anomalies
  - recommended_action
  - escalation_flags
---

# Skill: Detect Invoice Anomaly

Compare a single vendor invoice against the cited contract terms and
flag the four anomaly types this skill covers. Highest-leverage
cost-recovery skill — small overcharges compound. Draft only;
disputing or holding for legal is gated through
`[escalation].human_approval_required`.

1. **Contract terms cited, not inferred.** Every claim about
   "expected" pricing, discounts, fees, or scope must point at a
   specific clause in `contract_terms` via `evidence_clause` (e.g.
   `"section 4.1 (unit pricing)"`). If the contract is silent on a
   billed item, flag `contract_silent` in the anomaly notes — do not
   argue the legal interpretation.
2. **Pricing drift.** For each line item, compare
   `invoice.line.unit_price` against `contract_terms.unit_price` for
   that SKU. Drift = `(actual − expected) / expected`. If
   `|drift| > pricing_drift_threshold_pct` (default 5%), emit an
   anomaly of type `pricing_drift` with `expected`, `actual`, and
   `severity` = `high` if drift > 15%, `medium` if 5–15%.
3. **Unbilled discounts.** For each discount in `contract_terms`
   (volume tier, prompt-pay, multi-year, promotional), check whether
   the invoice applied it. Missed discount → anomaly of type
   `unbilled_discount` with `expected` (clause-cited discount amount),
   `actual` (0), `severity` = `medium`.
4. **Late-fee additions.** If the invoice carries late fees,
   surcharges, or interest lines, verify the contract authorises
   them. Contract silent or operator paid within the contract grace
   period → anomaly of type `late_fee_addition`, `severity` = `low`
   unless > 1% of invoice total (then `medium`).
5. **Scope expansion.** Compare invoiced line items against
   `contract_terms.scope`. Items outside the agreed scope (new
   services, additional seats beyond cap, professional-services
   hours not in the SOW) → anomaly of type `scope_expansion`,
   `severity` = `high` if cumulative scope-expansion total > 10% of
   the contracted base, else `medium`.
6. **Recommended action.** Exactly one value from the 4-action
   vocabulary, derived from the highest-severity anomaly:
   - `pay_as_is` — no anomalies, or only `low`-severity
     `late_fee_addition` under operator-set tolerance.
   - `request_clarification` — one or more `medium` anomalies with
     `contract_silent` notes (the gap may be a billing-system bug,
     not a dispute).
   - `dispute` — any `high`-severity anomaly with a clear
     `evidence_clause` (pricing_drift > 15% or scope_expansion > 10%
     of base) AND no `contract_silent` ambiguity.
   - `hold_for_legal` — `scope_expansion` paired with
     firm-terms-divergence history on the contract, or any anomaly
     above `policy.legal_review_threshold_usd`.
7. **Approval block.** Always emit `approval_required =
   "dispute_invoice"` when `recommended_action` is `dispute` or
   `hold_for_legal`. `pay_as_is` and `request_clarification` are
   internal drafts.

Output shape:

```json
{
  "anomalies": [
    {"type": "pricing_drift", "expected": 1500.00, "actual": 1725.00,
     "severity": "medium", "evidence_clause": "section 4.1 (unit pricing)",
     "notes": []},
    {"type": "unbilled_discount", "expected": 540.00, "actual": 0,
     "severity": "medium", "evidence_clause": "section 5.3 (prompt-pay 3%)",
     "notes": []}
  ],
  "recommended_action": "request_clarification",
  "escalation_flags": [],
  "approval_required": null
}
```
