---
name: research_keywords
version: "0.1.0"
description: Research keywords for a topic — search volume, difficulty, and intent for each — citing a source for every figure, marking anything it cannot source as unavailable, and never inventing a traffic or difficulty number.
provenance:
  source: vertical/gtm-seo
  hash: sha256-pending
---

# When to use

Use when the operator gives Seth a topic, seed keyword, or page and
wants the keywords worth targeting. This is Seth's anchor skill: it turns
a topic into a sourced keyword table (volume, difficulty, intent) that
feeds the content briefs, on-page audits, and backlink work downstream.
It produces research only — nothing here is published. Publishing a page
to the live site is a separate, approval-gated action
(`publish_seo_page`).

# Inputs you expect

- `topic` — the topic, seed keyword, or page to research keywords for.
- `keyword_metrics` — volume / difficulty per candidate keyword, each
  with a source. Pulled from the Search Console / Ahrefs connectors when
  wired, from operator paste-ins or memory pointers otherwise. Any
  keyword without a sourced figure is reported "unavailable" — never
  invented.
- `prior_research` — last research row from `keyword_research` (optional),
  so trends are stated against a sourced baseline.

# Output shape

```json
{
  "topic": "ai sales assistant",
  "keywords": [
    {
      "keyword": "ai sales assistant",
      "search_volume": 8100,
      "difficulty": 42,
      "intent": "commercial",
      "source_per_metric": { "search_volume": "ahrefs", "difficulty": "ahrefs" },
      "confidence": "high"
    },
    {
      "keyword": "best ai sdr tool",
      "search_volume": "unavailable",
      "difficulty": "unavailable",
      "intent": "transactional",
      "source_per_metric": { "search_volume": null, "difficulty": null },
      "confidence": "low"
    }
  ],
  "recommended_targets": ["ai sales assistant"],
  "rationale": "1-3 sentences citing sourced figures, not vibes."
}
```

`intent` is one of `informational | navigational | commercial |
transactional`. Every numeric figure carries a `source_per_metric`
entry; a figure with no source is the string `"unavailable"` and its row
is `confidence: "low"`. `recommended_targets` names only keywords with
sourced volume and a difficulty you can actually win.

# Hard rules — truth boundary (NO THEATER)

- **Cite every figure.** Each volume and difficulty carries its source
  (Search Console, Ahrefs, operator paste-in). A figure with no source is
  **`"unavailable"`** — never estimate, round to a plausible value, or
  fill a gap. "unavailable" is a correct answer; a made-up number is not.
- **Never invent traffic, volume, difficulty, or rankings.** If you do
  not have the number, you do not write it. "I don't have a sourced
  volume" is the honest answer.
- **Thin evidence → low-confidence, never a forecast.** A keyword with
  sparse data sets `confidence: "low"` and is never inflated into a
  ranking promise. Never say a page "will rank" — that is unknowable.
- **Label inferred as inferred.** Intent you infer from the keyword is
  fine, but a volume you "estimate" is not a figure — it is unavailable.
- **Research only — nothing public.** This skill never publishes.
  Publishing a page to the live site is `publish_seo_page`, which sets
  `requires_approval: true` and stops at the operator.

# Example

Synthetic only — no real metrics.

```
topic: "ai sales assistant"
keyword_metrics:
  "ai sales assistant":  { volume: 8100, difficulty: 42, source: "ahrefs" }
  "best ai sdr tool":    { volume: unavailable }   # connector down
```

```json
{
  "topic": "ai sales assistant",
  "keywords": [
    {
      "keyword": "ai sales assistant",
      "search_volume": 8100,
      "difficulty": 42,
      "intent": "commercial",
      "source_per_metric": { "search_volume": "ahrefs", "difficulty": "ahrefs" },
      "confidence": "high"
    },
    {
      "keyword": "best ai sdr tool",
      "search_volume": "unavailable",
      "difficulty": "unavailable",
      "intent": "transactional",
      "source_per_metric": { "search_volume": null, "difficulty": null },
      "confidence": "low"
    }
  ],
  "recommended_targets": ["ai sales assistant"],
  "rationale": "Only 'ai sales assistant' has a sourced volume (8100, ahrefs) at a winnable difficulty (42). 'best ai sdr tool' is unavailable — connector down — so it stays low-confidence and is not recommended yet."
}
```
