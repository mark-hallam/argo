---
name: triage_page
description: Given a PagerDuty incident payload (or hand-pasted page text), classify it as transient / runbook_applicable / human_required and produce an initial response with evidence.
inputs:
  - incident_id
  - service
  - alert_name
  - severity
  - summary
  - first_triggered_at
outputs:
  - classification
  - evidence
  - proposed_next_action
  - escalation_required
---

# Skill: Triage a PagerDuty page

Use this skill on every incoming page. Output the classification + a
4-bullet evidence block within 5 minutes of the page arriving.

1. **Acknowledge first.** Mark the incident acknowledged in PagerDuty.
   This is non-negotiable — the page auto-escalates otherwise.
2. **Read the alert payload.** Pull `service`, `alert_name`, `severity`,
   `first_triggered_at`. Note the alerting region if present.
3. **Match against transient signatures.** Single-replica blips, DNS
   flaps, known-noisy alerts (`*-canary-*`, ad-hoc test alerts), and
   alerts that auto-resolved within 60 seconds = `transient`. Document
   the signature; do not silently drop.
4. **Match against runbook signatures.** Hand the alert to
   `find_runbook`. A clean match (one runbook id, symptom line
   confidence ≥ 0.7) = `runbook_applicable`; cite the runbook.
5. **Apply the wake-the-human criteria.** See `prompts/system.md` for
   the full list. ANY hit → `human_required`. Hand to
   `escalate_to_human` with the criterion that fired.
6. **Emit evidence.** 3–5 bullets max: recent deploy, current error
   rate / p99, prior alerts in the last 24h, dependency health. Cite
   every metric.

Output shape:

```json
{
  "classification": "runbook_applicable",
  "evidence": [
    "Service A error rate 4.2% (Grafana: api-latency)",
    "Last deploy 17min ago by mhallam (deploy SHA abc123)",
    "Prior alert on Service A db-conn-pool 14h ago, resolved"
  ],
  "proposed_next_action": "Apply runbook rb-db-pool-saturation step 1: increase pool size 50→100",
  "escalation_required": false
}
```

Never propose a mutation step inline — if the runbook calls for a
restart or rollback, that's `restart_service` / `rollback_deploy` and
the proposal flows through human approval.
