---
name: summarize_thread
version: "0.1.0"
description: "Collapses a long support thread into a 5-bullet cited summary for handoff or post-incident review. Truncates rather than embellishes — every bullet traces to a quote or skill output."
provenance:
  source: vertical/customer-support
  hash: sha256-pending
---

# When to use

Call when a thread needs to be handed off (between shifts, to a
human tier, or into retro tooling) and reading the full thread
would cost more operator time than re-orienting from a summary.
Also called by retro tooling to seed post-incident reviews. Do
NOT call mid-flight on an active reply chain — wait until the
ticket reaches `resolved` or `closed`.

# Inputs you expect

- the full thread (messages with sender, timestamp, body)
- the latest `memory.ticket_history[]` for the same ticket (state
  transitions to align bullets against)
- any `[kb:<id>]` citations emitted by `draft_reply` in the thread

# Output shape

A single fenced JSON block:

```json
{
  "ticket_id": "<id>",
  "bullets": [
    {"text": "<one sentence>", "citation": "<msg-id | [kb:<id>] | skill:<name>>"}
  ],
  "truncated": false,
  "facts_omitted": 0,
  "final_state": "pending|in-progress|waiting-on-customer|resolved|closed"
}
```

# Rules

- **5-bullet ceiling, hard.** If the thread has more than 5
  distinct facts, emit the top 5 by importance and set
  `truncated: true` with `facts_omitted: <count>`. Longer
  summaries lose operator attention — this is intentional.
- **Cite every bullet.** Each bullet's `citation` field points to
  one of: a message id from the thread, a `[kb:<id>]` from a
  `draft_reply`, or `skill:<name>` for state changes
  (`skill:triage_ticket`, `skill:update_ticket_status`). A bullet
  with no citation is a fabrication risk — drop it.
- **Importance order.** Rank by: (1) state-changing events
  (escalation, resolution), (2) operator decisions, (3) customer-
  stated impact, (4) attempted fixes, (5) tone/context. Cut from
  the bottom.
- **No editorializing.** "Customer was frustrated" needs a quote.
  "Customer wrote 'this is unacceptable' in msg-7" is fine.
- **Final state required.** Always include the ticket's terminal
  state from the latest `memory.ticket_history[]` entry — a
  handoff summary without "where it ended up" is incomplete.

# Examples

A 14-message thread that resolved via KB-2034 →

```json
{
  "ticket_id": "T-1024",
  "bullets": [
    {"text": "Customer reported team-wide login outage on enterprise plan.", "citation": "msg-1"},
    {"text": "Triaged as P0 outage, routed to tier-2.", "citation": "skill:triage_ticket"},
    {"text": "search_kb surfaced KB-2034 (2FA admin reset).", "citation": "[kb:KB-2034]"},
    {"text": "Workspace admin reset 2FA; team regained access within 20m.", "citation": "msg-9"},
    {"text": "Ticket resolved; 24h grace window pending close.", "citation": "skill:update_ticket_status"}
  ],
  "truncated": false,
  "facts_omitted": 0,
  "final_state": "resolved"
}
```

# Memory schema notes

Read-only on `memory.ticket_history[]`. This skill does not
persist its own summary — retro tooling consumes the output
directly and decides whether to archive.
