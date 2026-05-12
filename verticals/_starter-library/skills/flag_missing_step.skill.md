---
name: flag_missing_step
version: "0.1.0"
description: "Reviews an existing runbook or procedure and flags steps that look missing, out of order, or under-specified — names the omission, doesn't rewrite."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has a runbook (their own or inherited) and wants a
review before it's adopted or re-used. Use any time the question
is "is anything missing from these steps?" or "what would break
this procedure?"

# Inputs

- `runbook`: the existing steps, ordered.
- `procedure`: one-sentence framing (used to spot off-scope or
  off-purpose steps).
- `risk_class`: `"low"`, `"medium"`, `"high"` — drives how
  strict the missing-step bar is (high-risk = no missing
  rollback hint is fine; high-risk = every step needs one).

# Output

A list of findings, each shaped as:
- **Where** (step number or "between step X and Y").
- **What's missing** (named omission: missing success check,
  missing rollback, missing owner, undefined preconditions,
  ambiguous outcome).
- **Why it matters** (the failure mode it enables).

If nothing is missing, return an empty findings list + a
`note: "no obvious gaps for risk_class <X>"`.

# Examples

- 6-step vendor onboarding missing the "revoke access on
  termination" companion → flag at the end with rationale.
- Deploy runbook (risk high) with no rollback hint on a
  state-changing step → flag with `risk_class` mismatch.

# Approval posture

Diagnostic. Skill names gaps; the operator decides whether to
rewrite the runbook (use `propose_runbook_steps` for that) or
accept the risk.

# Notes

- Don't rewrite. Naming the gap is the deliverable. If the
  operator wants the rewrite, they'll call `propose_runbook_steps`.
- A "missing step" must be specific — "needs more detail" isn't
  a finding; "no acceptance test on step 3" is.
- For risk_class `"high"`, missing rollback hints on
  state-changing steps are ALWAYS findings, regardless of how
  the rest of the runbook reads.
- If the runbook is already very tight, say so plainly. False
  positives are worse than honest empty findings — they train
  operators to ignore the skill.
