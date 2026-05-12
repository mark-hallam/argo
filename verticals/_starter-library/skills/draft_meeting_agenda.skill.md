---
name: draft_meeting_agenda
version: "0.1.0"
description: "Drafts a time-boxed meeting agenda from a topic, attendee list, and duration — outcome-shaped, not topic-shaped."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator is scheduling a meeting and wants an agenda before
sending the invite. Use any time the request is "we're meeting
about <topic>, draft an agenda for <duration>."

# Inputs

- `topic`: the meeting's purpose in one sentence.
- `duration_minutes`: total time available.
- `attendees`: list of names + roles (used to assign items, not
  just listed).
- `desired_outcomes`: 1-3 concrete results the meeting must
  produce. Optional; if omitted, ask before drafting.

# Output

A bulleted agenda. Each item has: time-box, lead, outcome (not
"discuss X" — "decide X" / "align on X" / "share X"). Sum of
time-boxes ≤ `duration_minutes`. Final item: 2-5 min wrap with
explicit action-item capture.

# Examples

- Topic `"q3 launch readiness"`, 30 min, attendees `<list>`,
  outcomes `["risk register signed off", "go/no-go owner named"]`
  → 5/15/8/2 min split with named leads and outcome verbs.
- Topic `"vendor selection"`, 45 min → criteria review → vendor
  walkthrough → decision → wrap.

# Approval posture

Read-only draft. Operator reviews and circulates. Skill does not
send calendar invites or notifications.

# Notes

- Reserve the last 10% of the meeting for action items. Always.
- An agenda item with no named lead is a hand-wave; either name
  someone or cut it.
- If `desired_outcomes` is missing, refuse to draft — a meeting
  without an outcome is the failure mode this skill exists to
  prevent. Ask the operator for the outcome first.
- Cap the agenda at 5 items for a 30 min meeting, 7 for 60 min.
  More items = nothing decided.
