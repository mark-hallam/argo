---
name: estimate_with_bounds
version: "0.1.0"
description: "Produces a low/best/high estimate for a quantity, with the load-bearing assumptions named — explicit ranges, not single-point guesses."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator needs a number where no exact figure is available —
a project's effort, a market size, a budget line, a timeline.
Use any time the question is "roughly how much/long/many?"

# Inputs

- `question`: what is being estimated, in one sentence with the
  unit ("how many engineering weeks to ship X", "what is the
  TAM for Y in EMEA").
- `known_inputs`: facts/numbers the operator has supplied that
  should constrain the estimate.
- `confidence`: `"90%"`, `"80%"`, `"50%"` — the operator's
  desired confidence interval for the low/high bounds. Default
  `"80%"`.

# Output

Three numbers — `low`, `best`, `high` — each with its unit.
Below them: a list of 3-5 named **Assumptions**, each one a
plain-language statement of a value that, if wrong by 2×, would
move the estimate noticeably. A `confidence` field echoes the
input.

# Examples

- "Engineering weeks for feature X" with known inputs (team
  size, similar past project) → low/best/high in weeks +
  assumptions naming scope creep, dependency-team latency, etc.
- "Annual revenue impact of churn reduction from 5% to 3%" →
  $ low/best/high + assumptions naming ARPU stability, time
  horizon, retention curve shape.

# Approval posture

Decision support. The operator weighs the range against their
risk tolerance and decides. Skill does not collapse the range
to a single number "for convenience" — that loses signal.

# Notes

- `best` is NOT the midpoint of `low` and `high`. It's the
  modal expectation. The bounds are asymmetric in real estimates.
- Refuse to estimate if no `known_inputs` are supplied AND the
  question is unfamiliar (i.e., couldn't be Fermi-estimated from
  generic priors). Ask the operator for at least one anchor.
- Name assumptions in language the operator can falsify ("we
  assume the API team responds in <5 business days"), not
  buzzwords.
- For probability-style questions, return a single probability
  range, not three numbers — and flag in `note`.
