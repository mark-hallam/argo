---
name: search_kb
version: "0.1.0"
description: "Searches the public knowledge base for snippets relevant to a triaged ticket. Returns ranked, cited results; abstains rather than fabricates when nothing matches."
provenance:
  source: vertical/customer-support
  hash: sha256-pending
---

# When to use

Call after `triage_ticket` when the category is `how-to`, `billing`,
or any case where a knowledge-base article could resolve the ticket
without a human. Output feeds `draft_reply` — the cited snippets
become the source-of-truth the reply is grounded in. Do NOT call on
`outage` (live infra issue, KB won't help) or `abuse` (route direct
to abuse-review).

# Inputs you expect

- the triaged ticket (output of `triage_ticket`)
- the customer's plan tier (some KB articles are plan-gated)
- optional: prior `memory.kb_hits[]` entries for the same customer
  (avoid recommending an article they've already seen)

# Output shape

A single fenced JSON block:

```json
{
  "ticket_id": "<id>",
  "query_used": "<the search query you executed>",
  "results": [
    {
      "kb_id": "<id used by draft_reply for [kb:<id>] citation>",
      "title": "<article title>",
      "snippet": "<≤200-char excerpt that answers the ticket>",
      "url": "<source URL>",
      "updated": "<YYYY-MM-DD>",
      "relevance": "low|medium|high"
    }
  ],
  "ranked_by": "relevance then recency",
  "abstained": false
}
```

# Rules

- **Top 3, no padding.** Return up to 3 snippets. Empty list +
  `abstained: true` + `reason` is preferred over a fourth weak hit.
- **Citation handle is load-bearing.** `kb_id` is what
  `draft_reply` emits as `[kb:<id>]`. Use the stable article id from
  the KB, not a synthesized slug — the SPA's audit drill-in parses
  this and renders cross-system jump-links.
- **Rank by relevance, break ties on recency.** A 2-month-old
  perfect-match beats a 1-week-old tangentially-related article.
- **No fabricated snippets.** The `snippet` field is a verbatim
  excerpt from the article, not a paraphrase. If you can't quote
  it, you can't cite it.
- **Plan-gating.** Filter out articles the customer's plan tier
  can't access — recommending an enterprise-only feature to a free-
  tier customer is a known anti-pattern.
- **Dedupe against history.** Skip any `kb_id` already in
  `memory.kb_hits[]` for this customer — they've seen it once and
  it didn't help.

# Examples

Triaged ticket category=how-to, query "how do I reset 2FA" →

```json
{
  "ticket_id": "T-3110",
  "query_used": "reset 2FA without backup codes",
  "results": [
    {
      "kb_id": "KB-2034",
      "title": "Resetting two-factor authentication",
      "snippet": "If you've lost your backup codes, ask your workspace admin to reset 2FA from the team settings page.",
      "url": "https://help.example.com/kb/2034",
      "updated": "2026-03-18",
      "relevance": "high"
    }
  ],
  "ranked_by": "relevance then recency",
  "abstained": false
}
```

# Memory schema notes

Append every emitted `kb_id` to `memory.kb_hits[]` so future
invocations can dedupe. The schema field is a future addition;
document the intent here so `memory/schema.toml` can adopt it.
