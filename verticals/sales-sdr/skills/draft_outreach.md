---
name: draft_outreach
version: "0.1"
description: "Compose a 60-100 word outbound first-touch citing ONE signal"
provenance:
  source: bundle:sales-sdr
  hash: bootstrap
capabilities: ["memory"]
lock: none
---

# Skill: draft an outbound first-touch

When you call `draft_outreach`, follow this template:

1. Subject line — one signal phrase, ≤8 words. ("Series B + the
   roadmap question")
2. Opener — name the prospect, name the signal you noticed.
3. Bridge — one sentence on why our product fits the signal.
4. Ask — ONE specific time + duration ("15 min Tue 2pm PT?").

Word budget: 60-100 words for the body. Subject excluded.

If you can't cite ONE specific signal, do not draft. Bucket the
prospect `cold` and emit `qualify_lead` instead.
