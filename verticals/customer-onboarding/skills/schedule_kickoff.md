---
name: schedule_kickoff
description: Given a new account profile and the operator's calendar availability, propose three 60-minute kickoff-call slots inside the first 5 business days post-signature. Drafting only — calendar writes are approval-gated.
inputs:
  - account_profile
  - operator_calendar
  - operator_timezone
  - kickoff_policy
outputs:
  - proposed_slots
  - engagement_summary
  - next_action
  - flags
---

# Skill: Schedule Kickoff (Day 0–5)

Propose three candidate kickoff-call slots for the named account.
The skill never books a slot; the operator confirms one and the
calendar write runs as an approval-gated downstream action.

1. **Window.** Anchor on `account_profile.contract_signed_at` and
   walk forward 5 business days in `operator_timezone`. Skip
   weekends and any date marked in `kickoff_policy.holidays`. If
   the resulting window contains fewer than three business days,
   set `flags: ["short_window"]` and propose what fits — never
   silently bleed into day 6+.
2. **Slot length.** 60 minutes. Use
   `kickoff_policy.working_hours` (default 09:00–17:00 in
   `operator_timezone`). Slots must end at or before the
   working-hours cutoff; never propose a 16:30 start.
3. **Three slots.** Spread across at least two distinct business
   days where possible (morning + afternoon + a different-day
   morning beats three back-to-back same-day slots). Score each
   slot against `operator_calendar` busy blocks; drop any slot
   that overlaps a busy block by ≥ 1 minute.
4. **Buyer-side hints.** If `account_profile.timezone` differs
   from `operator_timezone`, surface each slot in both
   timezones in `proposed_slots[*].buyer_local`. Do not pick
   slots that fall outside the buyer's 08:00–18:00 local window.
5. **Output.** `proposed_slots` is a JSON array of exactly three
   (or fewer, if flagged) objects: `start_at`, `end_at`,
   `operator_local`, `buyer_local`, `rationale` (one short line).
   `next_action` is `book_kickoff_call` with
   `requires_approval: true`.

Hard rules:

- **No same-day kickoffs.** Minimum offset is +1 business day
  from `contract_signed_at`. Same-day signals operator
  overreach.
- **Never write to the calendar.** This skill drafts slots
  only. The `book_kickoff_call` action is approval-gated and
  runs as a separate connector call downstream.
- **Conflict transparency.** If `operator_calendar` is missing
  or stale, set `flags: ["calendar_unavailable"]` and propose
  slots from `kickoff_policy.default_blocks` with a clear note
  that the operator must verify availability before approval.
- **No double-booking buyer.** If `account_profile.buyer_busy`
  is provided, treat it like the operator's busy blocks.
