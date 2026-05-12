---
name: schedule_review
version: "0.1.0"
description: "Proposes cross-functional review slots (Eng / Design / PM / CS) for a PRD in flight, factoring panel availability, time-zone spread, and prep-time buffer. Output is a ranked list of candidate slots; booking is the operator's call. Autonomous per charter — slot proposals are reversible and the event body never carries strategic detail."
provenance:
  source: vertical/product-manager
  hash: sha256-pending
---

# When to use

Call when a PRD draft is ready for cross-functional review. Surfaces
2-3 candidate slots; booking runs through the `schedule_review`
action on the gcal connector.

# Inputs you expect

- `prd_id` and short title
- panel: reviewer ids with role (`eng`/`design`/`pm`/`cs`) and TZ
- `duration_minutes` (default 30) and `prep_buffer_hours` (default 24)
- optional date window (default: next 5 business days)

# Output shape

```json
{
  "prd_id": "<slug>",
  "candidate_slots": [
    {"slot_iso": "<ISO 8601 UTC>", "duration_minutes": 30,
     "panel_attendance": [
       {"reviewer_id": "<id>", "available": true, "local_time": "<HH:MM TZ>"}
     ],
     "prep_buffer_hours": 24,
     "rank_reason": "<short phrase>"}
  ],
  "no_full_panel_slot": false,
  "notes": ["<operator-facing note>"]
}
```

# Examples

Top-ranked slot for a PST/EST/CET panel:

```json
{"slot_iso": "2026-05-15T15:00:00Z", "duration_minutes": 30,
 "panel_attendance": [
   {"reviewer_id": "eng-1", "available": true, "local_time": "08:00 PST"},
   {"reviewer_id": "design-1", "available": true, "local_time": "17:00 CET"}
 ],
 "prep_buffer_hours": 28, "rank_reason": "full panel + civilised hours"}
```

# Approval posture

Autonomous per charter. The skill proposes slots; the gcal MCP
executor performs the booking (also auto-approved — slot bookings are
reversible). Event titles and bodies never carry strategic content
(roadmap commitments, dollar figures, competitor names) — those
belong in the linked PRD doc.

# Rules

- **Never invent availability.** Without a free/busy hint, mark
  `available: null` and surface the gap in `notes`.
- **Honour prep buffer.** No slot inside `prep_buffer_hours` of now.
- **Surface no-full-panel explicitly.** Set `no_full_panel_slot:
  true` and propose best partial-panel slots; operator decides.
- **Customer-safe titles.** `Review: <prd_id> — <short_title>`. No
  internal codenames in invites that include external attendees.
