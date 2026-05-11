---
name: regulatory_brief
description: Given a jurisdiction and topic, summarize current regulations, recent changes in the last 24 months, and 3 horizon items, all primary-sourced.
inputs:
  - jurisdiction
  - topic
  - effective_date_filter
outputs:
  - current_state
  - recent_changes
  - horizon_items
  - sources_consulted
  - confidence
  - flags
---

# Skill: Regulatory Brief

For the named jurisdiction and topic, produce a regulator-facing brief
that an in-house counsel could rely on as a starting map. The output is
not legal advice; it is a sourced summary.

1. **Current state** — the operative statute, regulation, or rule
   citation. Use the official regulator URL or statute reference (e.g.
   17 CFR §240.10b-5, Regulation (EU) 2016/679 Article 32). Tier-2
   secondary commentary is acceptable as colour but cannot replace the
   primary citation.
2. **Recent changes (≤24 months)** — list each amendment, guidance
   note, enforcement action, or interpretive letter with effective
   date. Cite the regulator's own announcement, not a trade-press
   recap, unless the regulator is silent.
3. **Horizon items (next 12–24 months)** — exactly 3. Each names the
   proposing body, the current consultation / NPRM stage, the comment
   deadline if open, and a sourced view of probable adoption.
4. **Sources consulted** + **confidence** as per the system prompt.
5. **Flags** — populate when any escalation rule fires.

Hard rules:

- If the topic is a regulated industry (financial services, healthcare,
  pharma, defense, energy permitting, labor law) AND no primary
  regulator source is available, do NOT draft. Set
  `flags: ["regulated_without_primary"]` and escalate.
- Never offer a "what to do" recommendation. The brief stays
  descriptive; legal interpretation is for human counsel.
- If the request involves MNPI (pending enforcement before public
  announcement, sealed dockets, embargoed rule text), STOP, do not
  retrieve, escalate.
