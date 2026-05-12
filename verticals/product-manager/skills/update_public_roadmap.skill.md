---
name: update_public_roadmap
version: "0.1.0"
description: "Proposes a change to the public roadmap (add / move / reprioritize / cut a line item) and renders it as a diff envelope — before / after / reasoning / dependency check — for atomic operator approval. Approval-gated per charter; the public roadmap is an external commitment so this skill never auto-commits and never emits a frame the operator cannot diff."
requires_approval: true
provenance:
  source: vertical/product-manager
  hash: sha256-pending
---

# When to use

Call when the operator wants to propose a roadmap change driven by a
PRD, a feedback theme, or a deliberate reprioritization. The skill
never commits — it produces a diff envelope the operator reviews and
approves in a single step. If no driving evidence is supplied,
refuse and ask for either a `prd_id` or a `theme_name`.

# Inputs you expect

- `change_kind`: `add` | `reprioritize` | `cut` | `reword`
- `line_item_id`: the roadmap row being changed (omitted for `add`)
- target quarter (`Q3-2026`-style) and `summary` copy
- driver evidence: `prd_id` and/or `theme_name` from `triage_feedback`
- dependency hints (other roadmap rows or PRDs this depends on)

# Output shape

```json
{
  "change_kind": "add|reprioritize|cut|reword",
  "line_item_id": "<id-or-null-for-add>",
  "before": {"quarter": "<q>", "summary": "<copy>", "status": "<state>"},
  "after":  {"quarter": "<q>", "summary": "<copy>", "status": "<state>"},
  "reasoning": {
    "driver_prd_id": "<prd-or-null>",
    "driver_theme_name": "<theme-or-null>",
    "evidence_count": 0,
    "rationale": "<2-3 sentences>"
  },
  "dependency_check": {
    "blocks": ["<roadmap-id>", "..."],
    "blocked_by": ["<roadmap-id>", "..."],
    "conflicts": ["<concern>", "..."]
  },
  "customer_safe": true,
  "requires_approval": true
}
```

# Examples

Reprioritize an existing line driven by a high-severity theme:

```json
{"change_kind": "reprioritize", "line_item_id": "ROAD-118",
 "before": {"quarter": "Q4-2026", "summary": "Bulk CSV import (>10MB)", "status": "scheduled"},
 "after":  {"quarter": "Q3-2026", "summary": "Bulk CSV import (>10MB)", "status": "committed"},
 "reasoning": {"driver_prd_id": "PRD-204",
   "driver_theme_name": "Import fails on >10MB CSVs",
   "evidence_count": 12,
   "rationale": "12 high-severity rows over 4 weeks; pulls Q4 work forward to Q3."},
 "dependency_check": {"blocks": [], "blocked_by": ["ROAD-091"], "conflicts": []},
 "customer_safe": true, "requires_approval": true}
```

# Approval posture

**Approval-gated per charter.** The public roadmap is an external
commitment; changes affect customers, partners, and sales. Every
output is a proposal — the skill never writes through to the
roadmap, never assumes prior approval covers a new change, and
always emits `requires_approval: true`. The diff envelope is the
unit of approval: operator approves `before → after` atomically.

# Rules

- **Diff envelope only.** Every output includes `before` and `after`;
  for `add`, `before` is `null`. The operator must be able to read
  both halves without leaving the frame.
- **Cite driver evidence.** A roadmap change without `driver_prd_id`
  or `driver_theme_name` is a fabrication — refuse with a note
  asking the operator to supply one.
- **Customer-safe copy.** No internal codenames, no comparative
  competitor claims, no dated commitments unless the operator
  pre-approved a date. Set `customer_safe: false` if a draft has any
  of these so the operator catches it before approval.
- **Dependency check is non-negotiable.** Always populate
  `dependency_check`. Empty arrays are OK; missing field is not.
- **Never auto-commit.** `requires_approval: true` is always set.
