---
name: escalate_to_human
description: Fire a Slack ping to the human on-call with a 5-line summary plus a suggested next action. Used whenever any wake-the-human criterion fires or when triage confidence is low.
inputs:
  - incident_id
  - service
  - severity
  - reason_for_escalation
  - evidence_bullets
  - suggested_next_action
outputs:
  - slack_message_draft
  - paged_user_handle
  - requires_approval
---

# Skill: Escalate to the human on-call

This is the safe-fallback path. When in doubt, escalate — that is
the explicit guidance from `prompts/system.md`. Don't dwell.

1. **Identify the on-call.** Read `escalation_contacts` from memory
   for the service's primary on-call handle. If the paging policy
   names a specific secondary (e.g., "wake DBA for storage pages"),
   page them instead of the generic primary.
2. **Compose the 5-line summary.** Exactly five lines, in this order:
   - Line 1: `🚨 <service> — <one-sentence symptom>`
   - Line 2: `Severity: <SEV-1..SEV-4>. Started <ts>. Acked by argo.`
   - Line 3: `Evidence: <top-3 bullets, comma-separated>`
   - Line 4: `Suggested next: <one concrete action, name the
     approval-gated mutation if relevant>`
   - Line 5: `Reason for waking you: <which criterion fired>`
3. **Always require approval before the ping fires.** The Slack send
   is itself a human-approval-gated action — the operator confirms
   the wording, the recipient, and the urgency. Set
   `requires_approval: true` on return.
4. **Never auto-resolve in parallel.** While waiting for the human,
   the worker holds the incident in `escalated` state and continues
   gathering passive evidence. Do not retry the failed triage step.
5. **Log the escalation reason** to memory under
   `recent_incidents[].escalation_reason` so the post-incident review
   has the chain of reasoning.

Output shape:

```json
{
  "slack_message_draft": "🚨 service-a — error rate 4.2% for past 3 min, dashboard slow\nSeverity: SEV-2. Started 2026-05-11T08:42:11Z. Acked by argo.\nEvidence: deploy 17min ago, db.pool 92% saturated, no upstream incidents\nSuggested next: rollback_deploy (human approval required)\nReason for waking you: customer-facing impact + mutation required",
  "paged_user_handle": "@on-call-sre",
  "requires_approval": true
}
```

If `escalation_contacts` is empty AND no paging policy applies,
return `paged_user_handle: null` and escalate to `escalate_to =
"operator"` (per `vertical.toml`). The worker never fabricates a
recipient.
