---
name: detect_buying_signals
version: "0.1.0"
description: "Identifies buying signals (hiring, funding, exec moves, launches, tech migrations, regulatory pressure) for a company from recent news / web_extract output. Every signal cited; deterministic 3-level confidence vocabulary."
provenance:
  source: vertical/lead-research
  hash: sha256-pending
capabilities: ["web_search", "web_extract", "file_read"]
---

# When to use

Call after `research_company` populates the `accounts` row, or
when the operator names a company to triggered-watch. Pairs with
`score_lead_fit` and `rank_priority_list`. Buying signals are
about TIMING: a weaker-fit account with a last-week funding
event beats a perfect-fit account that went quiet 18 months ago.

# Inputs you expect

- company name (required)
- web-extract / news feed (press releases, blog posts, job-board
  listings, regulatory filings)
- ICP definition currently in scope (filters relevant signal types)

# Output shape

```json
{
  "company": "<name>",
  "signals": [
    {
      "signal": "<one-line description>",
      "signal_type": "hiring|funding|exec_move|product_launch|tech_migration|regulatory|other",
      "evidence": {"url": "<source URL>", "date": "<YYYY-MM-DD>"},
      "recency": "within_7d|within_30d|within_90d|older",
      "confidence": "high|medium|low"
    }
  ],
  "summary": "<one sentence on the strongest signal>"
}
```

# Rules

- **Deterministic confidence vocabulary.** Exactly three values:
  `high` (verified primary source — company blog, press release,
  SEC filing, official job-board posting), `medium` (credible
  secondary — TechCrunch, BuiltIn, LinkedIn role changes), `low`
  (single-source rumor). Never invent a fourth bucket.
- **Cite every signal.** A signal without `evidence.url` and
  `evidence.date` is invalid — drop it rather than ship it. No
  fabricated signals; charter parity with `research_company`.
- **Recency buckets are explicit.** `within_7d` / `within_30d` /
  `within_90d` / `older`, computed against today. Undated signals
  go to `older` with a `medium` ceiling on confidence.
- **High-confidence + recent wins on weaker fit.** A `within_7d`
  high-confidence funding event ranks above a perfect-fit account
  with no triggers. Salespeople prize timing.
- **Hiring is directional.** A burst of GTM hires is `medium`
  confidence — intent, not confirmation. Inflate to `high` only
  when the posting names the exact problem your ICP solves.
- **No PII scraping.** Exec-move signals come from public press
  or LinkedIn role-change announcements only.

# Examples

High-confidence recent funding:

```json
{"signal": "Closed $40M Series B led by Acme Ventures",
 "signal_type": "funding",
 "evidence": {"url": "https://acmeco.com/blog/series-b", "date": "2026-05-08"},
 "recency": "within_7d", "confidence": "high"}
```

Inferred tech migration from hiring:

```json
{"signal": "Hiring multiple Snowflake engineers — likely migrating off Redshift",
 "signal_type": "tech_migration",
 "evidence": {"url": "https://jobs.lever.co/acmeco/snowflake-eng", "date": "2026-02-15"},
 "recency": "within_90d", "confidence": "medium"}
```

# Memory

Append to the `accounts` row's `buying_signals` array with
`detected_at`. Re-runs replace signals older than 90 days; newer
ones merge in.
