---
name: coordinate_war_room
description: Produce a 5-section war-room coordination plan for an active incident — current state, investigation tracks (named owners), blocked items, external-comms status, next 15-min checkpoint. Internal Markdown, autonomous (no customer-facing copy).
inputs:
  - incident_id
  - severity
  - response_roster
  - active_findings
  - external_comms_state
outputs:
  - coordination_plan_md
  - next_checkpoint_at
  - blocked_unblockers
---

# Skill: Coordinate the incident war room

Internal coordination doc, regenerated on a 15-minute cadence. Never
customer-facing. The output replaces the prior tick — short and
structural, not a running narrative.

1. **Five sections, in order, every tick.** `## Current state` →
   `## Investigation tracks` → `## Blocked` → `## External comms` →
   `## Next checkpoint`. Operators skim by header; the order is the
   contract.
2. **Current state in two sentences.** Symptom + scope (which
   service, which surface, what fraction of users). No history; no
   speculation about cause. The timeline lives in audit memory.
3. **Investigation tracks each name an owner.** One bullet per track:
   `- <track> — owner: @<handle> — status: <one-line>`. If a track
   has no named owner, write `owner: UNASSIGNED` — that's a flag for
   the operator, not a placeholder to be filled silently.
4. **Blocked items name the unblocker.** Format
   `- <item> — blocked on: @<handle or system>`. Never list a blocker
   without naming who can clear it.
5. **External comms is a one-line state.** Pick from `none yet` /
   `draft pending approval` / `status page updated <hh:mm>` /
   `customer email sent <hh:mm>`. Defer wording to
   `draft_incident_update`; this skill only tracks state.
6. **Next checkpoint is always 15 min out.** `Next checkpoint:
   <hh:mm UTC>`. Long-cycle updates are how incidents extend — the
   15-min rhythm is non-negotiable. Sooner is fine; later requires
   an explicit operator override noted inline.
7. **Never name a root cause.** If investigation has not confirmed
   one, the doc says so. Speculation belongs in track status, never
   in the header.

Output shape:

```markdown
# War room — INC-2026-0142 (SEV-2)

## Current state
Dashboard load p95 4.2s for 18% of authenticated users since 08:42 UTC. Read-only impact; writes unaffected.

## Investigation tracks
- db-pool saturation — owner: @sre-primary — status: pool at 92%, increasing 5%/min
- recent deploy correlation — owner: @platform-oncall — status: deploy abc123 17min ago, rollback prepared
- upstream dependency check — owner: UNASSIGNED — status: pending

## Blocked
- rollback execution — blocked on: @platform-eng-lead (approval)

## External comms
draft pending approval

## Next checkpoint
09:15 UTC
```

Emit the doc as `coordination_plan_md` plus a structured
`next_checkpoint_at` (RFC3339) and a `blocked_unblockers[]` list of
the @-handles named under `## Blocked` — the runtime uses those to
fire reminders if the checkpoint passes without resolution.
