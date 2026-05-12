---
name: audit_inventory_readiness
description: Given one inventory.json entry plus cached comparable listings, produce a missing-fields checklist, a suggested price-band with rationale, and a listing-readiness Y/N. Read-only — never posts or messages.
inputs:
  - inventory_entry
  - comparable_listings
outputs:
  - missing_fields_checklist
  - suggested_price_band
  - listing_readiness
  - flags
---

# Skill: Audit Inventory Readiness

Decide whether one inventory item is ready to draft a public listing.

1. **Walk the missing-fields checklist.** Five mandatory fields:
   `photo_coverage` (hero side profile + cuff/liner size label +
   soulplate-photo + wheel-photo + included extras), `size_label`
   (boot shell + liner), `soulplate_photo`, `wheel_photo`,
   `pricing_anchor` (a comparable-listings row in
   `comparable_listings`). For each, output `present` or
   `missing` — never `partial`. If `comparable_listings` is empty
   or older than 30 days, mark `pricing_anchor` missing and flag
   `stale_comparable_data`.
2. **Suggest a price-BAND, not a point estimate.** Format:
   `low–high AUD` derived from the 25th and 75th percentile of
   `comparable_listings`. Rationale must cite at least 3
   comparable rows by id. If `comparable_listings` has fewer than
   3 rows, return `band: "insufficient_evidence"` and flag
   `not_enough_comparables`. Never invent a price from base rate.
3. **Listing-readiness Y/N.** `Y` only if every field in step 1
   is `present` AND the price-band has rationale. Any `missing`
   field → `N` with the specific blockers listed. Never partial.
4. **Cite the inventory entry by `item_id`** — never by buyer
   name, photographer, or freeform description.

Hard rules:

- **Truth-in-condition is load-bearing.** If the inventory entry
  lists `known_issues`, surface them in the checklist as a
  `condition_disclosure` line; never elide. The system prompt's
  "describe condition truthfully" rule applies — do not soften.
- **Read-only skill.** Never propose a `post_listing_public`,
  `send_buyer_message`, or `mark_sold` action. The output is a
  checklist + price-band; the operator decides next steps.
- **Price-band is a recommendation.** Never claim "this will
  sell for X" — surface the band + rationale + confidence
  (`low` / `medium` / `high`) and stop. No demand-velocity or
  sale-time prediction.
- **`next_action` is informational** — surface the
  readiness Y/N. The operator decides whether to proceed to
  `post_listing` (which is itself approval-gated).
- **If `inventory_entry` lacks an `item_id`** — flag
  `missing_item_id` and stop. Never audit an unidentified item.
