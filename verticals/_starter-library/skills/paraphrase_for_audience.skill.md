---
name: paraphrase_for_audience
version: "0.1.0"
description: "Rewrites a passage for a specified audience — adjusting vocabulary, register, and assumed context without changing the underlying facts."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has text that's correct but pitched at the wrong
reader — too technical for a board, too casual for a regulator,
too American-English for an international audience. Use whenever
the request is "say the same thing but for <audience>."

# Inputs

- `source`: the original text.
- `audience`: who the rewrite is for — describe role, expertise,
  context (`"non-technical board members"`, `"junior support agents"`,
  `"a 12-year-old"`).
- `preserve`: facts/figures/names that must survive verbatim.
- `length_constraint` (optional): `"same length"`, `"shorter"`,
  `"longer with more context"`. Default: same length.

# Output

The rewritten passage. Same factual content. Adjusted vocabulary,
analogies, and assumed background. No editorial preface.

# Examples

- Source `<engineering RCA>`, audience `"non-technical execs"` →
  drop stack-trace language, lead with business impact, keep
  the dollar figure and outage minutes verbatim.
- Source `<legal terms>`, audience `"customer support agents"` →
  plain language, keep clause numbers and dates from `preserve`.

# Approval posture

Drafts only. The operator (or a reviewer) confirms the rewrite
preserved meaning before sending.

# Notes

- Never invent context to "help" the audience. If the source
  doesn't say it, the rewrite doesn't say it.
- Numbers, named entities, and clause references in `preserve`
  must appear in the output verbatim. If they don't fit
  naturally, return the rewrite and a flag
  `note: "preserve item <X> dropped — please reinsert manually"`
  rather than rewording them.
- Match the audience's likely register, not a caricature of it.
  "Executive" doesn't mean "buzzwords"; it means "decision-shaped."
