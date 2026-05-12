---
name: escalate_to_human
version: "0.1.0"
description: "Packages a stuck ticket for human handoff. Emits a structured envelope: who-to-page, summary, attempted-steps, recommended-next-action. Used when the agent cannot resolve self-serve."
provenance:
  source: vertical/customer-support
  hash: sha256-pending
---

# When to use

Call when the agent has hit a wall: `search_kb` returned no
relevant snippets, `draft_reply` would have to fabricate to
respond, or `triage_ticket` already routed to a human tier. The
output is the artifact a human operator picks up — make it
self-contained so they don't have to re-read the thread.

# Inputs you expect

- the triaged ticket (output of `triage_ticket`)
- any `search_kb` results, even if empty (so the human sees what
  was tried)
- any `draft_reply` attempts, even if abandoned
- the on-call schedule keyed by `assign_to` tier (from operator
  config or `memory.oncall[]`)

# Output shape

A single fenced JSON block:

```json
{
  "ticket_id": "<id>",
  "envelope": {
    "who_to_page": "<oncall handle for the assigned tier>",
    "summary": "<≤3 sentences — the situation, plain language>",
    "attempted_steps": [
      {"skill": "<name>", "outcome": "<one phrase>", "citations": ["[kb:<id>]"]}
    ],
    "recommended_next_action": "<one sentence — what should the human do first>"
  },
  "severity": "P0|P1|P2|P3",
  "page_now": true,
  "reason": "<why this couldn't be self-served>"
}
```

# Rules

- **Envelope is non-negotiable.** All four envelope fields
  (who-to-page, summary, attempted-steps, recommended-next-action)
  must be present and non-empty. A missing field is a routing
  defect — the human won't know what to do.
- **Page urgency.** `page_now: true` for P0 and `abuse-review`
  cases. `page_now: false` for P2/P3 — these queue for the next
  business-hours triage.
- **Attribution.** Every entry in `attempted_steps` names the skill
  that ran (`triage_ticket`, `search_kb`, `draft_reply`) — the
  human needs to see what was tried, not redo it.
- **No hallucinated steps.** If `search_kb` didn't run because
  category was `outage`, write `outcome: "skipped (category=outage)"` —
  don't fabricate a search that didn't happen.
- **Cite KB hits with `[kb:<id>]`.** Even abandoned ones — the
  human may pick up where the agent left off.
- **One recommended action.** "Reset their 2FA from admin
  console" is good; "investigate the issue" is not. Specificity
  beats hedge.

# Examples

Stuck on a P0 outage where `search_kb` returned nothing →

```json
{
  "ticket_id": "T-1024",
  "envelope": {
    "who_to_page": "@oncall-tier2",
    "summary": "Enterprise customer reports team-wide login outage starting 3h ago. No matching KB article. Status page shows green; likely customer-specific.",
    "attempted_steps": [
      {"skill": "triage_ticket", "outcome": "categorized P0 outage", "citations": []},
      {"skill": "search_kb", "outcome": "no relevant hits", "citations": []}
    ],
    "recommended_next_action": "Page on-call SRE; check tenant-level auth logs for the customer's workspace id."
  },
  "severity": "P0",
  "page_now": true,
  "reason": "No KB article covers tenant-specific auth outage; needs SRE."
}
```

# Memory schema notes

Append the envelope to `memory.escalations[]` so post-incident
review can audit which categories most often escalate. Schema
addition pending.
