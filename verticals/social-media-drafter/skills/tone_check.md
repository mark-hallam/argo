---
name: tone_check
description: Given a candidate draft, score voice deviation against brand_voice memory, flag specific phrases and emoji that violate dont_say, and recommend rewrites.
inputs:
  - draft_body
  - channel
  - target_voice_id
outputs:
  - voice_deviation_score
  - flagged_phrases
  - rewrite_suggestions
  - voice_alignment
  - flags
---

# Skill: Tone Check

Given a candidate draft, evaluate how closely it matches the
operator's `brand_voice` memory before the draft enters review.

1. **Load voice** — pull `brand_voice.do_say`, `brand_voice.dont_say`,
   `brand_voice.signature_emoji`, and the last 20 `recent_posts` for
   the same `channel`. If `target_voice_id` is supplied, scope to
   that voice variant; otherwise use the default brand voice.
2. **Score voice deviation** — return `voice_deviation_score` on a
   0–100 scale where 0 = indistinguishable from `recent_posts` and
   100 = entirely different brand. Score is a weighted blend of:
   - lexical: `dont_say` hits, off-vocabulary words (40%)
   - rhythm: sentence-length variance vs `recent_posts` (20%)
   - emoji: signature-emoji misuse or non-signature emoji use (15%)
   - punctuation: exclamation density, em-dash habits, casing (15%)
   - claim style: hedged-vs-confident posture vs voice norm (10%)
3. **Flagged phrases** — explicit list. For each: the phrase, the
   rule it violated (`dont_say` exact hit / off-vocabulary / wrong
   register), and a one-line rewrite suggestion.
4. **Rewrite suggestions** — up to 3 rewrites of the worst-scoring
   sentence in the draft, each preserving the claim but moving the
   tone toward consensus with `brand_voice`.
5. **Voice alignment** — final call: `consensus` (≤ 15),
   `extension` (16–35), or `deviation` (≥ 36). Drafts with
   `deviation` should not ship without operator sign-off.

If the draft contains a substring match against `crisis_keywords`,
do NOT score — return `flags: ["crisis_keyword_match"]` and route
to escalation. Same rule for executive quotes, customer PII, and
regulated-product claims.

Tone-check is a read-only skill — no external mutation, no draft
side effects. It informs the operator's choice on whether to ship
the draft as-is, rewrite, or kill.
