---
name: schedule_intro_call
description: Given investor + founder calendar constraints, propose 3 meeting slot options phrased in the founder's voice. Drafts proposal text only; calendar invites route through send_email_external approval.
inputs:
  - investor_id
  - founder_calendar_constraints
  - investor_timezone_hint
  - meeting_duration_minutes
outputs:
  - proposed_slots
  - draft_message
  - _confidence
---

# Skill: Schedule Intro Call

Propose three meeting slots and a short message that lands the
meeting without making the investor pick from a long list.

1. **Three slots, not five.** Three options is the sweet spot:
   enough choice to land a meeting, few enough that the investor
   doesn't stall on a long picker. Two options reads as
   take-it-or-leave-it; five reads as logistics tax.
2. **Spread across days.** One slot in the next 2 business days
   (responsive signal), one mid-week, one early the following week.
   All within a 7–10 business-day window unless the founder's
   constraints force wider.
3. **Match the investor's timezone.** Use `investor_timezone_hint`
   if supplied; otherwise infer from the firm's HQ city and offer in
   that zone first, with founder's zone in parentheses. Never assume
   both parties are in the same zone.
4. **Avoid the standard traps.** No early-morning slots before 9am
   investor-local; no Friday-afternoon-pre-holiday slots; respect
   any blocked windows in `founder_calendar_constraints`.
5. **Buffer realistic.** Default 30 minutes for first calls, 45 for
   partner-meeting prep, 60 only when the investor has asked for it.
   Add 15-min buffer either side; never propose back-to-back from a
   prior known meeting on the founder's side.
6. **Voice-matched message.** Short, two sentences. Founder's voice
   per `founder_voice_samples`. End by offering to send a calendar
   invite once the investor picks — but the invite send itself routes
   through `send_email_external` approval.
7. **No automatic calendar holds.** Do NOT create tentative holds on
   either party's calendar from this skill. Slot proposal is text
   only until the founder approves the send and the investor confirms.

Output shape:

```json
{
  "proposed_slots": [
    {"start_iso": "2026-05-14T10:00:00-04:00", "duration_min": 30, "label": "Thu May 14, 10:00 ET"},
    {"start_iso": "2026-05-16T14:30:00-04:00", "duration_min": 30, "label": "Mon May 18 wkly basis"},
    {"start_iso": "2026-05-20T09:00:00-04:00", "duration_min": 30, "label": "Wed May 20, 09:00 ET"}
  ],
  "draft_message": "Hi Pat, would any of these work for a 30-min intro? Thu 10:00 ET, Mon 14:30 ET, or Wed 09:00 ET. Happy to send an invite once you pick.",
  "_confidence": {"timezone_inferred": 0.85},
  "_escalations": ["send_email_external"]
}
```
