---
name: rank_priority_list
version: "0.1.0"
description: "Ranks up to 50 researched companies into an operator-facing priority list by fit_score, buying-signal recency, sales-cycle-length estimate, and operator weights. Emits Markdown table + JSON sidecar."
capabilities: ["file_read"]
provenance:
  source: vertical/lead-research
  hash: sha256-pending
---

# When to use

Call AFTER `research_company`, `detect_buying_signals`, and
`score_lead_fit` have run for each candidate. Composes existing
signals into a ranked list — no fresh research. Operators eyeball
the top 10; Markdown is the human-review surface, the JSON sidecar
feeds downstream systems (CRM sync, `sales-sdr` outreach).

# Inputs you expect

- list of `accounts` rows with `fit_score`, `buying_signals`,
  optional `sales_cycle_length_estimate`
- operator business-context weights (e.g. "weight recency 2× this
  quarter")
- optional: ICP segment filter

# Output shape

Markdown first, JSON sidecar second:

```markdown
# Priority list — <segment or "all"> (<N> accounts)

| Rank | Company | Fit | Top signal | Recency | Priority |
| --- | --- | --- | --- | --- | --- |
| 1 | Acme Co | 88 | $40M Series B | within_7d | high |
```

```json
{"segment": "<segment or 'all'>",
 "ranked": [{"rank": 1, "company": "<name>", "fit_score": 88,
   "top_signal": "<one line>",
   "top_signal_recency": "within_7d|within_30d|within_90d|older",
   "priority": "high|medium|low",
   "rank_basis": "<one sentence>"}],
 "truncated": false, "input_count": 0}
```

# Rules

- **Cap at 50.** Excess input → emit top 50 by composite score AND
  set `truncated: true` with `input_count` for the unranked total.
- **`disqualify` rows excluded** from the ranked list — they belong
  in a separate "do not pursue" report.
- **Composite formula.** `composite = 0.5 * fit_score + 0.3 *
  recency_weight + 0.2 * cycle_weight`. `recency_weight`:
  `100/70/40/10` for the four buckets. `cycle_weight`: `100/60/30`
  for short/medium/long. Operator weights override coefficients.
- **`rank_basis` is mandatory.** Empty basis → drop the row.
  Operators read it when the rank surprises them.
- **Stable tie-breaking.** (1) more recent signal, (2) higher raw
  `fit_score`, (3) alphabetical. Deterministic ordering matters
  for diff-friendly re-runs.
- **Markdown is load-bearing.** Render cleanly at 80-col width;
  truncate company names with `…` past 24 chars in Markdown. JSON
  preserves full names.

# Examples

Truncated run:

```json
{"segment": "mid-market-saas", "truncated": true, "input_count": 73}
```

# Memory

Persist as a `priority_lists` row keyed by `(segment,
generated_at)` so operators diff week-over-week movement without
re-ranking.
