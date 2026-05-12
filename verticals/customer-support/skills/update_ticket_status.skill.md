---
name: update_ticket_status
version: "0.1.0"
description: "Tracks ticket state transitions and persists them to memory.ticket_history[]. The authoritative writer for ticket lifecycle state — other skills read but do not write the history."
provenance:
  source: vertical/customer-support
  hash: sha256-pending
---

# When to use

Call after any skill that changes the ticket's effective state:
right after `triage_ticket` (initial intake → `pending` →
`in-progress`), after `draft_reply` is sent (`waiting-on-customer`),
after `escalate_to_human` (`waiting-on-human`, mapped to
`in-progress` here), and on operator confirmation that the
customer's issue is closed. Other skills should NOT write
`memory.ticket_history[]` directly.

# Inputs you expect

- the ticket_id
- the prior state (from the latest `memory.ticket_history[]` entry,
  or `null` for a new ticket)
- the new state (one of the 5 transition vocabulary entries below)
- the reasoning for the transition (≤1 sentence, will be persisted)
- the skill that triggered the transition (provenance for audit)

# Output shape

A single fenced JSON block:

```json
{
  "ticket_id": "<id>",
  "prior_state": "pending|in-progress|waiting-on-customer|resolved|closed|null",
  "new_state": "pending|in-progress|waiting-on-customer|resolved|closed",
  "transitioned_at": "<ISO-8601 UTC>",
  "triggered_by": "<skill name>",
  "reasoning": "<one sentence>",
  "valid_transition": true
}
```

# Transition vocabulary

The 5 lifecycle states form a directed graph:

- `pending` → `in-progress` (agent picked it up)
- `in-progress` → `waiting-on-customer` (reply sent, ball in
  customer's court)
- `waiting-on-customer` → `in-progress` (customer replied, agent
  resumes)
- `in-progress` → `resolved` (agent + operator agree the issue is
  fixed)
- `resolved` → `closed` (24h grace window with no customer reopen)
- Any state → `closed` only via operator override (set `triggered_by:
  "operator-override"` and record the reason)

# Rules

- **Reject invalid transitions.** `pending` → `resolved` skips
  steps. Emit `valid_transition: false` and the rejection reason;
  do NOT persist to memory.
- **Cite the trigger.** Every transition names the skill or
  operator action that drove it. "It just happened" is not a valid
  trigger — that's a memory-corruption signal.
- **Idempotent on no-op.** If `prior_state == new_state`, return
  `valid_transition: false` with reason `"no-op transition"` and
  don't append to history.
- **Timestamp authoritatively.** Use the runtime's clock, not a
  customer-supplied timestamp; the audit trail breaks if these
  drift.
- **History is append-only.** Never edit a prior entry — if a
  mistake needs correcting, emit a new transition with reasoning
  `"correction: <details>"`.

# Examples

After `draft_reply` sent a response to T-1024 →

```json
{
  "ticket_id": "T-1024",
  "prior_state": "in-progress",
  "new_state": "waiting-on-customer",
  "transitioned_at": "2026-05-12T09:14:00Z",
  "triggered_by": "draft_reply",
  "reasoning": "Sent KB-2034 reset-2FA reply; awaiting customer confirmation.",
  "valid_transition": true
}
```

# Memory schema notes

Authoritative writer for `memory.ticket_history[]`. Each entry is
one transition (the full JSON output above). The schema field is a
future addition to `memory/schema.toml` — document the intent here.
