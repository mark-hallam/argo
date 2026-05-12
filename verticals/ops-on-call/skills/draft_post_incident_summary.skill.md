---
name: draft_post_incident_summary
description: Draft a 5-section post-incident summary from the audit-chain timeline + operator notes — impact, timeline, root cause, what worked, action items. Drafted only; approval-gated before any external-facing publication.
inputs:
  - incident_id
  - audit_chain_entries
  - operator_notes
  - confirmed_root_cause
outputs:
  - summary_md
  - action_items
  - requires_approval
---

# Skill: Draft a post-incident summary

Drafted internally, never published autonomously. The operator
approves the wording before it leaves the war room — including for
internal post-mortem channels if the operator scopes them that way.

1. **Five sections, in order.** `## Impact` → `## Timeline` →
   `## Root cause` → `## What worked` → `## Action items`. Same
   skeleton every time; downstream readers learn the shape.
2. **Impact in three numbers.** `users affected`, `duration`,
   `data integrity`. Each cites its source (`memory.incident.metrics`,
   audit-chain entry id, or operator note). If a number isn't
   confirmed, write `unconfirmed (source: <gap>)` rather than guessing.
3. **Timeline is audit-chain entries only.** One bullet per state
   transition: `- <hh:mm UTC> — <event> (audit: <entry_id>)`. No
   prose, no editorialising. Operator notes go in `## What worked` or
   `## Action items`, not the timeline.
4. **Root cause requires evidence.** If the operator has not
   confirmed a cause (`confirmed_root_cause` empty or null), the
   section reads: `Under investigation. Owner: @<handle>. Expected by
   <date>.` Never write a fabricated cause. Never list contributing
   factors as the cause.
5. **What worked is a 3-bullet max.** Bias short. Long "what worked"
   sections drift into self-congratulation; the action-items section
   is the load-bearing one.
6. **Action items each name owner + due date.**
   `- <action> — owner: @<handle> — due: <YYYY-MM-DD>`. Items without
   both fields are not action items — push them back to operator
   notes. No `TBD` owners.
7. **Always set `requires_approval: true`.** Even for the internal
   post-mortem channel. The operator is the gate; this skill is
   draft-only.

Output shape:

```markdown
# Post-incident — INC-2026-0142

## Impact
- Users affected: ~12,400 authenticated dashboard users (source: audit entry e-9f3a)
- Duration: 47 min (08:42 – 09:29 UTC)
- Data integrity: no data loss confirmed (source: db-team note 2026-05-12)

## Timeline
- 08:42 UTC — page fired: db.pool.saturation_pct > 90 (audit: e-9f12)
- 08:44 UTC — argo acknowledged, triage_page → runbook_applicable (audit: e-9f15)
- 08:51 UTC — escalate_to_human fired, @sre-primary acked (audit: e-9f23)
- 09:14 UTC — rollback approved + executed (audit: e-9f3a)
- 09:29 UTC — error rate returned to baseline (audit: e-9f44)

## Root cause
Under investigation. Owner: @platform-oncall. Expected by 2026-05-14.

## What worked
- 15-min checkpoint rhythm kept the war room aligned
- Pre-staged rollback shortened recovery once approval landed
- Customer-facing copy was drafted and approved in parallel with triage

## Action items
- Add pool-saturation early-warning alert at 75% — owner: @sre-primary — due: 2026-05-19
- Document rollback-approval handoff path — owner: @platform-eng-lead — due: 2026-05-26
```

Return the rendered Markdown as `summary_md`, the parsed
`action_items[]` as a structured list (each with `action`, `owner`,
`due`), and `requires_approval: true`. The runtime refuses to
publish without an explicit operator approval event.
