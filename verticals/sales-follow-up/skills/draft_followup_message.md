---
name: draft_followup_message
description: Given a lead context + a researched signal, produce a personalised first-touch follow-up draft, alternate tone variants (warm / professional / brief), a suggested subject line, and an explicit CAN-SPAM unsubscribe-line inclusion flag. Drafts only — never sends. CAN-SPAM Act + GDPR + UK-PECR compliance load-bearing.
inputs:
  - lead_context
  - signal
  - messaging_voice
outputs:
  - primary_draft
  - tone_variants
  - suggested_subject_line
  - can_spam_unsubscribe_line_included
  - flags
---

# Skill: Draft Follow-up Message

Produce one personalised follow-up draft + three tone variants.

1. **Open with the signal, not the pitch.** First sentence
   names the signal in plain language ("I saw you announced
   the Series B last week"). Never lead with "Just checking
   in" or "Hope you're well" — those are placeholder
   generators that signal "this is templated."
2. **One paragraph, four lines max.** ≤ 90-second read. The
   draft cites the signal, names one concrete relevance hook,
   and proposes one soft next step ("worth a 10-minute chat
   next week?"). No calendar links in a first follow-up.
3. **Subject line is direct.** "Following up on <topic>"
   beats "Quick question". Never clickbait. Never ALL CAPS.
4. **Three tone variants.**
   - `warm` — first-person, conversational, signals fluency
     with the lead's domain.
   - `professional` — neutral register, no first names if
     the operator's voice profile prefers titles.
   - `brief` — under 50 words, one signal-line + one CTA,
     for senior leads who scan in 5 seconds.
5. **Voice and forbidden phrases.** Apply
   `messaging_voice.tone_config` and reject any variant that
   contains a `messaging_voice.forbidden_phrases` entry
   (e.g. "limited time," "act now," "guaranteed"). If every
   variant trips the filter, flag `forbidden_phrase_violation`
   and escalate — eval escalation #3 exercises this.

Compliance:

- **CAN-SPAM Act (US).** Every draft includes a physical
  mailing address line and a one-click unsubscribe instruction.
  Set `can_spam_unsubscribe_line_included = true` only when
  both are present in the body.
- **GDPR (EU) + UK-PECR.** If `compliance_register` for the
  lead's jurisdiction shows `opt_out_registry_present = true`
  AND the lead is not on the operator's verified opt-in list,
  flag `gdpr_opt_out_required` and stop — never draft for a
  lead who hasn't consented.
- **Output is `primary_draft` + variants only.** The
  `next_action` is always `send_email` with
  `requires_approval: true` — the operator approves before any
  inbox is touched.
- **No commitments not in the operator brief.** Pricing,
  discounts, custom terms — none belong in a first follow-up
  unless the operator's brief explicitly says so.
