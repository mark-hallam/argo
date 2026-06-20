---
name: synthesize_voice_of_customer
version: "0.1.0"
description: Cluster a customer-feedback set into evidence-cited themes, score which segment is pulling hardest (the PMF compass), and mark thin evidence low-confidence — never fabricating a quote or statistic.
provenance:
  source: vertical/gtm-voice-of-customer
  hash: sha256-pending
---

# When to use

Use when the operator has a set of customer signals (sales notes, support
tickets, survey responses, usage data, interview transcripts) and wants
them turned into themes plus a product–market-fit (PMF) read. Produces a
clustered theme list plus a ranked segment-fit table. Neither the themes
nor the ranking is a decision — `file_product_insight` and
`request_user_interview` both `requires_approval: true`.

# Inputs you expect

- `source_set` — the feedback records to synthesize (already
  PII-minimised by the gateway). Each carries a `source_pointer`.
- `segments` — the candidate segments to score for fit (e.g. agencies,
  solo founders, mid-market ops).
- `corpus_id` — opaque ID for the audit trail.

# Output shape

```json
{
  "themes": [
    {
      "label": "Setup takes too long on day one",
      "segment": "agencies",
      "evidence_count": 14,
      "confidence": "high",
      "source_pointers": ["ticket:8821", "survey:441", "note:sales-30"]
    }
  ],
  "segment_fit": [
    {
      "segment": "agencies",
      "pull_score": 0,
      "retention_score": 0,
      "willingness_to_pay_score": 0,
      "rank": 1,
      "hardest_pulling": true,
      "confidence": "high"
    }
  ],
  "hardest_pulling_segment": "agencies",
  "rationale": "2-4 sentences citing source pointers, not vibes."
}
```

Each segment-fit dimension scores **0-100** independently. `rank` orders
the segments; exactly one row sets `hardest_pulling: true`. Show the full
table — never collapse the PMF read to a single number. `evidence_count`
is load-bearing: a theme backed by two sources is not a theme backed by
forty, and its `confidence` must reflect that.

# Hard rules

- **Cite or mark inferred.** Every theme, every score, every line of the
  rationale points to a `source_pointer`. If you draw an inference from the
  evidence, label it `inferred` and keep it separate from what the customer
  actually said.
- **Never fabricate a quote or a statistic.** If a customer did not say it,
  it does not appear. If you do not have the number, you do not invent it.
- **Thin evidence → low-confidence, never a conclusion.** A theme or
  dimension with sparse support sets `confidence: "low"` and is never
  inflated into a PMF call. When the evidence does not support a ranking,
  say "we don't have enough evidence yet" plainly.
- **Approval-gated downstream.** Filing an insight
  (`file_product_insight`) and requesting an interview
  (`request_user_interview`) both set `requires_approval: true`. Never
  claim an insight is shipped or on the roadmap, and never email a customer
  directly.

# Examples

Synthetic only — no real PII.

```
source_set: [
  "ticket:8821 (agencies) — 'I spent two hours wiring up the first project'",
  "survey:441 (agencies) — 'onboarding was the hardest part'",
  "note:sales-30 (agencies) — 'churned at day 7, setup never finished'"
]
segments: ["agencies", "solo founders", "mid-market ops"]
```

```json
{
  "themes": [
    {
      "label": "Setup is too hard in the first week",
      "segment": "agencies",
      "evidence_count": 3,
      "confidence": "medium",
      "source_pointers": ["ticket:8821", "survey:441", "note:sales-30"]
    }
  ],
  "segment_fit": [
    {
      "segment": "agencies",
      "pull_score": 62,
      "retention_score": 48,
      "willingness_to_pay_score": 70,
      "rank": 1,
      "hardest_pulling": true,
      "confidence": "medium"
    }
  ],
  "hardest_pulling_segment": "agencies",
  "rationale": "Agencies cited across ticket:8821, survey:441, note:sales-30 — strongest willingness-to-pay signal, but retention is dragged down by a day-7 setup cliff. Confidence medium: three sources, not forty."
}
```
