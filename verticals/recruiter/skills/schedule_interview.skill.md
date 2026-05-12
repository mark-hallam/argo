---
name: schedule_interview
version: "0.1.0"
description: Propose 3 interview slot options from candidate + panel availability, factoring time-zone, panel-conflict-free, and prep-time buffer. Approval-gated before booking.
provenance:
  source: vertical/recruiter
  hash: sha256-pending
---

# When to use

Use after a candidate confirms interest (post initial-outreach) or
when advancing between stages (phone-screen → technical → panel →
hiring-manager). Produces 3 slot proposals — never books directly.

# Inputs you expect

- `candidate_availability` — `{tz, hints: [...], windows: [...]}`.
  Hints are free-text ("weekday mornings only"); windows are
  explicit ISO-8601 intervals.
- `panel` — list of `{name, role, tz, busy: [intervals]}`. Every
  interviewer's TZ is required; never guess.
- `interview_type` — one of `phone-screen`, `technical`, `panel`,
  `hiring-manager`. Drives duration + prep-buffer per type.
- `requisition_id` — opaque ID for the audit trail.

# Output shape

```json
{
  "interview_type": "technical",
  "duration_minutes": 60,
  "proposed_slots": [
    {
      "start_utc": "2026-05-20T17:00:00Z",
      "end_utc": "2026-05-20T18:00:00Z",
      "candidate_tz": "America/Los_Angeles",
      "candidate_wall_clock": "2026-05-20 10:00 PT",
      "panel_tz": ["America/New_York", "Europe/London"],
      "panel_wall_clock": ["2026-05-20 13:00 ET", "2026-05-20 18:00 BST"],
      "prep_buffer_minutes": 15
    }
  ],
  "next_action": {
    "name": "book_interview_slot",
    "requires_approval": true
  },
  "flags": []
}
```

# Duration + prep-buffer per type

- `phone-screen` — 30 min, 0 min prep buffer.
- `technical` — 60 min, 15 min prep buffer either side.
- `panel` — 90 min, 30 min prep buffer either side.
- `hiring-manager` — 45 min, 15 min prep buffer either side.

# Hard rules

- **TZ on every slot.** Every proposed slot carries `candidate_tz`,
  `candidate_wall_clock`, every panelist's TZ, and every panelist's
  wall-clock. Downstream calendar tooling cannot recover TZ if
  dropped here.
- **Three distinct days.** Don't propose three slots on one day —
  one constraint can wipe a whole day. Three days = three chances.
- **No same-day slots.** Earliest is next business day from `now`.
- **No weekend slots** unless the candidate hints "weekend ok".
- **Panel conflict-free.** No proposed slot may overlap any
  `panel[*].busy` interval. Respect prep-buffer on both sides.
- **Honour candidate hints.** "Weekday mornings only", "after 4pm
  PT" — filter strictly. No slot may contradict a stated hint.
- **Approval-gated.** `next_action.requires_approval: true`. The
  actual calendar write happens via a Tier A connector once the
  operator approves; this skill proposes only.
- **Missing TZ → stop.** If any panelist's `tz` is missing, set
  `flags: ["missing_panel_tz"]` and escalate. Never guess.
- **No-overlap → stop.** If candidate windows and panel windows
  have zero overlap in the next 10 business days, set `flags:
  ["availability_conflict"]` and escalate.

# Examples

Synthetic — Alex Doe (`America/Los_Angeles`), 2-person panel in NY
and London for a `technical` interview. Output proposes 3 distinct
days in the next 10 business days with full TZ metadata on each
slot. Operator sees the JSON and approves one.
