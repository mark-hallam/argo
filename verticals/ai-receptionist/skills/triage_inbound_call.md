---
name: triage_inbound_call
description: Given a caller context + transcript snippet, classify the call intent (booking / question / complaint / spam), pick a suggested routing target from the roster, decide escalation, and draft a callback message. Drafts only — never books, sends, or transfers.
inputs:
  - caller_context
  - transcript_snippet
  - roster
  - business_hours
outputs:
  - intent_classification
  - suggested_routing_target
  - escalation
  - draft_callback_message
  - flags
---

# Skill: Triage Inbound Call

Classify one inbound call from a pre-transcribed snippet + caller
context and produce a drafted callback.

1. **Read the transcript snippet + caller context.** If both are
   missing, set `flags: ["insufficient_context"]` and stop — never
   guess intent from a phone number alone.
2. **Pick one intent.** Exactly one of `booking`, `question`,
   `complaint`, or `spam`. If the snippet supports two intents
   equally, choose the higher-stakes one (complaint > question;
   booking > question) and flag `intent_ambiguous`.
3. **Suggested routing target.** Pull a roster row whose
   `topics_they_own` overlaps the call subject. Cite the row's
   `name + role + handle` — never invent a teammate. If no
   roster row matches, set `suggested_routing_target = null` and
   flag `no_owner_match`.
4. **Escalation Y/N.** Set `escalation = true` when ANY of:
   - intent = `complaint`
   - caller requests customer data disclosure (PII, account
     details, payment info)
   - business hours are closed AND the caller asserts urgency
   - `intent_ambiguous` is flagged
5. **Draft callback message.** ≤ 4 lines, persona-voice ("we"),
   names the call subject in one line, proposes a concrete next
   step (callback window or owner intro). Never echo PII from
   the snippet back to the caller. Never claim the call was
   "logged" or "handled" — operator review pending.

Hard rules:

- **Spam → no callback.** When `intent = spam`, return an empty
  `draft_callback_message` and set
  `suggested_routing_target = null`. Spam never gets a roster
  introduction.
- **No autonomous transfer.** This skill drafts only. The
  `transfer_call` action lives behind
  `[escalation].human_approval_required` in `vertical.toml`.
- **PII-soft caller history.** If `caller_history` carries a
  prior-interaction summary, summarize it as initials + topic
  ("J.L. — last week, demo request"). Never echo full name +
  interaction reason together in the draft.
- **Voice-only intent classification is OUT of scope.** The
  vertical accepts pre-transcribed text only. If
  `transcript_snippet` is empty, escalate — do not infer from
  caller name alone.
