---
name: compare_two_documents
version: "0.1.0"
description: "Produces a structured diff of two documents — additions, removals, material changes, and unchanged sections — with operator-relevant framing."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has two versions of something (contract draft A vs
B, last week's report vs this week's, vendor proposal v1 vs v2)
and wants to know what's actually different — semantically, not
character-by-character. Use any time `diff` would be too noisy
and "what changed?" is the question.

# Inputs

- `doc_a`: the baseline text.
- `doc_b`: the comparison text.
- `lens` (optional): the angle that matters — `"financial"`,
  `"legal terms"`, `"scope"`, `"tone"`. Default: general.
- `granularity`: `"sentence"`, `"paragraph"`, `"section"`. Default
  `"paragraph"`.

# Output

Four sections:
- **Added** — content present in B, absent from A.
- **Removed** — content in A, gone from B.
- **Changed** — content that exists in both but materially shifted
  (cite both versions side-by-side, highlight the delta).
- **Unchanged** — one-line note: "<N> sections unchanged" (do not
  reproduce them).

# Examples

- Two contract drafts, lens `"financial"` → diff foregrounds
  payment terms, caps, liability numbers; ignores boilerplate
  reflows.
- Two status reports, lens `"scope"` → flags new/dropped
  initiatives, ignores prose rewrites of the same items.

# Approval posture

Read-only. The diff is for human review; the skill does not pick
"which version wins."

# Notes

- "Material change" means meaning shifted, not phrasing.
  `"due on the 5th"` → `"due by the 5th"` is not material.
  `"due on the 5th"` → `"due on the 15th"` is.
- Always include the verbatim before/after for Changed entries.
  Paraphrasing the change defeats the point.
- If `doc_a` and `doc_b` are too different to align (e.g.,
  completely different documents), say so and refuse to diff;
  do not force an alignment.
