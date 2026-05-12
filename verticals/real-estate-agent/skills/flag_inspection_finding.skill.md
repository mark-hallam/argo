---
name: flag_inspection_finding
version: "0.1.0"
description: "Categorizes inspection-report findings by severity (cosmetic / functional / safety / structural / environmental) and estimated remediation cost band. Output JSON. Always flags safety, structural, and environmental findings as 'requires immediate operator review' regardless of remediation cost. Approval-gated; the licensed agent owns any re-negotiation."
provenance:
  source: vertical/real-estate-agent
  hash: sha256-pending
---

# When to use

Call when the inspection report for a `listing_id` is back
AND the operator needs each finding categorized so the agent
can decide on re-negotiation, walk-away, or accept-as-is.
Output feeds the `flag_inspection_finding` action which
routes the categorization to the agent.

# Inputs you expect

- `listing_id`, `buyer_name`
- `inspector_name`, `inspected_at` (ISO date)
- `findings[]` — verbatim text from the report
- operator-supplied remediation cost estimates per finding
  (USD ranges from the agent's contractor network)

# Output shape

A single JSON block:

```json
{
  "listing_id": "<id>",
  "buyer_name": "<name>",
  "inspector_name": "<name>",
  "inspected_at": "<YYYY-MM-DD>",
  "findings": [
    {"summary": "<short verbatim quote>", "severity": "cosmetic|functional|safety|structural|environmental", "remediation_cost_band_usd": "0-500|500-2k|2k-10k|10k-50k|50k+", "requires_immediate_review": true, "rationale": "<why this severity>"}
  ],
  "summary_counts": {"cosmetic": 0, "functional": 0, "safety": 0, "structural": 0, "environmental": 0},
  "requires_approval": true
}
```

# Severity vocabulary (load-bearing — do not change order)

1. **`cosmetic`** — paint, scuffs, finish. No functional impact.
2. **`functional`** — appliance, fixture, system not working
   but not a hazard (broken disposal, one dead outlet).
3. **`safety`** — could injure an occupant (exposed wiring,
   missing handrail, gas leak risk). **Always
   `requires_immediate_review: true`.**
4. **`structural`** — foundation, load-bearing walls, roof
   structure, major framing. **Always immediate review.**
5. **`environmental`** — mold, asbestos, lead paint, radon,
   storage-tank, soil contamination. **Always immediate review.**

`cosmetic` and `functional` set `requires_immediate_review`
based on cost band: `10k-50k` or higher → `true`, else `false`.

# Rules

- **Order is durable.** Use the 5 categories in this exact
  order in every output and example.
- **One severity per finding.** If a finding spans categories
  (mold causing structural rot), emit TWO findings — one
  environmental, one structural — both flagged.
- **Cost band, not point estimate.** Bands prevent false precision.
- **Quote the report.** `summary` is a short verbatim quote,
  not a paraphrase, so the agent can cross-reference.
- **Never draft a re-negotiation message.** That is the
  agent's licensed work; this skill categorizes only.

# Approval posture

Approval-gated. `requires_approval = true` in `vertical.toml`.
The action is human-executor (routing-only) — no money or
documents move. Safety, structural, and environmental
findings ALWAYS surface to the agent regardless of cost band.

# Examples

```json
{
  "listing_id": "MLS-118822",
  "buyer_name": "Sam Buyer",
  "inspector_name": "Acme Home Inspectors",
  "inspected_at": "2026-05-08",
  "findings": [
    {"summary": "Hairline crack on master-bath ceiling", "severity": "cosmetic", "remediation_cost_band_usd": "0-500", "requires_immediate_review": false, "rationale": "paint-only repair"},
    {"summary": "Exposed wiring above water heater", "severity": "safety", "remediation_cost_band_usd": "500-2k", "requires_immediate_review": true, "rationale": "shock + water proximity hazard"},
    {"summary": "Visible mold colony in crawlspace", "severity": "environmental", "remediation_cost_band_usd": "2k-10k", "requires_immediate_review": true, "rationale": "environmental — always immediate"}
  ],
  "summary_counts": {"cosmetic": 1, "functional": 0, "safety": 1, "structural": 0, "environmental": 1},
  "requires_approval": true
}
```
