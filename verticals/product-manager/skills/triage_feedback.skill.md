---
name: triage_feedback
version: "0.1.0"
description: "Clusters a batch of inbound user feedback (support tickets, interview snippets, NPS comments, email) into themes capped at 8, buckets by surface area and severity, and emits a JSON theme set. Pure synthesis with no external mutation; output is internal context for downstream PRD work."
provenance:
  source: vertical/product-manager
  hash: sha256-pending
---

# When to use

Call first when the operator hands over inbound feedback rows and
asks "what are users telling us?" Output drives `draft_prd` (theme →
PRD) and `update_public_roadmap` (theme → reprioritization
evidence). Themes should exist before `draft_prd` runs.

# Inputs you expect

- feedback rows (Intercom / Zendesk / email / NPS), each with a
  stable `row_id` and verbatim text
- optional surface-area hints (e.g. `auth`, `import`, `billing`) used
  as candidate bucket labels
- optional prior `feedback_themes` memory rows for delta clustering

# Output shape

```json
{
  "themes": [
    {
      "name": "<short label, ≤6 words>",
      "count": 0,
      "severity": "low|medium|high",
      "surface_area": "<area label>",
      "sample_quotes": [
        {"row_id": "<stable id>", "text": "<verbatim>"}
      ],
      "suspected_drivers": ["<short phrase>"]
    }
  ],
  "truncated": false,
  "duplicate_signal": [
    {"row_ids": ["<id>", "<id>"], "reason": "<why deduped>"}
  ]
}
```

# Examples

One theme from a 47-row batch:

```json
{"name": "Import fails on >10MB CSVs", "count": 12, "severity": "high",
 "surface_area": "import",
 "sample_quotes": [{"row_id": "ZD-4421", "text": "stuck at 60% then errors"}],
 "suspected_drivers": ["streaming parser missing", "client-side size limit"]}
```

# Approval posture

Autonomous per charter — pure synthesis, internal-only output. Never
auto-promote a theme to a PRD or roadmap change; the operator (or
`draft_prd`) decides what to do with the cluster set.

# Rules

- **Cap at 8 themes.** If input yields >8 clusters, output the top 8
  by signal-weight (count × severity-weight, ties broken by highest
  individual-row severity) and set `truncated: true`.
- **Cluster by user-stated need, not surface keyword.** Two rows that
  both say "slow" but mean different things are different themes.
- **Cite verbatim only.** `sample_quote.text` is exact source text;
  never paraphrase, never invent.
- **Dedupe explicitly.** Near-duplicate rows surface in
  `duplicate_signal`, not silently merged.
- **Never name customers.** `row_id` only; redact names to
  `<redacted>` before emitting.
