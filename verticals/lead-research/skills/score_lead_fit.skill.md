---
name: score_lead_fit
version: "0.1.0"
description: "Scores a researched company against an ICP across 6 dimensions (segment/size/tech-stack/pain/geo/vertical). Prefers a disqualifier path over forcing low; recommends explicit priority bucket."
capabilities: ["file_read"]
provenance:
  source: vertical/lead-research
  hash: sha256-pending
---

# When to use

Call after `research_company` (and optionally
`detect_buying_signals`) populate the `accounts` row. This is
**top-of-funnel ICP screening**: does this account belong in the
pipeline at all? Contrast with `gtm-engineer/research_company`
which scores per-message fit for already-qualified accounts.

# Inputs you expect

- ICP definition (segment, size band, required tech-stack, pain
  signals, geography, vertical)
- the `accounts` row (firmographic + tech_stack + buying_signals
  + likely_pains, produced upstream)
- optional: operator override weights per dimension

# Output shape

```json
{
  "company": "<name>",
  "overall_fit": 0,
  "dimension_breakdown": {
    "segment":    {"score": 0, "evidence": "<one line>"},
    "size":       {"score": 0, "evidence": "<one line>"},
    "tech_stack": {"score": 0, "evidence": "<one line>"},
    "pain_signal":{"score": 0, "evidence": "<one line>"},
    "geography":  {"score": 0, "evidence": "<one line>"},
    "vertical":   {"score": 0, "evidence": "<one line>"}
  },
  "disqualifiers": ["<reason>"],
  "recommended_priority": "high|medium|low|disqualify"
}
```

`overall_fit` and dimension scores are `0–100`. `disqualifiers`
is empty when none apply.

# Rules

- **Canonical 6-dimension vocabulary.** `segment` / `size` /
  `tech_stack` / `pain_signal` / `geography` / `vertical`. Names
  are load-bearing — don't rename, collapse, or add a 7th.
- **Disqualifier path is preferred over forced-low.** Wrong
  vertical, banned geography, or a hard ICP gate (e.g. headcount
  5×below the floor) → `recommended_priority: "disqualify"` with
  the reason listed. A polite `low` for an account the operator
  should never see pollutes the funnel.
- **`overall_fit` is weighted, not averaged.** Defaults: segment
  25%, pain_signal 25%, size 15%, vertical 15%, tech_stack 10%,
  geography 10%. Operator weights override.
- **Cite per dimension.** Empty evidence forces that dimension to
  `0` — never invent evidence to keep the score presentable.
- **Priority bands.** `high`: `≥75` AND no disqualifiers AND ≥1
  recent buying signal. `medium`: `50–74` OR (`≥75` without
  signals). `low`: `25–49`. Below 25 or any disqualifier →
  `disqualify`.
- **No fabrication.** Missing data → score `0` with evidence
  `"unknown — not yet researched"`. Don't infer.

# Examples

Disqualified (wrong vertical):

```json
{"company": "Acme Defense Co", "overall_fit": 12,
 "dimension_breakdown": {"vertical": {"score": 0, "evidence": "Defense — excluded by ICP"}},
 "disqualifiers": ["Defense vertical is on the operator's exclude list"],
 "recommended_priority": "disqualify"}
```

# Memory

Append the score to the `accounts` row as `fit_score` with
`scored_at`, so the SDR can spot drift on ICP or signal refresh.
