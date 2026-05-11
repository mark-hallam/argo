---
name: research_company
version: "0.1.0"
description: "Researches a single company by name+domain and produces a structured 1-pager. Distinguishes verified facts from inferred signals; abstains on weak evidence rather than padding."
provenance:
  source: vertical/lead-research
  hash: sha256-pending
capabilities: ["web_search", "web_extract", "file_read"]
---

# When to use

Call when the operator names ONE specific company (or the matching
`accounts` row needs a refresh). For ICP-wide candidate discovery,
do that upstream first and call this skill per finalist.

# Inputs you expect

- company name (required)
- company domain (optional but strongly preferred — disambiguates
  homonyms)
- ICP definition currently in scope (from `icp_definitions` memory)

# Output shape

A single fenced JSON block matching the 6-section structure in the
system prompt:

```json
{
  "company": "<name>",
  "firmographic": {
    "legal_name": "<name>",
    "hq": "<city, country|unknown>",
    "employee_band": "<1-10|11-50|51-200|201-1000|1001-5000|5000+|unknown>",
    "revenue_band": "<band or 'undisclosed'>",
    "industry": "<industry>",
    "year_founded": "<year|unknown>",
    "evidence": [{"field": "<field>", "url": "<source>"}]
  },
  "news": [
    {"date": "<YYYY-MM-DD>", "headline": "<one line>", "url": "<source>", "relevance": "<one line>"}
  ],
  "tech_stack": [
    {"signal": "<tech>", "basis": "verified|inferred", "evidence": "<source>"}
  ],
  "confidence": "low|medium|high",
  "confidence_rationale": "<one sentence>"
}
```

# Rules

- **Verified vs. inferred** on every claim. Mixing them is the
  failure mode this skill is built to prevent.
- **Cite by URL.** A signal without an evidence string is invalid;
  drop it rather than ship it.
- **Don't hallucinate recency.** If the most recent verified news
  signal is > 12 months old, return an empty `news` array and lower
  `confidence` — don't invent a "recent pivot".
- **Abstain when blind.** If only the company name is given and the
  homepage doesn't load, return `confidence: low` and the
  firmographic block with most fields `unknown`. Honest beats
  invented.
- **Connectors are optional.** When `zoominfo` / `clearbit` /
  `similarweb` MCP servers aren't installed, note it in
  `confidence_rationale` and continue with web sources.

# Memory

Update the matching `accounts` row in place when one exists; append
a new one otherwise. Always set `researched_at` so the SDR can spot
stale rows. Append a one-line summary to `recent_research` for
operator recall.
