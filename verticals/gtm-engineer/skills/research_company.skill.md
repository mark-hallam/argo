---
name: research_company
version: "0.1.0"
description: "Researches a single company for GTM relevance. Distinguishes verified facts from inferred pains and abstains from personalization when evidence is weak."
provenance:
  source: vertical/gtm-engineer
  hash: sha256-pending
---

# When to use

Call per-account after `find_target_accounts` lists candidates,
or whenever the operator names a single company. The output
feeds `generate_outreach` and is the source of truth for the
fit-score on `memory.account_research[]`.

# Inputs you expect

- company name (and URL when available)
- ICP definition currently in scope
- current GTM hypothesis (what experiment are we running?)

# Output shape

A single fenced JSON block:

```json
{
  "company": "<name>",
  "summary": "<one paragraph, verified facts only>",
  "relevant_signals": [
    {"signal": "<what>", "evidence": "<source>", "recency": "<date or 'unknown'>"}
  ],
  "likely_pains": [
    {"pain": "<what>", "basis": "<verified|inferred>", "confidence": "low|medium|high"}
  ],
  "buying_triggers": ["<trigger 1>", "<trigger 2>"],
  "fit_score": "low|medium|high",
  "outreach_angle": {
    "angle": "<one sentence>",
    "evidence": "<the verified signal this is grounded in>"
  } 
}
```

# Rules

- **Verified vs. inferred.** Every `likely_pains` entry carries
  `basis: "verified"` (sourced) or `basis: "inferred"` (your
  hypothesis from the signals). Mixing the two is the most
  common path to bad outreach.
- **Cite the source per signal.** A signal without an evidence
  string ("the company recently…" → from where?) is invalid —
  drop it.
- **Don't hallucinate recent events.** If the most recent
  verified signal is 18 months old, say so. Pretending a "recent
  pivot" exists is the failure mode this skill is built to
  prevent.
- **Abstain on personalization when evidence is weak.** When all
  you have is the company name, return `outreach_angle: null`
  and an honest `fit_score: "low"` — `generate_outreach` will
  refuse to draft from a null angle.
- **Explain non-fits too.** A "this company is NOT a fit because
  X" is as valuable as a positive — it stops the operator from
  re-researching.

# How to source

1. `web_extract` the company's own URL (about page, recent blog
   posts, press releases).
2. `web_search` for "<company> funding", "<company> hiring",
   "<company> launch" within the last 12 months.
3. Cross-reference any pain-signal claim with at least one
   independent source.

# Memory

Update the matching `memory.account_research[]` row in place
when one exists; append a new one otherwise. Always include the
date of research so the operator can spot stale rows.
