---
name: draft_creative_concept
version: "0.1.0"
description: Draft a creative concept (big idea + visual direction) from a brief, ALWAYS on the locked SKYCOT brand (gradient purple #8B5CF6 → PINK #EC4899; fonts Audiowide / Inter / JetBrains Mono). Concepts are autonomous; commissioning paid production is approval-gated.
provenance:
  source: vertical/gtm-brand-creative
  hash: sha256-pending
---

# When to use

Use whenever the operator asks Vinnie for a creative concept, or at the
start of any creative brief. This is Vinnie's anchor skill: it turns a
brief into a single strong concept — the big idea plus a visual direction
— that consumes the locked SKYCOT brand tokens. The concept is a document
output; it never produces a paid asset and never publishes anything.

# Inputs you expect

- `brief` — what the concept is for (campaign, launch, channel, message).
- `audience` — who it is for (optional; synthetic placeholders only in
  eval / demo prompts).
- `constraints` — any hard constraints (channel, format, length). The
  brand is NOT a negotiable constraint — it is always the locked SKYCOT
  brand regardless of what the brief asks for.

# Output shape

```json
{
  "concept": {
    "brief": "...",
    "big_idea": "...",
    "visual_direction": "...",
    "brand_tokens": {
      "gradient": "purple #8B5CF6 → pink #EC4899",
      "fonts": { "display": "Audiowide", "body": "Inter", "code": "JetBrains Mono" }
    },
    "on_brand": true
  },
  "claims_needing_source": [],
  "next_action": {
    "name": "request_asset_production",
    "requires_approval": true
  }
}
```

`brand_tokens` is mandatory and is ALWAYS the locked set above —
`on_brand` is `true` only when the concept consumes exactly those tokens.
`next_action` is optional: include it only when the concept is ready to
become a paid asset, and it is always approval-gated.

# Hard rules — brand boundary (LOCKED)

The SKYCOT brand is locked and non-negotiable.

- **Always the locked tokens.** Gradient purple `#8B5CF6` → PINK
  `#EC4899`; fonts Audiowide (display) / Inter (body) / JetBrains Mono
  (code). Every concept consumes exactly these.
- **The purple→cyan `#0891B2` gradient is DEPRECATED — never use it.** Do
  not produce it, offer it as an option, or mention it as a colourway.
- **Refuse off-brand directions.** If the brief asks for purple-to-cyan,
  an off-brand colourway, or a different display font, refuse the off-brand
  direction and redirect to the locked brand. Do not comply.

# Hard rules — truth boundary (NO THEATER)

- **Never fabricate a metric, statistic, or customer quote.** If a claim
  needs a number or a quote, cite the source — or list it in
  `claims_needing_source` and keep it explicitly low-confidence. A made-up
  stat is never acceptable.
- **Thin evidence stays low-confidence.** Do not inflate a weak signal into
  a headline claim.
- **Inferred is labelled inferred.** Separate sourced from inferred.

# Approval boundary

- **Concepts are autonomous.** This skill produces a concept document with
  no external mutation; it runs without approval.
- **Paid production is gated.** When a concept should become a paid asset,
  the only path is `request_asset_production`, which is approval-gated
  (`requires_approval: true`). Vinnie NEVER commissions paid work itself —
  the request stops at the operator. Never claim an asset is "produced" or
  "live"; only that production is proposed.

# Example

Synthetic only — no real metrics or quotes.

```
brief: "Launch teaser for the Skycot teammate marketplace."
audience: "Founders evaluating AI teammates."
constraints: "Single hero frame, social-first."
```

```json
{
  "concept": {
    "brief": "Launch teaser for the Skycot teammate marketplace.",
    "big_idea": "Hire a teammate, not a tool — a roster that builds itself.",
    "visual_direction": "Audiowide wordmark over a purple→pink gradient field; roster cards in Inter; one JetBrains Mono code line as a proof detail.",
    "brand_tokens": {
      "gradient": "purple #8B5CF6 → pink #EC4899",
      "fonts": { "display": "Audiowide", "body": "Inter", "code": "JetBrains Mono" }
    },
    "on_brand": true
  },
  "claims_needing_source": [],
  "next_action": { "name": "request_asset_production", "requires_approval": true }
}
```
