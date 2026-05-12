---
name: draft_offer
version: "0.1.0"
description: "Drafts a purchase offer for a buyer + listing — price, contingencies, earnest, close date, requested concessions. Output is structured JSON mapping line-by-line to standard offer-form fields. ALWAYS approval-gated; this is a financial-commitment instrument and the licensed agent reviews every field before the real document leaves the workspace."
provenance:
  source: vertical/real-estate-agent
  hash: sha256-pending
---

# When to use

Call when a buyer has named the listing to bid on AND the
operator supplies (a) market comps and (b) an offer strategy.
Output is the draft for the agent to review; the `draft_offer`
action renders the Markdown file only after approval.

# Inputs you expect

- `listing_id` (must exist in `inventory/sample_listings.json`)
- `buyer_name`, `buyer_pre_approval_letter_on_file` (bool)
- list price + days-on-market from inventory
- operator comps: median sale-to-list ratio + median
  days-to-close, last 6 mo within ~0.5 mi
- `offer_strategy`: `aggressive` / `market` / `below_asking`
- `financing_type`: `conventional` / `fha` / `va` / `cash`
- requested concessions (credits, inclusions, repairs)

# Output shape

A single JSON block. Fields map 1:1 to the standard offer
form so the agent reviews line-by-line:

```json
{
  "listing_id": "<id>",
  "listing_address": "<from inventory>",
  "buyer_name": "<name>",
  "offer_price_usd": 0,
  "earnest_money_usd": 0,
  "down_payment_pct": 0,
  "financing_type": "conventional|fha|va|cash",
  "closing_date_iso": "<YYYY-MM-DD>",
  "contingencies": {"inspection": {"included": true, "days": 10}, "financing": {"included": true, "days": 21}, "appraisal": {"included": true, "days": 17}},
  "requested_concessions": ["<concession>"],
  "offer_expiry_iso": "<YYYY-MM-DDTHH:MM:SS-TZ>",
  "rationale": {"price_basis": "<cite comp ratio>", "earnest_basis": "<one sentence>", "contingency_basis": "<one sentence>"},
  "requires_approval": true,
  "financial_commitment": "This is a draft purchase offer — every field carries financial consequence. The licensed agent must review and convert it into the official offer document before it leaves the workspace."
}
```

# Rules

- **`financial_commitment` is mandatory** — emit verbatim.
- **Cite the comp basis.** `price_basis` must reference the
  median sale-to-list ratio. If comps are missing, abstain:
  `rationale.price_basis: "OPERATOR: comps missing"`.
- **All 3 contingencies included by default.** Waive only when
  strategy is `aggressive` AND the input names the waiver;
  waivers go in `rationale.contingency_basis`.
- **Earnest defaults**: 1% for `market`, 2% for `aggressive`,
  0.5% for `below_asking`; round to nearest $500.
- **Pre-approval check.** If `buyer_pre_approval_letter_on_file`
  is `false` AND `financing_type != "cash"`, prepend
  `"BLOCKING: pre-approval letter not on file"` to concessions.
- **No legally binding language.** Never "we are bound", "this
  is a contract", "buyer agrees to" — the agent's licensed
  document is the binding instrument.
- **Offer expiry default**: 48 hours from generation.

# Approval posture

ALWAYS approval-gated. `requires_approval = true` in
`vertical.toml` because every field carries financial
consequence. The agent reviews price, contingencies, earnest,
and close date before the real offer is created.

# Examples

```json
{
  "listing_id": "MLS-118822",
  "listing_address": "118 Cedar Lane, Oakland CA",
  "buyer_name": "Sam Buyer",
  "offer_price_usd": 685000,
  "earnest_money_usd": 7000,
  "down_payment_pct": 20,
  "financing_type": "conventional",
  "closing_date_iso": "2026-06-26",
  "contingencies": {"inspection": {"included": true, "days": 10}, "financing": {"included": true, "days": 21}, "appraisal": {"included": true, "days": 17}},
  "requested_concessions": ["seller pays $4,000 in closing costs"],
  "offer_expiry_iso": "2026-05-14T17:00:00-07:00",
  "rationale": {"price_basis": "median sale-to-list 0.99 over 6 mo · DOM 18 supports market offer", "earnest_basis": "1% of price (market strategy)", "contingency_basis": "all three included; financing days reflect 21-day underwriting"},
  "requires_approval": true,
  "financial_commitment": "This is a draft purchase offer — every field carries financial consequence. The licensed agent must review and convert it into the official offer document before it leaves the workspace."
}
```
