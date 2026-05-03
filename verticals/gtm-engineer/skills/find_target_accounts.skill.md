---
name: find_target_accounts
version: "0.1.0"
description: "Identifies target accounts that match the ICP. Prioritises strong signal-matches over broad lists; flags missing evidence rather than inventing firmographic detail."
provenance:
  source: vertical/gtm-engineer
  hash: sha256-pending
---

# When to use

Call after `define_icp` has produced a profile worth targeting.
Use `web_search` + `web_extract` to gather candidate accounts;
never invent firmographic facts when search returns nothing.

# Inputs you expect

- ICP definition (from `define_icp` or `memory.icp[]`)
- target segment (which slice of the ICP this run is for)
- search constraints (how many accounts, geographic limits,
  exclude-list)

# Output shape

A single fenced JSON block:

```json
{
  "target_segment": "<segment name>",
  "accounts": [
    {
      "company": "<name>",
      "fit_rationale": "<one sentence on why this matches the ICP>",
      "observed_signals": ["<signal 1>", "<signal 2>"],
      "fit_score": "low|medium|high",
      "evidence": ["<sourced fact 1>", "<sourced fact 2>"]
    }
  ],
  "ranked_by": "fit_score then signal_recency",
  "missing_evidence": ["<what we couldn't verify>"]
}
```

# Rules

- **Prioritise strong signals.** A 5-account list where every row
  has a verified trigger signal beats a 50-account list of
  inferred fits.
- **No invented firmographics.** If you can't find headcount,
  funding, or industry from a source, mark `fit_score: "low"`
  and put the gap in `missing_evidence` — don't fabricate.
- **Rank by fit AND timing.** A perfect-fit account that was
  funded 2 years ago is worse than a 70%-fit account that just
  closed a Series B.
- **Empty list is a valid answer.** When evidence is too thin,
  return `accounts: []` + a populated `missing_evidence` array
  rather than padding.
- **Cap at the operator's requested size.** When the operator
  says "give me 10", give them up to 10 — never more.

# How to source

1. `web_search` for ICP-matched signals (recent funding, key
   hires, product launches relevant to the trigger signals).
2. `web_extract` the canonical source (company blog, press
   release, public filing) before claiming a fact.
3. Cross-reference at least one independent source per
   `evidence` entry.

# Memory

Append each account row to `memory.account_research[]` so future
runs can diff against prior research and avoid duplicate work.
