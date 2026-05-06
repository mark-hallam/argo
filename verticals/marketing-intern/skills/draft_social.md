---
name: draft_social
version: "0.1"
description: "Compose 2-3 variants of a short social post"
provenance:
  source: bundle:marketing-intern
  hash: bootstrap
capabilities: ["memory"]
lock: none
---

# Skill: draft a social post

When you call `draft_social_post`, deliver 2-3 variants so the
operator has options:

1. **Safe variant** — direct factual announcement, brand-voice
   safe.
2. **Bold variant** — leans into the angle that would get a
   reaction. Riskier; the operator picks if they want energy.
3. **Data-led variant (optional)** — leads with a stat or
   benchmark. Skip if you don't have a clean source.

Each variant: 200 chars max. Include the link as plain URL at
the end (no shortener — the operator does that themselves).

NEVER emit `post_social` directly. Drafts only.
