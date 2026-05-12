---
name: find_calendar_overlap
version: "0.1.0"
description: "Finds shared free time across multiple calendars/availability blocks, returning the top N candidate windows ranked by fit."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has busy/free data for two or more people and needs
to know when they can all meet. Use any time the request reduces
to "when are <attendees> all free?"

# Inputs

- `availability`: a map of `attendee → list of free intervals`,
  each interval expressed as `[start, end]` in ISO-8601 with
  timezone.
- `duration_minutes`: how long the meeting needs to be.
- `window`: `[earliest, latest]` ISO-8601 bounds for acceptable
  start times.
- `top_n` (optional, default 3): how many candidate slots to
  return.

# Output

A list of up to `top_n` intervals, each `[start, end]` in
ISO-8601 with the same timezone the operator supplied. Sorted
best-fit first (see ranking below). Empty list if no overlap
exists in `window` — return the empty list, do not invent a
"closest" slot.

# Examples

- 3 attendees, 30-min meeting, window `[Mon 09:00, Fri 17:00]`
  → up to 3 slots ranked by ranking rules.
- 5 attendees with no shared overlap → empty list +
  `note: "no full overlap in window; consider partial-attendance"`.

# Approval posture

Read-only computation. Safe to call without confirmation. Skill
does not send invites or modify calendars.

# Notes

- Ranking: earlier in the work week > later; mid-day > edges of
  the day; back-to-back-meeting buffers (15 min either side) >
  no buffer.
- Treat all-day events as full-day blockers.
- If timezones differ across attendees, return slots in the
  timezone of the FIRST attendee in the input map and flag
  others with `note`.
- Never fabricate availability for a missing attendee — if their
  data is absent, return the empty list with that as the reason.
