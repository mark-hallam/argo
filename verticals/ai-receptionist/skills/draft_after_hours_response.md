---
name: draft_after_hours_response
description: Given an inbound message + business-hours policy, produce an on-brand acknowledgement, name the next available business window, and flag an escalation path if the caller asserts urgency. Drafts only — never sends.
inputs:
  - message
  - sender
  - business_hours
  - on_call_handle
outputs:
  - acknowledgement_draft
  - next_available_time
  - escalation_path
  - next_action
  - flags
---

# Skill: Draft After-Hours Response

Compose an out-of-hours acknowledgement that is honest about
when the operator will see the message + escalation routing if
the caller flags urgency.

1. **Confirm out-of-hours.** Compare message timestamp to
   `business_hours` (per-weekday open/close + holiday
   overrides). If the message arrived inside business hours,
   set `flags: ["in_hours_misroute"]` and escalate — this skill
   is for after-hours only.
2. **Compute `next_available_time`.** Find the next
   business-hours open per `business_hours`. Render in the
   sender's timezone if extractable from the message, otherwise
   in the operator's timezone with the abbreviation (e.g.
   "Monday 9:00am PT").
3. **Acknowledgement draft.** ≤ 4 lines. Persona-voice ("we").
   Acknowledge receipt in one line, name `next_available_time`
   in one line, close with the escalation path if applicable.
   Never say "we have looked into this" or "your issue is
   resolved" — the operator has not seen it yet.
4. **Escalation path on urgency.**
   - If the message asserts an active incident / safety issue /
     payment failure / regulatory deadline within 24h AND
     `on_call_handle` is set: name the on-call handle in the
     draft + suggested escalation channel.
   - If urgency is asserted but `on_call_handle` is null:
     flag `urgency_without_on_call` and escalate the draft
     to the operator instead of replying.
5. **`next_action` is always `send_reply` with
   `requires_approval: true`.** The acknowledgement is a draft;
   the operator approves before it goes out.

Hard rules:

- **No false promises.** Never include a specific reply time
  ("we will reply by 8am"); the operator may not be at their
  desk. Use windowed language ("we will respond during our
  next business window").
- **No autonomous reply.** This skill drafts and stops. The
  `send_reply` action lives behind
  `[escalation].human_approval_required` in `vertical.toml`.
- **PII-soft.** Do not echo the sender's full name back in the
  acknowledgement if the message includes account / payment /
  health details — switch to a neutral greeting ("Hi —") and flag
  `pii_soft_redaction`.
- **Holiday awareness.** If `next_available_time` falls on a
  declared holiday, advance to the next open day; don't quote a
  closed-for-holiday window.
