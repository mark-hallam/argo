---
name: summarize_long_doc
version: "0.1.0"
description: "Condenses a long document into a fixed-budget summary preserving the operator-relevant facts, not the prose."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator hands you a document (memo, report, transcript, RFC,
contract) and wants the gist in a budget they specify — a paragraph,
five bullets, three sentences. Use this skill any time the goal is
"tell me what's in this without me reading it."

# Inputs

- `document`: the full text (or a pointer the runtime can fetch).
- `budget`: target length — `"3 sentences"`, `"5 bullets"`, `"1 paragraph"`.
- `lens` (optional): the angle that matters — `"decisions"`,
  `"risks"`, `"action items"`, `"financial impact"`. Default: balanced.

# Output

Plain text or a bulleted list, sized to `budget`. No preamble
("Here is a summary…"), no trailing meta ("Let me know if…").
Facts first, opinions last, attribution only when load-bearing.

# Examples

- `<document>`, budget `"5 bullets"`, lens `"decisions"` → five
  bullets, each a single decision and its rationale.
- `<document>`, budget `"3 sentences"`, no lens → three sentences:
  what it's about, the headline finding, the next step.

# Approval posture

Read-only — no side effects. Safe to call without confirmation.
Operator reviews the summary and decides whether to act on it.

# Notes

- Cap each bullet at one line. Multi-line bullets mean you missed a
  separate point.
- If the document is shorter than the budget, return it verbatim
  and flag `note: "shorter than budget; returned as-is"`.
- Quote numbers and proper nouns verbatim — paraphrasing them is
  the leading source of summary errors.
