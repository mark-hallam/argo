---
name: brand_mention_summary
version: "0.1.0"
description: "Collapses brand mentions from web/social search into a sentiment-bucketed report with top-3 cited mentions per bucket."
provenance:
  source: vertical/marketing-intern
  hash: sha256-pending
---

# When to use

Call after `scan_brand_mentions` populates raw mention rows for
the trailing window (default 7 days). Output feeds the weekly
internal summary the operator reads — never auto-published.

# Inputs you expect

- raw mention rows: `[{text, source, date, author?}]`
- date window (default: trailing 7 days)
- workspace brand name + known aliases

# Tools you may invoke

- `web_extract` — fetch parent pages when surrounding context
  is needed for accurate sentiment

# Output shape

A single fenced JSON block:

```json
{
  "window": {"start": "<YYYY-MM-DD>", "end": "<YYYY-MM-DD>"},
  "totals": {"positive": 0, "neutral": 0, "negative": 0, "question": 0, "complaint": 0},
  "buckets": {
    "positive": [{"text": "<verbatim>", "source": "<URL>", "date": "<YYYY-MM-DD>", "author": "<handle|null>"}],
    "neutral": [], "negative": [], "question": [], "complaint": []
  },
  "requires_approval": true
}
```

# Hard rules

- **Top-3 per bucket, by reach when known, recency otherwise.**
  Don't pad — empty bucket → empty array. Inventing mentions
  to fill buckets is a fireable offence in real life.
- **Verbatim, not paraphrased.** `text` is the actual mention
  (truncated to 280 chars with `…` if longer). Paraphrasing
  distorts sentiment in subtle ways the operator misses.
- **Cite every mention.** A row without a `source` URL is
  invalid — drop it.
- **`complaint` ≠ `negative`.** A complaint asks for a fix; a
  negative mention is opinion. Complaints route to support
  follow-up; negatives to PR awareness.
- **Approval-gated downstream.** Always emit
  `requires_approval: true`. Anything that surfaces these
  publicly (e.g. quote-back testimonial) goes through the
  operator first.

# Examples

```json
{
  "window": {"start": "2026-05-05", "end": "2026-05-12"},
  "totals": {"positive": 4, "neutral": 1, "negative": 0, "question": 2, "complaint": 1},
  "buckets": {
    "positive": [{"text": "Switched to @yourbrand. CSV export alone paid for it.",
       "source": "https://x.example/post/123", "date": "2026-05-08", "author": "@somecustomer"}],
    "complaint": [{"text": "Login flow on iOS Safari hangs after MFA.",
       "source": "https://x.example/post/124", "date": "2026-05-10", "author": "@another"}]
  },
  "requires_approval": true
}
```

# Memory

Append to `memory.brand_mention_summaries[]` keyed by ISO week.
