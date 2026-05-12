---
name: flag_decision_blockers
version: "0.1.0"
description: "Surfaces the specific facts, approvals, or inputs a decision is missing — naming who/what each is waiting on."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

A decision is stuck and the operator wants to know why — what's
missing that, if obtained, would unblock the call. Use any time
the question is "why can't we decide this yet?" or "what do we
still need?"

# Inputs

- `decision`: one sentence framing what needs to be decided.
- `known_facts`: what the operator already has — data, votes,
  approvals, drafts.
- `decision_criteria` (optional): the rules the decision must
  satisfy if the operator has stated them. If absent, the skill
  infers candidate criteria and asks the operator to confirm.

# Output

A bulleted list of blockers, each shaped as:
- **What's missing** (the fact, signature, data point, opinion).
- **Why it matters** (which criterion or option it gates).
- **Who has it / How to get it** (named person, system, or
  research step).

Followed by a one-line **Unblocking sequence**: the minimal
order in which to chase them.

# Examples

- Decision `"sign vendor X contract"`, known facts `<legal
  reviewed, pricing approved>` → blockers: security review
  pending (waiting on CISO), data residency answer (vendor
  CSM), final approver TBD; sequence names CISO first as
  longest lead-time.
- Decision with no blockers → "no obvious blockers — likely
  ready to decide; recommend naming the decision owner if not
  already done."

# Approval posture

Diagnostic, not action-taking. Skill does not ping people,
schedule reviews, or escalate; it names the gaps for the
operator to chase.

# Notes

- A "blocker" must be specific. "Need more research" is not a
  blocker; "need market-size estimate from analyst report" is.
- If no criteria were supplied AND none can be inferred,
  refuse to enumerate blockers — ask the operator for criteria
  first. Decision-without-criteria is the upstream bug.
- Order the sequence by longest lead-time first, not by
  importance. The slowest unblocker is the schedule.
