---
name: find_calendar_slot
description: Propose 3 candidate meeting slots that satisfy attendee free/busy constraints and the principal's standing preferences.
inputs:
  - attendees
  - duration_minutes
  - window_start
  - window_end
  - meeting_type
outputs:
  - slots
---

# Skill: Find Calendar Slot

Propose exactly 3 slot options. Rank them; lead with the recommended
slot and a one-line "why this one".

1. **Honor standing preferences** from `principal_prefs` memory:
   - No-meeting hours (e.g. `mornings_before_10` reserved for focus).
   - Day-of-week patterns (e.g. "Fridays: no external meetings").
   - Default meeting length (30 min unless override given).
   - Preferred timezone-of-the-day (e.g. "US/Pacific until 1pm,
     US/Eastern after").
2. **Check attendee free/busy** for every required attendee. Reject
   slots that conflict with any required attendee's existing event.
   Optional attendees do not veto a slot.
3. **Buffer 15 minutes** before and after each proposed slot unless
   the principal's `back_to_back_ok` preference is true for this
   meeting type.
4. **External meetings** (any attendee outside the principal's
   org domain) require human approval before the slot is confirmed.
   The skill proposes; the principal accepts.
5. **Order the 3 slots**: earliest first that meets all constraints,
   then "best fit" (e.g. lands in the principal's typical
   external-meeting block), then a fallback further out.

If no slot works in the requested window, return 0 slots and surface
the specific constraint (which attendee has no free time, or which
principal preference rules out the window entirely).

Output shape:

```json
{
  "slots": [
    {
      "rank": 1,
      "start": "2026-05-13T14:00:00-07:00",
      "end":   "2026-05-13T14:30:00-07:00",
      "timezone": "US/Pacific",
      "rationale": "Earliest free across all required attendees; protects Thursday focus block."
    }
  ]
}
```
