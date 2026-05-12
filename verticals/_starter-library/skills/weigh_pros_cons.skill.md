---
name: weigh_pros_cons
version: "0.1.0"
description: "Produces a balanced pros/cons table for a decision, with an explicit weight per item and a tentative recommendation flagged as such."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator is weighing a single option (proceed / not, accept /
not, switch / stay) and wants the trade-offs laid out. Use any
time the question is "should we do <X>?" with a binary answer.

For multi-option ranking, use `rank_options_by_criteria` instead.

# Inputs

- `decision`: the one-sentence question.
- `context`: any constraints, deadlines, stakeholder asks the
  operator has volunteered.
- `weight_scale`: `"low/med/high"` (default) or `"1-5"`.

# Output

Two parallel lists — **Pros** and **Cons** — each item a short
sentence with an inline weight (`(high)`, `(med)`, `(low)` or
numeric). Followed by a 1-2 sentence **Tentative read** that
notes which side is heavier on weighted count, explicitly
labelled as the model's read, not a verdict.

# Examples

- Decision `"should we switch CRM vendors?"` → pros (lower cost,
  better API) + cons (migration risk, training) with weights;
  tentative read names the heavier side without claiming a
  decision was made.
- Decision `"accept this contract offer?"` → same shape; if
  pros and cons balance, say so plainly.

# Approval posture

Decision support, not decision-making. The operator decides.
Skill never marks a decision "made" or sends notifications.

# Notes

- Always include at least one pro AND at least one con. If you
  can't find any of either, the question isn't decision-shaped
  — push back ("this looks like it has no meaningful downside —
  is there a constraint I'm missing?").
- Never omit weight. An un-weighted pro/cons list is a worse
  artefact than no list, because it implies equal weight.
- "Tentative read" must use hedged language ("the pros weigh
  somewhat heavier"). Never use "you should" or "the right
  answer is."
