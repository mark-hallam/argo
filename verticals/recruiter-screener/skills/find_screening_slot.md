---
name: find_screening_slot
description: Given a candidate's availability hints + the recruiter's open windows for the next 10 working days, propose 3 concrete 30-minute screening slots in the candidate's timezone with the recruiter's name attached.
inputs:
  - candidate_availability_hints
  - recruiter_calendar
  - candidate_timezone
outputs:
  - proposed_slots
  - next_action
  - flags
---

# Skill: Find Screening Slot

Propose three concrete 30-minute phone-screen slots.

1. **Read the recruiter's calendar.** Use
   `recruiter_calendar.open_windows` for the next 10 working
   days. Do not propose slots outside the listed windows. If the
   calendar is empty for the window, flag
   `no_recruiter_availability` and escalate.
2. **Honour candidate availability hints.** If the candidate
   wrote "weekday mornings only" or "after 4pm PT", filter to
   slots that match. Never propose a slot that contradicts a
   stated hint. If hints and calendar have no overlap, flag
   `availability_conflict` and escalate.
3. **Convert to candidate's timezone.** Use
   `candidate_timezone` (IANA, e.g. `America/Los_Angeles`). Show
   the slot in the candidate's TZ + the recruiter's TZ on a
   second line, never just one TZ.
4. **Three slots, three distinct days.** Don't propose three
   slots on one day — the candidate may have a constraint that
   eliminates the day. Three days = three chances.
5. **Name the interviewer.** Pull the recruiter's name from
   `recruiter_calendar.interviewer_name`. Never invent.

Hard rules:

- **No same-day slots.** Earliest is next business day, to give
  the candidate prep time.
- **No weekend slots** unless the candidate explicitly asked.
- **No back-to-back proposals.** If the recruiter's calendar
  already has an interview at 10:00, do not propose 10:30 — leave
  ≥ 15 minutes buffer.
- **`next_action` is informational** — proposing slots is
  autonomous-safe; the actual booking happens when the candidate
  replies and the operator approves
  `advance_to_phone_screen`.
- **If `recruiter_calendar.timezone` is missing** — flag
  `missing_calendar_metadata` and escalate. Never guess a TZ.
