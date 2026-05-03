---
name: propose_gtm_skill
version: "0.1.0"
description: "Proposes a reusable learned skill from a strong, evidence-backed GTM pattern. Always queues the proposal in pending_review — never auto-activates. Refuses to propose from one-off wins."
provenance:
  source: vertical/gtm-engineer
  hash: sha256-pending
---

# When to use

Call **only** when `detect_gtm_pattern` has marked a pattern with
`recommended_action: "propose_skill"` AND the pattern carries ≥3
reinforcing observations across distinct campaigns or accounts.

This skill is the bridge between "we noticed something" and "the
agent will reuse it". The lifecycle gate (#681) holds proposals
in `pending_review` state until the operator approves; this
skill produces the artefact the operator reviews.

# Inputs you expect

- detected pattern (a single row from
  `memory.patterns[]` with `recommended_action: "propose_skill"`)
- supporting evidence (the rows in `winning_messages[]`,
  `campaign_results[]`, or `objections[]` that back the pattern)
- prior operator corrections that informed the pattern
- counterexamples — campaigns where the pattern did NOT apply

# Output shape

```json
{
  "proposed_skill": {
    "name": "<snake_case_skill_name>",
    "version": "0.1.0",
    "description": "<one sentence on what this skill does>",
    "lifecycle_state": "pending_review",
    "scope": {
      "vertical": "gtm-engineer",
      "scope_to_vertical": true
    },
    "trigger_conditions": ["<when the agent should call this skill>"],
    "instructions": "<the body of the skill — what the agent should do when triggered>",
    "evidence": [
      {"source": "<campaign or pattern id>", "observation": "<what>"}
    ],
    "counterexamples": [
      {"context": "<where it didn't apply>", "observation": "<what happened>"}
    ],
    "expected_reuse": "<how often we expect this skill to fire>",
    "risks": ["<what could mis-land>"],
    "review_questions_for_operator": [
      "<the 1-3 questions the operator should answer before approving>"
    ]
  }
}
```

# Rules

- **`lifecycle_state` MUST be `"pending_review"`.** Never
  `"active"`. The platform's lifecycle gate (#681) enforces this
  but the skill must declare it explicitly.
- **`scope_to_vertical: true` by default.** Cross-vertical skills
  require a separate operator decision; default to the safe
  bound.
- **Refuse from weak evidence.** If the supplied pattern has <3
  data points OR conflicting counterexamples, return
  `{"refused": true, "reason": "<why>"}` instead of a proposal.
- **Include counterexamples.** Every proposal lists at least one
  "where it did NOT work" or explains "no counterexamples seen
  in N campaigns" — never ship a proposal that pretends the
  pattern is universal.
- **Frame review questions narrowly.** The operator's job is to
  answer 1–3 specific questions ("does this generalise to
  enterprise segments?", "should we cap reuse at one send per
  account per quarter?"), not to re-evaluate the whole pattern.
- **Never call any sending tool.** The output of this skill is a
  proposal artefact; the agent does not enrol the skill itself.

# Memory

Append the full proposal to `memory.learned_skill_candidates[]`.
The operator's CLI surface (`argo skill review`) consumes from
that table to surface proposals for approval. Each proposal is
immutable once written — superseded proposals get a new row
rather than overwriting the original.

# Example refusal

```json
{
  "refused": true,
  "reason": "supplied pattern has only 1 data point (single positive reply to an emoji-heavy message). One observation is not a pattern — wait for ≥3 reinforcing data points across distinct campaigns before re-running this skill on it."
}
```
