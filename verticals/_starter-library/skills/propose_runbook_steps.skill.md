---
name: propose_runbook_steps
version: "0.1.0"
description: "Drafts an ordered, executable runbook for a procedure — each step has an owner, an explicit success check, and a rollback hint."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator wants to turn a one-time procedure into a repeatable
runbook — onboarding, a deploy, a customer escalation, an
incident response, a quarterly close. Use any time the request
is "draft the steps for <procedure>."

# Inputs

- `procedure`: what the runbook is for, in one sentence.
- `success_definition`: what "done" looks like; the final step
  verifies this.
- `known_steps` (optional): any steps the operator has already
  in mind. The skill orders and gap-fills around them.
- `risk_class`: `"low"`, `"medium"`, `"high"` — drives how
  strict the rollback hints and approval gates are.

# Output

A numbered list of steps. Each step has:
- **What** — the action verb + concrete object.
- **Owner role** — who runs this step (role, not named person).
- **Success check** — how to verify the step worked.
- **Rollback hint** — what to do if it didn't (only present for
  steps that have non-trivial side effects).

A final **Done when** matches `success_definition` verbatim.

# Examples

- "Onboard a new vendor" (risk medium) → ~6 steps from contract
  signed → access provisioned → success metric set; rollback
  hints on access provisioning.
- "Deploy hotfix to production" (risk high) → ~8 steps with
  approval gates on every state-changing action; explicit
  rollback per step.

# Approval posture

Drafts only. Skill does not execute or assume any step is safe.
For `risk_class: "high"`, every step's rollback hint must be
non-empty.

# Notes

- Refuse to draft if `success_definition` is missing. A runbook
  without a "done when" produces unfinished procedures.
- Each "Owner role" must be a role that could plausibly exist
  at any tenant — "engineer on-call," not "Sarah from infra."
- Reversible steps have empty rollback hints; non-reversible
  steps must have a hint or an explicit "NOT reversible — verify
  before proceeding" warning.
- Cap at 12 steps. Longer runbooks should be split into nested
  procedures.
