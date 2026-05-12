---
name: rank_options_by_criteria
version: "0.1.0"
description: "Ranks N options against a weighted criteria set, returning a transparent scorecard — not a black-box recommendation."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has multiple options (vendors, candidates, designs,
proposals) and a set of criteria with relative weights. Use any
time the request is "which of these <options> wins on
<criteria>?"

For a single yes/no, use `weigh_pros_cons` instead.

# Inputs

- `options`: list of `{name, attributes}` — each option's data
  on the criteria of interest.
- `criteria`: list of `{name, weight, direction}` where direction
  is `higher_is_better` or `lower_is_better`.
- `tie_break` (optional): a criterion to use when scores are
  within 5%. Default: lowest-weight criterion.

# Output

A scorecard table:
- One row per option, one column per criterion, plus a
  `weighted_total` column and a final `rank` column.
- Below the table: a 1-2 sentence read on the winner and the
  closest runner-up, naming the criterion that separated them.
- A `_caveats` note if any criterion was inferred or scored on
  thin data.

# Examples

- 4 vendors, criteria `[price, sla, region, support]` with
  weights `[3, 2, 2, 1]` → 4×6 table with winner identified
  and the margin to second place.
- 2 designs scored within 5% on weighted total → tie-break
  criterion explicitly cited.

# Approval posture

Decision support. The scorecard is the artefact; the operator
decides. Skill does not pick winners silently — the score path
is always visible.

# Notes

- Refuse to rank if weights are not supplied. Equal weighting is
  rarely what the operator actually wants; ask before assuming.
- Normalise each criterion to a 0-1 scale before weighted sum,
  otherwise raw-unit dominance (a $-criterion with values in the
  thousands) swamps everything else.
- Always show the per-criterion score, not just the total. A
  black-box rank with no breakdown is unusable for review.
- If two options are within 1% on weighted total, label them
  "effectively tied" — do not pick a winner on rounding noise.
