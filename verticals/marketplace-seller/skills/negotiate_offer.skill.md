---
name: negotiate_offer
version: "0.1.0"
description: "Given an incoming buyer offer + the listing's floor price (memory.listing_floors[listing_id]), draft a measured response — accept / counter-with-justification / decline-politely. Output is Markdown reply text plus a structured action block. Approval-gated. Never auto-accepts below floor."
provenance:
  source: vertical/marketplace-seller
  hash: sha256-pending
---

# When to use

Call when a buyer has named a price below the listing's asking
price. The output is a draft reply + a structured action proposal;
the operator approves before any send. Never call on opening
messages that haven't named a number — `draft_buyer_response` owns
those.

# Inputs you expect

- the buyer's offer (`buyer_offer_aud`, `listing_id`, `buyer_handle`,
  raw message text)
- the listing's floor (`memory.listing_floors[listing_id]` — the
  hard "do not go below" price). Schema addition pending.
- the listing's asking price (`current_price_aud`)
- optional `memory.pricing_comps[]` rows for the same `item_id`

# Output shape

A Markdown reply followed by a single fenced JSON action block:

```
<polite reply text — 40–80 words; tone matches firm-on-price>
```

```json
{
  "listing_id": "<id>",
  "buyer_handle": "<handle>",
  "buyer_offer_aud": 150,
  "current_price_aud": 220,
  "floor_price_aud": 180,
  "action": "accept|counter|decline",
  "counter_price_aud": 200,
  "rationale": "<one sentence — why this action>",
  "requires_approval": true
}
```

`counter_price_aud` is omitted when `action` is `accept` or
`decline`.

# Rules

- **Floor-price respect is a hard rule.** Never propose
  `action: "accept"` when `buyer_offer_aud < floor_price_aud`.
  Accepting below floor breaks the operator's economics — there is
  no override path inline. If the offer is below floor, the only
  legal actions are `counter` (at or above floor) or `decline`.
- **Counter lands between floor and asking.** A `counter` proposal
  must satisfy `floor_price_aud <= counter_price_aud < current_price_aud`.
  Default to the midpoint of floor and asking when comps are absent;
  cite a comp's `sold_price_aud` when one is available.
- **Decline is polite, not curt.** When declining, the Markdown body
  thanks the buyer, restates the asking price as the seller's
  position, and leaves the door open for a higher offer. Never
  insult the offer ("lowball", "insulting") regardless of size.
- **Approval-gated.** `requires_approval: true` is the floor. The
  reply is a draft; `send_buyer_message` remains operator-only.
- **Never share private contact info.** No phone, email, or street
  address in the Markdown body. Pickup logistics stay in
  `draft_buyer_response`'s template — this skill is price-only.
- **If `memory.listing_floors[listing_id]` is missing** — flag
  `missing_listing_floor` and stop. Never invent a floor; the
  operator must set it before negotiation can begin.
- **Cite the listing by `listing_id`** and the buyer by their
  platform handle. Never include the buyer's real name.
