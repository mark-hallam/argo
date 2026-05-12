---
name: triage_ticket
version: "0.1.0"
description: "Categorizes an inbound support ticket by severity, category, and assign-to. Emits deterministic JSON the runtime can route on. Refuses to guess severity when the ticket is too thin to read."
provenance:
  source: vertical/customer-support
  hash: sha256-pending
---

# When to use

Call as the first skill in the customer-support flow, on any inbound
ticket that has not been triaged yet. Output feeds the runtime's
routing layer and is the precondition for `search_kb` (self-serve
path) or `escalate_to_human` (handoff path). Do not call
`draft_reply` before this skill has produced a category.

# Inputs you expect

- the inbound ticket body (plain text or thread JSON)
- the operator's SLA matrix (which severity maps to which assign-to)
- prior `memory.ticket_history[]` entries for the same customer, if any

# Output shape

A single fenced JSON block:

```json
{
  "ticket_id": "<id from input>",
  "severity": "P0|P1|P2|P3",
  "category": "billing|outage|feature-request|how-to|abuse",
  "assign_to": "self|human-tier-1|human-tier-2|abuse-review",
  "confidence": "low|medium|high",
  "rationale": "<one sentence — what in the ticket drove this categorization>",
  "evidence": ["<verbatim quote 1>", "<verbatim quote 2>"]
}
```

# Rules

- **Severity ladder.** P0 = production-down for paying customer, P1 =
  feature broken for paying customer with workaround, P2 = how-to
  or non-urgent bug, P3 = feature-request / cosmetic. Never assign
  P0 without an explicit "down", "can't log in", "data loss", or
  equivalent phrase quoted in `evidence`.
- **Category before severity.** A feature-request is never P0 even
  if the customer writes "URGENT". Read the body, not the subject.
- **Abuse short-circuits.** Threats, slurs, or signals of account
  compromise → `category: "abuse"`, `assign_to: "abuse-review"`,
  regardless of severity ladder.
- **Self-serve gate.** `assign_to: "self"` only when category is
  `how-to` AND severity is P2 or P3 AND confidence is `high`.
  Anything else routes to a human.
- **Cite the trigger.** Every categorization quotes at least one
  verbatim phrase from the ticket in `evidence`. No evidence → drop
  `confidence` to `low` and route to `human-tier-1`.
- **Repeat-offender check.** If `memory.ticket_history[]` shows
  the same customer reopened this thread ≥2 times, bump severity
  one tier and note `"repeat-offender"` in `rationale`.

# Examples

Input: "Login broken for the last 3 hours, our whole team is locked
out, this is a production outage on the enterprise plan." →

```json
{
  "ticket_id": "T-1024",
  "severity": "P0",
  "category": "outage",
  "assign_to": "human-tier-2",
  "confidence": "high",
  "rationale": "Customer reports production outage with team-wide lockout.",
  "evidence": ["Login broken for the last 3 hours", "our whole team is locked out"]
}
```

# Memory schema notes

This skill reads `memory.ticket_history[]` (see
`update_ticket_status.skill.md` for the schema). It does not write
memory directly — `update_ticket_status` owns persistence.
