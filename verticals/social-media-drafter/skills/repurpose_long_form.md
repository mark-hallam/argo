---
name: repurpose_long_form
description: Given a long-form article URL, produce a 5-piece pack of platform-native posts (X thread, LinkedIn post, Instagram caption, Facebook post, plus a quote-card prompt) that adapt — not just resize — the source.
inputs:
  - source_url
  - campaign_theme_id
  - allowed_channels
outputs:
  - pack
  - sources_consulted
  - voice_alignment
  - flags
---

# Skill: Repurpose Long Form

Given a long-form article URL (or memory-referenced doc id), produce
a pack of platform-native posts that adapt the source for each
channel's affordances — never a copy-paste excerpt.

1. **Source ingest** — pull the article. Record the URL, the title,
   the author byline, and the publication date in `sources_consulted`.
   If the article cites stats, retain the original citation chain so
   the resulting posts can substantiate each claim.
2. **Pack composition** — produce one of each for the channels in
   `allowed_channels`:
   - **X thread** — 4–8 tweets, each ≤ 280 chars, numbered `1/n`. The
     first tweet is the hook; the last is the takeaway + source link.
   - **LinkedIn post** — single post ≤ 3000 chars; aim 1300–2000.
     First two lines above the fold = the argument.
   - **Instagram caption** — ≤ 2200 chars; first 125 chars = preview
     hook; finish with up to 3 `campaign_themes` hashtags.
   - **Facebook post** — ≤ 2000 chars; conversational lift, one
     question prompt at the end.
   - **Quote-card image prompt** — a parallel image-prompt suitable
     for the design system: one pulled-quote (≤ 18 words) +
     palette/composition notes consistent with `brand_voice`.
3. **Adapt, don't resize** — each piece earns its channel. A
   LinkedIn post with no first-person framing is a defect; an X
   thread with the LinkedIn intro pasted into tweet 1 is a defect.
4. **Voice alignment** — record `consensus` / `extension` / `deviation`
   for the pack as a whole, with one-line justification per channel
   if it deviates.
5. **Claim substantiation** — every factual claim in the pack traces
   to a citation in the source article or to `brand_voice.approved_claims`.
   Surface unsubstantiated claims with `[unsourced — flagged for
   operator review]` rather than silently restating.

If the source article triggers a `crisis_keywords` substring match,
contains executive quotes the brand has not vetted, or makes a
regulated-product claim — STOP, return the pack empty with
`flags` set, and escalate.
