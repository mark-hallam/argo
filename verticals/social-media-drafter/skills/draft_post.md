---
name: draft_post
description: Draft a single platform-native post for one of x / linkedin / instagram / facebook from a channel and topic, with parallel image prompt and hashtag suggestions.
inputs:
  - channel
  - topic
  - campaign_theme_id
outputs:
  - body
  - char_count
  - hashtags
  - image_prompt
  - voice_alignment
  - flags
---

# Skill: Draft Post

Produce one platform-native draft for the named `channel` on the
named `topic`, optionally scoped to a `campaign_theme_id` from memory.

1. **Voice load** — read `brand_voice` (do_say, dont_say, approved
   claims, signature emoji) and the last 10 entries in `recent_posts`
   for the same `channel`. Mimic rhythm, vocabulary, emoji policy.
2. **Channel constraints** — enforce the platform cap as a hard
   limit: X ≤ 280, LinkedIn ≤ 3000, Instagram ≤ 2200, Facebook ≤ 2000
   (soft). Count includes body + hashtags + visible URLs.
3. **Body** — write the post. Front-load the hook (X: first sentence;
   LI: first two lines above the fold; IG: first 125 chars). Default
   to consensus-of-tone with `brand_voice`; never deviate without
   recording the deviation in `voice_alignment`.
4. **Hashtags** — pull from `campaign_themes` and `recent_posts`
   only. No fresh tags without operator approval. IG ≤ 3, LI ≤ 2,
   X ≤ 1 (and only if the brand uses any).
5. **Image prompt** — supply a parallel prompt the design system can
   send to mission-image-generation-mcp-server. Describe subject,
   composition, palette consistent with `brand_voice`. Drafter does
   NOT generate the image.
6. **Char count** — return as integer. If `char_count` exceeds the
   platform cap, the draft is a defect — re-draft, do not return.
7. **Voice alignment** — `consensus` / `extension` / `deviation`,
   one-line justification keyed to a `brand_voice` entry.
8. **Substantiation** — every factual claim must trace to
   `brand_voice.approved_claims`, a `campaign_themes` row, or a prior
   `recent_posts` claim. Unsubstantiated claims either drop or carry
   `[unsourced — flagged for operator review]`.

If the prompt or topic hits any string in `crisis_keywords`
(case-insensitive substring), STOP, do not draft, set
`flags: ["crisis_keyword_match"]`, and escalate. Same rule for
regulated-product claims, executive quotes, customer PII, and
political content per the system prompt.
