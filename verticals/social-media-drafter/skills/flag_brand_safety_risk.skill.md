---
name: flag_brand_safety_risk
version: "0.1.0"
description: Scan a drafted post for brand-safety risks (legal claims, competitor disparagement, controversial topics, undisclosed sponsorship, regulated-industry compliance) and return a 3-severity verdict. A blocker always halts; never auto-posts.
provenance:
  source: vertical/social-media-drafter
  hash: sha256-pending
---

# When to use

Use after `draft_post` / `tone_check` and before any scheduler
hand-off. Read-only over the draft — flags risks but never mutates
or posts. Operator decides whether to ship, revise, or hold.

# Inputs you expect

- `draft_body` — the proposed post text, exactly as it would send.
- `channel` — `x` / `linkedin` / `instagram` / `facebook`. Drives
  channel-specific rules (e.g. disclosure norms).
- `brand_voice_id` — optional; scopes approved claims for the
  legal-claim check.
- `compliance_framework` — read from `memory.brand_compliance_framework`.
  One of `none` (default), `HIPAA`, `FINRA`, `FERPA`.

# Output shape

```json
{
  "risks": [
    {
      "type": "legal_claim",
      "severity": "advisory",
      "evidence_quote": "the most effective treatment available",
      "suggested_revision": "soften, or cite an approved_claims entry"
    }
  ],
  "overall_recommendation": "hold_for_legal_review",
  "next_action": {
    "name": "route_to_operator_review",
    "requires_approval": true,
    "auto_send_allowed": false
  },
  "flags": []
}
```

# Risk taxonomy

- `legal_claim` — superlatives, efficacy claims, guarantees, or any
  factual claim not in `brand_voice.approved_claims`.
- `competitor_disparagement` — named competitor + negative framing.
- `controversial_topic` — politics / religion / hot-button issues
  outside the brand's declared voice.
- `undisclosed_sponsorship` — paid-partnership language without
  `#ad` / `#sponsored` / channel-appropriate disclosure.
- `regulated_industry` — fires only when `compliance_framework != none`
  (HIPAA → PHI; FINRA → performance claims; FERPA → student info).

# Severity vocab (exactly 3) and Action vocab (exactly 3)

- Severity: `info` (surface only) / `advisory` (recommended revision;
  may still ship) / `blocker` (never ship as-is; always halts).
- Action: `send_as_is` (no risks above `info`) /
  `revise_per_suggestion` (≥ 1 `advisory`, zero `blocker`) /
  `hold_for_legal_review` (any `blocker` present).

# Hard rules

- **Any blocker halts.** A single `blocker` forces
  `overall_recommendation: "hold_for_legal_review"` regardless of
  other severities. Never average, weight, or down-rank a blocker.
- **No auto-post.** `auto_send_allowed: false` is load-bearing;
  no code path flips it to true.
- **Read-only.** Returns revisions but never mutates `draft_body`.
- **Compliance gates regulated checks.** With `compliance_framework:
  none` the `regulated_industry` check is skipped; never flag
  PHI/PII patterns speculatively without a declared framework.
- **Verbatim evidence.** `evidence_quote` is a literal substring of
  `draft_body`; never paraphrased. Empty quote → no finding.
- **Synthetic data in examples.** No real names, claims, or PII.

# Examples

Synthetic — LinkedIn draft "Synthetic-Co is the most effective
platform — switch from CompetitorY today" → `legal_claim: advisory`
+ `competitor_disparagement: advisory`, no blockers →
`revise_per_suggestion`. Add "in partnership with @SyntheticPartner"
without `#ad` → `undisclosed_sponsorship: blocker`, flips to
`hold_for_legal_review`.
