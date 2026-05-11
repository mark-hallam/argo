---
name: draft_followup
description: Given last-interaction context for an investor thread, produce a one-paragraph follow-up in the founder's voice ending in one clear ask. Drafts only — never sends; routes through send_email_external approval.
inputs:
  - investor_id
  - last_interaction_summary
  - founder_voice_samples_ref
  - ask_kind
outputs:
  - draft_subject
  - draft_body
  - ask
  - voice_match_notes
---

# Skill: Draft Follow-up

Write a follow-up email to an investor thread. Drafts only — output
is for founder review and routes through `send_email_external` for
any actual send.

1. **Read voice samples.** Load at least two `founder_voice_samples`
   entries before drafting. Note opening pattern, sentence length,
   contraction use, sign-off, and any quirks (no em-dashes, prefers
   first-name only, etc.). If fewer than two samples exist, surface
   that to the founder and offer two variants instead of one draft.
2. **One paragraph.** Body is one paragraph. Three to six sentences.
   Longer follow-ups read like asks-disguised-as-updates and
   underperform in investor inboxes.
3. **One specific update.** Cite one concrete development since the
   last contact: a customer win, a hire, a product milestone, a
   metric. The update must come from the latest fundraise update or
   from material the founder has shared publicly. Do not invent.
4. **One clear ask.** Close with one specific ask:
   - "next call in 2 weeks?" — for re-engaging
   - "intro to [name] at [portfolio co] for a reference?"
   - "happy to send the updated deck — okay?" (deck-share itself
     stays gated, but proposing it is fine)
   - "any blockers I can address before the partnership meeting?"
   Never bundle two asks. Two asks halves the response rate.
5. **Avoid AI-tells.** No "I hope this finds you well", no "exciting
   opportunity", no "delve", no "synergies", no em-dashes as a
   stylistic crutch. Read the draft back: would the founder send
   this exact sentence?
6. **Subject line.** Short. Threaded replies should reuse the
   existing subject. New threads: action-oriented, no clickbait.

Output shape:

```json
{
  "draft_subject": "Re: Acme x [founder co] — quick update",
  "draft_body": "Hi Pat, quick update on us since we last spoke. ...",
  "ask": "next call in 2 weeks?",
  "voice_match_notes": "Matched terse-with-numbers register from sample #2; no contractions; no em-dashes; first-name only sign-off.",
  "_voice_samples_used": ["sample_2026_03", "sample_2026_04"],
  "_escalations": ["send_email_external"]
}
```
