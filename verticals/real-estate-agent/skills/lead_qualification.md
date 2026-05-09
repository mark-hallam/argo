# Skill: lead qualification

Use this skill when a new lead arrives or an existing lead's
criteria change. Output is the `bucket` field on the `leads` memory
entry plus a list of 1–3 listing matches.

## When to apply

- The lead has just arrived (web form, sign call, referral) and is
  not yet in the `leads` memory entry.
- An existing lead has changed price band, geography, or bed/bath
  requirements and should be re-bucketed.

## Buckets

- `qualified` — has stated price band, geography, beds-min,
  baths-min, AND a contact email. Ready for `send_listing_packet`.
- `nurture` — partial criteria (e.g. price band but no geography).
  Operator decides whether to ask follow-ups or wait.
- `unqualified` — no price band, won't share contact, or asks for
  steering by protected class. Do NOT send a packet.

## Questions to ask

The lead form should already capture these. If any are missing, the
operator asks the lead via their preferred channel:

1. Price band (low/high in USD).
2. Geography (city/neighbourhood by name; never inferred from a
   protected class).
3. Beds-min and baths-min.
4. Square-footage minimum (optional).
5. Type preference (single-family / townhouse / condo).
6. Timeline ("looking now", "3-6 months", "exploring").
7. Pre-approval letter on hand? (yes/no/not yet).

## Red-flag patterns

Treat these as immediate `unqualified` + escalate to operator:

- Lead asks for filtering by race, color, religion, national origin,
  sex, familial status, disability, or local protected class. The
  Fair Housing rule in `prompts/system.md` is non-negotiable.
- Lead claims a financing instrument the agent doesn't recognise
  ("seller carry-back", "subject-to", "lease-option"). Operator
  routes to the agent for licensed advice.
- Lead references a listing the inventory does not contain. Confirm
  the listing id with the operator before quoting any details.
- Pressure tactics ("close in 48 hours", "no inspection"). The agent
  decides; Jamie does not.
- Lead self-discloses a protected class and ties it to a property
  preference. Acknowledge politely; never use the disclosure as a
  filter or amplifier.

## Match scoring (after bucket = `qualified`)

For each listing in `inventory/sample_listings.json`:

1. Hard filters: price within band, beds ≥ beds-min, baths ≥
   baths-min. Drop on miss.
2. Geography: include if the listing's locality is in the lead's
   stated geography. Drop on miss.
3. Soft signals (rank within passing set): square-footage
   match, year-built recency, listing-age (newer = stronger signal).

Pick the top 3 by rank. If fewer than 3 pass the hard filters, send
what passes and note the count to the operator.
