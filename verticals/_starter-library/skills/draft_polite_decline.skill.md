---
name: draft_polite_decline
version: "0.1.0"
description: "Drafts a courteous, professional rejection of a request — clear about the no, warm about the relationship, brief about the reason."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator needs to say no — to a meeting invite, a sales pitch,
a feature request, a vendor proposal, a partnership ask — and wants
a draft that closes the loop without burning the relationship.

# Inputs

- `request`: what is being declined (1-2 sentences from the operator).
- `reason` (optional): the honest "why" — capacity, scope, timing,
  fit. Omit if the operator prefers a soft "not right now."
- `tone`: `"formal"`, `"warm"`, `"neutral"`. Default: `"warm"`.
- `door_open`: bool — does the decliner want to leave a future
  possibility open? Default: false.

# Output

A 2-4 sentence draft ready to send. Structure: acknowledgement →
clear no → brief reason (if supplied) → optional future-door.

# Examples

- Request `<vendor demo invite>`, reason `<scope/timing>`, door_open
  true → "Thanks for reaching out — this isn't a fit for us right
  now given <reason>. Happy to revisit when <condition>."
- Request `<feature ask from customer>`, no reason, tone formal →
  "Thank you for the suggestion. After review, we won't be able
  to take this on. We appreciate you flagging it."

# Approval posture

The skill drafts; the operator sends. Treat the output as a draft
that may be edited before delivery. Never auto-send.

# Notes

- No "unfortunately." It signals weakness and lengthens the reply.
- Never apologise more than once.
- If `reason` is sensitive (e.g. "budget constraints"), soften to
  a category ("priorities for this quarter") unless explicitly told
  to keep it specific.
- Match the recipient's register: a formal vendor email gets a
  formal decline; a Slack DM gets a casual one.
