---
name: detect_gtm_pattern
version: "0.1.0"
description: "Detects repeatable GTM patterns from accumulated research, outreach drafts, and campaign outcomes. Requires evidence — refuses to call a single observation a pattern."
provenance:
  source: vertical/gtm-engineer
  hash: sha256-pending
---

# When to use

Call after `analyze_campaign` when the analysis flagged
`learned_skill_candidates` OR when the operator points at
multiple campaign-result rows in `memory.campaign_results[]` and
asks "is there a pattern".

# Inputs you expect

- account_research rows (≥3) for cross-account signals
- outreach messages + their outcomes (`memory.winning_messages[]`
  + `memory.failed_messages[]`)
- campaign outcomes (`memory.campaign_results[]`)
- operator corrections from prior reviews
- objection set (`memory.objections[]`)

# Output shape

```json
{
  "patterns": [
    {
      "pattern": "<one sentence statement>",
      "context": "<which segment/persona/timing this applies to>",
      "evidence": [
        {"source": "<campaign or row id>", "observation": "<what>"}
      ],
      "data_points": 0,
      "confidence": "low|medium|high",
      "do_not_use_when": "<the boundary>",
      "recommended_action": "watch|test|propose_skill",
      "candidate_skill_name": "<name when recommended_action == propose_skill>"
    }
  ],
  "non_patterns": [
    {"observation": "<what>", "why_not_a_pattern": "<single data point | conflicting evidence>"}
  ]
}
```

# Rules

- **Three or it isn't a pattern.** A "pattern" requires ≥3
  reinforcing observations. Two-and-a-feeling goes in
  `non_patterns` so the next analysis pass can revisit when
  more data lands.
- **Identify the boundary.** Every pattern carries
  `do_not_use_when` — the segment / persona / timing where the
  pattern *doesn't* apply. A pattern without a boundary is a
  superstition.
- **Avoid universal claims.** "Technical founders prefer X" is
  too broad. "Series A technical founders building dev tools
  responded better to X than Y in our last 12 sends" is a
  pattern.
- **Recommend explicitly.** `recommended_action`:
  - `watch` — interesting but evidence too thin to act on
  - `test` — design a controlled experiment to confirm
  - `propose_skill` — strong evidence, run `propose_gtm_skill`
- **Surface non-patterns.** "We thought X but the data doesn't
  support it" is as useful as a real pattern — it stops the
  operator from chasing it.

# Memory

Append every detected pattern to `memory.patterns[]`. Append
non-patterns as well, tagged with `confidence: "rejected"`, so
future runs don't re-propose them.
