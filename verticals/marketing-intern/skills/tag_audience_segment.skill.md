---
name: tag_audience_segment
version: "0.1.0"
description: "Categorizes an inbound social engagement (reply, mention, DM) into customer / prospect / press / community / spam. Routes to human review when confidence < 0.6."
provenance:
  source: vertical/marketing-intern
  hash: sha256-pending
---

# When to use

Call per inbound engagement before `draft_social_post` is
called to respond. The label shapes reply tone and routing,
and is logged to `memory.audience_segments[]`.

# Inputs you expect

- engagement text (verbatim)
- author handle + profile metadata when available (bio,
  follower_count, account_age_days, prior interactions)
- channel (twitter / linkedin / instagram / other)

# Tools you may invoke

- `memory` — read prior `audience_segments[]` rows for the
  same author

# Output shape

```json
{"author": "<handle>", "segment": "customer|prospect|press|community|spam",
 "confidence": 0.00, "evidence": ["<signal 1>", "<signal 2>"],
 "route": "auto-reply-ok|human-review-required", "requires_approval": true}
```

# Hard rules

- **Confidence threshold = 0.6.** If `confidence < 0.6`, set
  `route: "human-review-required"`. The skill must NOT mask
  uncertainty — a wrong label routes to a wrong reply tone.
- **`requires_approval: true` always.** Reply drafts are
  approval-gated by the vertical (see `vertical.toml`).
- **Evidence-based label.** A label with empty `evidence` is
  invalid.
- **`spam` default for empty signals.** Blank bio + zero
  followers + boilerplate text → `spam` with high confidence.
  Don't escalate spam to human review; drop it.
- **Press and customer override prospect.** A handle that is
  BOTH gets the higher-stakes label (`press`).
- **Re-use prior labels when fresh.** If
  `memory.audience_segments[]` has a row ≤30 days old, use
  it unless the current engagement contradicts it.

# Examples

```json
{"author": "@somecustomer", "segment": "customer", "confidence": 0.92,
 "evidence": ["prior support ticket #4421", "bio mentions our product"],
 "route": "auto-reply-ok", "requires_approval": true}
```

```json
{"author": "@unknownreplier", "segment": "prospect", "confidence": 0.45,
 "evidence": ["bio mentions adjacent industry only"],
 "route": "human-review-required", "requires_approval": true}
```
