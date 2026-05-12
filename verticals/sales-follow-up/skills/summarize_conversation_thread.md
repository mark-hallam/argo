---
name: summarize_conversation_thread
description: Given a message thread with the lead (email, LinkedIn, or notes), surface objection themes, buying signals, a recommended next action, and CRM-tag suggestions. The vertical never writes to the CRM; the operator commits any tags.
inputs:
  - message_thread
  - lead_context
outputs:
  - objection_themes
  - buying_signals
  - recommended_next_action
  - crm_tag_suggestions
  - flags
---

# Skill: Summarize Conversation Thread

Distil a thread into a one-screen scan for the operator.

1. **Read the thread in order.** Group adjacent messages from
   the same author; preserve timestamps so recency is legible.
   If the thread is empty or the lead never replied, flag
   `no_reply_yet` and stop.
2. **Objection themes (≤ 3).** Cite the message line and the
   theme — "pricing concern (Msg 4)", "timing constraint
   (Msg 7)", "competitor evaluation (Msg 9)". One theme per
   underlying concern; don't double-count restatements.
3. **Buying signals (≤ 3).** Same shape: cite the message
   line. Examples: "asked about onboarding timeline",
   "looped in their head of ops", "referenced specific
   feature by name". A request to talk pricing is a signal,
   not an objection.
4. **Recommended next action.** One of:
   `send_email` (with a follow-up draft handed back to
   `draft_followup_message`), `schedule_call`,
   `loop_in_specialist`, `pause_30d`, `drop_lead`. Always
   `requires_approval: true` — the operator commits.
5. **CRM-tag suggestions.** Up to 5 tags drawn from the
   operator's voice config + thread evidence (e.g.
   `priority-q2`, `competitor-acme`, `timing-h2`). Suggestion
   only — the operator commits the tags to the CRM through
   their own workflow.

Hard rules:

- **The vertical never writes to the CRM.** No `update_crm`,
  no API call, no MCP write. The operator pastes the
  approved tags into their CRM themselves, OR a separate
  approved `update_crm` action carries them across — that
  action is gated through `[escalation].human_approval_required`.
- **Quotes, never paraphrases for objections.** The operator
  needs the lead's actual phrasing to scan; a sanitised
  paraphrase reads as second-hand and erodes trust in the
  summary.
- **No protected-class signals.** Personal context the lead
  volunteered (health, family, immigration) never appears in
  themes, signals, or tags. Acknowledge in your own thinking
  if relevant; never emit it.
- **If the thread shows the lead explicitly asked NOT to be
  contacted** — flag `explicit_do_not_contact`, recommend
  `drop_lead`, and escalate. Eval escalation #2 exercises
  this exactly.
