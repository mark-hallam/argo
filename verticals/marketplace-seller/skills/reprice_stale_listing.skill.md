---
name: reprice_stale_listing
version: "0.1.0"
description: "Given a listing that hasn't sold within the operator's staleness threshold (default 14 days), propose 3 reprice options — aggressive (−15%), moderate (−7%), minimal (−3%) — plus a no-change rationale. Each option carries evidence or an explicit generic-elasticity assumption. Approval-gated. Draft-only; never reprices."
provenance:
  source: vertical/marketplace-seller
  hash: sha256-pending
---

# When to use

Call when an active listing has been live longer than the operator's
staleness threshold (default `14 days`) and has not sold. The output is
a draft proposal; the operator approves before any platform reprice
goes out. Never call to "freshen" a listing under the threshold — that
is not what this skill exists for.

# Inputs you expect

- the listing entry (`listing_id`, `current_price_aud`, `posted_at`,
  `platform`, `item_id`)
- the operator's `staleness_days` threshold (default `14`)
- optional `memory.pricing_comps[]` rows for the same `item_id` —
  used as evidence when present
- optional `memory.seller_learnings[]` rows tagged
  `pattern: "stale_listing_reprice"`

# Output shape

A single fenced JSON block:

```json
{
  "listing_id": "<id>",
  "item_id": "<id>",
  "current_price_aud": 220,
  "days_listed": 21,
  "reprice_options": [
    {
      "level": "aggressive",
      "delta_pct": -15,
      "new_price_aud": 187,
      "expected_sale_lift_band": "high",
      "evidence": [{"comp_id": "<id>", "sold_price_aud": 185}]
    },
    {
      "level": "moderate",
      "delta_pct": -7,
      "new_price_aud": 205,
      "expected_sale_lift_band": "medium",
      "evidence": []
    },
    {
      "level": "minimal",
      "delta_pct": -3,
      "new_price_aud": 213,
      "expected_sale_lift_band": "low",
      "assumption": "generic-elasticity"
    }
  ],
  "no_change_rationale": "<one sentence — the case for holding price>",
  "requires_approval": true
}
```

# Rules

- **Default band is load-bearing.** `−15% / −7% / −3%` are the
  operator-tuned defaults. Don't drift them per-listing unless the
  operator has set a different `reprice_band` in config; document the
  default in the rationale.
- **Evidence beats assumption.** If `memory.pricing_comps[]` carries
  rows for the same `item_id`, attach the closest 1–2 by
  `sold_price_aud` to each applicable option. Otherwise mark the
  option `"assumption": "generic-elasticity"` — never fabricate comps.
- **Always include `no_change_rationale`.** Hold-price is a valid
  fourth option. State the case (e.g. "comps are sparse; one more
  week of signal before cutting") so the operator can choose it.
- **Never auto-reprice.** `requires_approval: true` is the floor.
  No `update_listing_price` action is proposed inline; the operator
  routes through the existing approval flow.
- **Cite the listing by `listing_id`** and the inventory by
  `item_id`. Never address by buyer name or freeform description.
- **If `days_listed < staleness_days`** — flag
  `not_yet_stale` and stop. Don't propose a reprice early.
- **Hold the floor.** If the operator's `memory.listing_floors[listing_id]`
  is set and an option would land below it, mark that option
  `"below_floor": true` and recommend the next-higher option in the
  rationale. Schema addition pending.
