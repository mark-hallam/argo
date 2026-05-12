---
name: route_message_to_owner
description: Given an inbound message + sender + roster cache, recommend an owner (name + role + handle), classify urgency, and suggest a response template. Drafts only — never sends; send_reply gates through human approval.
inputs:
  - message
  - sender
  - roster
  - routing_policy
outputs:
  - recommended_owner
  - urgency_classification
  - suggested_response_template
  - next_action
  - flags
---

# Skill: Route Message To Owner

Map one inbound message (email / voicemail transcript / SMS) to
exactly one roster owner + urgency band + template.

1. **Subject-extract the message.** Pull the implicit subject in
   ≤ 8 words. If the message is too vague to extract a subject,
   flag `unclear_subject` and escalate.
2. **Recommended owner via routing_policy.** Read the
   `routing_policy` map: `intent → owner_handle`. If the subject
   maps to a policy entry, cite the roster row for that handle.
   If no policy entry matches AND the roster has no
   `topics_they_own` overlap, set
   `recommended_owner = null` and flag
   `ambiguous_owner` — escalate.
3. **Urgency classification.** Exactly one of `low`, `normal`,
   `high`, `urgent`:
   - `urgent` — caller asserts an active incident / deadline
     within 24h AND on-call coverage indicated.
   - `high` — deadline within 5 business days OR named
     stakeholder (auditor, regulator, paying customer flagged
     in roster as VIP).
   - `normal` — default for verified business contacts.
   - `low` — recruiters, vendors, generic outreach.
4. **Suggested response template.** ≤ 4 lines. Persona-voice
   ("we"). Acknowledges the message in one line, names the
   owner who will follow up + the expected window. Never
   commits the owner ("Alex will reply by 5pm" — only
   "we will route this to Alex; expect a reply within one
   business day").
5. **`next_action` is always `send_reply` with
   `requires_approval: true`.** Outbound reply is gated; the
   skill drafts and stops.

Hard rules:

- **No silent re-routing.** Even when the policy is clear, the
  skill produces a draft + recommended owner; the operator
  decides whether to send the template or forward the message
  directly.
- **VIP escalation always.** If sender matches a roster row
  tagged `vip`, set `urgency_classification` to at least
  `high` and flag `vip_sender` — never `normal` or `low`.
- **Do NOT classify urgency from sender domain alone.**
  "@important-company.com" is not a signal. Urgency comes from
  message content + roster tags, not domain prestige.
- **If the message is a complaint** — flag `complaint` and
  escalate regardless of urgency band. Complaints route to
  the operator first, never directly to the implicated owner.
