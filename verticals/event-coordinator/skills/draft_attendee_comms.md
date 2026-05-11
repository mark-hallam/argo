---
name: draft_attendee_comms
description: Given an event stage and attendee context, draft pre-event, pre-deadline, on-day, and post-event communications with replacement-slot tokens for operator-filled fields.
inputs:
  - event_id
  - event_stage
  - attendee_segment
outputs:
  - comms_drafts
  - replacement_slots
  - approval_checklist
  - escalation_flags
---

# Skill: Draft Attendee Comms

Event-stage in; templated comms drafts out with replacement-slot
tokens (e.g. `{{event_name}}`, `{{rsvp_url}}`) for fields the operator
must fill before send. Sending is a `send_attendee_email_blast`
action — gated through escalation.

1. **Stage routing.** Emit only the drafts requested by `event_stage`:
   - `pre_event` — save-the-date + invite with RSVP link.
   - `pre_deadline` — RSVP reminder + agenda preview.
   - `on_day` — arrival logistics, parking, on-site contact.
   - `post_event` — thank-you, materials link, feedback survey.
2. **VIP-attendee check.** If `attendee_segment` includes any
   attendee with a populated `vip_tier` field in
   `attendee_preferences_history`, flag `vip_attendee_named` and
   name the attendees. Draft still ships for operator review.
3. **Tone matching.** Pull voice from prior approved comms on the
   same event_id. If none, default to concise, declarative, no hedging.
4. **Slot inventory.** Every draft emits a `replacement_slots` list
   naming every token used. Operator fills before send.
5. **Never widen recipients.** The draft targets the named
   `attendee_segment` only.
6. **Approval block.** End with `APPROVAL_REQUIRED:
   send_attendee_email_blast` naming recipient count and subject.

Output shape:

```json
{
  "comms_drafts": [
    {"stage": "pre_event", "subject": "Save the date — {{event_name}}",
     "body": "Hello {{first_name}}, …",
     "replacement_slots": ["event_name", "first_name", "rsvp_url"]}
  ],
  "replacement_slots": ["event_name", "first_name", "rsvp_url", "venue_address"],
  "approval_checklist": [
    {"action": "send_attendee_email_blast", "recipient_count": 87,
     "subject": "Save the date — {{event_name}}"}
  ],
  "escalation_flags": []
}
```
