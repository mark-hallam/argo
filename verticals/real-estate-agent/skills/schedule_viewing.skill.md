---
name: schedule_viewing
version: "0.1.0"
description: "Given a qualified lead and listing, propose 3 viewing-time options that respect agent availability, property access, commute, and the lead's weekday/weekend preference. Output JSON; the gcal-booking action that follows is autonomous, but only after the lead picks one option."
provenance:
  source: vertical/real-estate-agent
  hash: sha256-pending
---

# When to use

Call after `lead_qualification` has bucketed a lead `qualified`
and the lead has named interest in a specific `listing_id`.
Output is the slot menu sent to the lead; the booking is the
`schedule_viewing` action in `vertical.toml`.

# Inputs you expect

- `lead_id` and the lead's stated time preference
  (`weekday_evening` / `weekend_morning` / `flexible`)
- `listing_id` (must exist in `inventory/sample_listings.json`)
- agent availability for the next 7 days (busy/free blocks)
- property access mode: `lockbox` / `on_site_agent` / `by_appointment`
- lead's home or work zip for commute estimation (optional)

# Output shape

A single fenced JSON block:

```json
{
  "lead_id": "<id>",
  "listing_id": "<id>",
  "listing_address": "<address from inventory>",
  "access_mode": "lockbox|on_site_agent|by_appointment",
  "options": [
    {"slot_start_iso": "<ISO-8601>", "slot_end_iso": "<ISO-8601>", "weekday": "Mon|...|Sun", "rationale": "<one sentence>"}
  ],
  "fallback_note": "<one sentence if fewer than 3 viable slots exist>"
}
```

`options[]` carries up to 3 entries.

# Rules

- **Three options when possible.** If the calendar permits only
  1–2, fill the array honestly and explain in `fallback_note`.
- **Honour the access mode.** `by_appointment` listings need
  the listing-side agent; `on_site_agent` listings only work
  during the on-site window; `lockbox` is the most flexible.
- **30-minute default slot** unless the listing notes say otherwise.
- **Never auto-confirm.** The lead picks one option before the
  `schedule_viewing` action books the calendar.
- **Respect commute.** When a zip is supplied, don't propose
  back-to-back slots implying >30 min commute the lead hasn't
  accounted for — flag it in the rationale.

# Approval posture

Autonomous. The skill only generates the slot menu; the
`schedule_viewing` action is auto-approved per `vertical.toml`
because the calendar is the agent's own diary.

# Examples

```json
{
  "lead_id": "L-204",
  "listing_id": "MLS-118822",
  "listing_address": "118 Cedar Lane, Oakland CA",
  "access_mode": "lockbox",
  "options": [
    {"slot_start_iso": "2026-05-14T18:00:00-07:00", "slot_end_iso": "2026-05-14T18:30:00-07:00", "weekday": "Thu", "rationale": "weekday-evening preference; lockbox access permits after-hours"},
    {"slot_start_iso": "2026-05-17T10:00:00-07:00", "slot_end_iso": "2026-05-17T10:30:00-07:00", "weekday": "Sat", "rationale": "weekend backup if Thursday slips"},
    {"slot_start_iso": "2026-05-19T18:30:00-07:00", "slot_end_iso": "2026-05-19T19:00:00-07:00", "weekday": "Mon", "rationale": "second weekday option, lockbox available"}
  ],
  "fallback_note": ""
}
```
