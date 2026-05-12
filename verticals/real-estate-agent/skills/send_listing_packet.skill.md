---
name: send_listing_packet
version: "0.1.0"
description: "Drafts a listing packet for a qualified lead — property facts, neighborhood comps, recent sales, school district, tax history. Output is Markdown body plus a JSON metadata sidecar so the runner can audit which listings went to which lead. Always approval-gated; never auto-sends."
provenance:
  source: vertical/real-estate-agent
  hash: sha256-pending
---

# When to use

Call after `lead_qualification` buckets the lead as `qualified`
AND 1–3 listings pass match scoring. Output is the draft for
the operator to review before `send_listing_packet` emails it.

# Inputs you expect

- `lead_id`, `lead_name`, `lead_email` (from `leads` memory)
- 1–3 `listing_id`s that passed match scoring
- operator-provided comps per listing (recent sales within
  ~0.5 mi, last 6 mo) — NOT auto-sourced in v1
- school district + `tax_history` (inventory or operator)

# Output shape

A Markdown body plus a JSON metadata sidecar:

````markdown
**Subject:** <one short line, listing address first>

Hi <lead_name>,

<one short paragraph framing the match against stated criteria
— never quote a protected-class signal>

### <address> — $<price>
- <beds>bd / <baths>ba / <sqft> sqft / HOA $<hoa>/mo
- School district: <district>
- Comps (within ~0.5 mi, last 6 mo): <count> sales, $<low>–$<high>
- Tax: assessed $<assessed> · annual $<tax>

— Jamie, on behalf of <agent_name>
````

```json
{
  "requires_approval": true,
  "lead_id": "<id>",
  "recipient_email": "<email>",
  "listing_ids": ["<id>"],
  "comp_count_per_listing": {"<listing_id>": 0},
  "generated_at": "<ISO-8601>"
}
```

# Rules

- **Facts only.** Quote price, beds, baths, sqft, HOA, district,
  comps, tax verbatim from `inventory/sample_listings.json` and
  operator-supplied comps. No qualitative copy ("great schools").
- **Fair Housing.** No neighborhood-character language; no
  inference from protected-class signals even if the lead
  self-discloses. See `prompts/system.md`.
- **Cite the comp count, not the addresses.** Give the lead a
  count and price range; addresses are the agent's call.
- **Sign the packet.** "Jamie, on behalf of <agent_name>".
- **Stop on missing inputs.** If `tax_history` or comps are
  empty, ask the operator instead of guessing.

# Approval posture

Approval-gated. `requires_approval = true` in `vertical.toml`;
the operator reviews the body before send so brand voice and
Fair Housing language are double-checked.

# Examples

```json
{
  "requires_approval": true,
  "lead_id": "L-204",
  "recipient_email": "buyer@example.com",
  "listing_ids": ["MLS-118822", "MLS-118901", "MLS-119014"],
  "comp_count_per_listing": {"MLS-118822": 4, "MLS-118901": 3, "MLS-119014": 5},
  "generated_at": "2026-05-12T15:30:00-07:00"
}
```

Abstain (no comps for a listing): drop it, note the gap.
