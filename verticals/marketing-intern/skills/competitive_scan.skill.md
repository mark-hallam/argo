---
name: competitive_scan
version: "0.1.0"
description: "Weekly scan of named competitors for pricing changes, product launches, hiring posts, exec moves, and funding events. Cites every signal."
provenance:
  source: vertical/marketing-intern
  hash: sha256-pending
---

# When to use

Call once per week (or on operator demand) against the
configured competitor list in `memory.competitors[]`. Output
feeds `draft_newsletter` and the operator's weekly summary —
nothing here ships to the public unmediated.

# Inputs you expect

- list of competitor names + canonical URLs
- date window (default: trailing 7 days)
- optional `priority_signals` filter (e.g. `["pricing", "funding"]`)

# Tools you may invoke

- `web_search` — discovery of recent posts / press / job ads
- `web_extract` — pull primary-source pages for verbatim quotes
- `memory` — read prior scans to avoid duplicating last week's signals

# Output shape

A single fenced JSON block:

```json
[
  {
    "competitor": "<name>",
    "signal_type": "pricing|launch|hiring|exec_move|funding|other",
    "summary": "<one sentence, neutral voice>",
    "source": "<URL of primary source>",
    "date": "<YYYY-MM-DD>",
    "verbatim_quote": "<short quote when positioning is involved, else null>"
  }
]
```

# Hard rules

- **Cite every signal.** A row without a `source` URL is invalid —
  drop it. Fabricating a competitor pricing change to fill the
  weekly digest is a fireable offence in real life.
- **Primary source over aggregator.** Prefer the competitor's own
  page / press release over a blog rehash. Use the aggregator
  only if you also link the primary.
- **No rumour signals.** "I heard X is pivoting" with no public
  evidence does not belong here. If you can't link it, drop it.
- **Dedupe against last week's scan.** Read
  `memory.competitor_scans[]` first; do not re-emit a signal
  already captured. Same launch, two outlets = one row.
- **Mark stale dates explicitly.** If the most recent verifiable
  signal for a competitor is >30 days old, emit one row with
  `signal_type: "other"` and a summary noting "no fresh activity"
  rather than padding with old material.

# Examples

```json
[
  {
    "competitor": "Acme Inbox",
    "signal_type": "pricing",
    "summary": "Acme moved their Pro tier from $29 to $39/month and added a 14-day trial.",
    "source": "https://acmeinbox.example/pricing",
    "date": "2026-05-08",
    "verbatim_quote": "Pro — $39/month, billed annually"
  }
]
```

# Memory

Append the full result list to `memory.competitor_scans[]` keyed
by ISO week. Always tag every row `requires_approval: true` for
any downstream output that surfaces it publicly.
