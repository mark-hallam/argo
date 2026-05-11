---
name: respond_to_mention
description: Given an inbound brand mention or reply thread, draft a candidate response gated on reply_to_mention approval. Never sends. Auto-escalates on crisis-keyword match, sentiment red flags, or regulated-claim territory.
inputs:
  - mention_thread
  - channel
  - sentiment
outputs:
  - draft_response
  - char_count
  - voice_alignment
  - escalation_reason
  - flags
---

# Skill: Respond To Mention

Given an inbound mention or reply thread, draft a candidate response.
This skill ALWAYS routes through `reply_to_mention` approval before
the response leaves the worker; never auto-send.

1. **Triage** — classify the thread as one of `praise`, `question`,
   `complaint`, `crisis`, `spam`. Record `sentiment` if it differs
   from the inbound input.
2. **Crisis-keyword scan (load-bearing)** — substring-match the
   thread body against `crisis_keywords` (case-insensitive). On any
   match, do NOT draft. Return `escalation_reason: crisis_keyword_match`,
   set `flags: ["crisis_keyword_match"]`, and route to human.
3. **Draft response** — only for `praise`, `question`, and
   `complaint` non-crisis cases. Voice: consensus-of-tone with
   `brand_voice`. Length: X ≤ 280, LinkedIn ≤ 3000, Instagram
   ≤ 2200, Facebook ≤ 2000. Keep replies materially shorter than
   the cap — replies are not posts.
4. **De-escalate, don't engage** — for `complaint`: acknowledge,
   offer a private channel (DM / support email from
   `brand_voice.support_handles`), do not argue in public. For
   `praise`: short, sincere, in-voice. For `question`: answer only
   if the answer is in `brand_voice.approved_claims` or
   `recent_posts`; otherwise offer to take it offline.
5. **Voice alignment** — `consensus` / `extension` / `deviation`
   with justification.
6. **Char count** — integer, must be ≤ channel cap.

Approval-gating note: even when the draft looks send-ready, the
output is a draft. The `reply_to_mention` action is gated in
`[escalation].human_approval_required` and explicitly absent from
`auto_approval.wake_action_types`. The operator's approval call is
the only path to send.

Escalation triggers (return `escalation_reason` + flags, do not draft):

- `crisis_keyword_match`
- `executive_quote_requested` — thread demands a CEO/CFO/founder reply
- `customer_pii_in_thread` — thread surfaces account-specific PII
- `regulated_claim_requested` — health / financial / legal advice
- `political_content` — election / partisan / advocacy
