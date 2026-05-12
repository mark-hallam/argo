---
name: synthesize_signal_cluster
description: Given a batch of related observations (news items, filings, social signals), cluster into 3-5 themes with confidence ratings and per-theme evidence citations.
inputs:
  - observations
  - scope_note
outputs:
  - themes
  - truncated
  - methodology_note
  - sources_consulted
---

# Skill: Synthesize Signal Cluster

Group a batch of related observations into a small set of themes the
analyst can hold simultaneously. Output is structured JSON.

1. **Cluster** — group the observations by underlying driver, not by
   surface topic. Two filings about a different company can belong in
   the same theme if the driver is the same (e.g. same regulator, same
   customer churn pattern, same supply constraint). One signal can
   appear under at most one theme; pick the strongest fit.
2. **Cap at 5 themes** — humans cannot hold more than roughly five
   simultaneous themes. If clustering yields more, merge the two
   weakest by evidence count, or fold low-evidence themes into a
   residual `other` bucket and set `truncated: true` in the output.
   Truncation is honest, not lossy — name what was dropped in
   `methodology_note`.
3. **Theme naming** — descriptive, not evocative. "Tier-2 cloud
   providers losing enterprise deals to hyperscalers" beats "the
   great cloud unbundling". Prefer the noun-phrase form the operator
   could paste into a brief without rewriting.
4. **Confidence per theme** — assign one of:
   - `high` — ≥3 independent tier-1 / tier-2 signals, all dated within
     the scope window, no contrary signal in the batch.
   - `medium` — 2 independent signals OR 3+ with one tier-3 source OR
     mixed recency.
   - `low` — single source, contested signals, or majority tier-3.
5. **Evidence per theme** — every theme carries an `evidence` array.
   Each entry: `{source, tier: 1|2|3, date_accessed, excerpt_or_claim}`.
   No theme ships without ≥1 cited signal. No paraphrasing without an
   anchor URL or filing reference.
6. **Methodology note** — declare the clustering rule, the input
   batch size, the dropped-to-other count, and the scope window.

Output shape:

```json
{
  "themes": [
    {"label": "...", "confidence": "high|medium|low", "evidence": [...]}
  ],
  "truncated": false,
  "methodology_note": "...",
  "sources_consulted": [...]
}
```

# Hard rules

- Never invent a theme to round out a tidy count. Three well-evidenced
  themes beat five with one fabricated.
- Never cite a signal that was not in the input batch. The skill
  clusters what it was given; it does not fetch.
- If the input batch is fewer than 3 signals, do NOT cluster. Return
  `themes: []`, `truncated: false`, and a methodology note
  `"insufficient_signals: need >=3 to cluster"`.
- If a signal looks like MNPI (sealed dockets, pre-announcement deal
  terms, embargoed earnings), STOP. Do not cluster. Escalate with
  `flags: ["mnpi_risk"]`.
