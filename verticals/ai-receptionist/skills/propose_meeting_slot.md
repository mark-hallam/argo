---
name: propose_meeting_slot
description: Given a caller + requested attendees + topic, propose 3 candidate meeting slots from the calendar-availability cache, summarize any conflicts, and draft a confirmation message to the caller. NEVER books — proposes only; the book_meeting action gates through human approval.
inputs:
  - caller
  - requested_attendees
  - topic
  - calendar_availability_cache
  - business_hours
outputs:
  - proposed_slots
  - conflict_summary
  - draft_confirmation_message
  - next_action
  - flags
---

# Skill: Propose Meeting Slot

Propose three concrete 30-minute slots; never book.

1. **Read calendar-availability cache.** Use cached free-busy
   data for each requested attendee. If the cache is missing
   ANY attendee, flag `missing_availability` and escalate — do
   not propose a slot you cannot verify.
2. **Three slots, three distinct business days.** Earliest is
   next business day per `business_hours`. Never propose
   outside business hours unless the caller explicitly asked
   for after-hours. Never propose three slots on the same day —
   one constraint can eliminate all three.
3. **Conflict summary.** For each proposed slot, name any
   soft conflicts (overlapping tentative holds, back-to-back
   with an existing meeting). Hard conflicts (existing
   confirmed meeting) disqualify the slot — pick a different
   one.
4. **Topic in the confirmation message.** ≤ 4 lines.
   Persona-voice ("we"). Name the topic, list the 3 slots in
   the caller's stated timezone if known, otherwise in the
   operator's timezone with the abbreviation. Close with
   "reply with your pick and we will confirm" — never "we have
   booked" or "see you Tuesday".
5. **`next_action` is always `book_meeting` with
   `requires_approval: true`.** Slot proposal is
   autonomous-safe; the actual calendar write happens only
   after the caller picks AND the operator approves.

Hard rules:

- **Never assert a booking.** The skill output must not contain
  "booked", "confirmed", "added to your calendar", or
  "calendar invite sent". Drafts only.
- **CEO / leadership conflict → escalate.** If any proposed
  slot collides with a meeting tagged `leadership` in the
  availability cache, flag
  `leadership_conflict` and escalate. The operator decides
  whether to reshuffle.
- **Buffer ≥ 15 minutes** between proposed slots and existing
  meetings. No back-to-back proposals.
- **If `requested_attendees` is empty** — flag
  `no_attendees` and escalate. A meeting with no attendees is
  ill-formed; do not propose slots blind.
