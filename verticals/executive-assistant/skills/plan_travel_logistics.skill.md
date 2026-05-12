---
name: plan_travel_logistics
version: "0.1.0"
description: Given a trip request (destination + date range + purpose), draft an itinerary with 3 flight options, 3 hotel options, ground transport, and trip-meeting addresses. Approval-gated — never books.
requires_approval: true
provenance:
  source: vertical/executive-assistant
  hash: sha256-pending
---

# When to use

Use when the principal accepts a trip in principle and needs concrete
options for review. Draft only — `book_travel` routes via
`[escalation].human_approval_required` in `vertical.toml`. Re-run if
dates shift; regenerate, never patch.

# Inputs

- `trip_request` — `{destination_city, arrive_date, depart_date, purpose}`.
- `principal_prefs` — memory: cabin, seat, ground-transport defaults.
- `travel_aliases` — memory: `home_airport`, preferred airlines / hotel chains, loyalty aliases (opaque — never echo raw numbers).
- `trip_meetings` — optional list of `{counterparty, location_hint, requested_window}`.

# Output shape

```json
{
  "trip": {"destination_city": "...", "arrive_date": "...", "depart_date": "...", "purpose": "..."},
  "flights": [
    {"rank": 1, "airline": "united", "outbound": {"depart_iso": "...", "arrive_iso": "..."},
     "return": {"depart_iso": "...", "arrive_iso": "..."}, "cabin": "premium_economy",
     "est_cost_usd": 1850, "tradeoffs": "earliest arrival; matches loyalty preference",
     "preference_source": "memory.travel_aliases.preferred_airlines"}
  ],
  "hotels": [
    {"rank": 1, "chain": "marriott", "property": "...", "nightly_usd": 420,
     "walk_min_to_first_meeting": 6, "tradeoffs": "closest to first meeting",
     "preference_source": "memory.travel_aliases.preferred_hotel_chains"}
  ],
  "ground_transport": [
    {"leg": "airport_to_hotel", "mode": "ride_share"},
    {"leg": "between_meetings", "mode": "walk_or_ride_share"},
    {"leg": "hotel_to_airport_return", "mode": "ride_share"}
  ],
  "trip_meetings": [{"counterparty": "...", "address": "...", "scheduled_window": "..."}],
  "next_action": {"name": "book_travel", "requires_approval": true, "auto_send_allowed": false},
  "flags": []
}
```

Also emit a Markdown summary: one row per flight, one per hotel,
ranked. Trip-meeting addresses follow as a second table.

# Hard rules

- **Approval-gated.** `requires_approval: true` and
  `auto_send_allowed: false`. Travel commits spend; the skill never
  books. Booking runs via a Tier A connector once the principal
  approves a specific flight + hotel pair.
- **3 + 3 with trade-offs.** Always 3 flight + 3 hotel options with
  explicit `tradeoffs` (price / timing / preference-match). If fewer
  than 3 viable options exist, set `flags: ["insufficient_options"]`
  and return what's available — never pad with weak options.
- **Cite preference source per recommendation.** Each row carries
  `preference_source` pointing to a memory key
  (`memory.travel_aliases.preferred_airlines`,
  `principal_prefs.default_cabin`, etc). If unknown, set
  `preference_source: "assumption: generic-best-practice"` and name
  the reason in `tradeoffs`.
- **Loyalty numbers stay opaque.** Reference loyalty by alias only.
  Never echo a raw loyalty number, even if the connector surfaces one.
- **Cost is estimate.** `est_cost_usd` is a planning estimate; the
  booking flow re-prices at approval. Never claim a price is locked.
- **No commitments on principal's behalf.** Output is proposal-shaped
  ("Recommend: United 218, departs SFO 08:15"). Never phrase as
  "booked", "confirmed", or "scheduled".
- **Trip-meeting addresses only.** Surface address + scheduled
  window. Never draft outreach to a counterparty; that lives in other
  skills behind separate approval gates.

# Example

Synthetic — principal flies SFO → NYC for a board prep day. Memory:
`preferred_airlines: [united, alaska]`, `preferred_hotel_chains:
[marriott, hyatt]`, `default_cabin: premium_economy`. Output proposes
3 United/Alaska flights by arrival time, 3 Marriott/Hyatt hotels by
walk to the board address, ride-share ground transport, and the
trip-meeting block. Principal approves United 218 + Marriott Park
Lane; booking then runs through the approval-gated connector.
