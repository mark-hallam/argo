---
name: draft_incident_update
description: Produce a customer-facing, status-page-shaped update for the affected service. Plain-language, no internal jargon, never published autonomously — operator approves before send.
inputs:
  - service
  - severity
  - current_state
  - eta_or_unknown
outputs:
  - status_page_line
  - internal_summary
  - tags
---

# Skill: Draft a status-page incident update

Customer-facing copy only. Drafted, never published. The operator
approves the wording before it reaches a public status page or a
customer-facing channel.

1. **Lead with what the customer experiences,** not the internal
   cause. "The dashboard is slow to load", not "p99 latency on
   `api-prod` exceeds 2s".
2. **State current state in three words or fewer.** `Investigating` /
   `Identified` / `Monitoring` / `Resolved`. Pick from this fixed
   vocabulary so the status page renders cleanly.
3. **Be honest about ETA.** If you don't know, say "We don't have an
   ETA yet". Never invent a window. Never promise "shortly".
4. **No internal jargon.** Forbidden words in customer copy: "p99",
   "saturation", "pool", "replica", "GC", "OOM", "5xx", "QPS",
   "shard". Translate to plain language.
5. **No blame.** Don't name a service, deploy author, or team. Don't
   say "due to a recent deployment" unless the operator approves it.
6. **One sentence, ideally under 140 characters.** The status page
   doesn't render paragraphs well. Tweet-length is the target.
7. **Always include an internal_summary** (separate field). That's
   the version that goes in the Slack incident channel — same facts,
   internal vocabulary OK there.

Output shape:

```json
{
  "status_page_line": "Some users are seeing slower-than-usual dashboard load times. We're investigating.",
  "internal_summary": "Service A p99 latency 2.4s sustained 4min, error rate 0.8%. Last deploy 17min ago. Investigating db-pool saturation per rb-db-pool-saturation.",
  "tags": ["service-a", "investigating", "dashboard"]
}
```

If the operator hasn't approved a customer-facing comms yet, return
the draft with `status_page_line` populated AND a top-level
`requires_approval: true` so the worker doesn't accidentally publish.
