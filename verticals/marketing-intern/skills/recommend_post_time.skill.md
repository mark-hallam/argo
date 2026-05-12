---
name: recommend_post_time
version: "0.1.0"
description: "Recommends 3 optimal post times for the upcoming week given audience + channel. Marks generic-best-practice fallback explicitly when memory is empty."
provenance:
  source: vertical/marketing-intern
  hash: sha256-pending
---

# When to use

Call when the operator is scheduling drafts from
`draft_social_post` and asks "when should we post these?".
Output is a recommendation; the skill never auto-publishes.

# Inputs you expect

- target audience (e.g. "us-west developers")
- channel (e.g. "twitter", "linkedin")
- timezone hint (IANA tz, e.g. `America/Los_Angeles`)
- optional `lookback_days` (default 28)

# Tools you may invoke

- `memory` — read `social_engagement[]` for prior post-time +
  engagement pairs

# Output shape

```json
{
  "channel": "<name>", "audience": "<descriptor>", "timezone": "<IANA tz>",
  "recommendations": [
    {"day": "Tue", "local_time": "09:30", "rationale": "<one sentence>"},
    {"day": "Wed", "local_time": "12:15", "rationale": "<one sentence>"},
    {"day": "Thu", "local_time": "16:00", "rationale": "<one sentence>"}
  ],
  "assumption": "memory-driven|generic-best-practice", "sample_size": 0
}
```

# Hard rules

- **`assumption` field is mandatory.** Set `"memory-driven"`
  only when `social_engagement[]` has ≥10 rows for the
  requested channel within `lookback_days`. Otherwise set
  `"generic-best-practice"` — never blend silently. Operators
  trust bespoke data disproportionately.
- **Three recommendations, exactly.** Not five (paralysis),
  not one (no choice).
- **Local time, not UTC.** State the tz in the output.
- **Rationale per row.** Bare timestamps don't help.
- **No publishing side effect.** The operator owns scheduling.

# Examples

```json
{
  "channel": "linkedin", "audience": "uk smb founders", "timezone": "Europe/London",
  "recommendations": [
    {"day": "Tue", "local_time": "08:15", "rationale": "Pre-standup scroll for founders."},
    {"day": "Wed", "local_time": "12:30", "rationale": "Lunch-break engagement peak."},
    {"day": "Thu", "local_time": "17:00", "rationale": "End-of-day catch-up window."}
  ],
  "assumption": "generic-best-practice", "sample_size": 0
}
```
