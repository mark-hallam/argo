# Incident postmortem — {{summary}}

> **Blameless.** This document is about the system, not the people who
> operated it. The goal is durable corrective action, not attribution.
> Reviewers should focus on which controls, signals, or defaults would
> have prevented or shortened the incident.

## Summary

{{summary}}

## Severity

{{severity}}

Severity scale (Skycot convention):

- **SEV1** — production down, customer-visible outage, or data loss.
- **SEV2** — degraded service, partial outage, or significant
  customer-facing impact without total loss.
- **SEV3** — minor incident, internal-only impact, or near-miss caught
  by a guardrail.
- **SEV4** — no operational impact; documented for the record (e.g.
  noisy alert, cosmetic regression).

## Timeline

All times in UTC. One bullet per discrete event. Include detection,
escalation, mitigation, and resolution checkpoints.

{{timeline}}

## Root cause

The single proximate cause that, if removed, would have prevented the
incident. Be specific — name the file, config row, or invariant that
failed. Resist the urge to list multiple causes here; those go under
"Contributing factors."

{{root_cause}}

## Contributing factors

Conditions that allowed the root cause to escalate or persist.
Examples: missing alerting, stale runbook, manual deploy step skipped,
a guardrail that was disabled for an unrelated reason.

{{contributing_factors}}

## Detection

How was the incident first noticed? By whom? How long after onset?
Note whether automated alerting fired, and if not, why not.

{{detection}}

## Resolution

What stopped the bleeding? Was the fix a rollback, a config flip, a
hotfix, or a manual intervention? How was the system verified healthy
again?

{{resolution}}

## Action items

Concrete, owner-tagged, time-bound. Each item should reduce the
likelihood or blast radius of a recurrence. Avoid "investigate X" —
prefer "decide whether to add X by <date>."

{{action_items}}

## Lessons learned

What did this incident teach us about the system, the team, or the
process? What surprised the responders? What would we do differently
next time?

{{lessons_learned}}
